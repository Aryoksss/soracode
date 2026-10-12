//! Connection and agent-loop settings, with defaults and partial-overlay rules.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the model lives and how to talk to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// OpenAI-compatible endpoint, e.g. `https://9router.example/v1`.
    pub base_url: String,
    /// Bearer token.
    pub api_key: String,
    /// Model identifier, e.g. `gh/gpt-4.1`.
    pub model: String,
    /// Hard cap on completion tokens.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Sampling temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Attempts per request before giving up, including the first one.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Base delay in milliseconds for the exponential retry backoff.
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    /// Extra headers sent with every request.
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
}

pub(super) fn default_max_tokens() -> u32 {
    16_384
}
pub(super) fn default_temperature() -> f32 {
    0.3
}
pub(super) fn default_timeout() -> u64 {
    600
}
pub(super) fn default_max_attempts() -> u32 {
    4
}
pub(super) fn default_retry_backoff_ms() -> u64 {
    500
}

/// How much autonomy the agent is given over an engagement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Full autonomy: run tools, record state, keep going. The default.
    #[default]
    Auto,
    /// Same tools, but the agent presents a plan and waits for the operator
    /// before any exploit or state-changing action.
    Guided,
    /// No engagement-state ceremony: a plain assistant for one-off scripting
    /// and questions.
    Free,
}

impl Mode {
    /// Parse a mode name, accepting the exact serde spelling.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Mode::Auto),
            "guided" => Some(Mode::Guided),
            "free" => Some(Mode::Free),
            _ => None,
        }
    }

    /// The lowercase name used in config, CLI and the prompt.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Guided => "guided",
            Mode::Free => "free",
        }
    }
}

/// Runtime behaviour of the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Autonomy mode for the engagement.
    #[serde(default)]
    pub mode: Mode,
    /// Maximum tool-calling turns per top-level instruction.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// Maximum wall-clock seconds for a single top-level instruction (0 = unlimited).
    #[serde(default)]
    pub max_seconds: u64,
    /// Working directory tools operate in.
    #[serde(default = "default_workspace")]
    pub workspace: PathBuf,
    /// Path to the system prompt file. If absent, the built-in prompt is used.
    #[serde(default)]
    pub system_prompt_file: Option<PathBuf>,
    /// Allow spawning parallel subagents.
    #[serde(default = "default_true")]
    pub subagents_enabled: bool,
    /// Max depth of nested subagents.
    #[serde(default = "default_subagent_depth")]
    pub subagent_max_depth: u32,
    /// Max subagents running at once.
    #[serde(default = "default_subagent_parallel")]
    pub subagent_max_parallel: u32,
    /// Per-tool timeout in seconds.
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout_secs: u64,
    /// Max bytes of tool output kept in context per call.
    #[serde(default = "default_tool_output")]
    pub tool_output_limit: usize,
    /// Approximate character budget for the whole conversation before the
    /// oldest turns are elided. `0` disables compaction.
    #[serde(default = "default_context_budget")]
    pub context_budget: usize,
}

pub(super) fn default_max_turns() -> u32 {
    10_000
}
pub(super) fn default_true() -> bool {
    true
}
pub(super) fn default_workspace() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
pub(super) fn default_subagent_depth() -> u32 {
    4
}
pub(super) fn default_subagent_parallel() -> u32 {
    8
}
pub(super) fn default_tool_timeout() -> u64 {
    1800
}
pub(super) fn default_tool_output() -> usize {
    60_000
}
pub(super) fn default_context_budget() -> usize {
    400_000
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "https://9router.aryoks.tech/v1".into(),
            api_key: String::new(),
            model: "gh/gpt-4.1".into(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            timeout_secs: default_timeout(),
            max_attempts: default_max_attempts(),
            retry_backoff_ms: default_retry_backoff_ms(),
            headers: Default::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            max_turns: default_max_turns(),
            max_seconds: 0,
            workspace: default_workspace(),
            system_prompt_file: None,
            subagents_enabled: true,
            subagent_max_depth: default_subagent_depth(),
            subagent_max_parallel: default_subagent_parallel(),
            tool_timeout_secs: default_tool_timeout(),
            tool_output_limit: default_tool_output(),
            context_budget: default_context_budget(),
        }
    }
}
