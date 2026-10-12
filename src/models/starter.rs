//! Locating, and generating, the standalone models file.

/// Where the model file lives when the operator wants one place to edit.
#[must_use]
pub fn models_path() -> std::path::PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("soracode");
    p.push("soracode.jsonc");
    p
}

/// The file written by `--init-models`.
///
/// The shape matches OpenCode's `provider`/`models` layout, so an operator who
/// already knows that format can point soracode at the same endpoints. Comments
/// and trailing commas are allowed, and `{env:VAR}` keeps keys out of the file.
#[must_use]
pub fn starter_models_jsonc() -> String {
    let router = std::env::var("SORACODE_BASE_URL")
        .unwrap_or_else(|_| "https://9router.aryoks.tech/v1".to_string());
    format!(
        r#"{{
  // soracode models — OpenCode-compatible layout.
  //
  // Switch models with `-m <provider>/<model-id>`, or set `"model"` below.
  // List what is configured with `soracode --models`.
  //
  // A provider is an endpoint plus a key. Each entry under `models` becomes a
  // selectable model named `<provider>/<model-id>`. Add as many as you like;
  // nothing else in the config changes.

  "model": "9router/gh/gpt-4.1",

  "provider": {{
    "9router": {{
      "npm": "@ai-sdk/openai-compatible",
      "name": "9Router",
      "options": {{
        "baseURL": "{router}",
        // Read the key from the environment so it never lands in this file.
        "apiKey": "{{env:NINEROUTER_API_KEY}}"
      }},
      "models": {{
        "gh/gpt-4.1": {{
          "name": "GPT-4.1",
          "limit": {{ "output": 16384 }}
        }},
        "kr/deepseek-3.2": {{
          "name": "DeepSeek 3.2",
          "limit": {{ "output": 32768 }},
          "temperature": 0.2
        }}
      }}
    }}

    // A local server is the same block with a different baseURL.
    // "local": {{
    //   "options": {{ "baseURL": "http://127.0.0.1:1234/v1", "apiKey": "lm-studio" }},
    //   "models": {{ "qwen2.5-coder-32b": {{ "name": "Qwen Coder" }} }}
    // }}

    // Native Anthropic or Google: only `wire` changes.
    // "anthropic": {{
    //   "options": {{ "baseURL": "https://api.anthropic.com/v1", "wire": "anthropic",
    //                "apiKey": "{{env:ANTHROPIC_API_KEY}}" }},
    //   "models": {{ "claude-sonnet-4-5": {{ "name": "Claude Sonnet 4.5" }} }}
    // }},
    // "gemini": {{
    //   "options": {{ "baseURL": "https://generativelanguage.googleapis.com/v1beta",
    //                "wire": "google", "apiKey": "{{env:GEMINI_API_KEY}}" }},
    //   "models": {{ "gemini-3-flash": {{ "name": "Gemini 3 Flash" }} }}
    // }}
  }}
}}
"#
    )
}
