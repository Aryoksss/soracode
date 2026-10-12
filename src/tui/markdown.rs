use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme;

pub(super) fn push_markdown(lines: &mut Vec<Line>, text: &str, base: Color) {
    for raw in text.lines() {
        let mut spans: Vec<Span> = Vec::new();
        let mut rest = raw;
        let bullet = raw.starts_with("- ") || raw.starts_with("* ");
        let indent = if bullet { "  • " } else { "  " };
        if bullet {
            rest = &raw[2..];
        }
        spans.push(Span::styled(indent.to_string(), Style::default().fg(base)));
        for (i, seg) in rest.split('`').enumerate() {
            if seg.is_empty() {
                continue;
            }
            if i % 2 == 1 {
                spans.push(Span::styled(
                    seg.to_string(),
                    Style::default().fg(theme::SUCCESS),
                ));
                continue;
            }
            if seg.starts_with("**") && seg.ends_with("**") && seg.len() > 4 {
                spans.push(Span::styled(
                    seg[2..seg.len() - 2].to_string(),
                    Style::default().fg(base).add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            if seg.starts_with('#') {
                spans.push(Span::styled(
                    seg.trim_start_matches('#').trim_start().to_string(),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ));
                continue;
            }
            spans.push(Span::styled(seg.to_string(), Style::default().fg(base)));
        }
        lines.push(Line::from(spans));
    }
}
