//! Configuration loading.
//!
//! Resolution order (later wins):
//!   1. Built-in defaults
//!   2. `~/.config/harness/config.toml`
//!   3. `./harness.toml`
//!   4. Environment variables (`HARNESS_*`)
//!   5. CLI flags
//!
//! There is deliberately **no allowlist / permission model** here: the operator
//! running the harness is the authority. Any target or command given is executed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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

fn default_max_tokens() -> u32 {
    16_384
}
fn default_temperature() -> f32 {
    0.3
}
fn default_timeout() -> u64 {
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

fn default_max_turns() -> u32 {
    10_000
}
fn default_true() -> bool {
    true
}
fn default_workspace() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
fn default_subagent_depth() -> u32 {
    4
}
fn default_subagent_parallel() -> u32 {
    8
}
fn default_tool_timeout() -> u64 {
    1800
}
fn default_tool_output() -> usize {
    60_000
}

/// Full harness configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM connection settings.
    pub llm: LlmConfig,
    /// Agent loop settings.
    #[serde(default)]
    pub agent: AgentConfigDefaults,
    /// Where sessions are stored.
    #[serde(default = "default_db")]
    pub db_path: PathBuf,
}

/// Wrapper so `agent` can be omitted entirely in config files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigDefaults(#[serde(default)] pub AgentConfig);

fn default_db() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("harness");
    p.push("sessions.db");
    p
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

impl Default for AgentConfigDefaults {
    fn default() -> Self {
        Self(AgentConfig::default())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                base_url: "https://9router.aryoks.tech/v1".into(),
                api_key: String::new(),
                model: "gh/gpt-4.1".into(),
                max_tokens: default_max_tokens(),
                temperature: default_temperature(),
                timeout_secs: default_timeout(),
                headers: Default::default(),
            },
            agent: AgentConfigDefaults::default(),
            db_path: default_db(),
        }
    }
}

impl Config {
    /// The effective agent configuration.
    #[must_use]
    pub fn agent(&self) -> &AgentConfig {
        &self.agent.0
    }

    /// Load configuration from disk and environment.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when a config file exists but cannot be parsed.
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let mut cfg = Self::default();

        // 2. user config
        if let Some(dir) = dirs::config_dir() {
            let p = dir.join("harness").join("config.toml");
            if p.exists() {
                cfg.merge_file(&p)?;
            }
            let json = dir.join("harness").join("config.json");
            if json.exists() {
                cfg.merge_json(&json)?;
            }
        }

        // 3. project config
        let local = PathBuf::from("harness.toml");
        if local.exists() {
            cfg.merge_file(&local)?;
        }

        // 4. explicit path wins over discovered files
        if let Some(p) = explicit {
            if !p.exists() {
                return Err(Error::Config(format!("no such config: {}", p.display())));
            }
            cfg.merge_file(p)?;
        }

        // 5. environment
        cfg.merge_env();

        Ok(cfg)
    }

    fn merge_file(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let parsed: Config = toml::from_str(&text)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
        self.overlay(parsed);
        Ok(())
    }

    fn merge_json(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let parsed: Config = serde_json::from_str(&text)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
        self.overlay(parsed);
        Ok(())
    }

    /// Overlay non-empty fields from `other` onto `self`.
    fn overlay(&mut self, other: Config) {
        if !other.llm.base_url.is_empty() {
            self.llm.base_url = other.llm.base_url;
        }
        if !other.llm.api_key.is_empty() {
            self.llm.api_key = other.llm.api_key;
        }
        if !other.llm.model.is_empty() {
            self.llm.model = other.llm.model;
        }
        if other.llm.max_tokens != default_max_tokens() {
            self.llm.max_tokens = other.llm.max_tokens;
        }
        if (other.llm.temperature - default_temperature()).abs() > f32::EPSILON {
            self.llm.temperature = other.llm.temperature;
        }
        if other.llm.timeout_secs != default_timeout() {
            self.llm.timeout_secs = other.llm.timeout_secs;
        }
        if !other.llm.headers.is_empty() {
            self.llm.headers.extend(other.llm.headers);
        }

        let a = other.agent.0;
        if a.max_turns != default_max_turns() {
            self.agent.0.max_turns = a.max_turns;
        }
        if a.max_seconds != 0 {
            self.agent.0.max_seconds = a.max_seconds;
        }
        if a.workspace != default_workspace() {
            self.agent.0.workspace = a.workspace;
        }
        if a.system_prompt_file.is_some() {
            self.agent.0.system_prompt_file = a.system_prompt_file;
        }
        if a.tool_timeout_secs != default_tool_timeout() {
            self.agent.0.tool_timeout_secs = a.tool_timeout_secs;
        }
        if a.tool_output_limit != default_tool_output() {
            self.agent.0.tool_output_limit = a.tool_output_limit;
        }
        if a.subagent_max_depth != default_subagent_depth() {
            self.agent.0.subagent_max_depth = a.subagent_max_depth;
        }
        if a.subagent_max_parallel != default_subagent_parallel() {
            self.agent.0.subagent_max_parallel = a.subagent_max_parallel;
        }

        if other.db_path != default_db() {
            self.db_path = other.db_path;
        }
    }

    fn merge_env(&mut self) {
        if let Ok(v) = std::env::var("HARNESS_BASE_URL") {
            self.llm.base_url = v;
        }
        if let Ok(v) = std::env::var("HARNESS_API_KEY") {
            self.llm.api_key = v;
        }
        if let Ok(v) = std::env::var("HARNESS_MODEL") {
            self.llm.model = v;
        }
    }

    /// Apply CLI overrides last.
    pub fn apply_overrides(
        &mut self,
        base_url: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
        workspace: Option<PathBuf>,
    ) {
        if let Some(v) = base_url {
            self.llm.base_url = v;
        }
        if let Some(v) = api_key {
            self.llm.api_key = v;
        }
        if let Some(v) = model {
            self.llm.model = v;
        }
        if let Some(v) = workspace {
            self.agent.0.workspace = v;
        }
    }

    /// Validate that the configuration is usable.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when the API key or base URL is missing.
    pub fn validate(&self) -> Result<()> {
        if self.llm.api_key.trim().is_empty() {
            return Err(Error::Config(
                "no API key: set HARNESS_API_KEY or llm.api_key in config".into(),
            ));
        }
        if self.llm.base_url.trim().is_empty() {
            return Err(Error::Config("no base_url configured".into()));
        }
        Ok(())
    }
}
