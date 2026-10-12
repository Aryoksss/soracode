use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::{command, theme, App};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn shorten(s: &str) -> String {
    s.rsplit('/').next().unwrap_or(s).to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

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

        self.draw_transcript(f, chunks[0]);
        if self.palette.open {
            self.draw_palette(f, chunks[1]);
        }
        self.draw_input(f, chunks[2]);
        self.draw_status(f, chunks[3]);
    }

    fn draw_sidebar(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let lines: Vec<Line> = if self.sessions.is_empty() {
            vec![Line::from(Span::styled(
                " no recent sessions ",
                Style::default().fg(theme::TEXT_MUTED),
            ))]
        } else {
            self.sessions
                .iter()
                .enumerate()
                .map(|(i, (id, summary))| {
                    let selected = i == self.sidebar_index;
                    let style = if selected {
                        Style::default()
                            .fg(theme::INTERACTIVE)
                            .bg(theme::SELECTION_BG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme::TEXT)
                    };
                    let marker = if selected { "▌" } else { " " };
                    let name = format!("{} {}", &id[..id.len().min(8)], shorten(summary));
                    let text = format!("{marker} {}", truncate(&name, 26));
                    Line::from(Span::styled(text, style))
                })
                .collect()
        };

        let para = Paragraph::new(Text::from(lines))
            .style(Style::default().bg(theme::BG_ELEMENT))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER))
                    .title(Span::styled(
                        " sessions ",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(para, area);
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
            .style(Style::default().bg(theme::BG_ELEMENT))
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
        let para = Paragraph::new(self.input.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER_ACTIVE))
                    .title(Span::styled(
                        " instruction ",
                        Style::default().fg(theme::INTERACTIVE),
                    )),
            )
            .style(Style::default().fg(theme::TEXT))
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }

    fn draw_status(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let tokens = self.prompt_tokens + self.completion_tokens;
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.model),
                Style::default().fg(theme::BG).bg(theme::ACCENT),
            ),
            Span::raw(" "),
        ];
        if self.busy {
            spans.extend(self.shimmer(&format!("{} working…  Ctrl-C cancel", self.spinner())));
        } else {
            spans.push(Span::styled(
                "Enter send  ↑↓ history  PgUp/PgDn scroll  / commands",
                Style::default().fg(theme::TEXT_MUTED),
            ));
        }
        let left = Line::from(spans);
        let right = Line::from(Span::styled(
            format!("{} tok ", tokens),
            Style::default().fg(theme::TEXT_MUTED),
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
