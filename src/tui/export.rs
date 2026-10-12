//! `/export` — write the session to a Markdown file.

use super::{App, Line2};
use crate::session::Entry;

/// Render session entries as a Markdown transcript.
pub(super) fn to_markdown(session_id: &str, model: &str, entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# soracode session `{session_id}`\n\n"));
    out.push_str(&format!("model: `{model}`\n\n---\n\n"));
    for e in entries {
        match e.role.as_str() {
            "user" => out.push_str(&format!("## you\n\n{}\n\n", e.content)),
            "assistant" => out.push_str(&format!("## assistant\n\n{}\n\n", e.content)),
            "tool" => {
                let name = e.tool_name.as_deref().unwrap_or("tool");
                out.push_str(&format!("```text\n[{name}] {}\n```\n\n", e.content));
            }
            other => out.push_str(&format!("### {other}\n\n{}\n\n", e.content)),
        }
    }
    out
}

impl App {
    /// Export the whole session to `arg`, or a default file beside the cwd.
    pub(super) fn export(&mut self, arg: &str, default_name: &str) {
        let entries = match self.shared_session.lock() {
            Ok(s) => match s.entries() {
                Ok(e) => e,
                Err(err) => {
                    self.transcript
                        .push(Line2::Error(format!("export failed: {err}")));
                    self.status = "export failed".into();
                    return;
                }
            },
            Err(_) => {
                self.status = "export failed".into();
                return;
            }
        };
        if entries.is_empty() {
            self.status = "nothing to export".into();
            self.transcript
                .push(Line2::Info("nothing to export yet".into()));
            return;
        }
        let md = to_markdown(&self.session_id, &self.model, &entries);
        let path = if arg.is_empty() {
            std::path::PathBuf::from(default_name)
        } else {
            std::path::PathBuf::from(arg)
        };
        match std::fs::write(&path, md) {
            Ok(()) => {
                self.transcript.push(Line2::Info(format!(
                    "exported {} entries to {}",
                    entries.len(),
                    path.display()
                )));
                self.status = "exported".into();
            }
            Err(e) => {
                self.transcript
                    .push(Line2::Error(format!("export failed: {e}")));
                self.status = "export failed".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::to_markdown;
    use crate::session::Entry;

    fn entry(seq: i64, role: &str, content: &str, tool: Option<&str>) -> Entry {
        Entry {
            seq,
            role: role.into(),
            content: content.into(),
            tool_name: tool.map(str::to_string),
            ts: 0,
        }
    }

    #[test]
    fn renders_a_heading_and_each_role() {
        let entries = vec![
            entry(1, "user", "scan it", None),
            entry(2, "assistant", "ok", None),
            entry(3, "tool", "open 80", Some("nmap_parse")),
        ];
        let md = to_markdown("abc123", "openai/gpt-4.1", &entries);
        assert!(md.starts_with("# soracode session `abc123`\n"));
        assert!(md.contains("model: `openai/gpt-4.1`"));
        assert!(md.contains("## you\n\nscan it"));
        assert!(md.contains("## assistant\n\nok"));
        assert!(md.contains("[nmap_parse] open 80"));
    }

    #[test]
    fn an_unknown_role_still_renders() {
        let md = to_markdown("s", "m", &[entry(1, "system", "boot", None)]);
        assert!(md.contains("### system\n\nboot"));
    }
}
