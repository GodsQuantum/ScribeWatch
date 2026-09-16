use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Database {
    inner: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let connection =
            Connection::open(path).with_context(|| format!("open {}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("set private permissions on {}", path.display()))?;
        }
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        migrate(&connection)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    pub fn upsert<T: Serialize>(&self, kind: &str, id: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.conn().execute(
            "INSERT INTO sw_objects(kind,id,json,updated_at) VALUES(?1,?2,?3,unixepoch()) ON CONFLICT(kind,id) DO UPDATE SET json=excluded.json,updated_at=excluded.updated_at",
            params![kind, id, json],
        )?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, kind: &str, id: &str) -> Result<Option<T>> {
        let json: Option<String> = self
            .conn()
            .query_row(
                "SELECT json FROM sw_objects WHERE kind=?1 AND id=?2",
                params![kind, id],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }

    pub fn list<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT json FROM sw_objects WHERE kind=?1 ORDER BY updated_at DESC,id")?;
        let rows = stmt.query_map([kind], |row| row.get::<_, String>(0))?;
        let mut values = Vec::new();
        for row in rows {
            values.push(serde_json::from_str(&row?)?);
        }
        Ok(values)
    }

    pub fn delete(&self, kind: &str, id: &str) -> Result<bool> {
        Ok(self.conn().execute(
            "DELETE FROM sw_objects WHERE kind=?1 AND id=?2",
            params![kind, id],
        )? > 0)
    }

    pub fn seen(&self, workflow_id: &str, path: &str, size: u64, mtime_ns: i128) -> Result<bool> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM sw_seen WHERE workflow_id=?1 AND path=?2 AND size=?3 AND mtime_ns=?4",
            params![workflow_id, path, i64::try_from(size).unwrap_or(i64::MAX), i64::try_from(mtime_ns).unwrap_or(i64::MAX)],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_seen(
        &self,
        workflow_id: &str,
        path: &str,
        size: u64,
        mtime_ns: i128,
    ) -> Result<()> {
        self.conn().execute(
            "INSERT OR IGNORE INTO sw_seen(workflow_id,path,size,mtime_ns) VALUES(?1,?2,?3,?4)",
            params![
                workflow_id,
                path,
                i64::try_from(size).unwrap_or(i64::MAX),
                i64::try_from(mtime_ns).unwrap_or(i64::MAX)
            ],
        )?;
        Ok(())
    }
}

fn migrate(connection: &Connection) -> Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 1 {
        connection.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE IF NOT EXISTS sw_objects (
              kind TEXT NOT NULL,
              id TEXT NOT NULL,
              json TEXT NOT NULL,
              updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
              PRIMARY KEY(kind,id)
            );
            CREATE TABLE IF NOT EXISTS sw_seen (
              workflow_id TEXT NOT NULL,
              path TEXT NOT NULL,
              size INTEGER NOT NULL,
              mtime_ns INTEGER NOT NULL,
              PRIMARY KEY(workflow_id,path,size,mtime_ns)
            );
            CREATE INDEX IF NOT EXISTS sw_objects_kind_updated ON sw_objects(kind,updated_at DESC);
            PRAGMA user_version=1;
            COMMIT;
            "#,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_survives_database_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db.sqlite3");
        {
            let db = Database::open(&path).unwrap();
            db.mark_seen("w", "/watch/a.m4a", 12, 34).unwrap();
            assert!(db.seen("w", "/watch/a.m4a", 12, 34).unwrap());
        }
        let db = Database::open(&path).unwrap();
        assert!(db.seen("w", "/watch/a.m4a", 12, 34).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn database_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private.sqlite3");
        let _db = Database::open(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
