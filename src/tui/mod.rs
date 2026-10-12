//! Interactive terminal UI.

mod accessors;
mod apply;
mod command;
mod copy;
mod diff;
mod draw;
mod export;
mod git;
mod keys;
mod markdown;
mod mention;
mod mouse;
mod palette;
mod panel;
mod runner;
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

use std::sync::{Arc, Mutex};

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
    busy_since: Option<std::time::Instant>,
    queued: Option<String>,
    current_tool: Option<String>,
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
            status: String::new(),
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
            busy_since: None,
            queued: None,
            current_tool: None,
            tick: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            model,
            show_thinking: false,
            thinking: String::new(),
            sessions,
            sidebar_index: 0,
            show_sidebar: false,
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
}
