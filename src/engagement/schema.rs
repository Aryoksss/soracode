//! Engagement table definitions, created in the session database.

use rusqlite::Connection;

use crate::error::Result;

/// Create the engagement tables if they do not exist. Every row is scoped by
/// `session_id` so one database can hold several independent engagements.
pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS eng_hosts (
             session_id TEXT NOT NULL,
             addr       TEXT NOT NULL,
             hostname   TEXT,
             os         TEXT,
             PRIMARY KEY (session_id, addr)
         );
         CREATE TABLE IF NOT EXISTS eng_services (
             session_id TEXT NOT NULL,
             host       TEXT NOT NULL,
             proto      TEXT NOT NULL,
             port       INTEGER NOT NULL,
             name       TEXT,
             version    TEXT,
             PRIMARY KEY (session_id, host, proto, port)
         );
         CREATE TABLE IF NOT EXISTS eng_vulns (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             session_id TEXT NOT NULL,
             host       TEXT NOT NULL,
             title      TEXT NOT NULL,
             severity   TEXT NOT NULL,
             status     TEXT NOT NULL,
             evidence   TEXT NOT NULL DEFAULT ''
         );
         CREATE TABLE IF NOT EXISTS eng_creds (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             session_id TEXT NOT NULL,
             username   TEXT NOT NULL,
             secret     TEXT NOT NULL,
             kind       TEXT NOT NULL,
             domain     TEXT
         );
         CREATE TABLE IF NOT EXISTS eng_access (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             session_id TEXT NOT NULL,
             host       TEXT NOT NULL,
             method     TEXT NOT NULL,
             level      TEXT NOT NULL,
             username   TEXT
         );
         CREATE TABLE IF NOT EXISTS eng_edges (
             session_id TEXT NOT NULL,
             from_node  TEXT NOT NULL,
             kind       TEXT NOT NULL,
             to_node    TEXT NOT NULL,
             PRIMARY KEY (session_id, from_node, kind, to_node)
         );",
    )?;
    Ok(())
}
