//! Slash-command execution.

use tokio::sync::mpsc::UnboundedSender;

use super::{
    command, describe_creds, describe_dashboard, describe_matrix, describe_services,
    describe_sessions, describe_targets, describe_tools, describe_vulns, App, Line2, Palette,
};
use crate::agent::AgentEvent;
use crate::llm::{Client, ClientConfig};

impl App {
    /// Execute a slash command by name.
    pub(super) fn run_slash<F>(&mut self, input: &str, _on_submit: &mut F)
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        let mut parts = input.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("");
        let arg = parts.next().map(str::trim).unwrap_or("");
        let Some(cmd) = command::by_name(name) else {
            self.transcript.push(Line2::Error(format!(
                "unknown command `/{name}` — type / for the list"
            )));
            return;
        };
        match cmd.action {
            command::Action::Quit => self.quit = true,
            command::Action::Clear => {
                self.transcript.clear();
                self.current_stream.clear();
                self.status = "cleared".into();
            }
            command::Action::ShowHelp => {
                for c in command::COMMANDS {
                    self.transcript
                        .push(Line2::Info(format!("  /{:<9} {}", c.name, c.desc)));
                }
                for k in [
                    "Enter send",
                    "↑/↓ history",
                    "Ctrl-U clear input",
                    "PgUp/PgDn scroll",
                    "Home/End top/newest",
                    "Tab sidebar",
                    "sidebar: click or ↑↓ then Enter to resume",
                    "sidebar: d or Delete deletes a session",
                    "Ctrl-B hide sidebar",
                    "Ctrl-T thinking",
                    "Ctrl-L clear transcript",
                    "F2 copy",
                    "Ctrl-C cancel turn / quit",
                    "Ctrl-Q quit",
                ] {
                    self.transcript.push(Line2::Info(format!("  key: {k}")));
                }
                self.status = "help".into();
            }
            command::Action::ShowTools => {
                self.transcript.push(Line2::Tool(describe_tools()));
                self.status = "tools".into();
            }
            command::Action::Status => {
                for line in describe_dashboard(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "status".into();
            }
            command::Action::Targets => {
                for line in describe_targets(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "targets".into();
            }
            command::Action::Services => {
                for line in describe_services(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "services".into();
            }
            command::Action::Matrix => {
                for line in describe_matrix(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "matrix".into();
            }
            command::Action::Vulns => {
                for line in describe_vulns(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "vulns".into();
            }
            command::Action::Creds => {
                for line in describe_creds(&self.shared_session) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "creds".into();
            }
            command::Action::Pause => {
                self.pause.store(true, std::sync::atomic::Ordering::Relaxed);
                self.transcript.push(Line2::Info(
                    "paused — the agent stops before its next tool call; /resume to continue"
                        .into(),
                ));
                self.status = "paused".into();
            }
            command::Action::Resume => {
                self.pause
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.transcript
                    .push(Line2::Info("resumed — the agent may continue".into()));
                self.status = "running".into();
            }
            command::Action::SetMode => match crate::config::Mode::parse(arg) {
                Some(m) => {
                    self.mode = m;
                    self.transcript.push(Line2::Info(format!(
                        "mode → {} (applies from the next instruction)",
                        m.as_str()
                    )));
                    self.status = format!("mode {}", m.as_str());
                }
                None => self.transcript.push(Line2::Info(format!(
                    "mode is {} — use /mode auto|guided|free",
                    self.mode.as_str()
                ))),
            },
            command::Action::ShowSessions => {
                for line in describe_sessions(&self.cfg.db_path) {
                    self.transcript.push(Line2::Info(line));
                }
                self.status = "sessions".into();
            }
            command::Action::PickModel => {
                let mut names: Vec<String> = self.cfg.models.keys().cloned().collect();
                names.sort();
                if names.is_empty() {
                    self.transcript.push(Line2::Error(
                        "no named models configured — add a models.toml".into(),
                    ));
                    return;
                }
                self.palette = Palette {
                    open: true,
                    selected: 0,
                    models: Some(names),
                };
                self.input.clear();
            }
        }
        self.auto_scroll = true;
    }

    /// Rebuild the shared client for a newly chosen model name.
    pub(super) fn switch_model(&mut self, name: &str) {
        match self.cfg.resolve(Some(name)) {
            Ok(r) => match Client::new(ClientConfig {
                base_url: r.base_url.clone(),
                wire: r.wire,
                api_key: r.api_key.clone(),
                model: r.model.clone(),
                max_tokens: r.max_tokens,
                temperature: r.temperature,
                timeout_secs: r.timeout_secs,
                max_attempts: self.cfg.llm.max_attempts,
                retry_backoff_ms: self.cfg.llm.retry_backoff_ms,
                extra: r.headers.clone(),
            }) {
                Ok(c) => {
                    let wire = c.wire();
                    *self.client.lock().expect("client mutex") = c;
                    self.model = r.model.clone();
                    self.transcript.push(Line2::Info(format!(
                        "model → {name} ({} via {})",
                        r.model,
                        wire.as_str()
                    )));
                    self.status = format!("model {name}");
                }
                Err(e) => self
                    .transcript
                    .push(Line2::Error(format!("model switch failed: {e}"))),
            },
            Err(e) => self
                .transcript
                .push(Line2::Error(format!("model switch failed: {e}"))),
        }
    }
}
