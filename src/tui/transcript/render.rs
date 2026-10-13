//! Rendering the transcript a session keeps into styled terminal lines.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::super::markdown::push_markdown_indented;
use super::super::theme;
use super::Line2;

/// Build the transcript's rendered lines from session state.
pub(super) fn collect_lines(
    lines: &mut Vec<Line>,
    transcript: &[Line2],
    thinking: &str,
    show_thinking: bool,
    stream: &str,
    active_tool: Option<usize>,
    spinner: &str,
) {
    for (idx, l) in transcript.iter().enumerate() {
        match l {
            Line2::User(t) => {
                let mut iter = t.lines();
                let first = iter.next().unwrap_or("").to_string();
                lines.push(Line::from(vec![
                    Span::styled("> ", Style::default().fg(theme::role_user())),
                    Span::styled(
                        first,
                        Style::default()
                            .fg(theme::role_user())
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                for sub in iter {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(
                            sub.to_string(),
                            Style::default()
                                .fg(theme::role_user())
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
            Line2::Assistant(t) => {
                let start = lines.len();
                push_markdown_indented(lines, t, theme::text(), "  ");
                if let Some(first) = lines.get_mut(start) {
                    first.spans.insert(
                        0,
                        Span::styled("⏺ ", Style::default().fg(theme::role_assistant())),
                    );
                } else {
                    lines.push(Line::from(Span::styled(
                        "⏺",
                        Style::default().fg(theme::role_assistant()),
                    )));
                }
            }
            Line2::Tool(t) => {
                let start = lines.len();
                push_markdown_indented(lines, t, theme::role_tool(), "");
                if let Some(first) = lines.get_mut(start) {
                    first.spans.insert(
                        0,
                        Span::styled("⏺ ", Style::default().fg(theme::role_tool())),
                    );
                } else {
                    lines.push(Line::from(Span::styled(
                        "⏺",
                        Style::default().fg(theme::role_tool()),
                    )));
                }
                if active_tool == Some(idx) {
                    if let Some(last) = lines.last_mut() {
                        last.spans.push(Span::styled(
                            format!(" {spinner}"),
                            Style::default().fg(theme::accent()),
                        ));
                    }
                }
            }
            Line2::Result(t) => {
                let mut iter = t.lines();
                let first = iter.next().unwrap_or("").to_string();
                lines.push(Line::from(vec![
                    Span::styled("  ⎿  ", Style::default().fg(theme::text_dim())),
                    Span::styled(first, Style::default().fg(theme::text_muted())),
                ]));
                for sub in iter {
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default()),
                        Span::styled(sub.to_string(), Style::default().fg(theme::text_dim())),
                    ]));
                }
            }
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
            Line2::Diff(t) => {
                let color = if t.starts_with('+') {
                    theme::success()
                } else if t.starts_with('-') {
                    theme::error()
                } else {
                    theme::text_dim()
                };
                lines.push(Line::from(Span::styled(
                    format!("  {t}"),
                    Style::default().fg(color),
                )));
            }
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

    if show_thinking && !thinking.is_empty() {
        lines.push(Line::from(Span::styled(
            "  ┌ thinking".to_string(),
            Style::default().fg(theme::warning()),
        )));
        for sub in thinking.lines() {
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

    if !stream.is_empty() {
        let start = lines.len();
        push_markdown_indented(lines, stream, theme::text(), "  ");
        if let Some(first) = lines.get_mut(start) {
            first.spans.insert(
                0,
                Span::styled("⏺ ", Style::default().fg(theme::role_assistant())),
            );
        }
        if let Some(last) = lines.last_mut() {
            last.spans
                .push(Span::styled("▏", Style::default().fg(theme::accent())));
        }
    }
}
