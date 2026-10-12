use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

use super::{theme, App};
use crate::session::Session;

mod render;

/// Rebuild the transcript pane from a session's entries, for resume.
pub(crate) fn rebuild(session: &Session) -> Vec<Line2> {
    let Ok(entries) = session.entries() else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter_map(|e| {
            let content = e.content;
            if content.trim().is_empty() {
                return None;
            }
            Some(match e.role.as_str() {
                "user" => Line2::User(content),
                "tool" => Line2::Tool(content),
                _ => Line2::Assistant(content),
            })
        })
        .collect()
}

pub(crate) fn load(session: &super::SharedSession) -> (String, Vec<Line2>) {
    let guard = session.lock().unwrap();
    (guard.id().to_string(), rebuild(&guard))
}

/// A line of conversation shown in the transcript pane.
#[derive(Clone)]
pub(crate) enum Line2 {
    User(String),
    Assistant(String),
    Tool(String),
    Heading(String),
    /// A finding line: `(badge, text)` where the badge picks the color.
    Severity(String, String),
    /// One changed line in a file diff, rendered by its leading `+`/`-`.
    Diff(String),
    /// A tool result line, rendered under the call with a `⎿` gutter.
    Result(String),
    Info(String),
    Error(String),
}

impl App {
    pub(super) fn draw_transcript(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        render::collect_lines(
            &mut lines,
            &self.transcript,
            &self.thinking,
            self.show_thinking,
            &self.current_stream,
        );

        self.apply_scroll(area);
        let para = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, area);
        self.draw_scrollbar(f, area);
    }

    fn apply_scroll(&mut self, area: Rect) {
        let height = area.height;
        let total = self.content_height(area).min(u16::MAX as usize) as u16;
        let max_scroll = total.saturating_sub(height);
        if self.auto_scroll {
            self.scroll = max_scroll;
        } else {
            self.scroll = self.scroll.min(max_scroll);
        }
    }

    /// Rendered line count after wrapping to the pane width.
    fn content_height(&self, area: Rect) -> usize {
        let width = area.width.max(1) as usize;
        self.rendered_lines(width)
    }

    fn rendered_lines(&self, width: usize) -> usize {
        let mut lines: Vec<Line> = Vec::new();
        render::collect_lines(
            &mut lines,
            &self.transcript,
            &self.thinking,
            self.show_thinking,
            &self.current_stream,
        );
        lines
            .iter()
            .map(|l| {
                let w: usize = l.spans.iter().map(|s| s.content.chars().count()).sum();
                if w == 0 {
                    1
                } else {
                    w.div_ceil(width)
                }
            })
            .sum()
    }

    /// Drawn only when content overflows the viewport.
    fn draw_scrollbar(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let content = self.content_height(area);
        let view = area.height as usize;
        if content <= view {
            return;
        }
        let bar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("▐")
            .style(Style::default().fg(theme::text_dim()))
            .thumb_style(Style::default().fg(theme::accent()));
        let mut state = ScrollbarState::new(content)
            .position(self.scroll as usize)
            .viewport_content_length(view);
        f.render_stateful_widget(bar, area, &mut state);
    }
}

#[cfg(test)]
mod tests;
