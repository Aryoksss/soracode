//! Slash-command execution.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::{
    command, describe_creds, describe_dashboard, describe_sessions, describe_targets,
    describe_tools, describe_vulns, App, Line2, Palette,
};
use crate::agent::AgentEvent;
use crate::llm::Client;

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
                    self.model = r.model.clone();
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

impl App {
    /// Handle keys while the palette is open.
    pub(super) fn handle_palette_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        let items = match &self.palette.models {
            Some(models) => models.len(),
            None => command::matching(&self.input).len(),
        };

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.palette = Palette::closed();
                self.input.clear();
            }
            (KeyCode::Up, _) => {
                self.palette.selected = self.palette.selected.saturating_sub(1);
            }
            (KeyCode::Down, _) => {
                if items > 0 && self.palette.selected + 1 < items {
                    self.palette.selected += 1;
                }
            }
            (KeyCode::Enter, _) | (KeyCode::Tab, _) => {
                self.accept_palette(on_submit);
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return true,
            (KeyCode::Char(c), _) => {
                self.input.push(c);
                self.palette.models = None;
                self.refresh_palette();
            }
            (KeyCode::Backspace, _) => {
                self.input.pop();
                self.palette.models = None;
                self.refresh_palette();
            }
            _ => {}
        }
        self.quit
    }

    /// Act on the highlighted palette entry.
    fn accept_palette<F>(&mut self, on_submit: &mut F)
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        if let Some(models) = self.palette.models.take() {
            if let Some(name) = models.get(self.palette.selected) {
                self.switch_model(name);
            }
            self.palette = Palette::closed();
            self.input.clear();
            return;
        }

        let matched = command::matching(&self.input);
        if let Some(cmd) = matched.get(self.palette.selected) {
            let name = cmd.name;
            self.palette = Palette::closed();
            self.input.clear();
            self.run_slash(name, on_submit);
        }
    }
}
