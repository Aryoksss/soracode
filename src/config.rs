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

use std::collections::BTreeMap;
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
}

/// Wrapper so `agent` can be omitted entirely in config files.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_db() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("soracode");
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
            providers: BTreeMap::new(),
            models: BTreeMap::new(),
            default_model: None,
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

impl ModelSelection {
    /// Is this selection usable without further configuration?
    #[must_use]
    pub fn is_usable(&self) -> bool {
        !matches!(self, Self::Unknown(_))
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
            let p = dir.join("soracode").join("config.toml");
            if p.exists() {
                cfg.merge_file(&p)?;
            }
            let json = dir.join("soracode").join("config.json");
            if json.exists() {
                cfg.merge_json(&json)?;
            }
        }

        // 3. project config
        let local = PathBuf::from("soracode.toml");
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

    /// Load config plus any models files, then apply the environment.
    ///
    /// Models are merged as a separate layer so the operator can keep every
    /// provider and model in one file and still let project or user config
    /// override the general settings.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when a config file exists but cannot be parsed.
    pub fn load_with_models(explicit: Option<&Path>) -> Result<Self> {
        let mut cfg = Self::load(explicit)?;

        for path in Self::models_files(explicit) {
            if path.exists() {
                cfg.merge_models(&path)?;
            }
        }

        cfg.apply_models_default();
        cfg.merge_env();

        Ok(cfg)
    }

    /// Models files in increasing precedence order.
    fn models_files(explicit: Option<&Path>) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(dir) = dirs::config_dir() {
            out.push(dir.join("soracode").join("models.toml"));
        }
        out.push(PathBuf::from("soracode.models.toml"));
        if let Some(p) = explicit {
            let sibling = p.with_file_name("soracode.models.toml");
            if sibling.exists() {
                out.push(sibling);
            }
        }
        out
    }

    fn merge_models(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let parsed: ModelsFile = toml::from_str(&text)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;

        for (k, v) in parsed.providers {
            self.providers.insert(k, v);
        }
        for (k, v) in parsed.models {
            self.models.insert(k, v);
        }
        if parsed.default_model.is_some() {
            self.default_model = parsed.default_model;
        }
        Ok(())
    }

    fn apply_models_default(&mut self) {
        let Some(name) = self.default_model.clone() else {
            return;
        };
        let Some(m) = self.models.get(&name) else {
            return;
        };
        let Some(p) = self.providers.get(&m.provider) else {
            return;
        };
        self.llm.base_url.clone_from(&p.base_url);
        if !p.api_key.is_empty() {
            self.llm.api_key.clone_from(&p.api_key);
        }
        self.llm.model.clone_from(&m.model);
        if let Some(v) = m.max_tokens {
            self.llm.max_tokens = v;
        }
        if let Some(v) = m.temperature {
            self.llm.temperature = v;
        }
        if let Some(v) = m.timeout_secs {
            self.llm.timeout_secs = v;
        }
        self.llm.headers.extend(p.headers.clone());
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
    ///
    /// Detection is by emptiness, never by comparison against a default value:
    /// an operator who writes `max_tokens = 16384` must get 16384 even though
    /// that happens to be the built-in default, and a value of zero must be
    /// distinguishable from "not mentioned".
    fn overlay(&mut self, other: Config) {
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

    fn merge_env(&mut self) {
        if let Ok(v) = std::env::var("SORACODE_BASE_URL") {
            self.llm.base_url = v;
        }
        if let Ok(v) = std::env::var("SORACODE_API_KEY") {
            self.llm.api_key = v;
        }
        if let Ok(v) = std::env::var("SORACODE_MODEL") {
            self.llm.model = v;
        }
        if let Ok(v) = std::env::var("SORACODE_DEFAULT_MODEL") {
            self.default_model = Some(v);
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

    /// Validate that the configuration is usable for the chosen model.
    ///
    /// A named model carries its own endpoint and possibly its own key, so the
    /// flat `[llm]` settings are only required when no named model is selected.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when the resolved model has no usable endpoint
    /// or the flat configuration is incomplete.
    pub fn validate(&self, requested: Option<&str>) -> Result<()> {
        match self.selection(requested) {
            ModelSelection::Named(name) => {
                let catalog = crate::models::Catalog {
                    providers: self.providers.clone(),
                    models: self.models.clone(),
                };
                let fallback = crate::models::Fallback::from_flat(&self.llm.api_key);
                catalog.resolve(
                    &name,
                    fallback,
                    self.llm.max_tokens,
                    self.llm.temperature,
                    self.llm.timeout_secs,
                )?;
                Ok(())
            }
            ModelSelection::Literal(_) | ModelSelection::Default => {
                if self.llm.api_key.trim().is_empty() {
                    return Err(Error::Config(
                        "no API key: set SORACODE_API_KEY, llm.api_key, or a provider api_key"
                            .into(),
                    ));
                }
                if self.llm.base_url.trim().is_empty() {
                    return Err(Error::Config("no base_url configured".into()));
                }
                Ok(())
            }
            ModelSelection::Unknown(name) => Err(Error::Config(format!(
                "unknown model `{name}`; configured models: {}. \
                 Use `literal:{name}` to send it verbatim.",
                self.models.keys().cloned().collect::<Vec<_>>().join(", ")
            ))),
        }
    }

    /// Decide what `--model`/`SORACODE_MODEL` meant.
    ///
    /// A request matching a configured model name selects it. A raw identifier
    /// such as `gh/gpt-4.1` is taken literally, so the flag keeps working with
    /// no models file. Once models are configured, a bare word that matches
    /// nothing is treated as a typo rather than silently sent to the provider,
    /// because that failure is otherwise a confusing 404. Use a slash or the
    /// `literal:` prefix to force a literal name.
    #[must_use]
    pub fn selection(&self, requested: Option<&str>) -> ModelSelection {
        match requested.map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) if self.models.contains_key(name) => {
                ModelSelection::Named(name.to_string())
            }
            Some(literal) => {
                let forced = literal.strip_prefix("literal:");
                let looks_like_typo = self.models_configured()
                    && !literal.contains('/')
                    && !literal.contains(':');
                match forced {
                    Some(raw) => ModelSelection::Literal(raw.to_string()),
                    None if looks_like_typo => ModelSelection::Unknown(literal.to_string()),
                    None => ModelSelection::Literal(literal.to_string()),
                }
            }
            None => match &self.default_model {
                Some(name) if self.models.contains_key(name) => {
                    ModelSelection::Named(name.clone())
                }
                _ => ModelSelection::Default,
            },
        }
    }

    fn models_configured(&self) -> bool {
        !self.models.is_empty()
    }

    /// Produce the concrete connection settings for the chosen model.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when a named model cannot be resolved.
    pub fn resolve(&self, requested: Option<&str>) -> Result<crate::models::Resolved> {
        match self.selection(requested) {
            ModelSelection::Named(name) => {
                let catalog = crate::models::Catalog {
                    providers: self.providers.clone(),
                    models: self.models.clone(),
                };
                let fallback = crate::models::Fallback::from_flat(&self.llm.api_key);
                catalog.resolve(
                    &name,
                    fallback,
                    self.llm.max_tokens,
                    self.llm.temperature,
                    self.llm.timeout_secs,
                )
            }
            ModelSelection::Literal(model) => Ok(crate::models::Resolved {
                name: model.clone(),
                provider: "<inline>".into(),
                base_url: self.llm.base_url.clone(),
                api_key: self.llm.api_key.clone(),
                headers: self.llm.headers.clone(),
                model,
                max_tokens: self.llm.max_tokens,
                temperature: self.llm.temperature,
                timeout_secs: self.llm.timeout_secs,
            }),
            ModelSelection::Default => Ok(crate::models::Resolved {
                name: self.llm.model.clone(),
                provider: "<default>".into(),
                base_url: self.llm.base_url.clone(),
                api_key: self.llm.api_key.clone(),
                headers: self.llm.headers.clone(),
                model: self.llm.model.clone(),
                max_tokens: self.llm.max_tokens,
                temperature: self.llm.temperature,
                timeout_secs: self.llm.timeout_secs,
            }),
            ModelSelection::Unknown(name) => Err(Error::Config(format!(
                "unknown model `{name}`; configured models: {}. \
                 Use `literal:{name}` to send it verbatim.",
                self.models.keys().cloned().collect::<Vec<_>>().join(", ")
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Model;

    fn with_models() -> Config {
        let mut c = Config::default();
        c.llm.api_key = "sk-flat".into();
        c.llm.base_url = "https://flat.test/v1".into();
        c.llm.model = "flat-model".into();
        c.providers.insert(
            "router".into(),
            crate::models::Provider {
                base_url: "https://router.test/v1".into(),
                api_key: "sk-router".into(),
                headers: BTreeMap::new(),
            },
        );
        c.models.insert(
            "fast".into(),
            Model {
                provider: "router".into(),
                model: "gh/gpt-4.1".into(),
                ..Default::default()
            },
        );
        c
    }

    #[test]
    fn named_model_selects_its_provider() {
        let c = with_models();
        let r = c.resolve(Some("fast")).unwrap();
        assert_eq!(r.base_url, "https://router.test/v1");
        assert_eq!(r.model, "gh/gpt-4.1");
        assert_eq!(r.api_key, "sk-router");
    }

    #[test]
    fn slash_name_stays_literal_even_with_models() {
        let c = with_models();
        assert_eq!(
            c.selection(Some("gh/gpt-4o-mini")),
            ModelSelection::Literal("gh/gpt-4o-mini".into())
        );
    }

    #[test]
    fn bare_unknown_name_is_a_typo_not_a_literal() {
        let c = with_models();
        assert_eq!(
            c.selection(Some("fst")),
            ModelSelection::Unknown("fst".into())
        );
        let err = c.resolve(Some("fst")).unwrap_err().to_string();
        assert!(err.contains("unknown model `fst`"));
        assert!(err.contains("fast"));
        assert!(err.contains("literal:fst"));
    }

    #[test]
    fn literal_prefix_forces_verbatim_even_with_models() {
        let c = with_models();
        assert_eq!(
            c.selection(Some("literal:fst")),
            ModelSelection::Literal("fst".into())
        );
        assert_eq!(c.resolve(Some("literal:fst")).unwrap().model, "fst");
    }

    #[test]
    fn without_models_everything_stays_literal() {
        let mut c = Config::default();
        c.llm.api_key = "k".into();
        c.llm.base_url = "https://flat.test/v1".into();
        assert_eq!(
            c.selection(Some("anything")),
            ModelSelection::Literal("anything".into())
        );
        assert!(c.validate(Some("anything")).is_ok());
    }

    #[test]
    fn default_model_drives_a_plain_run() {
        let mut c = with_models();
        c.default_model = Some("fast".into());
        assert_eq!(
            c.selection(None),
            ModelSelection::Named("fast".into())
        );
    }

    #[test]
    fn unknown_default_falls_back_instead_of_failing() {
        let mut c = with_models();
        c.default_model = Some("ghost".into());
        assert_eq!(c.selection(None), ModelSelection::Default);
        assert!(c.resolve(None).is_ok());
    }

    #[test]
    fn named_model_needs_no_flat_api_key() {
        let mut c = with_models();
        c.llm.api_key = String::new();
        assert!(c.validate(Some("fast")).is_ok());
        assert!(c.validate(None).is_err());
    }

    #[test]
    fn model_overrides_beat_flat_settings() {
        let mut c = with_models();
        c.llm.max_tokens = 111;
        c.models.insert(
            "big".into(),
            Model {
                provider: "router".into(),
                model: "m".into(),
                max_tokens: Some(999),
                temperature: Some(0.9),
                timeout_secs: Some(7),
            },
        );
        let r = c.resolve(Some("big")).unwrap();
        assert_eq!(r.max_tokens, 999);
        assert!((r.temperature - 0.9).abs() < f32::EPSILON);
        assert_eq!(r.timeout_secs, 7);
    }

    #[test]
    fn explicit_value_equal_to_default_is_not_treated_as_absent() {
        let mut c = Config::default();
        c.llm.max_tokens = 1;
        let parsed: LlmConfig = toml::from_str(
            "base_url = \"u\"\napi_key = \"k\"\nmodel = \"m\"\nmax_tokens = 16384\n",
        )
        .unwrap();
        c.llm.overlay(&parsed);
        assert_eq!(c.llm.max_tokens, 16_384);
    }
}
