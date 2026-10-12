//! Configuration loading.
//!
//! Resolution order (later wins):
//!   1. Built-in defaults
//!   2. `~/.config/soracode/config.toml`
//!   3. `./soracode.toml`
//!   4. Environment variables (`SORACODE_*`)
//!   5. CLI flags
//!
//! There is deliberately **no allowlist / permission model** here: the operator
//! running soracode is the authority. Any target or command given is executed.

mod load;
mod select;
mod types;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use self::types::{AgentConfig, LlmConfig, Mode};

/// Full soracode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM connection settings. Used directly when no named model is selected.
    pub llm: LlmConfig,
    /// Agent loop settings.
    #[serde(default)]
    pub agent: AgentConfigDefaults,
    /// Where sessions are stored.
    #[serde(default = "default_db")]
    pub db_path: PathBuf,
    /// Named providers and models. When present, `--model` selects from these.
    #[serde(default)]
    pub providers: BTreeMap<String, crate::models::Provider>,
    /// Named models, keyed by the name used on the command line.
    #[serde(default)]
    pub models: BTreeMap<String, crate::models::Model>,
    /// Model used when neither `--model` nor `SORACODE_MODEL` is given.
    #[serde(default)]
    pub default_model: Option<String>,
    /// Explicit endpoint override that wins over a selected provider.
    #[serde(skip)]
    pub cli_base_url: Option<String>,
    /// Explicit key override that wins over a selected provider.
    #[serde(skip)]
    pub cli_api_key: Option<String>,
}

/// Wrapper so `agent` can be omitted entirely in config files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConfigDefaults(#[serde(default)] pub AgentConfig);

/// Shape of a standalone `models.toml`: the same maps as `Config`, plus the
/// default selector, and none of the unrelated agent settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelsFile {
    /// Providers keyed by name.
    #[serde(default)]
    pub providers: BTreeMap<String, crate::models::Provider>,
    /// Models keyed by name.
    #[serde(default)]
    pub models: BTreeMap<String, crate::models::Model>,
    /// Model used when none is requested.
    #[serde(default)]
    pub default_model: Option<String>,
}

pub(super) fn default_db() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("soracode");
    p.push("sessions.db");
    p
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            agent: AgentConfigDefaults::default(),
            db_path: default_db(),
            providers: BTreeMap::new(),
            models: BTreeMap::new(),
            default_model: None,
            cli_base_url: None,
            cli_api_key: None,
        }
    }
}

impl Config {
    /// The effective agent configuration.
    #[must_use]
    pub fn agent(&self) -> &AgentConfig {
        &self.agent.0
    }

    /// Overlay non-empty fields from `other` onto `self`.
    ///
    /// Detection is by emptiness, never by comparison against a default value:
    /// an operator who writes `max_tokens = 16384` must get 16384 even though
    /// that happens to be the built-in default, and a value of zero must be
    /// distinguishable from "not mentioned".
    pub(super) fn overlay(&mut self, other: Config) {
        self.llm.overlay(&other.llm);
        self.agent.0.overlay(&other.agent.0);

        self.providers.extend(other.providers);
        self.models.extend(other.models);

        if other.default_model.is_some() {
            self.default_model = other.default_model;
        }
        if other.db_path != default_db() {
            self.db_path = other.db_path;
        }
    }
}

/// Which name the operator asked for, and how they asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSelection {
    /// A name resolved against `[models.*]`.
    Named(String),
    /// A raw model string used with the flat `[llm]` connection.
    Literal(String),
    /// A bare name that matches no configured model, most likely a typo.
    Unknown(String),
    /// Nothing was requested; fall back to the configured default.
    Default,
}

#[cfg(test)]
mod tests;
