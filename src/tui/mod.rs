//! Interactive terminal UI.

mod accessors;
mod apply;
mod command;
mod copy;
mod diff;
mod draw;
mod export;
mod keys;
mod markdown;
mod mention;
mod mouse;
mod palette;
mod panel;
mod sidebar;
mod slash;
mod status;
mod table;
mod theme;
mod transcript;
mod welcome;

#[cfg(test)]
mod markdown_tests;
#[cfg(test)]
mod palette_tests;

use palette::Palette;
use panel::{
    describe_creds, describe_dashboard, describe_matrix, describe_services, describe_sessions,
    describe_targets, describe_tools, describe_vulns,
};
use sidebar::load_sessions;

pub(crate) use sidebar::shared;

use std::io;
use std::io::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::agent::AgentEvent;
use crate::config::Config;
use crate::llm::Client;
use crate::session::Session;

/// Button-event SGR mouse tracking: wheel and clicks reach the app, but drag
/// is never reported, so the terminal keeps drag-selection for copy-paste with
/// no toggle. `?1002h` (drag motion) is deliberately omitted; crossterm's own
/// capture adds `?1003h` (all motion) and would also steal the drag.
const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1006h";
const MOUSE_OFF: &str = "\x1b[?1000l\x1b[?1006l";

/// Bracketed paste: the terminal wraps a paste in markers so newlines in a
/// pasted block arrive as one event instead of a burst of Enter keys.
const PASTE_ON: &str = "\x1b[?2004h";
const PASTE_OFF: &str = "\x1b[?2004l";

/// The session the next submission will run against, swappable from the
/// sidebar so a user can resume a different session without restarting.
pub(super) type SharedSession = Arc<Mutex<Arc<Session>>>;

pub(super) use transcript::Line2;

/// TUI application state.
pub struct App {
    transcript: Vec<Line2>,
    input: String,
    history: Vec<String>,
    /// Transcript length at each submit, so `/undo` can cut a whole turn.
    turn_marks: Vec<usize>,
    history_at: Option<usize>,
    status: String,
    scroll: u16,
    auto_scroll: bool,
    events: Option<UnboundedReceiver<AgentEvent>>,
    tx: UnboundedSender<AgentEvent>,
    current_stream: String,
    palette: Palette,
    cfg: Arc<Config>,
    client: Arc<Mutex<Client>>,
    quit: bool,
    busy: bool,
    tick: usize,
    prompt_tokens: u64,
    completion_tokens: u64,
    model: String,
    show_thinking: bool,
    thinking: String,
    sessions: Vec<(String, i64, String)>,
    sidebar_index: usize,
    show_sidebar: bool,
    sidebar_focus: bool,
    shared_session: SharedSession,
    db_path: std::path::PathBuf,
    mouse_capture: bool,
    ctrl_c_armed: bool,
    cancel: Arc<tokio::sync::Notify>,
    pause: Arc<std::sync::atomic::AtomicBool>,
    /// One-shot flag: the next turn compacts against a tighter budget.
    compact_now: Arc<std::sync::atomic::AtomicBool>,
    mode: crate::config::Mode,
    sidebar_area: ratatui::layout::Rect,
    /// Mirrors `shared_session`'s id for the transcript header.
    session_id: String,
}

/// A submitted instruction plus the channel to receive its events.
#[allow(dead_code)] // public UI type; retained for embedding the TUI loop
pub struct Submission {
    /// The instruction text.
    pub instruction: String,
    /// Receiver for agent events.
    pub events: UnboundedReceiver<AgentEvent>,
}

impl App {
    /// Create a new UI state bound to a configuration and a shared client.
    ///
    /// The client is shared so `/model` can replace it for later submissions.
    #[must_use]
    pub fn new(cfg: Arc<Config>, client: Arc<Mutex<Client>>, session: SharedSession) -> Self {
        theme::install(theme::from_map(&cfg.theme));
        let (tx, rx) = unbounded_channel();
        let model = cfg.active_model_name();
        let mode = cfg.agent().mode;
        let sessions = load_sessions(&cfg);
        let db_path = cfg.db_path.clone();
        let (session_id, transcript) = crate::tui::transcript::load(&session);
        Self {
            transcript,
            input: String::new(),
            history: Vec::new(),
            turn_marks: Vec::new(),
            history_at: None,
            status: "ready — Enter send  Tab sessions  ↑↓ history  F2 copy  / commands".into(),
            scroll: 0,
            auto_scroll: true,
            events: Some(rx),
            tx,
            current_stream: String::new(),
            palette: Palette::closed(),
            cfg,
            client,
            quit: false,
            busy: false,
            tick: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            model,
            show_thinking: false,
            thinking: String::new(),
            sessions,
            sidebar_index: 0,
            show_sidebar: true,
            sidebar_focus: false,
            mouse_capture: true,
            ctrl_c_armed: false,
            shared_session: session,
            db_path,
            cancel: Arc::new(tokio::sync::Notify::new()),
            pause: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            compact_now: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            mode,
            sidebar_area: ratatui::layout::Rect::default(),
            session_id,
        }
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
        write!(stdout, "{MOUSE_ON}{PASTE_ON}")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.event_loop(&mut terminal, &mut on_submit);

        disable_raw_mode()?;
        write!(terminal.backend_mut(), "{MOUSE_OFF}{PASTE_OFF}")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        res
    }

    fn event_loop<B, F>(&mut self, terminal: &mut Terminal<B>, on_submit: &mut F) -> io::Result<()>
    where
        B: ratatui::backend::Backend,
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        loop {
            self.tick = self.tick.wrapping_add(1);
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
                let before = self.mouse_capture;
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key, on_submit) {
                            return Ok(());
                        }
                    }
                    Event::Resize(_, _) => {}
                    Event::Mouse(m) => self.handle_mouse(m),
                    Event::Paste(s) => self.input.push_str(&s),
                    _ => {}
                }
                if before != self.mouse_capture {
                    let mut out = io::stdout();
                    if self.mouse_capture {
                        write!(out, "{MOUSE_ON}")?;
                    } else {
                        write!(out, "{MOUSE_OFF}")?;
                    }
                }
            }
        }
    }
}
