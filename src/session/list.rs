//! Read-only session queries that operate on a database path, not a live
//! `Session` handle. Used by `--sessions` and `--continue` before any session
//! is opened.

use std::path::Path;

use rusqlite::{params, Connection};

use super::Session;
use crate::error::Result;

impl Session {
    /// List recent sessions, newest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn list(path: &Path, limit: usize) -> Result<Vec<(String, i64, String)>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, created INTEGER NOT NULL, cwd TEXT);",
        )?;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.created, COALESCE(s.cwd,''),
                    (SELECT COUNT(*) FROM entries e WHERE e.session_id = s.id)
             FROM sessions s ORDER BY s.created DESC, s.rowid DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            let id: String = r.get(0)?;
            let created: i64 = r.get(1)?;
            let cwd: String = r.get(2)?;
            let count: i64 = r.get(3)?;
            Ok((id, created, format!("{cwd}  ({count} entries)")))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// The newest session id in the database, if any exists.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn latest(path: &Path) -> Result<Option<String>> {
        Ok(Self::list(path, 1)?.into_iter().next().map(|(id, _, _)| id))
    }
}
