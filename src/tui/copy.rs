//! Clipboard copying through the terminal (OSC 52).

use super::{App, Line2};

impl App {
    /// Text for the copy binding: the latest assistant turn, or the whole
    /// transcript when nothing has been answered yet.
    pub(super) fn copy_text(&self) -> String {
        for l in self.transcript.iter().rev() {
            if let Line2::Assistant(t) = l {
                return t.clone();
            }
        }
        self.transcript
            .iter()
            .filter_map(|l| match l {
                Line2::User(t) => Some(format!("you: {t}")),
                Line2::Assistant(t) => Some(t.clone()),
                Line2::Error(t) => Some(format!("error: {t}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Copy to the terminal's clipboard with OSC 52, which works over SSH and
    /// needs no native clipboard library. Payload is base64 per the spec.
    pub(super) fn copy_last(&mut self) {
        let text = self.copy_text();
        if text.is_empty() {
            self.status = "nothing to copy".into();
            return;
        }
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use std::io::Write as _;
        let mut out = std::io::stdout();
        let _ = write!(out, "\x1b]52;c;{}\x07", STANDARD.encode(text.as_bytes()));
        let _ = out.flush();
        self.status = "copied to clipboard".into();
    }
}
