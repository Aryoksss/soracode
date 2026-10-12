//! The first screen: what to do when the transcript is still empty.
//!
//! Shown instead of a blank pane so a fresh launch tells the operator which
//! model and workspace they are in, and the three or four things worth trying.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::{theme, App};

/// "soracode" as block letters (art ASCII, not obvious from the bytes).
const LOGO: [&str; 6] = [
    "█▀▀ █▀█ █▀█ ▄▀█ █▀▀ █▀█ █▀▄ █▀▀",
    "▄██ █▄█ █▀▄ █▀█ █▄▄ █▄█ █▄▀ ██▄",
    "",
    "",
    "",
    "",
];

impl App {
    /// Render the welcome pane into `area`.
    pub(super) fn draw_welcome(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let workspace = self.cfg.agent().workspace.display().to_string();
        let mode = self.mode.as_str();

        let mut lines: Vec<Line> = Vec::new();
        for row in LOGO {
            if row.is_empty() {
                lines.push(Line::default());
            } else {
                lines.push(Line::from(Span::styled(
                    row,
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                )));
            }
        }
        lines.push(Line::from(Span::styled(
            "operator-controlled security agent",
            Style::default().fg(theme::text_muted()),
        )));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("  model      ", Style::default().fg(theme::text_dim())),
            Span::styled(
                self.model.clone(),
                Style::default().fg(theme::interactive()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  mode       ", Style::default().fg(theme::text_dim())),
            Span::styled(mode.to_string(), Style::default().fg(theme::text())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  workspace  ", Style::default().fg(theme::text_dim())),
            Span::styled(workspace, Style::default().fg(theme::info())),
        ]));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  describe a target and the agent plans, runs tools, and records",
            Style::default().fg(theme::text_muted()),
        )));
        lines.push(Line::from(Span::styled(
            "  what it finds.  type / for commands, Tab for sessions.",
            Style::default().fg(theme::text_muted()),
        )));

        let para = Paragraph::new(Text::from(lines))
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::border()))
                    .title(self.transcript_title()),
            );
        f.render_widget(para, area);
    }
}
