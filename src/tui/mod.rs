//! Interactive terminal UI.

mod apply;
mod command;
mod draw;
mod keys;
mod markdown;
mod panel;
mod sidebar;
mod slash;
mod theme;
mod transcript;

use panel::{describe_sessions, describe_tools};
use sidebar::load_sessions;

pub(crate) use sidebar::shared;

use std::io;
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

/// The session the next submission will run against, swappable from the
/// sidebar so a user can resume a different session without restarting.
pub(super) type SharedSession = Arc<Mutex<Arc<Session>>>;

/// A line of conversation shown in the transcript pane.
#[derive(Clone)]
enum Line2 {
    User(String),
    Assistant(String),
    Tool(String),
    Info(String),
    Error(String),
}

/// State of the slash-command palette.
struct Palette {
    open: bool,
    selected: usize,
    /// `Some` while picking a model: the names offered.
    models: Option<Vec<String>>,
}

impl Palette {
    fn closed() -> Self {
        Self {
            open: false,
            selected: 0,
            models: None,
        }
    }
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
    sessions: Vec<(String, String)>,
    sidebar_index: usize,
    show_sidebar: bool,
    sidebar_focus: bool,
    shared_session: SharedSession,
    db_path: std::path::PathBuf,
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
        let (tx, rx) = unbounded_channel();
        let model = cfg.active_model_name();
        let sessions = load_sessions(&cfg);
        let db_path = cfg.db_path.clone();
        Self {
            transcript: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            history_at: None,
            status: "ready — type an instruction, or / for commands".into(),
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
            shared_session: session,
            db_path,
        }
    }

    /// Sender the agent should publish events to.
    #[must_use]
    #[allow(dead_code)] // retained for external event sources
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
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.event_loop(&mut terminal, &mut on_submit);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            crossterm::event::DisableMouseCapture,
            LeaveAlternateScreen
        )?;
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
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key, on_submit) {
                            return Ok(());
                        }
                    }
                    Event::Resize(_, _) => {}
                    Event::Mouse(m) => self.handle_mouse(m),
                    _ => {}
                }
            }
        }
    }
}
