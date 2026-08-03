use crate::{db::Database, error::InstallorResult, models::*, sync::sheets_client::SheetsClient};

pub async fn sync_now(db: &Database, client: &SheetsClient) -> InstallorResult<CatalogBundle> {
    let remote = client.list().await?;
    let local = db.list_bundle()?;
    for row in remote.catalog {
        let accept = local
            .catalog
            .iter()
            .find(|l| l.id == row.id)
            .is_none_or(|l| row.updated_at > l.updated_at);
        if accept {
            db.upsert_catalog(&row, false)?;
        }
    }
    for row in remote.installations {
        let accept = local
            .installations
            .iter()
            .find(|l| l.id == row.id)
            .is_none_or(|l| row.updated_at > l.updated_at);
        if accept && db.get_catalog(&row.software_id).is_ok() {
            db.upsert_installation(&row, false)?;
        }
    }
    for (id, op, payload) in db.queued()? {
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
            db.remove_queue(id)?;
        } else {
            result?;
        }
    }
    db.list_bundle()
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
}
