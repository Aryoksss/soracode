//! Palette key handling: navigation, filtering and acceptance.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::{command, App, Palette};
use crate::agent::AgentEvent;

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
