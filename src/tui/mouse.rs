//! Mouse handling: wheel scrolling and sidebar click-to-resume.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::App;

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
            MouseEventKind::Down(MouseButton::Left) => self.click_at(m.column, m.row),
            _ => {}
        }
    }

    fn click_at(&mut self, col: u16, row: u16) {
        let a = self.sidebar_area;
        let inside = col >= a.x
            && col < a.x.saturating_add(a.width)
            && row >= a.y
            && row < a.y.saturating_add(a.height);
        if !inside {
            self.sidebar_focus = false;
            return;
        }
        let Some(offset) = row.checked_sub(a.y).and_then(|r| r.checked_sub(1)) else {
            return;
        };
        let index = offset as usize;
        if index >= self.sessions.len() {
            return;
        }
        self.sidebar_index = index;
        if let Some(id) = self.resume_selected() {
            self.status = format!("resumed session {id}");
        }
        self.sidebar_focus = false;
    }
}
