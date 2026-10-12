//! Read and write operations over the engagement tables.

use rusqlite::params;

use super::{Access, Cred, Edge, Engagement, Host, Service, Vuln};
use crate::error::Result;

impl Engagement {
    /// Insert or update a host.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_host(&self, host: &Host) -> Result<()> {
        self.conn().execute(
            "INSERT INTO eng_hosts (session_id, addr, hostname, os)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, addr) DO UPDATE SET
                 hostname = COALESCE(excluded.hostname, hostname),
                 os = COALESCE(excluded.os, os)",
            params![self.session_id, host.addr, host.hostname, host.os],
        )?;
        Ok(())
    }

    /// Insert or update a service.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_service(&self, svc: &Service) -> Result<()> {
        self.conn().execute(
            "INSERT INTO eng_services (session_id, host, proto, port, name, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id, host, proto, port) DO UPDATE SET
                 name = COALESCE(excluded.name, name),
                 version = COALESCE(excluded.version, version)",
            params![
                self.session_id,
                svc.host,
                svc.proto,
                svc.port,
                svc.name,
                svc.version
            ],
        )?;
        Ok(())
    }

    /// Record a vulnerability.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_vuln(&self, v: &Vuln) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO eng_vulns (session_id, host, title, severity, status, evidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                self.session_id,
                v.host,
                v.title,
                v.severity,
                v.status,
                v.evidence
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Record a credential.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_cred(&self, c: &Cred) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO eng_creds (session_id, username, secret, kind, domain)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![self.session_id, c.username, c.secret, c.kind, c.domain],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Record a foothold.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_access(&self, a: &Access) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO eng_access (session_id, host, method, level, username)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![self.session_id, a.host, a.method, a.level, a.username],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Record an edge in the relationship graph.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn add_edge(&self, e: &Edge) -> Result<()> {
        self.conn().execute(
            "INSERT OR IGNORE INTO eng_edges (session_id, from_node, kind, to_node)
             VALUES (?1, ?2, ?3, ?4)",
            params![self.session_id, e.from, e.kind, e.to],
        )?;
        Ok(())
    }

    /// All hosts, ordered by address.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn hosts(&self) -> Result<Vec<Host>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT addr, hostname, os FROM eng_hosts WHERE session_id = ?1 ORDER BY addr",
        )?;
        let rows = stmt.query_map(params![self.session_id], |r| {
            Ok(Host {
                addr: r.get(0)?,
                hostname: r.get(1)?,
                os: r.get(2)?,
            })
        })?;
        collect(rows)
    }

    /// Services, optionally limited to one host.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn services(&self, host: Option<&str>) -> Result<Vec<Service>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT host, proto, port, name, version FROM eng_services
             WHERE session_id = ?1 AND (?2 IS NULL OR host = ?2)
             ORDER BY host, port",
        )?;
        let rows = stmt.query_map(params![self.session_id, host], |r| {
            Ok(Service {
                host: r.get(0)?,
                proto: r.get(1)?,
                port: r.get(2)?,
                name: r.get(3)?,
                version: r.get(4)?,
            })
        })?;
        collect(rows)
    }

    /// Findings, newest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn vulns(&self) -> Result<Vec<Vuln>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT host, title, severity, status, evidence FROM eng_vulns
             WHERE session_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map(params![self.session_id], |r| {
            Ok(Vuln {
                host: r.get(0)?,
                title: r.get(1)?,
                severity: r.get(2)?,
                status: r.get(3)?,
                evidence: r.get(4)?,
            })
        })?;
        collect(rows)
    }

    /// Credentials, newest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn creds(&self) -> Result<Vec<Cred>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT username, secret, kind, domain FROM eng_creds
             WHERE session_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map(params![self.session_id], |r| {
            Ok(Cred {
                username: r.get(0)?,
                secret: r.get(1)?,
                kind: r.get(2)?,
                domain: r.get(3)?,
            })
        })?;
        collect(rows)
    }

    /// Footholds, newest first.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn access(&self) -> Result<Vec<Access>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT host, method, level, username FROM eng_access
             WHERE session_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map(params![self.session_id], |r| {
            Ok(Access {
                host: r.get(0)?,
                method: r.get(1)?,
                level: r.get(2)?,
                username: r.get(3)?,
            })
        })?;
        collect(rows)
    }

    /// Relationship edges.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn edges(&self) -> Result<Vec<Edge>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT from_node, kind, to_node FROM eng_edges
             WHERE session_id = ?1 ORDER BY from_node, kind, to_node",
        )?;
        let rows = stmt.query_map(params![self.session_id], |r| {
            Ok(Edge {
                from: r.get(0)?,
                kind: r.get(1)?,
                to: r.get(2)?,
            })
        })?;
        collect(rows)
    }
}

fn collect<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
