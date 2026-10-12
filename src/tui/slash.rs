//! Slash-command execution.

use tokio::sync::mpsc::UnboundedSender;

use super::{command, describe_sessions, describe_tools, App, Line2, Palette};
use crate::agent::AgentEvent;
use crate::llm::Client;

impl App {
    /// Execute a slash command by name.
    pub(super) fn run_slash<F>(&mut self, name: &str, _on_submit: &mut F)
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
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
                self.transcript.push(Line2::Info(
                    "  keys: Enter send · ↑/↓ history · Ctrl-U clear · PgUp/PgDn scroll · Ctrl-C quit"
                        .into(),
                ));
                self.status = "help".into();
            }
            command::Action::ShowTools => {
                self.transcript.push(Line2::Tool(describe_tools()));
                self.status = "tools".into();
            }
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
            Ok(r) => match Client::new(
                &r.base_url,
                &r.api_key,
                &r.model,
                r.max_tokens,
                r.temperature,
                r.timeout_secs,
                r.headers.clone(),
            ) {
                Ok(c) => {
                    *self.client.lock().expect("client mutex") = c;
                    self.transcript
                        .push(Line2::Info(format!("model → {name} ({})", r.model)));
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
