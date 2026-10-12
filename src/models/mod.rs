//! Named providers and named models.
//!
//! The operator should never have to remember an endpoint, a key, and a model
//! string in three different places. A *provider* is an endpoint plus a key; a
//! *model* is a provider, a model string, and optional overrides. Switching
//! models is then one word: `--model smart`.
//!
//! Layering is flat and predictable: `[llm]` stays supported so existing configs
//! keep working, and when named models are present they take precedence.

mod starter;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub use self::starter::{models_path, starter_models_toml};

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

impl Catalog {
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
    #[allow(dead_code)] // layered-config support; exercised by the tests below
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.providers {
            self.providers.insert(k.clone(), v.clone());
        }
        for (k, v) in &other.models {
            self.models.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests;
