use crate::{db::Database, error::InstallorResult, models::*, sync::sheets_client::SheetsClient};
use std::collections::HashSet;

pub async fn sync_now(db: &Database, client: &SheetsClient) -> InstallorResult<SyncResult> {
    let remote = client.list().await?;
    let local = db.list_bundle()?;
    let mut report = SyncReport::default();
    let queued = db.queued()?;
    let pending_delete_ids = |op_type: &str| -> InstallorResult<HashSet<String>> {
        queued
            .iter()
            .filter(|(_, op, _)| op == op_type)
            .map(|(_, _, payload)| {
                Ok(serde_json::from_str::<serde_json::Value>(payload)?["id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string())
            })
            .collect()
    };
    let pending_catalog_deletes = pending_delete_ids("delete_catalog")?;
    let pending_installation_deletes = pending_delete_ids("delete_installation")?;
    let mut remote_catalog_deletes_won = HashSet::new();
    let mut remote_installation_deletes_won = HashSet::new();
    for marker in remote.deleted_installations {
        if local
            .installations
            .iter()
            .find(|row| row.id == marker.id)
            .is_some_and(|row| row.updated_at <= marker.deleted_at)
        {
            db.delete_installation(&marker.id, false)?;
            report.removed_installations += 1;
            remote_installation_deletes_won.insert(marker.id);
        }
    }
    for marker in remote.deleted_catalog {
        if local
            .catalog
            .iter()
            .find(|row| row.id == marker.id)
            .is_some_and(|row| row.updated_at <= marker.deleted_at)
        {
            db.delete_catalog(&marker.id, false)?;
            report.removed_catalog += 1;
            remote_catalog_deletes_won.insert(marker.id);
        }
    }
    for row in remote.catalog {
        if pending_catalog_deletes.contains(&row.id) {
            continue;
        }
        let accept = local
            .catalog
            .iter()
            .find(|l| l.id == row.id)
            .is_none_or(|l| row.updated_at > l.updated_at);
        if accept {
            db.upsert_catalog(&row, false)?;
            report.pulled_catalog += 1;
        }
    }
    for row in remote.installations {
        if pending_installation_deletes.contains(&row.id)
            || pending_catalog_deletes.contains(&row.software_id)
        {
            continue;
        }
        let accept = local
            .installations
            .iter()
            .find(|l| l.id == row.id)
            .is_none_or(|l| row.updated_at > l.updated_at);
        if accept && db.get_catalog(&row.software_id).is_ok() {
            db.upsert_installation(&row, false)?;
            report.pulled_installations += 1;
        }
    }
    for (id, op, payload) in queued {
        let superseded_by_remote_deletion = match op.as_str() {
            "upsert_catalog" => remote_catalog_deletes_won
                .contains(&serde_json::from_str::<CatalogEntry>(&payload)?.id),
            "upsert_installation" => {
                let row = serde_json::from_str::<InstallationEntry>(&payload)?;
                remote_installation_deletes_won.contains(&row.id)
                    || remote_catalog_deletes_won.contains(&row.software_id)
            }
            _ => false,
        };
        if superseded_by_remote_deletion {
            db.remove_queue(id)?;
            continue;
        }
        let result = match op.as_str() {
            "upsert_catalog" => {
                client
                    .upsert_catalog(&serde_json::from_str(&payload)?)
                    .await
            }
            "upsert_installation" => {
                client
                    .upsert_installation(&serde_json::from_str(&payload)?)
                    .await
            }
            "delete_catalog" => {
                client
                    .delete_catalog(
                        serde_json::from_str::<serde_json::Value>(&payload)?["id"]
                            .as_str()
                            .unwrap_or_default(),
                    )
                    .await
            }
            "delete_installation" => {
                client
                    .delete_installation(
                        serde_json::from_str::<serde_json::Value>(&payload)?["id"]
                            .as_str()
                            .unwrap_or_default(),
                    )
                    .await
            }
            _ => Ok(()),
        };
        if result.is_ok() {
            match op.as_str() {
                "upsert_catalog" => report.pushed_catalog += 1,
                "upsert_installation" => report.pushed_installations += 1,
                "delete_catalog" | "delete_installation" => report.pushed_deletions += 1,
                _ => {}
            }
            db.remove_queue(id)?;
        } else {
            result?;
        }
    }
    report.pending_after = db.pending_count()?;
    report.last_synced_at = now_rfc3339();
    db.set_setting("last_synced_at", &report.last_synced_at)?;
    Ok(SyncResult {
        bundle: db.list_bundle()?,
        report,
    })
}

pub fn import_bundle(db: &Database, bundle: CatalogBundle) -> InstallorResult<CatalogBundle> {
    for row in bundle.catalog {
        let accept = db.get_catalog(&row.id).is_err()
            || db
                .get_catalog(&row.id)
                .is_ok_and(|l| row.updated_at >= l.updated_at);
        if accept {
            db.upsert_catalog(&row, false)?;
        }
    }
    for row in bundle.installations {
        if db.get_catalog(&row.software_id).is_ok() {
            db.upsert_installation(&row, false)?;
        }
    }
    db.list_bundle()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_partial_json, method, query_param},
    };

    fn row(id: &str, name: &str, updated: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.into(),
            name: name.into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated.into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn merge_newer_remote_pushes_newer_local_and_retries_offline_queue() {
        let server = MockServer::start().await;
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        db.upsert_catalog(&row("remote", "Old local", "2026-01-01T00:00:00Z"), false)
            .unwrap();
        db.upsert_catalog(&row("local", "New local", "2026-04-01T00:00:00Z"), true)
            .unwrap();
        let remote = CatalogBundle {
            catalog: vec![
                row("remote", "New remote", "2026-05-01T00:00:00Z"),
                row("local", "Old remote", "2026-02-01T00:00:00Z"),
            ],
            installations: vec![],
            ..Default::default()
        };
        Mock::given(method("GET"))
            .and(query_param("action", "list"))
            .and(query_param("token", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&remote))
            .mount(&server)
            .await;
        let client = SheetsClient::new(server.uri(), "secret".into());
        assert!(
            sync_now(&db, &client).await.is_err(),
            "unmocked push simulates offline/failure"
        );
        assert_eq!(db.get_catalog("remote").unwrap().name, "New remote");
        assert_eq!(db.get_catalog("local").unwrap().name, "New local");
        assert_eq!(db.queued().unwrap().len(), 1, "failed push stays queued");
        Mock::given(method("POST"))
            .and(body_partial_json(
                serde_json::json!({"action":"upsert_catalog"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server)
            .await;
        sync_now(&db, &client).await.unwrap();
        assert!(
            db.queued().unwrap().is_empty(),
            "successful retry flushes queue"
        );
    }

    #[tokio::test]
    async fn pending_local_removal_is_not_resurrected_by_remote_list() {
        let server = MockServer::start().await;
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        let deleted = row("deleted", "Remote copy", "2026-05-01T00:00:00Z");
        db.upsert_catalog(&deleted, false).unwrap();
        db.delete_catalog("deleted", true).unwrap();
        Mock::given(method("GET"))
            .and(query_param("action", "list"))
            .and(query_param("token", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(CatalogBundle {
                catalog: vec![deleted],
                ..Default::default()
            }))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(body_partial_json(
                serde_json::json!({"action":"delete_catalog"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server)
            .await;
        let result = sync_now(&db, &SheetsClient::new(server.uri(), "secret".into()))
            .await
            .unwrap();
        assert!(result.bundle.catalog.is_empty());
        assert_eq!(result.report.pushed_deletions, 1);
    }

    #[tokio::test]
    async fn newer_remote_removal_discards_an_older_queued_upsert() {
        let server = MockServer::start().await;
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        db.upsert_catalog(
            &row("superseded", "Old queued copy", "2026-01-01T00:00:00Z"),
            true,
        )
        .unwrap();
        Mock::given(method("GET"))
            .and(query_param("action", "list"))
            .and(query_param("token", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(CatalogBundle {
                deleted_catalog: vec![DeleteMarker {
                    id: "superseded".into(),
                    deleted_at: "2026-05-01T00:00:00Z".into(),
                }],
                ..Default::default()
            }))
            .mount(&server)
            .await;
        let result = sync_now(&db, &SheetsClient::new(server.uri(), "secret".into()))
            .await
            .unwrap();
        assert!(result.bundle.catalog.is_empty());
        assert!(db.queued().unwrap().is_empty());
        assert_eq!(result.report.removed_catalog, 1);
        assert_eq!(result.report.pushed_catalog, 0);
    }
}
