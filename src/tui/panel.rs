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
pub(super) fn describe_vulns(session: &SharedSession) -> Vec<String> {
    let Some(e) = engagement(session) else {
        return vec!["could not open engagement state".into()];
    };
    let vulns = e.vulns().unwrap_or_default();
    let mut out = vec![format!("vulnerabilities ({})", vulns.len())];
    for v in vulns {
        out.push(format!(
            "  [{}] {} — {} ({})",
            v.severity, v.title, v.status, v.host
        ));
    }
    out
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
