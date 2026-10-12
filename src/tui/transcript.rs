use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use super::markdown::push_markdown;
use super::{theme, App};

/// A line of conversation shown in the transcript pane.
#[derive(Clone)]
pub(crate) enum Line2 {
    User(String),
    Assistant(String),
    Tool(String),
    Heading(String),
    /// A finding line: `(badge, text)` where the badge picks the color.
    Severity(String, String),
    Info(String),
    Error(String),
}

impl App {
    pub(super) fn draw_transcript(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        self.collect_lines(&mut lines);

        self.apply_scroll(area);
        let para = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::border()))
                    .title(self.transcript_title()),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, area);
        self.draw_scrollbar(f, area);
    }

    /// Build the transcript's rendered lines from session state.
    fn collect_lines(&self, lines: &mut Vec<Line>) {
        for l in &self.transcript {
            match l {
                Line2::User(t) => lines.push(Line::from(vec![
                    Span::styled("▌ ", Style::default().fg(theme::role_user())),
                    Span::styled(
                        "you ",
                        Style::default()
                            .fg(theme::role_user())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t.clone(), Style::default().fg(theme::text())),
                ])),
                Line2::Assistant(t) => {
                    lines.push(Line::from(Span::styled(
                        "⏺ soracode",
                        Style::default()
                            .fg(theme::role_assistant())
                            .add_modifier(Modifier::BOLD),
                    )));
                    push_markdown(lines, t, theme::text());
                }
                Line2::Tool(t) => push_markdown(lines, t, theme::role_tool()),
                Line2::Heading(t) => lines.push(Line::from(Span::styled(
                    format!("▍ {t}"),
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ))),
                Line2::Severity(badge, text) => lines.push(Line::from(vec![
                    Span::styled("  ● ", Style::default().fg(theme::severity(badge))),
                    Span::styled(
                        format!("{badge:<8} "),
                        Style::default()
                            .fg(theme::severity(badge))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(text.clone(), Style::default().fg(theme::text_muted())),
                ])),
                Line2::Info(t) => {
                    for sub in t.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {sub}"),
                            Style::default().fg(theme::text_muted()),
                        )));
                    }
                }
                Line2::Error(t) => {
                    for sub in t.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {sub}"),
                            Style::default().fg(theme::error()),
                        )));
                    }
                }
            }
        }

        if self.show_thinking && !self.thinking.is_empty() {
            lines.push(Line::from(Span::styled(
                "  ┌ thinking".to_string(),
                Style::default().fg(theme::warning()),
            )));
            for sub in self.thinking.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  │ {sub}"),
                    Style::default().fg(theme::text_muted()),
                )));
            }
            lines.push(Line::from(Span::styled(
                "  └".to_string(),
                Style::default().fg(theme::warning()),
            )));
        }

        if !self.current_stream.is_empty() {
            push_markdown(lines, &self.current_stream, theme::text());
            if let Some(last) = lines.last_mut() {
                last.spans
                    .push(Span::styled("▏", Style::default().fg(theme::accent())));
            }
        }
    }

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

    /// Transcript top border: wordmark, session id, and active model.
    pub(super) fn transcript_title(&self) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                " soracode ",
                Style::default()
                    .fg(theme::bg())
                    .bg(theme::accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{} ", self.session_id),
                Style::default().fg(theme::text_dim()),
            ),
            Span::styled("· ", Style::default().fg(theme::text_dim())),
            Span::styled(
                format!("{} ", self.model),
                Style::default().fg(theme::text_muted()),
            ),
        ])
    }

    fn apply_scroll(&mut self, area: Rect) {
        let height = area.height.saturating_sub(2);
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
        let width = area.width.saturating_sub(2).max(1) as usize;
        self.rendered_lines(width)
    }

    fn rendered_lines(&self, width: usize) -> usize {
        let mut lines: Vec<Line> = Vec::new();
        self.collect_lines(&mut lines);
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
        let view = area.height.saturating_sub(2) as usize;
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
