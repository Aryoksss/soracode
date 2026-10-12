//! Palette key handling: navigation, filtering and acceptance.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::{command, App};
use crate::agent::AgentEvent;

/// State of the slash-command palette.
pub(super) struct Palette {
    pub(super) open: bool,
    pub(super) selected: usize,
    /// `Some` while picking a model: the names offered.
    pub(super) models: Option<Vec<String>>,
    /// `Some` while picking a file for an `@` mention: the paths offered.
    pub(super) files: Option<Vec<String>>,
}

impl Palette {
    pub(super) fn closed() -> Self {
        Self {
            open: false,
            selected: 0,
            models: None,
            files: None,
        }
    }
}
impl App {
    /// Handle keys while the palette is open.
    pub(super) fn handle_palette_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        let items = match (&self.palette.models, &self.palette.files) {
            (Some(models), _) => models.len(),
            (_, Some(files)) => files.len(),
            _ => command::matching(&self.input).len(),
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
            (KeyCode::Enter, _) => {
                self.accept_palette(on_submit);
            }
            (KeyCode::Tab, _) => {
                self.complete_palette();
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

    /// Insert the highlighted command into the input, leaving it editable.
    fn complete_palette(&mut self) {
        if self.palette.models.is_some() || self.palette.files.is_some() {
            return;
        }
        if let Some(cmd) = command::matching(&self.input).get(self.palette.selected) {
            self.input = format!("/{} ", cmd.name);
            self.palette = Palette::closed();
        }
    }

    /// Act on the highlighted palette entry.
    fn accept_palette<F>(&mut self, on_submit: &mut F)
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        if let Some(files) = self.palette.files.take() {
            if let Some(path) = files.get(self.palette.selected) {
                if let Some(start) = self.input.rfind('@') {
                    self.input.truncate(start);
                    self.input.push('@');
                    self.input.push_str(path);
                    self.input.push(' ');
                }
            }
            self.palette = Palette::closed();
            self.refresh_palette();
            return;
        }

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
