//! Reading OpenCode-shaped config: `soracode.json` / `soracode.jsonc`.
//!
//! The shape mirrors OpenCode so an operator who already knows it can point
//! soracode at the same providers. A provider carries its endpoint, key and
//! dialect under `options`; each entry under `models` becomes a selectable
//! model named `provider/model-id`, which is also the request string.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::models::{Model, Provider, Wire};

use super::env_expand;

/// One provider block as written in the file.
#[derive(Debug, Default, Deserialize)]
struct OcProvider {
    #[serde(default)]
    options: OcOptions,
    #[serde(default)]
    models: BTreeMap<String, OcModel>,
}

#[derive(Debug, Default, Deserialize)]
struct OcOptions {
    #[serde(default, rename = "baseURL")]
    base_url: String,
    #[serde(default, rename = "apiKey")]
    api_key: String,
    /// Dialect name; `openai` (default), `anthropic`, or `google`.
    #[serde(default)]
    wire: Option<String>,
    /// Extra request headers, as a JSON object.
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct OcModel {
    #[serde(default)]
    limit: OcLimit,
    /// Optional per-model overrides, accepted for convenience.
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
struct OcLimit {
    #[serde(default)]
    output: Option<u32>,
}

/// The whole OpenCode-shaped document.
#[derive(Debug, Default, Deserialize)]
struct OcFile {
    /// Active model as `provider/model-id`, or a bare name.
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: BTreeMap<String, OcProvider>,
    #[serde(default)]
    mcp: BTreeMap<String, OcMcp>,
}

#[derive(Debug, Default, Deserialize)]
struct OcMcp {
    #[serde(default)]
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    disabled: bool,
}

/// What a parsed OpenCode file contributes to the configuration.
#[derive(Debug, Default)]
pub(super) struct OpenCode {
    /// Providers keyed by name.
    pub(super) providers: BTreeMap<String, Provider>,
    /// Models keyed by `provider/model-id`.
    pub(super) models: BTreeMap<String, Model>,
    /// The `model` key from the file, if any.
    pub(super) default_model: Option<String>,
    /// MCP servers keyed by name.
    pub(super) mcp: BTreeMap<String, super::McpServer>,
}

/// Parse an OpenCode-shaped JSON or JSONC document.
///
/// `{env:VAR}` references inside string values are expanded from the
/// environment, so a key never has to be written into the file.
///
/// # Errors
/// Returns [`Error::Config`] when the document is not valid JSON/JSONC.
pub(super) fn parse(text: &str) -> Result<OpenCode> {
    let json = strip_jsonc(text);
    let file: OcFile =
        serde_json::from_str(&json).map_err(|e| Error::Config(format!("soracode.json: {e}")))?;

    let mut out = OpenCode::default();
    for (name, p) in file.provider {
        let wire = match p.options.wire.as_deref() {
            Some(w) => Wire::parse(w).ok_or_else(|| {
                Error::Config(format!(
                    "provider `{name}`: unknown wire `{w}` — use openai|anthropic|google"
                ))
            })?,
            None => Wire::default(),
        };
        out.providers.insert(
            name.clone(),
            Provider {
                base_url: env_expand(&p.options.base_url),
                wire,
                api_key: env_expand(&p.options.api_key),
                headers: p
                    .options
                    .headers
                    .into_iter()
                    .map(|(k, v)| (k, env_expand(&v)))
                    .collect(),
            },
        );

        for (id, m) in p.models {
            let key = format!("{name}/{id}");
            out.models.insert(
                key.clone(),
                Model {
                    provider: name.clone(),
                    model: id,
                    max_tokens: m.max_tokens.or(m.limit.output),
                    temperature: m.temperature,
                    timeout_secs: None,
                },
            );
        }
    }
    out.default_model = file.model.map(|m| env_expand(&m));
    for (name, m) in file.mcp {
        out.mcp.insert(
            name,
            super::McpServer {
                command: env_expand(&m.command),
                args: m.args.iter().map(|a| env_expand(a)).collect(),
                env: m
                    .env
                    .into_iter()
                    .map(|(k, v)| (k, env_expand(&v)))
                    .collect(),
                disabled: m.disabled,
            },
        );
    }
    Ok(out)
}

/// Remove `//` and `/* */` comments and trailing commas so a JSONC file parses
/// as JSON. String contents are preserved verbatim.
fn strip_jsonc(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
        } else if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    drop_trailing_commas(&out)
}

/// Drop a comma that sits just before a closing brace or bracket.
fn drop_trailing_commas(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}
