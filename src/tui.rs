//! Interactive terminal UI.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::agent::AgentEvent;

/// A line of conversation shown in the transcript pane.
#[derive(Clone)]
enum Line2 {
    User(String),
    Assistant(String),
    Tool(String),
    Info(String),
    Error(String),
}

/// TUI application state.
pub struct App {
    transcript: Vec<Line2>,
    input: String,
    history: Vec<String>,
    history_at: Option<usize>,
    status: String,
    scroll: u16,
    auto_scroll: bool,
    events: Option<UnboundedReceiver<AgentEvent>>,
    tx: UnboundedSender<AgentEvent>,
    current_stream: String,
}

/// A submitted instruction plus the channel to receive its events.
pub struct Submission {
    /// The instruction text.
    pub instruction: String,
    /// Receiver for agent events.
    pub events: UnboundedReceiver<AgentEvent>,
}

impl App {
    /// Create a new UI state.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            transcript: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            history_at: None,
            status: "ready — type an instruction and press Enter".into(),
            scroll: 0,
            auto_scroll: true,
            events: Some(rx),
            tx,
            current_stream: String::new(),
        }
    }

    /// Sender the agent should publish events to.
    #[must_use]
    pub fn sender(&self) -> UnboundedSender<AgentEvent> {
        self.tx.clone()
    }

    /// Run the event loop until the user quits.
    ///
    /// # Errors
    /// Returns terminal IO errors.
    pub fn run<F>(&mut self, mut on_submit: F) -> io::Result<()>
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.event_loop(&mut terminal, &mut on_submit);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        res
    }

    fn event_loop<B, F>(
        &mut self,
        terminal: &mut Terminal<B>,
        on_submit: &mut F,
    ) -> io::Result<()>
    where
        B: ratatui::backend::Backend,
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        loop {
            terminal.draw(|f| self.draw(f))?;

            // Drain agent events without blocking.
            let mut pending = Vec::new();
            if let Some(rx) = &mut self.events {
                while let Ok(ev) = rx.try_recv() {
                    pending.push(ev);
                }
            }
            for ev in pending {
                self.apply(ev);
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key, on_submit) {
                            return Ok(());
                        }
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
    }

    /// Returns true when the app should exit.
    fn handle_key<F>(&mut self, key: KeyEvent, on_submit: &mut F) -> bool
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::CONTROL) => return true,

            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.input.clear();
                self.history_at = None;
            }

            (KeyCode::Enter, _) => {
                let text = self.input.trim().to_string();
                if !text.is_empty() {
                    self.input.clear();
                    self.history_at = None;
                    if self.history.last() != Some(&text) {
                        self.history.push(text.clone());
                    }
                    self.transcript.push(Line2::User(text.clone()));
                    self.status = "running…".into();
                    self.auto_scroll = true;
                    on_submit(text, self.tx.clone());
                }
            }
            (KeyCode::Up, _) => {
                if !self.history.is_empty() {
                    let idx = match self.history_at {
                        Some(i) => i.saturating_sub(1),
                        None => self.history.len() - 1,
                    };
                    self.history_at = Some(idx);
                    self.input = self.history[idx].clone();
                }
            }
            (KeyCode::Down, _) => match self.history_at {
                Some(i) if i + 1 < self.history.len() => {
                    self.history_at = Some(i + 1);
                    self.input = self.history[i + 1].clone();
                }
                Some(_) => {
                    self.history_at = None;
                    self.input.clear();
                }
                None => {}
            },
            (KeyCode::Char(c), _) => self.input.push(c),
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }
            (KeyCode::PageUp, _) => {
                self.auto_scroll = false;
                self.scroll = self.scroll.saturating_add(5);
            }
            (KeyCode::PageDown, _) => {
                self.scroll = self.scroll.saturating_sub(5);
            }
            (KeyCode::End, _) => self.auto_scroll = true,
            _ => {}
        }
        false
    }

    fn apply(&mut self, ev: AgentEvent) {
        match ev {
            AgentEvent::Text(t) => {
                self.current_stream.push_str(&t);
            }
            AgentEvent::Reasoning(_) => {}
            AgentEvent::ToolStart { name, args } => {
                self.flush_stream();
                let short = if args.len() > 160 {
                    format!("{}…", &args[..160])
                } else {
                    args
                };
                self.transcript
                    .push(Line2::Tool(format!("▶ {name} {short}")));
                self.auto_scroll = true;
            }
            AgentEvent::ToolEnd { name, output } => {
                let first = output.lines().next().unwrap_or("").to_string();
                let extra = output.lines().count().saturating_sub(1);
                let text = if extra > 0 {
                    format!("◀ {name}: {first}  (+{extra} more lines)")
                } else {
                    format!("◀ {name}: {first}")
                };
                self.transcript.push(Line2::Info(text));
            }
            AgentEvent::SubagentStart { label } => {
                self.transcript
                    .push(Line2::Info(format!("⇢ subagent `{label}` started")));
            }
            AgentEvent::Info(m) => self.transcript.push(Line2::Info(m)),
            AgentEvent::Error(m) => self.transcript.push(Line2::Error(m)),
            AgentEvent::Done => {
                self.flush_stream();
                self.status = "ready — type an instruction and press Enter".into();
                self.auto_scroll = true;
            }
        }
    }

    fn flush_stream(&mut self) {
        if !self.current_stream.trim().is_empty() {
            self.transcript
                .push(Line2::Assistant(self.current_stream.clone()));
        }
        self.current_stream.clear();
    }

    fn draw(&mut self, f: &mut ratatui::Frame<'_>) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_header(f, chunks[0]);
        self.draw_transcript(f, chunks[1]);
        self.draw_input(f, chunks[2]);
        self.draw_status(f, chunks[3]);
    }

    fn draw_header(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled(" soracode ", Style::default().fg(Color::Black).bg(Color::Cyan)),
            Span::raw("  security execution engine"),
        ]));
        f.render_widget(title, area);
    }

    fn draw_transcript(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for l in &self.transcript {
            match l {
                Line2::User(t) => {
                    lines.push(Line::from(Span::styled(
                        format!("▌ you  {t}"),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                Line2::Assistant(t) => {
                    for sub in t.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {sub}"),
                            Style::default().fg(Color::White),
                        )));
                    }
                }
                Line2::Tool(t) => lines.push(Line::from(Span::styled(
                    format!("  {t}"),
                    Style::default().fg(Color::Yellow),
                ))),
                Line2::Info(t) => lines.push(Line::from(Span::styled(
                    format!("  {t}"),
                    Style::default().fg(Color::DarkGray),
                ))),
                Line2::Error(t) => lines.push(Line::from(Span::styled(
                    format!("  {t}"),
                    Style::default().fg(Color::Red),
                ))),
            }
        }

        if !self.current_stream.is_empty() {
            for sub in self.current_stream.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {sub}▏"),
                    Style::default().fg(Color::White),
                )));
            }
        }

        let total = lines.len() as u16;
        let height = area.height.saturating_sub(2);
        let max_scroll = total.saturating_sub(height);
        if self.auto_scroll {
            self.scroll = max_scroll;
        } else {
            self.scroll = self.scroll.min(max_scroll);
        }

        let para = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title("transcript"))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, area);
    }

    fn draw_input(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let para = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("instruction"))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(para, area);
    }

    fn draw_status(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let para = Paragraph::new(self.status.as_str())
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(para, area);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
