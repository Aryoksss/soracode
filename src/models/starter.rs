//! Locating, and generating, the standalone models file.

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
