pub mod schema;

use crate::{error::InstallorResult, models::*};
use rusqlite::{Connection, params};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Clone, Debug)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> InstallorResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let this = Self { path };
        schema::create_schema(&this.connect()?)?;
        Ok(this)
    }

    fn connect(&self) -> InstallorResult<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        Ok(conn)
    }

    pub fn list_bundle(&self) -> InstallorResult<CatalogBundle> {
        Ok(CatalogBundle {
            catalog: self.list_catalog()?,
            installations: self.list_installations()?,
            ..Default::default()
        })
    }

    pub fn list_catalog(&self) -> InstallorResult<Vec<CatalogEntry>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id,name,publisher,category,platforms,source_type,source_id,download_url,installer_type,silent_args,checksum_sha256,tags,notes,created_at,updated_at FROM catalog ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |r| {
            Ok(CatalogEntry {
                id: r.get(0)?,
                name: r.get(1)?,
                publisher: r.get(2)?,
                category: r.get(3)?,
                platforms: r.get(4)?,
                source_type: SourceType::from_str(&r.get::<_, String>(5)?).unwrap_or_default(),
                source_id: r.get(6)?,
                download_url: r.get(7)?,
                installer_type: InstallerType::from_str(&r.get::<_, String>(8)?)
                    .unwrap_or_default(),
                silent_args: r.get(9)?,
                checksum_sha256: r.get(10)?,
                tags: r.get(11)?,
                notes: r.get(12)?,
                created_at: r.get(13)?,
                updated_at: r.get(14)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_catalog(&self, id: &str) -> InstallorResult<CatalogEntry> {
        self.list_catalog()?
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| {
                crate::error::InstallorError::Operation(format!("catalog entry not found: {id}"))
            })
    }

    pub fn upsert_catalog(&self, entry: &CatalogEntry, queue: bool) -> InstallorResult<()> {
        let conn = self.connect()?;
        conn.execute(r#"INSERT INTO catalog VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
          ON CONFLICT(id) DO UPDATE SET name=excluded.name,publisher=excluded.publisher,category=excluded.category,platforms=excluded.platforms,source_type=excluded.source_type,source_id=excluded.source_id,download_url=excluded.download_url,installer_type=excluded.installer_type,silent_args=excluded.silent_args,checksum_sha256=excluded.checksum_sha256,tags=excluded.tags,notes=excluded.notes,updated_at=excluded.updated_at"#,
          params![entry.id,entry.name,entry.publisher,entry.category,entry.platforms,entry.source_type.to_string(),entry.source_id,entry.download_url,entry.installer_type.to_string(),entry.silent_args,entry.checksum_sha256,entry.tags,entry.notes,entry.created_at,entry.updated_at])?;
        if queue {
            self.queue("upsert_catalog", &serde_json::to_string(entry)?)?;
        }
        Ok(())
    }

    pub fn delete_catalog(&self, id: &str, queue: bool) -> InstallorResult<()> {
        let installation_ids: Vec<String> = self
            .list_installations()?
            .into_iter()
            .filter(|row| row.software_id == id)
            .map(|row| row.id)
            .collect();
        self.connect()?
            .execute("DELETE FROM catalog WHERE id=?1", [id])?;
        if queue {
            for installation_id in installation_ids {
                self.queue(
                    "delete_installation",
                    &serde_json::json!({"id":installation_id}).to_string(),
                )?;
            }
            self.queue("delete_catalog", &serde_json::json!({"id":id}).to_string())?;
        }
        Ok(())
    }

    pub fn delete_catalog_bulk(&self, ids: &[String], queue: bool) -> InstallorResult<usize> {
        let mut removed = 0;
        for id in ids {
            if self.get_catalog(id).is_ok() {
                self.delete_catalog(id, queue)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn list_installations(&self) -> InstallorResult<Vec<InstallationEntry>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id,software_id,machine_id,machine_label,installed,installed_version,installed_path,installer_file_path,last_verified_at,updated_at FROM installations")?;
        let rows = stmt.query_map([], |r| {
            Ok(InstallationEntry {
                id: r.get(0)?,
                software_id: r.get(1)?,
                machine_id: r.get(2)?,
                machine_label: r.get(3)?,
                installed: r.get::<_, i64>(4)? != 0,
                installed_version: r.get(5)?,
                installed_path: r.get(6)?,
                installer_file_path: r.get(7)?,
                last_verified_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn upsert_installation(&self, e: &InstallationEntry, queue: bool) -> InstallorResult<()> {
        self.connect()?.execute(r#"INSERT INTO installations VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
          ON CONFLICT(id) DO UPDATE SET software_id=excluded.software_id,machine_id=excluded.machine_id,machine_label=excluded.machine_label,installed=excluded.installed,installed_version=excluded.installed_version,installed_path=excluded.installed_path,installer_file_path=excluded.installer_file_path,last_verified_at=excluded.last_verified_at,updated_at=excluded.updated_at"#,
          params![e.id,e.software_id,e.machine_id,e.machine_label,e.installed as i64,e.installed_version,e.installed_path,e.installer_file_path,e.last_verified_at,e.updated_at])?;
        if queue {
            self.queue("upsert_installation", &serde_json::to_string(e)?)?;
        }
        Ok(())
    }

    pub fn delete_installation(&self, id: &str, queue: bool) -> InstallorResult<()> {
        self.connect()?
            .execute("DELETE FROM installations WHERE id=?1", [id])?;
        if queue {
            self.queue(
                "delete_installation",
                &serde_json::json!({"id":id}).to_string(),
            )?;
        }
        Ok(())
    }

    pub fn rename_machine(
        &self,
        machine_id: &str,
        device_name: &str,
        queue: bool,
    ) -> InstallorResult<usize> {
        let mut changed = 0;
        for mut row in self
            .list_installations()?
            .into_iter()
            .filter(|row| row.machine_id == machine_id && row.machine_label != device_name)
        {
            row.machine_label = device_name.to_string();
            row.updated_at = now_rfc3339();
            self.upsert_installation(&row, queue)?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn setting(&self, key: &str) -> InstallorResult<Option<String>> {
        let conn = self.connect()?;
        let mut s = conn.prepare("SELECT value FROM settings WHERE key=?1")?;
        let mut rows = s.query([key])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> InstallorResult<()> {
        self.connect()?.execute("INSERT INTO settings(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
        Ok(())
    }

    fn queue(&self, op: &str, payload: &str) -> InstallorResult<()> {
        self.connect()?.execute(
            "INSERT INTO sync_queue(op_type,payload_json,created_at) VALUES (?1,?2,?3)",
            params![op, payload, now_rfc3339()],
        )?;
        Ok(())
    }

    pub fn queued(&self) -> InstallorResult<Vec<(i64, String, String)>> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT id,op_type,payload_json FROM sync_queue ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn remove_queue(&self, id: i64) -> InstallorResult<()> {
        self.connect()?
            .execute("DELETE FROM sync_queue WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn pending_count(&self) -> InstallorResult<usize> {
        let count: i64 =
            self.connect()?
                .query_row("SELECT COUNT(*) FROM sync_queue", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn queue_full_library(&self) -> InstallorResult<(usize, usize)> {
        let catalog = self.list_catalog()?;
        let installations = self.list_installations()?;
        self.connect()?.execute(
            "DELETE FROM sync_queue WHERE op_type IN ('upsert_catalog','upsert_installation')",
            [],
        )?;
        for row in &catalog {
            self.queue("upsert_catalog", &serde_json::to_string(row)?)?;
        }
        for row in &installations {
            self.queue("upsert_installation", &serde_json::to_string(row)?)?;
        }
        Ok((catalog.len(), installations.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn catalog(id: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.into(),
            name: format!("App {id}"),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            ..Default::default()
        }
    }

    fn installation(id: &str, software_id: &str) -> InstallationEntry {
        InstallationEntry {
            id: id.into(),
            software_id: software_id.into(),
            machine_id: "machine".into(),
            machine_label: "Office PC".into(),
            installed: true,
            updated_at: "2026-01-01T00:00:00Z".into(),
            ..Default::default()
        }
    }

    #[test]
    fn catalog_removal_queues_installation_and_catalog_tombstones() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        db.upsert_catalog(&catalog("one"), false).unwrap();
        db.upsert_installation(&installation("install-one", "one"), false)
            .unwrap();
        db.delete_catalog("one", true).unwrap();
        let operations: Vec<String> = db
            .queued()
            .unwrap()
            .into_iter()
            .map(|(_, operation, _)| operation)
            .collect();
        assert_eq!(operations, ["delete_installation", "delete_catalog"]);
        assert!(db.list_catalog().unwrap().is_empty());
        assert!(db.list_installations().unwrap().is_empty());
    }

    #[test]
    fn full_library_queue_replaces_duplicate_upserts_but_keeps_deletions() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        db.upsert_catalog(&catalog("one"), true).unwrap();
        db.upsert_installation(&installation("install-one", "one"), true)
            .unwrap();
        db.delete_installation("already-remote", true).unwrap();
        db.queue_full_library().unwrap();
        db.queue_full_library().unwrap();
        let operations: Vec<String> = db
            .queued()
            .unwrap()
            .into_iter()
            .map(|(_, operation, _)| operation)
            .collect();
        assert_eq!(
            operations,
            [
                "delete_installation",
                "upsert_catalog",
                "upsert_installation"
            ]
        );
    }

    #[test]
    fn renaming_a_device_updates_its_existing_category_records() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.sqlite")).unwrap();
        db.upsert_catalog(&catalog("one"), false).unwrap();
        db.upsert_installation(&installation("install-one", "one"), false)
            .unwrap();
        assert_eq!(
            db.rename_machine("machine", "Research Laptop", true)
                .unwrap(),
            1
        );
        assert_eq!(
            db.list_installations().unwrap()[0].machine_label,
            "Research Laptop"
        );
        assert_eq!(db.queued().unwrap()[0].1, "upsert_installation");
    }
}
