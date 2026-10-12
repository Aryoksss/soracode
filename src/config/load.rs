//! Loading configuration from disk, environment, and CLI flags.

use std::path::{Path, PathBuf};

use super::{Config, ModelsFile};
use crate::error::{Error, Result};

/// Load configuration from disk and environment.
///
/// # Errors
/// Returns [`Error::Config`] when a config file exists but cannot be parsed.
pub fn load(explicit: Option<&Path>) -> Result<Config> {
    let mut cfg = Config::default();

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
    for name in ["soracode.toml", "soracode.json", "soracode.jsonc"] {
        let local = PathBuf::from(name);
        if local.exists() {
            if name.ends_with(".toml") {
                cfg.merge_file(&local)?;
            } else {
                cfg.merge_opencode(&local)?;
            }
        }
    }

    // 4. explicit path wins over discovered files
    if let Some(p) = explicit {
        if !p.exists() {
            return Err(Error::Config(format!("no such config: {}", p.display())));
        }
        if p.extension().and_then(|e| e.to_str()) == Some("toml") {
            cfg.merge_file(p)?;
        } else {
            cfg.merge_opencode(p)?;
        }
    }

    // 5. environment
    cfg.merge_env();

    Ok(cfg)
}

/// Models files in increasing precedence order.
fn models_files(explicit: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = dirs::config_dir() {
        let base = dir.join("soracode");
        out.push(base.join("models.toml"));
        out.push(base.join("soracode.json"));
        out.push(base.join("soracode.jsonc"));
    }
    out.push(PathBuf::from("soracode.models.toml"));
    out.push(PathBuf::from("soracode.json"));
    out.push(PathBuf::from("soracode.jsonc"));
    if let Some(p) = explicit {
        let sibling = p.with_file_name("soracode.models.toml");
        if sibling.exists() {
            out.push(sibling);
        }
    }
    out
}

impl Config {
    /// Load config plus any models files, then apply the environment.
    ///
    /// Models are merged as a separate layer so the operator can keep every
    /// provider and model in one file and still let project or user config
    /// override the general settings.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when a config file exists but cannot be parsed.
    pub fn load_with_models(explicit: Option<&Path>) -> Result<Self> {
        let mut cfg = load(explicit)?;

        for path in models_files(explicit) {
            if path.exists() {
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    cfg.merge_models(&path)?;
                } else {
                    cfg.merge_opencode(&path)?;
                }
            }
        }

        cfg.apply_models_default();
        cfg.merge_env();

        Ok(cfg)
    }

    /// Apply CLI overrides last.
    pub fn apply_overrides(
        &mut self,
        base_url: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
        workspace: Option<PathBuf>,
        mode: Option<crate::config::Mode>,
        wire: Option<crate::models::Wire>,
    ) {
        if let Some(v) = base_url {
            self.llm.base_url = v.clone();
            self.cli_base_url = Some(v);
        }
        if let Some(v) = api_key {
            self.llm.api_key = v.clone();
            self.cli_api_key = Some(v);
        }
        if let Some(v) = model {
            self.llm.model = v;
        }
        if let Some(v) = workspace {
            self.agent.0.workspace = v;
        }
        if let Some(m) = mode {
            self.agent.0.mode = m;
        }
        if let Some(w) = wire {
            self.cli_wire = Some(w);
        }
    }

    /// Merge a standalone models file into the provider/model maps.
    fn merge_models(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let parsed: ModelsFile =
            toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;

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

    /// Merge an OpenCode-shaped JSON/JSONC file into the provider/model maps.
    fn merge_opencode(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let oc = super::opencode::parse(&text)?;

        for (k, v) in oc.providers {
            self.providers.insert(k, v);
        }
        for (k, v) in oc.models {
            self.models.insert(k, v);
        }
        if oc.default_model.is_some() {
            self.default_model = oc.default_model;
        }
        Ok(())
    }

    /// Push the configured default model's settings into the flat `[llm]` view.
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
        let parsed: Config =
            toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
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

    fn merge_env(&mut self) {
        if let Ok(v) = std::env::var("SORACODE_BASE_URL") {
            self.llm.base_url = v;
        }
        if let Ok(v) = std::env::var("SORACODE_API_KEY") {
            self.llm.api_key = v;
        }
        if let Ok(v) = std::env::var("SORACODE_DEFAULT_MODEL") {
            self.default_model = Some(v);
        }
    }
}
