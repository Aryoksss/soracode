//! Transcript content for informational slash commands.

use crate::session::Session;
use crate::tui::SharedSession;

/// Render the cached host tool inventory, or a hint to scan.
pub(super) fn describe_tools() -> String {
    match crate::tools::catalog::load_cache() {
        Some(inv) => inv.render(),
        None => "no tool cache yet — run `soracode --tools --rescan`".into(),
    }
}

/// List recent sessions from the database, newest first.
pub(super) fn describe_sessions(db_path: &std::path::Path) -> Vec<String> {
    match Session::list(db_path, 10) {
        Ok(rows) if rows.is_empty() => vec!["no sessions yet".into()],
        Ok(rows) => rows
            .into_iter()
            .map(|(id, ts, summary)| {
                let when = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                format!("  {id}  {when}  {summary}")
            })
            .collect(),
        Err(e) => vec![format!("could not read sessions: {e}")],
    }
}

fn engagement(session: &SharedSession) -> Option<crate::engagement::Engagement> {
    let s = session.lock().unwrap_or_else(|e| e.into_inner()).clone();
    s.engagement().ok()
}

/// Engagement dashboard: counts plus the open session id.
pub(super) fn describe_dashboard(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    let hosts = e.hosts().map(|v| v.len()).unwrap_or(0);
    let services = e.services(None).map(|v| v.len()).unwrap_or(0);
    let vulns = e.vulns().map(|v| v.len()).unwrap_or(0);
    let creds = e.creds().map(|v| v.len()).unwrap_or(0);
    let access = e.access().map(|v| v.len()).unwrap_or(0);
    vec![
        "engagement dashboard".into(),
        format!("  session  {}", e.session_id()),
        format!("  hosts    {hosts}"),
        format!("  services {services}"),
        format!("  vulns    {vulns}"),
        format!("  creds    {creds}"),
        format!("  access   {access}"),
    ]
}

/// Discovered hosts and their services.
pub(super) fn describe_targets(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    let hosts = e.hosts().unwrap_or_default();
    let mut out = vec![format!("targets ({} hosts)", hosts.len())];
    for h in hosts {
        let host = h.hostname.as_deref().unwrap_or("-");
        let os = h.os.as_deref().unwrap_or("-");
        out.push(format!("  {} ({host}) os={os}", h.addr));
        for s in e.services(Some(&h.addr)).unwrap_or_default() {
            let name = s.name.as_deref().unwrap_or("-");
            let ver = s.version.as_deref().unwrap_or("");
            out.push(format!("    {}/{} {name} {ver}", s.port, s.proto));
        }
    }
    out
}

/// Recorded findings.
pub(super) fn describe_vulns(session: &SharedSession) -> Vec<(String, String)> {
    let Some(e) = engagement(session) else {
        return vec![("unknown".into(), "could not open engagement state".into())];
    };
    let vulns = e.vulns().unwrap_or_default();
    if vulns.is_empty() {
        return vec![("unknown".into(), "no findings recorded yet".into())];
    }
    vulns
        .into_iter()
        .map(|v| {
            (
                v.severity.to_ascii_lowercase(),
                format!("{} — {} ({})", v.title, v.status, v.host),
            )
        })
        .collect()
}

/// Discovered credentials.
pub(super) fn describe_creds(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    let creds = e.creds().unwrap_or_default();
    let mut out = vec![format!("credentials ({})", creds.len())];
    for c in creds {
        let dom = c.domain.as_deref().unwrap_or("-");
        out.push(format!(
            "  {}:{} [{}] domain={dom}",
            c.username, c.secret, c.kind
        ));
    }
    out
}

/// Every service across all hosts, collapsed into a port matrix.
pub(super) fn describe_services(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    services_matrix(&e)
}

fn services_matrix(e: &crate::engagement::Engagement) -> Vec<String> {
    let services = e.services(None).unwrap_or_default();
    let mut out = vec![format!("services ({} across all hosts)", services.len())];
    if services.is_empty() {
        out.push("  none recorded yet — parse an nmap scan with `nmap_parse`".into());
        return out;
    }
    let mut sorted = services;
    sorted.sort_by_key(|s| (s.host.clone(), s.port));
    let mut current = String::new();
    for s in sorted {
        if s.host != current {
            current.clone_from(&s.host);
            out.push(format!("  {current}"));
        }
        let name = s.name.as_deref().unwrap_or("-");
        let ver = s.version.as_deref().unwrap_or("");
        out.push(format!("    {:>5}/{:<3} {name} {ver}", s.port, s.proto));
    }
    out
}

/// The access and reachability graph: footholds plus directed edges.
pub(super) fn describe_matrix(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    access_matrix(&e)
}

fn access_matrix(e: &crate::engagement::Engagement) -> Vec<String> {
    let access = e.access().unwrap_or_default();
    let edges = e.edges().unwrap_or_default();
    let mut out = vec![format!(
        "access matrix ({} footholds, {} edges)",
        access.len(),
        edges.len()
    )];
    if access.is_empty() && edges.is_empty() {
        out.push("  nothing yet — record a foothold with `state_update` (`access`)".into());
        return out;
    }
    for a in &access {
        let user = a.username.as_deref().unwrap_or("-");
        out.push(format!("  [{}/{}] {} as {user}", a.method, a.level, a.host));
    }
    for edge in &edges {
        out.push(format!("  {} -[{}]-> {}", edge.from, edge.kind, edge.to));
    }
    out
}

#[cfg(test)]
mod panel_tests {
    use super::{access_matrix, services_matrix};
    use crate::engagement::{Access, Edge, Engagement, Host, Service};

    #[test]
    fn services_matrix_groups_by_host() {
        let e = Engagement::in_memory().unwrap();
        e.add_host(&Host {
            addr: "10.10.10.5".into(),
            hostname: Some("DC01".into()),
            os: None,
        })
        .unwrap();
        for (port, name) in [(88u16, "kerberos"), (389, "ldap")] {
            e.add_service(&Service {
                host: "10.10.10.5".into(),
                proto: "tcp".into(),
                port,
                name: Some(name.into()),
                version: None,
            })
            .unwrap();
        }
        let out = services_matrix(&e);
        assert_eq!(out[0], "services (2 across all hosts)");
        assert!(out.iter().any(|l| l.contains("10.10.10.5")));
        assert!(out
            .iter()
            .any(|l| l.contains("88") && l.contains("kerberos")));
        assert!(out.iter().any(|l| l.contains("389") && l.contains("ldap")));
    }

    #[test]
    fn services_matrix_hints_when_empty() {
        let e = Engagement::in_memory().unwrap();
        let out = services_matrix(&e);
        assert_eq!(out[0], "services (0 across all hosts)");
        assert!(out[1].contains("nmap_parse"));
    }

    #[test]
    fn access_matrix_renders_footholds_and_edges() {
        let e = Engagement::in_memory().unwrap();
        e.add_access(&Access {
            host: "10.10.10.5".into(),
            method: "smb".into(),
            level: "user".into(),
            username: Some("svc".into()),
        })
        .unwrap();
        e.add_edge(&Edge {
            from: "svc".into(),
            kind: "ADMIN_TO".into(),
            to: "10.10.10.5".into(),
        })
        .unwrap();
        let out = access_matrix(&e);
        assert_eq!(out[0], "access matrix (1 footholds, 1 edges)");
        assert!(out
            .iter()
            .any(|l| l.contains("[smb/user] 10.10.10.5 as svc")));
        assert!(out
            .iter()
            .any(|l| l.contains("svc -[ADMIN_TO]-> 10.10.10.5")));
    }

    #[test]
    fn access_matrix_hints_when_empty() {
        let e = Engagement::in_memory().unwrap();
        let out = access_matrix(&e);
        assert_eq!(out[0], "access matrix (0 footholds, 0 edges)");
        assert!(out[1].contains("state_update"));
    }
}
