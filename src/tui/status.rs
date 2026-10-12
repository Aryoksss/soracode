//! The hint bar under the input: model, mode, live status, token counter.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::{theme, App};

impl App {
    /// A single quiet line at the top: directory, branch, model and any tool
    /// currently in flight — the Claude Code status line.
    pub(super) fn draw_topbar(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let workspace = self.cfg.agent().workspace.clone();
        let dir = workspace
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| workspace.display().to_string());
        let mut spans = vec![
            Span::styled("  ", Style::default()),
            Span::styled(dir, Style::default().fg(theme::text_muted())),
        ];
        if let Some(branch) = super::git::branch(&workspace) {
            spans.push(Span::styled("  ", Style::default()));
            spans.push(Span::styled(
                format!("⎇ {branch}"),
                Style::default().fg(theme::interactive()),
            ));
        }
        spans.push(Span::styled("   ", Style::default()));
        spans.push(Span::styled(
            self.model.clone(),
            Style::default().fg(theme::text_dim()),
        ));
        spans.push(Span::styled(
            "  ·  ",
            Style::default().fg(theme::text_dim()),
        ));
        spans.push(Span::styled(
            self.mode.as_str().to_string(),
            Style::default().fg(theme::text_dim()),
        ));
        if let Some(tool) = &self.current_tool {
            spans.push(Span::styled("   ", Style::default()));
            spans.push(Span::styled(
                format!("{} {tool}", self.spinner()),
                Style::default().fg(theme::role_tool()),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

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
        let right = Line::from(vec![
            Span::styled(
                self.context_meter(),
                Style::default().fg(self.context_color()),
            ),
            Span::styled(
                format!("{} tok  ", self.prompt_tokens + self.completion_tokens),
                Style::default().fg(theme::text_dim()),
            ),
        ])
        .alignment(Alignment::Right);
        f.render_widget(Paragraph::new(Text::from(vec![left])), area);
        f.render_widget(Paragraph::new(Text::from(vec![right])), area);
    }

    /// Share of the context budget the last request used, as `NN% context`.
    fn context_meter(&self) -> String {
        let budget = self.cfg.agent.0.context_budget.max(1);
        let used = self.prompt_tokens as usize;
        format!("{}% context  ·  ", (used * 100 / budget).min(999))
    }

    fn context_color(&self) -> ratatui::style::Color {
        let budget = self.cfg.agent.0.context_budget.max(1);
        let ratio = self.prompt_tokens as usize * 100 / budget;
        if ratio >= 85 {
            theme::error()
        } else if ratio >= 60 {
            theme::warning()
        } else {
            theme::text_dim()
        }
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
