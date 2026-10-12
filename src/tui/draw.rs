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
                Constraint::Length(1),
                Constraint::Min(5),
                Constraint::Length(palette_h),
                Constraint::Length(input_h),
                Constraint::Length(1),
            ])
            .split(main);

        self.draw_topbar(f, chunks[0]);
        self.draw_pane(f, chunks[1]);
        if self.palette.open {
            self.draw_palette(f, chunks[2]);
        }
        self.draw_input(f, chunks[3]);
        self.draw_status(f, chunks[4]);
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
        match (&self.palette.models, &self.palette.files) {
            (Some(m), _) => m.len() as u16,
            (_, Some(f)) => f.len() as u16,
            _ => command::matching(&self.input).len() as u16,
        }
    }

    pub(super) fn spinner(&self) -> &'static str {
        if self.busy {
            SPINNER[self.tick % SPINNER.len()]
        } else {
            " "
        }
    }

    /// Seconds since the running turn started, for the busy indicator.
    pub(super) fn elapsed(&self) -> u64 {
        self.busy_since.map_or(0, |t| t.elapsed().as_secs())
    }

    fn draw_palette(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let (title, rows): (&str, Vec<(String, String)>) =
            match (&self.palette.models, &self.palette.files) {
                (Some(models), _) => (
                    " models ",
                    models.iter().map(|m| (m.clone(), String::new())).collect(),
                ),
                (_, Some(files)) => (
                    " files ",
                    files.iter().map(|f| (f.clone(), String::new())).collect(),
                ),
                _ => (
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
                let bg = if selected {
                    theme::selection_bg()
                } else {
                    theme::panel_bg()
                };
                let name_fg = if selected {
                    theme::accent()
                } else {
                    theme::text()
                };
                let marker = if selected { "▌" } else { " " };
                Line::from(vec![
                    Span::styled(
                        format!("{marker} {name:<12}"),
                        Style::default()
                            .fg(name_fg)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {desc}"),
                        Style::default().fg(theme::text_muted()).bg(bg),
                    ),
                ])
            })
            .collect();

        let para = Paragraph::new(Text::from(lines))
            .style(Style::default().bg(theme::panel_bg()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::border_active()))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(theme::accent())
                            .add_modifier(Modifier::BOLD),
                    )),
            );
        f.render_widget(para, area);
    }

    fn draw_input(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let border = if self.busy {
            theme::border()
        } else {
            theme::border_active()
        };
        let body: Text = if self.input.is_empty() {
            Text::from(Line::from(vec![
                Span::styled(" > ", Style::default().fg(theme::accent())),
                Span::styled(
                    "ask about a target, or type / for commands",
                    Style::default().fg(theme::text_dim()),
                ),
            ]))
        } else {
            Text::from(
                self.input
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        let prompt = if i == 0 { " > " } else { "   " };
                        Line::from(vec![
                            Span::styled(prompt, Style::default().fg(theme::accent())),
                            Span::styled(line.to_string(), Style::default().fg(theme::text())),
                        ])
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        if let Some(q) = &self.queued {
            let chip = format!(" queued: {} ", truncate(q, 40));
            block = block.title(
                Line::from(Span::styled(chip, Style::default().fg(theme::warning())))
                    .alignment(Alignment::Right),
            );
        }
        let para = Paragraph::new(body)
            .block(block)
            .style(Style::default().bg(theme::bg()))
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }
}

/// Shorten `s` to `max` characters, adding an ellipsis when it was cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
