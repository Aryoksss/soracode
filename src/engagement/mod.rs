//! Shared engagement state for offensive work: hosts, services, findings,
//! credentials and the relationships between them.

mod render;
mod schema;
mod store;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A discovered host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    /// IP address or hostname used as the identity key.
    pub addr: String,
    /// Resolved hostname, when known.
    pub hostname: Option<String>,
    /// Operating system guess.
    pub os: Option<String>,
}

/// An open port and the service listening on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Host address this service belongs to.
    pub host: String,
    /// Transport: `tcp` or `udp`.
    pub proto: String,
    /// Port number.
    pub port: u16,
    /// Service name, e.g. `http`.
    pub name: Option<String>,
    /// Product / version banner.
    pub version: Option<String>,
}

/// A recorded vulnerability or finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vuln {
    /// Host the finding applies to, or empty for network-wide findings.
    pub host: String,
    /// Short title.
    pub title: String,
    /// Severity: `info`, `low`, `medium`, `high` or `critical`.
    pub severity: String,
    /// Lifecycle: `suspected`, `confirmed` or `exploited`.
    pub status: String,
    /// Evidence backing the finding.
    pub evidence: String,
}

/// A discovered credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cred {
    /// Username.
    pub username: String,
    /// Password, hash or token.
    pub secret: String,
    /// Secret kind: `password`, `hash` or `token`.
    pub kind: String,
    /// Authentication domain, when applicable.
    pub domain: Option<String>,
}

/// A confirmed foothold on a host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Access {
    /// Host address.
    pub host: String,
    /// Access method, e.g. `winrm` or `ssh`.
    pub method: String,
    /// Privilege level, e.g. `user` or `root`.
    pub level: String,
    /// Account that granted the access.
    pub username: Option<String>,
}

/// A directed relationship in the entity graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source node.
    pub from: String,
    /// Relationship kind, e.g. `EXPLOITED_VIA`.
    pub kind: String,
    /// Destination node.
    pub to: String,
}

/// The engagement store, shared across agents and backed by the session
/// database so state survives a restart.
#[derive(Clone)]
pub struct Engagement {
    inner: Arc<Mutex<Connection>>,
    session_id: String,
}

impl Engagement {
    /// Build the engagement tables on an existing session connection.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn from_conn(conn: &Arc<Mutex<Connection>>, session_id: &str) -> Result<Self> {
        schema::create(&conn.lock().unwrap_or_else(|e| e.into_inner()))?;
        Ok(Self {
            inner: Arc::clone(conn),
            session_id: session_id.to_string(),
        })
    }

    /// Build an isolated in-memory engagement, for tests and previews.
    ///
    /// # Errors
    /// Propagates database errors.
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::create(&conn)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            session_id: "memory".into(),
        })
    }

    pub(super) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests;
