//! Deciding which model a run uses, and checking that it is usable.

use super::{Config, ModelSelection};
use crate::error::{Error, Result};
use crate::models::{Catalog, Fallback, Resolved};

impl Config {
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
                self.catalog().resolve(
                    &name,
                    self.fallback(),
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
            ModelSelection::Unknown(name) => Err(self.unknown_model(&name)),
        }
    }

    /// Resolve the effective model request.
    ///
    /// An explicit command-line value wins; otherwise `SORACODE_MODEL` is used
    /// as the request. Routing the environment variable through the same path
    /// as `--model` means a misspelled or unknown name is rejected locally
    /// instead of being sent to the provider as a literal string.
    #[must_use]
    pub fn requested(&self, cli: Option<&str>) -> Option<String> {
        cli.map(str::to_string).or_else(|| {
            std::env::var("SORACODE_MODEL")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
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
            Some(name) if self.models.contains_key(name) => ModelSelection::Named(name.to_string()),
            Some(literal) => {
                let forced = literal.strip_prefix("literal:");
                let looks_like_typo =
                    self.models_configured() && !literal.contains('/') && !literal.contains(':');
                match forced {
                    Some(raw) => ModelSelection::Literal(raw.to_string()),
                    None if looks_like_typo => ModelSelection::Unknown(literal.to_string()),
                    None => ModelSelection::Literal(literal.to_string()),
                }
            }
            None => match &self.default_model {
                Some(name) if self.models.contains_key(name) => ModelSelection::Named(name.clone()),
                _ => ModelSelection::Default,
            },
        }
    }

    fn models_configured(&self) -> bool {
        !self.models.is_empty()
    }

    /// The model string shown in the TUI, resolved the same way a run would be.
    #[must_use]
    pub fn active_model_name(&self) -> String {
        self.resolve(None)
            .map(|r| r.model)
            .unwrap_or_else(|_| self.llm.model.clone())
    }

    /// Produce the concrete connection settings for the chosen model.
    ///
    /// # Errors
    /// Returns [`Error::Config`] when a named model cannot be resolved.
    pub fn resolve(&self, requested: Option<&str>) -> Result<Resolved> {
        match self.selection(requested) {
            ModelSelection::Named(name) => {
                let mut r = self.catalog().resolve(
                    &name,
                    self.fallback(),
                    self.llm.max_tokens,
                    self.llm.temperature,
                    self.llm.timeout_secs,
                )?;
                if let Some(u) = &self.cli_base_url {
                    r.base_url = u.clone();
                }
                if let Some(k) = &self.cli_api_key {
                    r.api_key = k.clone();
                }
                Ok(r)
            }
            ModelSelection::Literal(model) => Ok(Resolved {
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
            ModelSelection::Default => Ok(Resolved {
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
            ModelSelection::Unknown(name) => Err(self.unknown_model(&name)),
        }
    }

    fn catalog(&self) -> Catalog {
        Catalog {
            providers: self.providers.clone(),
            models: self.models.clone(),
        }
    }

    fn fallback(&self) -> Fallback {
        Fallback::from_flat(&self.llm.api_key)
    }

    fn unknown_model(&self, name: &str) -> Error {
        Error::Config(format!(
            "unknown model `{name}`; configured models: {}. \
             Use `literal:{name}` to send it verbatim.",
            self.models.keys().cloned().collect::<Vec<_>>().join(", ")
        ))
    }
}
