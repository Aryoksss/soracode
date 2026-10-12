//! Public types shared by the agent loop, its callers, and the UI.

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::config::AgentConfig;
use crate::llm::Client;
use crate::session::Session;
use crate::tools::subagent::Spawner;

/// Events emitted by an agent for the UI to render.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Assistant visible text chunk.
    Text(String),
    /// Assistant reasoning chunk.
    #[allow(dead_code)] // payload rendered by future UIs; currently streamed but unused
    Reasoning(String),
    /// A tool is about to run.
    ToolStart {
        /// Tool name.
        name: String,
        /// Raw JSON arguments.
        args: String,
    },
    /// A tool finished.
    ToolEnd {
        /// Tool name.
        name: String,
        /// Truncated output for display.
        output: String,
    },
    /// A subagent started.
    SubagentStart {
        /// Label.
        label: String,
    },
    /// Token usage reported by the provider for one turn.
    Usage {
        /// Prompt tokens billed.
        prompt: u64,
        /// Completion tokens billed.
        completion: u64,
    },
    /// Informational message.
    Info(String),
    /// A fatal or recoverable error.
    Error(String),
    /// The turn finished.
    Done,
    /// The turn stopped because the operator paused the engagement.
    Paused,
    /// The turn was cancelled by the operator.
    #[allow(dead_code)] // emitted by the TUI cancel path
    Cancelled,
}

/// Result of a completed top-level instruction.
#[derive(Debug, Clone)]
pub struct TurnResult {
    /// Final assistant text.
    pub final_text: String,
    /// Number of model round-trips used.
    #[allow(dead_code)] // kept for callers that report turn counts
    pub turns: u32,
}

/// Configuration for constructing an agent.
pub struct AgentSpec {
    /// The client to talk to the model.
    pub client: Client,
    /// Effective agent configuration.
    pub config: Arc<AgentConfig>,
    /// Whether this agent is a subagent.
    pub is_subagent: bool,
    /// Nesting depth (0 = root).
    pub depth: u32,
    /// Specialist role for a subagent, if any.
    pub role: Option<String>,
    /// Session store.
    pub session: Arc<Session>,
    /// Where UI events are sent.
    pub events: Option<UnboundedSender<AgentEvent>>,
    /// Bridge used by the `subagent` tool.
    pub spawner: Option<Arc<dyn Spawner>>,
    /// Shared pause flag; when set, the loop stops before the next tool call.
    pub pause: Arc<std::sync::atomic::AtomicBool>,
}
