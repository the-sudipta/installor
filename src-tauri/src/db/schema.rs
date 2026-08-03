use rusqlite::Connection;

pub const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS catalog (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, publisher TEXT, category TEXT,
    platforms TEXT, source_type TEXT NOT NULL, source_id TEXT, download_url TEXT,
    installer_type TEXT, silent_args TEXT, checksum_sha256 TEXT, tags TEXT, notes TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS installations (
    id TEXT PRIMARY KEY, software_id TEXT NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    machine_id TEXT NOT NULL, machine_label TEXT, installed INTEGER NOT NULL DEFAULT 0,
    installed_version TEXT, installed_path TEXT, installer_file_path TEXT,
    last_verified_at TEXT, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT, op_type TEXT NOT NULL,
    payload_json TEXT NOT NULL, created_at TEXT NOT NULL, retries INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#;

pub fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_accepts_and_reads_every_table() {
        let db = Connection::open_in_memory().unwrap();
        create_schema(&db).unwrap();
        db.execute("INSERT INTO catalog VALUES (?1,'Installor','','','windows','custom','','','other','','','','',?2,?2)", ("c1", "2026-08-03T00:00:00Z")).unwrap();
        db.execute(
            "INSERT INTO installations VALUES ('i1','c1','m1','PC',1,'1.0','','','',?1)",
            ["2026-08-03T00:00:00Z"],
        )
        .unwrap();
        db.execute("INSERT INTO sync_queue(op_type,payload_json,created_at) VALUES ('upsert_catalog','{}',?1)", ["2026-08-03T00:00:00Z"]).unwrap();
        db.execute("INSERT INTO settings VALUES ('machine_label','PC')", [])
            .unwrap();
        for table in ["catalog", "installations", "sync_queue", "settings"] {
            let count: i64 = db
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1, "{table}");
        }
    }
}
