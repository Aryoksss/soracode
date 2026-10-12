use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::{command, theme, App};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl App {
    pub(super) fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        let area = f.area();
        let (sidebar, main) = if self.show_sidebar && area.width >= 60 {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(30), Constraint::Min(20)])
                .split(area);
            (Some(cols[0]), cols[1])
        } else {
            (None, area)
        };

        if let Some(sb) = sidebar {
            self.draw_sidebar(f, sb);
        }

        let palette_h = if self.palette.open {
            self.palette_rows().min(10) + 2
        } else {
            0
        };
        let input_h = self.input_height();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),
                Constraint::Length(palette_h),
                Constraint::Length(input_h),
                Constraint::Length(1),
            ])
            .split(main);

        self.draw_pane(f, chunks[0]);
        if self.palette.open {
            self.draw_palette(f, chunks[1]);
        }
        self.draw_input(f, chunks[2]);
        self.draw_status(f, chunks[3]);
    }

    fn draw_pane(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        if self.transcript.is_empty() && self.current_stream.is_empty() && self.thinking.is_empty()
        {
            self.draw_welcome(f, area);
            return;
        }
        self.draw_transcript(f, area);
    }

    fn draw_sidebar(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        super::sidebar::draw(self, f, area);
    }

    fn input_height(&self) -> u16 {
        let rows = self.input.lines().count().max(1) as u16;
        (rows + 2).min(8)
    }

    fn palette_rows(&self) -> u16 {
        match &self.palette.models {
            Some(m) => m.len() as u16,
            None => command::matching(&self.input).len() as u16,
        }
    }

    fn spinner(&self) -> &'static str {
        if self.busy {
            SPINNER[self.tick % SPINNER.len()]
        } else {
            " "
        }
    }

    fn draw_palette(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let (title, rows): (&str, Vec<(String, String)>) = match &self.palette.models {
            Some(models) => (
                " models ",
                models.iter().map(|m| (m.clone(), String::new())).collect(),
            ),
            None => (
                " commands ",
                command::matching(&self.input)
                    .iter()
                    .map(|c| (format!("/{}", c.name), c.desc.to_string()))
                    .collect(),
            ),
        };

        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .map(|(i, (name, desc))| {
                let selected = i == self.palette.selected;
                let style = if selected {
                    Style::default()
                        .fg(theme::INTERACTIVE)
                        .bg(theme::SELECTION_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };
                let marker = if selected { "▌" } else { " " };
                Line::from(vec![
                    Span::styled(format!("{marker} {name:<12}"), style),
                    Span::styled(format!(" {desc}"), Style::default().fg(theme::TEXT_MUTED)),
                ])
            })
            .collect();

        let para = Paragraph::new(Text::from(lines))
            .style(Style::default().bg(theme::PANEL_BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER_ACTIVE))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(para, area);
    }

    fn draw_input(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let border = if self.busy {
            theme::BORDER
        } else {
            theme::BORDER_ACTIVE
        };
        let body: Text = if self.input.is_empty() {
            Text::from(Line::from(vec![
                Span::styled("  ❯ ", Style::default().fg(theme::ACCENT)),
                Span::styled(
                    "ask about a target, or type / for commands",
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]))
        } else {
            let mut spans = vec![Span::styled("  ❯ ", Style::default().fg(theme::ACCENT))];
            spans.push(Span::styled(
                self.input.clone(),
                Style::default().fg(theme::TEXT),
            ));
            Text::from(Line::from(spans))
        };
        let para = Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border))
                    .title(Span::styled(
                        " instruction ",
                        Style::default().fg(theme::INTERACTIVE),
                    )),
            )
            .style(Style::default().bg(theme::PANEL_BG))
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    fn draw_status(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let tokens = self.prompt_tokens + self.completion_tokens;
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.model),
                Style::default()
                    .fg(theme::BG)
                    .bg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{} ", self.mode.as_str()),
                Style::default().fg(theme::INTERACTIVE),
            ),
            Span::styled("· ", Style::default().fg(theme::TEXT_DIM)),
        ];
        if self.busy {
            spans.extend(self.shimmer(&format!("{} working…  Ctrl-C cancel", self.spinner())));
        } else {
            spans.push(Span::styled(
                "ready — Enter send  / commands",
                Style::default().fg(theme::TEXT_MUTED),
            ));
        }
        let left = Line::from(spans);
        let right = Line::from(Span::styled(
            format!("{tokens} tok "),
            Style::default().fg(theme::TEXT_DIM),
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
                    theme::INTERACTIVE
                } else {
                    theme::TEXT_MUTED
                };
                Span::styled(c.to_string(), Style::default().fg(color))
            })
            .collect()
    }
}
