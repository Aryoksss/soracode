//! Named providers and named models.
//!
//! The operator should never have to remember an endpoint, a key, and a model
//! string in three different places. A *provider* is an endpoint plus a key; a
//! *model* is a provider, a model string, and optional overrides. Switching
//! models is then one word: `--model smart`.
//!
//! Layering is flat and predictable: `[llm]` stays supported so existing configs
//! keep working, and when named models are present they take precedence.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// An endpoint plus credentials, referenced by name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provider {
    /// OpenAI-compatible base URL, e.g. `https://9router.example/v1`.
    pub base_url: String,
    /// Bearer token. May be empty for local servers that ignore auth.
    #[serde(default)]
    pub api_key: String,
    /// Extra headers sent on every request to this provider.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

/// A selectable model: which provider, which model string, and any overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Model {
    /// Name of the provider this model runs on.
    pub provider: String,
    /// Model identifier sent to the provider, e.g. `gh/gpt-4.1`.
    pub model: String,
    /// Optional per-model overrides; `None` means inherit the global setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Optional sampling temperature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional request timeout in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// The full model registry, named exactly as it appears in the config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    /// Providers keyed by name.
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
    /// Models keyed by name.
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
}

/// Everything needed to open a connection, after a model has been selected.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Which model name was chosen.
    pub name: String,
    /// Which provider it came from.
    pub provider: String,
    /// Endpoint to call.
    pub base_url: String,
    /// Credential to present.
    pub api_key: String,
    /// Provider-level extra headers.
    pub headers: BTreeMap<String, String>,
    /// Model string sent in the request body.
    pub model: String,
    /// Effective token cap.
    pub max_tokens: u32,
    /// Effective temperature.
    pub temperature: f32,
    /// Effective timeout.
    pub timeout_secs: u64,
}

impl Catalog {
    /// Is any named model configured?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Names of every configured model, sorted.
    #[must_use]
    pub fn model_names(&self) -> Vec<&str> {
        self.models.keys().map(String::as_str).collect()
    }

    /// Names of every configured provider, sorted.
    #[must_use]
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.keys().map(String::as_str).collect()
    }

    /// Resolve a model name into a complete connection description.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when the model is unknown, its provider is
    /// unknown, or either is missing required fields.
    pub fn resolve(
        &self,
        name: &str,
        fallback: Fallback,
        max_tokens: u32,
        temperature: f32,
        timeout_secs: u64,
    ) -> Result<Resolved> {
        let Some(model) = self.models.get(name) else {
            let known = self.model_names().join(", ");
            return Err(Error::Config(format!(
                "unknown model `{name}`{}",
                if known.is_empty() {
                    String::new()
                } else {
                    format!("; configured models: {known}")
                }
            )));
        };

        let Some(provider) = self.providers.get(&model.provider) else {
            let known = self.provider_names().join(", ");
            return Err(Error::Config(format!(
                "model `{name}` references unknown provider `{}`{}",
                model.provider,
                if known.is_empty() {
                    String::new()
                } else {
                    format!("; configured providers: {known}")
                }
            )));
        };

        if provider.base_url.trim().is_empty() {
            return Err(Error::Config(format!(
                "provider `{}` has no base_url",
                model.provider
            )));
        }
        if model.model.trim().is_empty() {
            return Err(Error::Config(format!("model `{name}` has no model string")));
        }

        let api_key = if provider.api_key.trim().is_empty() {
            fallback.api_key
        } else {
            provider.api_key.clone()
        };

        Ok(Resolved {
            name: name.to_string(),
            provider: model.provider.clone(),
            base_url: provider.base_url.clone(),
            api_key,
            headers: provider.headers.clone(),
            model: model.model.clone(),
            max_tokens: model.max_tokens.unwrap_or(max_tokens),
            temperature: model.temperature.unwrap_or(temperature),
            timeout_secs: model.timeout_secs.unwrap_or(timeout_secs),
        })
    }

    /// Merge `other` into `self`, with `other` winning on conflicts.
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.providers {
            self.providers.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.models {
            self.models.insert(k.clone(), v.clone());
        }
    }
}

/// Values used when a named model or provider omits a credential.
#[derive(Debug, Clone, Default)]
pub struct Fallback {
    /// Credential from the legacy `[llm]` block or the environment.
    pub api_key: String,
}

impl Fallback {
    /// Build a fallback from the flat `[llm]` settings.
    #[must_use]
    pub fn from_flat(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
        }
    }
}

/// Where the model file lives when the operator wants one place to edit.
#[must_use]
pub fn models_path() -> std::path::PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("soracode");
    p.push("models.toml");
    p
}

/// The file written by `--init-models`.
#[must_use]
pub fn starter_models_toml() -> String {
    let router = std::env::var("SORACODE_BASE_URL")
        .unwrap_or_else(|_| "https://9router.aryoks.tech/v1".to_string());
    format!(
        r#"# soracode models
#
# Switch models with `--model <name>`, or set `default_model` below.
# List what is configured with `soracode --models`.
#
# A provider is an endpoint plus a key. A model points at a provider and names
# the remote model string. Add as many of either as you like; nothing else in
# the config needs to change.

default_model = "fast"

[providers.router]
base_url = "{router}"
api_key  = "put-your-key-here"

# Any OpenAI-compatible server works, including a local one.
# [providers.local]
# base_url = "http://127.0.0.1:1234/v1"
# api_key  = "lm-studio"

# [providers.anthropic]
# base_url = "https://api.anthropic.com/v1"
# api_key  = "sk-ant-..."

[models.fast]
provider = "router"
model    = "gh/gpt-4.1"
max_tokens = 16384

[models.smart]
provider = "router"
model    = "kr/deepseek-3.2"
max_tokens = 32768
temperature = 0.2

# Point a model at a different provider at any time; this is the whole change.
# [models.local-fast]
# provider = "local"
# model    = "qwen2.5-coder-32b"
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        let mut c = Catalog::default();
        c.providers.insert(
            "router".into(),
            Provider {
                base_url: "https://example.test/v1".into(),
                api_key: "sk-test".into(),
                headers: BTreeMap::new(),
            },
        );
        c.providers.insert(
            "local".into(),
            Provider {
                base_url: "http://127.0.0.1:1234/v1".into(),
                api_key: String::new(),
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
        c.models.insert(
            "smart".into(),
            Model {
                provider: "router".into(),
                model: "kr/deepseek-3.2".into(),
                max_tokens: Some(999),
                temperature: Some(0.1),
                timeout_secs: None,
            },
        );
        c.models.insert(
            "local".into(),
            Model {
                provider: "local".into(),
                model: "qwen".into(),
                ..Default::default()
            },
        );
        c
    }

    #[test]
    fn resolves_provider_and_model_string() {
        let r = catalog()
            .resolve("fast", Fallback::default(), 100, 0.3, 60)
            .unwrap();
        assert_eq!(r.base_url, "https://example.test/v1");
        assert_eq!(r.model, "gh/gpt-4.1");
        assert_eq!(r.api_key, "sk-test");
    }

    #[test]
    fn per_model_overrides_win_over_globals() {
        let r = catalog()
            .resolve("smart", Fallback::default(), 100, 0.3, 60)
            .unwrap();
        assert_eq!(r.max_tokens, 999);
        assert!((r.temperature - 0.1).abs() < f32::EPSILON);
        assert_eq!(r.timeout_secs, 60);
    }

    #[test]
    fn empty_provider_key_falls_back_to_flat_key() {
        let r = catalog()
            .resolve("local", Fallback::from_flat("sk-global"), 100, 0.3, 60)
            .unwrap();
        assert_eq!(r.api_key, "sk-global");
    }

    #[test]
    fn unknown_model_lists_what_is_available() {
        let err = catalog()
            .resolve("nope", Fallback::default(), 100, 0.3, 60)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown model `nope`"));
        assert!(err.contains("fast"));
        assert!(err.contains("smart"));
    }

    #[test]
    fn unknown_provider_is_reported() {
        let mut c = Catalog::default();
        c.models.insert(
            "broken".into(),
            Model {
                provider: "ghost".into(),
                model: "x".into(),
                ..Default::default()
            },
        );
        let err = c
            .resolve("broken", Fallback::default(), 100, 0.3, 60)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown provider `ghost`"));
    }

    #[test]
    fn merge_lets_later_files_add_models() {
        let mut base = catalog();
        let mut extra = Catalog::default();
        extra.providers.insert(
            "other".into(),
            Provider {
                base_url: "https://other.test/v1".into(),
                api_key: "k".into(),
                headers: BTreeMap::new(),
            },
        );
        extra.models.insert(
            "added".into(),
            Model {
                provider: "other".into(),
                model: "m".into(),
                ..Default::default()
            },
        );
        base.merge(&extra);
        assert!(base.model_names().contains(&"added"));
        assert_eq!(base.model_names().len(), 4);
    }

    #[test]
    fn starter_file_is_valid_toml_with_a_default_model() {
        let parsed: Catalog = toml::from_str(&starter_models_toml()).unwrap();
        assert_eq!(parsed.model_names().len(), 2);
        assert!(parsed.models.contains_key("fast"));
    }
}

/// Read the `default_model` key out of a models file before full parsing, so a
/// broken model entry does not hide which model was meant to be used.
///
/// # Errors
/// Returns [`Error::Config`] when the file exists but is not valid TOML.
pub fn read_default_model(path: &std::path::Path) -> Result<Option<String>> {
    let text = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    Ok(value
        .get("default_model")
        .and_then(toml::Value::as_str)
        .map(str::to_string))
}
