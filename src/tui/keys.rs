//! Key handling for the interactive terminal UI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use super::{command, sidebar, App, Line2};
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

    /// Keys while the session sidebar holds focus: move the highlight, resume
    /// the highlighted session, or hand focus back to the input.
    fn handle_sidebar_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,
            (KeyCode::Esc, _) | (KeyCode::Tab, _) => self.sidebar_focus = false,
            (KeyCode::Up, _) => self.sidebar_index = self.sidebar_index.saturating_sub(1),
            (KeyCode::Down, _) => {
                if self.sidebar_index + 1 < self.sessions.len() {
                    self.sidebar_index += 1;
                }
            }
            (KeyCode::Enter, _) => {
                if let Some(id) = self.resume_selected() {
                    self.status = format!("resumed session {id}");
                }
                self.sidebar_focus = false;
            }
            _ => {}
        }
        self.quit
    }

    /// Returns true when the app should exit.
    pub(super) fn handle_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        if self.palette.open {
            return self.handle_palette_key(key, on_submit);
        }
        if self.sidebar_focus {
            return self.handle_sidebar_key(key);
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if self.busy {
                    self.cancel.notify_one();
                    self.status = "cancelling…".into();
                    return false;
                }
                return true;
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,

            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.input.clear();
                self.history_at = None;
            }

            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.show_thinking = !self.show_thinking;
                self.status = if self.show_thinking {
                    "thinking shown".into()
                } else {
                    "thinking hidden".into()
                };
            }

            (KeyCode::F(2), _) => {
                self.mouse_capture = !self.mouse_capture;
                self.status = if self.mouse_capture {
                    "mouse capture on — wheel scrolls the transcript".into()
                } else {
                    "mouse capture off — drag to select and copy".into()
                };
            }

            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.show_sidebar = !self.show_sidebar;
                if self.show_sidebar {
                    sidebar::refresh(self);
                } else {
                    self.sidebar_focus = false;
                }
            }

            (KeyCode::Tab, _) if self.show_sidebar => {
                sidebar::refresh(self);
                self.sidebar_focus = true;
            }

            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.transcript.clear();
                self.current_stream.clear();
                self.status = "transcript cleared".into();
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
    pub(super) fn refresh_palette(&mut self) {
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
}
