//! The hint bar under the input: model, mode, live status, token counter.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::{theme, App};

impl App {
    pub(super) fn draw_status(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let mut spans = vec![
            Span::styled(
                self.model.clone(),
                Style::default()
                    .fg(theme::accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                self.mode.as_str().to_string(),
                Style::default().fg(theme::text_dim()),
            ),
            Span::styled("  ·  ", Style::default().fg(theme::text_dim())),
        ];
        if self.busy {
            spans.extend(self.shimmer(&format!(
                "{}  working…  {}s  ·  esc to interrupt",
                self.spinner(),
                self.elapsed()
            )));
        } else if self.status.is_empty() || self.status == "ready" {
            spans.push(Span::styled(
                "ready  ·  / for commands  ·  Tab for sessions",
                Style::default().fg(theme::text_dim()),
            ));
        } else {
            spans.push(Span::styled(
                self.status.clone(),
                Style::default().fg(theme::text_muted()),
            ));
        }
        let left = Line::from(spans);
        let tokens = self.prompt_tokens + self.completion_tokens;
        let right = Line::from(Span::styled(
            format!("{tokens} tok  "),
            Style::default().fg(theme::text_dim()),
        ))
        .alignment(Alignment::Right);
        f.render_widget(Paragraph::new(Text::from(vec![left])), area);
        f.render_widget(Paragraph::new(Text::from(vec![right])), area);
    }

    fn shimmer(&self, text: &str) -> Vec<Span<'static>> {
        let chars: Vec<char> = text.chars().collect();
        let crest = self.tick % (chars.len().max(1) + 8);
        chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let near = i.abs_diff(crest) <= 2;
                let color = if near {
                    theme::interactive()
                } else {
                    theme::text_muted()
                };
                Span::styled(c.to_string(), Style::default().fg(color))
            })
            .collect()
    }
}
