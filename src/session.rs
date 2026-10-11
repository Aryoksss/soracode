//! Session persistence in SQLite: transcript, notes, resume.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::util::short_id;

/// A stored conversation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Monotonic sequence within the session.
    pub seq: i64,
    /// Role: `user`, `assistant`, `tool`, `system` or `subagent`.
    pub role: String,
    /// Text content.
    pub content: String,
    /// Tool name, for tool entries.
    pub tool_name: Option<String>,
    /// Unix epoch seconds.
    pub ts: i64,
}

/// SQLite-backed session store, safe to share across tasks.
#[derive(Clone)]
pub struct Session {
    inner: Arc<Mutex<Connection>>,
    id: String,
}

impl Session {
    /// Open (creating if needed) the database at `path` and start/attach a session.
    ///
    /// # Errors
    /// Propagates database and IO errors.
    pub fn open(path: &Path, resume: Option<&str>) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 id TEXT PRIMARY KEY,
                 created INTEGER NOT NULL,
                 cwd TEXT
             );
             CREATE TABLE IF NOT EXISTS entries (
                 session_id TEXT NOT NULL,
                 seq INTEGER NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 tool_name TEXT,
                 ts INTEGER NOT NULL,
                 PRIMARY KEY (session_id, seq)
             );
             CREATE TABLE IF NOT EXISTS notes (
                 session_id TEXT NOT NULL,
                 tag TEXT NOT NULL,
                 body TEXT NOT NULL,
                 ts INTEGER NOT NULL
             );",
        )?;

        let id = match resume {
            Some(existing) => existing.to_string(),
            None => {
                let id = short_id();
                let now = now();
                let cwd = std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                conn.execute(
                    "INSERT INTO sessions (id, created, cwd) VALUES (?1, ?2, ?3)",
                    params![id, now, cwd],
                )?;
                id
            }
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            id,
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The active session id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Append an entry, returning its sequence number.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn append(&self, role: &str, content: &str, tool_name: Option<&str>) -> Result<i64> {
        let conn = self.conn();
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM entries WHERE session_id = ?1",
                params![self.id],
                |r| r.get(0),
            )
            .map_err(|e| Error::Session(format!("seq: {e}")))?;

        conn.execute(
            "INSERT INTO entries (session_id, seq, role, content, tool_name, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![self.id, seq, role, content, tool_name, now()],
        )?;
        Ok(seq)
    }

    /// Load all entries for the session.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn entries(&self) -> Result<Vec<Entry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT seq, role, content, tool_name, ts FROM entries
             WHERE session_id = ?1 ORDER BY seq",
        )?;
        let rows = stmt.query_map(params![self.id], |r| {
            Ok(Entry {
                seq: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                tool_name: r.get(3)?,
                ts: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Persist a note.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_note(&self, tag: &str, body: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO notes (session_id, tag, body, ts) VALUES (?1, ?2, ?3, ?4)",
            params![self.id, tag, body, now()],
        )?;
        Ok(())
    }

    /// Load all notes for the session.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn notes(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT tag, body FROM notes WHERE session_id = ?1 ORDER BY ts")?;
        let rows = stmt.query_map(params![self.id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

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

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::Session;

    fn temp_db(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("soracode-session-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (dir.clone(), dir.join("s.db"))
    }

    #[test]
    fn latest_is_none_on_an_empty_database() {
        let (dir, path) = temp_db("empty");
        assert_eq!(Session::latest(&path).unwrap(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn latest_returns_the_newest_session() {
        let (dir, path) = temp_db("newest");
        let a = Session::open(&path, None).unwrap();
        a.append("user", "first", None).unwrap();
        let b = Session::open(&path, None).unwrap();

        let got = Session::latest(&path).unwrap();
        assert_eq!(got.as_deref(), Some(b.id()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
