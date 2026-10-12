//! Key handling for the interactive terminal UI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use super::{command, App, Line2, Palette};
use crate::agent::AgentEvent;

impl App {
    /// Wheel scrolling over the transcript, three lines per notch.
    pub(super) fn handle_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::ScrollUp => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_sub(3);
            }
            MouseEventKind::ScrollDown => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_add(3);
            }
            _ => {}
        }
    }

    /// Returns true when the app should exit.
    pub(super) fn handle_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        if self.palette.open {
            return self.handle_palette_key(key, on_submit);
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,

            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.input.clear();
                self.history_at = None;
            }

            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.show_thinking = !self.show_thinking;
            }

            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.show_sidebar = !self.show_sidebar;
            }

            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.transcript.clear();
                self.current_stream.clear();
            }

            (KeyCode::Enter, _) => {
                let text = self.input.trim().to_string();
                self.input.clear();
                self.history_at = None;
                if text.is_empty() {
                    return false;
                }
                if let Some(rest) = text.strip_prefix('/') {
                    self.run_slash(rest, on_submit);
                    return false;
                }
                if self.history.last() != Some(&text) {
                    self.history.push(text.clone());
                }
                self.transcript.push(Line2::User(text.clone()));
                self.status = "running…".into();
                self.auto_scroll = true;
                self.busy = true;
                self.thinking.clear();
                on_submit(text, self.tx.clone());
            }
            (KeyCode::Up, _) => {
                if !self.history.is_empty() {
                    let idx = match self.history_at {
                        Some(i) => i.saturating_sub(1),
                        None => self.history.len() - 1,
                    };
                    self.history_at = Some(idx);
                    self.input = self.history[idx].clone();
                }
            }
            (KeyCode::Down, _) => match self.history_at {
                Some(i) if i + 1 < self.history.len() => {
                    self.history_at = Some(i + 1);
                    self.input = self.history[i + 1].clone();
                }
                Some(_) => {
                    self.history_at = None;
                    self.input.clear();
                }
                None => {}
            },
            (KeyCode::Char(c), _) => {
                self.input.push(c);
                self.refresh_palette();
            }
            (KeyCode::Backspace, _) => {
                self.input.pop();
                self.refresh_palette();
            }
            (KeyCode::PageUp, _) => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_sub(5);
            }
            (KeyCode::PageDown, _) => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_add(5);
            }
            (KeyCode::Home, _) => {
                self.auto_scroll = false;
                self.scroll = 0;
            }
            (KeyCode::End, _) => self.auto_scroll = true,
            _ => {}
        }
        self.quit
    }

    /// Open the palette when the input is a bare slash prefix, else close it.
    fn refresh_palette(&mut self) {
        let is_slash = self.input.starts_with('/') && !self.input.contains(' ');
        if is_slash && !command::matching(&self.input).is_empty() {
            self.palette.open = true;
            self.palette.models = None;
            let n = command::matching(&self.input).len();
            if self.palette.selected >= n {
                self.palette.selected = 0;
            }
        } else {
            self.palette.open = false;
            self.palette.selected = 0;
        }
    }

    /// Handle keys while the palette is open.
    fn handle_palette_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
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
