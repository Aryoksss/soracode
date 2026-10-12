//! Human-readable rendering of engagement state.

use super::Engagement;
use crate::error::Result;

impl Engagement {
    /// A compact multi-section summary for the model or the operator.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn render(&self) -> Result<String> {
        let hosts = self.hosts()?;
        let services = self.services(None)?;
        let vulns = self.vulns()?;
        let creds = self.creds()?;
        let access = self.access()?;
        let edges = self.edges()?;

        if hosts.is_empty()
            && services.is_empty()
            && vulns.is_empty()
            && creds.is_empty()
            && access.is_empty()
            && edges.is_empty()
        {
            return Ok("(engagement state empty)".into());
        }

        let mut s = String::new();
        s.push_str(&format!("## Hosts ({})\n", hosts.len()));
        for h in &hosts {
            let host = h.hostname.as_deref().unwrap_or("-");
            let os = h.os.as_deref().unwrap_or("-");
            s.push_str(&format!("- {} ({host}) os={os}\n", h.addr));
        }

        s.push_str(&format!("\n## Services ({})\n", services.len()));
        for sv in &services {
            let name = sv.name.as_deref().unwrap_or("-");
            let ver = sv.version.as_deref().unwrap_or("");
            s.push_str(&format!(
                "- {} {}/{} {name} {ver}\n",
                sv.host, sv.port, sv.proto
            ));
        }

        s.push_str(&format!("\n## Vulnerabilities ({})\n", vulns.len()));
        for v in &vulns {
            s.push_str(&format!(
                "- [{}] {} — {} ({})\n",
                v.severity, v.title, v.status, v.host
            ));
        }

        s.push_str(&format!("\n## Credentials ({})\n", creds.len()));
        for c in &creds {
            let dom = c.domain.as_deref().unwrap_or("-");
            s.push_str(&format!(
                "- {}:{} [{}] domain={dom}\n",
                c.username, c.secret, c.kind
            ));
        }

        s.push_str(&format!("\n## Access ({})\n", access.len()));
        for a in &access {
            let user = a.username.as_deref().unwrap_or("-");
            s.push_str(&format!(
                "- {} via {} as {} ({user})\n",
                a.host, a.method, a.level
            ));
        }

        s.push_str(&format!("\n## Graph ({})\n", edges.len()));
        for e in &edges {
            s.push_str(&format!("- {} -[{}]-> {}\n", e.from, e.kind, e.to));
        }
        Ok(s)
    }
}
