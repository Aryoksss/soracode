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

/// Runtime behaviour of the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
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

impl LlmConfig {
    /// Overlay a partial `[llm]` block, treating empty strings as absent.
    pub fn overlay(&mut self, o: &Self) {
        if !o.base_url.is_empty() {
            self.base_url.clone_from(&o.base_url);
        }
        if !o.api_key.is_empty() {
            self.api_key.clone_from(&o.api_key);
        }
        if !o.model.is_empty() {
            self.model.clone_from(&o.model);
        }
        if o.max_tokens != 0 {
            self.max_tokens = o.max_tokens;
        }
        if o.temperature != 0.0 {
            self.temperature = o.temperature;
        }
        if o.timeout_secs != 0 {
            self.timeout_secs = o.timeout_secs;
        }
        self.headers.extend(o.headers.clone());
    }
}

impl AgentConfig {
    /// Overlay a partial `[agent]` block, treating sentinel values as absent.
    pub fn overlay(&mut self, o: &Self) {
        if o.max_turns != 0 {
            self.max_turns = o.max_turns;
        }
        if o.max_seconds != 0 {
            self.max_seconds = o.max_seconds;
        }
        if o.workspace != default_workspace() {
            self.workspace.clone_from(&o.workspace);
        }
        if o.system_prompt_file.is_some() {
            self.system_prompt_file.clone_from(&o.system_prompt_file);
        }
        if o.subagents_enabled {
            self.subagents_enabled = true;
        }
        if o.subagent_max_depth != 0 {
            self.subagent_max_depth = o.subagent_max_depth;
        }
        if o.subagent_max_parallel != 0 {
            self.subagent_max_parallel = o.subagent_max_parallel;
        }
        if o.tool_timeout_secs != 0 {
            self.tool_timeout_secs = o.tool_timeout_secs;
        }
        if o.tool_output_limit != 0 {
            self.tool_output_limit = o.tool_output_limit;
        }
    }
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
            headers: Default::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: default_max_turns(),
            max_seconds: 0,
            workspace: default_workspace(),
            system_prompt_file: None,
            subagents_enabled: true,
            subagent_max_depth: default_subagent_depth(),
            subagent_max_parallel: default_subagent_parallel(),
            tool_timeout_secs: default_tool_timeout(),
            tool_output_limit: default_tool_output(),
        }
    }
}
