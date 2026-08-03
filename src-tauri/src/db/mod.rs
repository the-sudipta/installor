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
        self.connect()?
            .execute("DELETE FROM catalog WHERE id=?1", [id])?;
        if queue {
            self.queue("delete_catalog", &serde_json::json!({"id":id}).to_string())?;
        }
        Ok(())
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
}
