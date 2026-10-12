//! Markdown engagement report assembled from the state.

use super::path::Path;
use super::Engagement;
use crate::error::Result;

impl Engagement {
    /// Render the whole engagement as a Markdown report.
    ///
    /// The report leads with the confirmed attack path (when the graph has
    /// one), then lists findings by descending severity, then credentials,
    /// footholds and the full host inventory. It is written to be pasted into
    /// `findings.md` unchanged.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn report(&self) -> Result<String> {
        let hosts = self.hosts()?;
        let services = self.services(None)?;
        let mut vulns = self.vulns()?;
        let creds = self.creds()?;
        let access = self.access()?;
        let paths = self.attack_paths(None, None)?;

        let mut s = String::new();
        s.push_str("# Engagement report\n\n");
        s.push_str(&format!("- Session: `{}`\n", self.session_id));
        s.push_str(&format!("- Hosts: {}\n", hosts.len()));
        s.push_str(&format!("- Findings: {}\n", vulns.len()));
        s.push_str(&format!("- Credentials: {}\n", creds.len()));
        s.push_str(&format!("- Footholds: {}\n", access.len()));
        s.push('\n');

        if !paths.is_empty() {
            s.push_str("## Attack paths\n\n");
            for p in paths.iter().take(10) {
                s.push_str(&format!("- {}\n", render_path(p)));
            }
            s.push('\n');
        }

        s.push_str("## Findings\n\n");
        if vulns.is_empty() {
            s.push_str("_None recorded._\n\n");
        } else {
            vulns.sort_by(|a, b| {
                severity_rank(&b.severity)
                    .cmp(&severity_rank(&a.severity))
                    .then_with(|| a.title.cmp(&b.title))
            });
            for v in &vulns {
                s.push_str(&format!(
                    "### [{}] {} ({})\n\n- Host: `{}`\n- Status: {}\n",
                    v.severity.to_uppercase(),
                    v.title,
                    v.status,
                    if v.host.is_empty() { "-" } else { &v.host },
                    v.status
                ));
                if !v.evidence.is_empty() {
                    s.push_str(&format!("- Evidence: `{}`\n", v.evidence));
                }
                s.push('\n');
            }
        }

        s.push_str("## Credentials\n\n");
        if creds.is_empty() {
            s.push_str("_None recorded._\n\n");
        } else {
            s.push_str("| user | secret | kind | domain |\n| --- | --- | --- | --- |\n");
            for c in &creds {
                s.push_str(&format!(
                    "| `{}` | `{}` | {} | {} |\n",
                    c.username,
                    c.secret,
                    c.kind,
                    c.domain.as_deref().unwrap_or("-")
                ));
            }
            s.push('\n');
        }

        s.push_str("## Footholds\n\n");
        if access.is_empty() {
            s.push_str("_None recorded._\n\n");
        } else {
            for a in &access {
                s.push_str(&format!(
                    "- `{}` via {} as {} ({})\n",
                    a.host,
                    a.method,
                    a.level,
                    a.username.as_deref().unwrap_or("-")
                ));
            }
            s.push('\n');
        }

        s.push_str("## Hosts\n\n");
        if hosts.is_empty() {
            s.push_str("_None recorded._\n");
        } else {
            for h in &hosts {
                s.push_str(&format!(
                    "- `{}` ({}) os={} — {} service(s)\n",
                    h.addr,
                    h.hostname.as_deref().unwrap_or("-"),
                    h.os.as_deref().unwrap_or("-"),
                    services.iter().filter(|sv| sv.host == h.addr).count()
                ));
            }
        }
        Ok(s)
    }
}

/// Render one path as `a -[KIND]-> b -[KIND]-> c`.
fn render_path(p: &Path) -> String {
    let mut out = String::new();
    for (i, node) in p.nodes.iter().enumerate() {
        if i > 0 {
            let kind = p.hops.get(i - 1).map(String::as_str).unwrap_or("?");
            out.push_str(&format!(" -[{kind}]-> "));
        }
        out.push_str(node);
    }
    out
}

/// Sort key so `critical` outranks `info`.
fn severity_rank(severity: &str) -> u8 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
