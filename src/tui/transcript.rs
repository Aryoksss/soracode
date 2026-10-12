use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::markdown::push_markdown;
use super::{theme, App, Line2};

impl App {
    pub(super) fn draw_transcript(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for l in &self.transcript {
            match l {
                Line2::User(t) => lines.push(Line::from(vec![
                    Span::styled("▌ ", Style::default().fg(theme::INTERACTIVE)),
                    Span::styled(
                        "you ",
                        Style::default()
                            .fg(theme::INTERACTIVE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(t.clone(), Style::default().fg(theme::TEXT)),
                ])),
                Line2::Assistant(t) => push_markdown(&mut lines, t, theme::TEXT),
                Line2::Tool(t) => push_markdown(&mut lines, t, theme::WARNING),
                Line2::Info(t) => {
                    for sub in t.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {sub}"),
                            Style::default().fg(theme::TEXT_MUTED),
                        )));
                    }
                }
                Line2::Error(t) => {
                    for sub in t.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {sub}"),
                            Style::default().fg(theme::ERROR),
                        )));
                    }
                }
            }
        }

        if self.show_thinking && !self.thinking.is_empty() {
            lines.push(Line::from(Span::styled(
                "  ┌ thinking".to_string(),
                Style::default().fg(theme::TEXT_MUTED),
            )));
            for sub in self.thinking.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  │ {sub}"),
                    Style::default().fg(theme::TEXT_MUTED),
                )));
            }
            lines.push(Line::from(Span::styled(
                "  └".to_string(),
                Style::default().fg(theme::TEXT_MUTED),
            )));
        }

        if !self.current_stream.is_empty() {
            push_markdown(&mut lines, &self.current_stream, theme::TEXT);
            if let Some(last) = lines.last_mut() {
                last.spans
                    .push(Span::styled("▏", Style::default().fg(theme::ACCENT)));
            }
        }

        self.apply_scroll(area, &lines);
        let title = Line::from(vec![
            Span::styled(
                " soracode ",
                Style::default().fg(theme::BG).bg(theme::ACCENT),
            ),
            Span::styled(
                " security execution engine ",
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]);
        let para = Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER))
                    .title(title),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, area);
    }

    fn apply_scroll(&mut self, area: Rect, lines: &[Line]) {
        let height = area.height.saturating_sub(2);
        let width = area.width.saturating_sub(2).max(1) as usize;
        let rendered: usize = lines
            .iter()
            .map(|l| {
                let w: usize = l.spans.iter().map(|s| s.content.chars().count()).sum();
                if w == 0 {
                    1
                } else {
                    w.div_ceil(width)
                }
            })
            .sum();
        let total = rendered.min(u16::MAX as usize) as u16;
        let max_scroll = total.saturating_sub(height);
        if self.auto_scroll {
            self.scroll = max_scroll;
        } else {
            self.scroll = self.scroll.min(max_scroll);
        }
    }
}
