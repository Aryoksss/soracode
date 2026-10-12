//! The first screen: what to do when the transcript is still empty.
//!
//! Shown instead of a blank pane so a fresh launch tells the operator which
//! model and workspace they are in, and the three or four things worth trying.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::{theme, App};

impl App {
    /// Render the welcome pane into `area`.
    pub(super) fn draw_welcome(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let workspace = self.cfg.agent().workspace.display().to_string();

        let mut lines: Vec<Line> = vec![
            Line::default(),
            Line::from(vec![
                Span::styled(
                    "  soracode",
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "   operator-controlled security agent",
                    Style::default().fg(theme::text_dim()),
                ),
            ]),
            Line::default(),
            Line::from(vec![
                Span::styled("  model      ", Style::default().fg(theme::text_dim())),
                Span::styled(self.model.clone(), Style::default().fg(theme::info())),
            ]),
            Line::from(vec![
                Span::styled("  mode       ", Style::default().fg(theme::text_dim())),
                Span::styled(
                    self.mode.as_str().to_string(),
                    Style::default().fg(theme::text()),
                ),
            ]),
            Line::from(vec![
                Span::styled("  workspace  ", Style::default().fg(theme::text_dim())),
                Span::styled(workspace, Style::default().fg(theme::text_muted())),
            ]),
            Line::default(),
            Line::from(Span::styled(
                "  describe a target and the agent plans, runs tools, and records what",
                Style::default().fg(theme::text_muted()),
            )),
            Line::from(Span::styled(
                "  it finds.  type / for commands, Tab for the session list.",
                Style::default().fg(theme::text_muted()),
            )),
        ];
        lines.push(Line::default());

        let para = Paragraph::new(Text::from(lines)).alignment(Alignment::Left);
        f.render_widget(para, area);
    }
}
