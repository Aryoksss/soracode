//! OpenCode-style config, env substitution, and provider/model selection tests.

use super::*;
use crate::config::env_expand;

const OPENCODE_JSON: &str = r#"
{
  // active model, OpenCode style
  "model": "example/cbai/deepseek-v4.1-flash",
  "provider": {
    "example": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Example",
      "options": {
        "baseURL": "https://router.test/v1",
        "apiKey": "{env:TEST_SORACODE_KEY}"
      },
      "models": {
        "cbai/deepseek-v4.1-flash": {
          "name": "DeepSeek V4.1 Flash",
          "limit": { "context": 1000000, "output": 384000 },
        }
      }
    }
  }
}
"#;

#[test]
fn opencode_json_parses_providers_and_models() {
    std::env::set_var("TEST_SORACODE_KEY", "sk-from-env");
    let oc = crate::config::opencode::parse(OPENCODE_JSON).unwrap();
    std::env::remove_var("TEST_SORACODE_KEY");

    assert_eq!(oc.providers["example"].base_url, "https://router.test/v1");
    assert_eq!(oc.providers["example"].api_key, "sk-from-env");
    assert_eq!(oc.providers["example"].wire, Wire::OpenAi);

    let m = &oc.models["example/cbai/deepseek-v4.1-flash"];
    assert_eq!(m.provider, "example");
    assert_eq!(m.model, "cbai/deepseek-v4.1-flash");
    assert_eq!(m.max_tokens, Some(384000));
    assert_eq!(
        oc.default_model.as_deref(),
        Some("example/cbai/deepseek-v4.1-flash")
    );
}

#[test]
fn opencode_wire_and_env_are_honoured() {
    let text = r#"
    { "provider": { "anth": {
        "options": { "baseURL": "https://api.anthropic.com/v1", "wire": "anthropic", "apiKey": "k" },
        "models": { "claude": {} }
    } } }
    "#;
    let oc = crate::config::opencode::parse(text).unwrap();
    assert_eq!(oc.providers["anth"].wire, Wire::Anthropic);
}

#[test]
fn opencode_rejects_unknown_wire() {
    let text = r#"{ "provider": { "x": { "options": { "wire": "bogus" } } } }"#;
    assert!(crate::config::opencode::parse(text).is_err());
}

#[test]
fn env_expand_replaces_and_tolerates_missing() {
    std::env::set_var("TEST_SORACODE_BASE", "https://x.test");
    assert_eq!(
        env_expand("${env:TEST_SORACODE_BASE}/v1"),
        "${env:TEST_SORACODE_BASE}/v1"
    );
    assert_eq!(
        env_expand("{env:TEST_SORACODE_BASE}/v1"),
        "https://x.test/v1"
    );
    assert_eq!(env_expand("{env:TEST_SORACODE_ABSENT}"), "");
    assert_eq!(env_expand("plain"), "plain");
    std::env::remove_var("TEST_SORACODE_BASE");
}

#[test]
fn provider_slash_model_key_is_selectable_by_name() {
    let mut c = Config::default();
    c.providers.insert(
        "example".into(),
        crate::models::Provider {
            base_url: "https://router.test/v1".into(),
            api_key: "sk".into(),
            wire: Wire::default(),
            headers: BTreeMap::new(),
        },
    );
    c.models.insert(
        "example/cbai/deepseek-v4.1-flash".into(),
        Model {
            provider: "example".into(),
            model: "cbai/deepseek-v4.1-flash".into(),
            ..Default::default()
        },
    );
    let r = c.resolve(Some("example/cbai/deepseek-v4.1-flash")).unwrap();
    assert_eq!(r.provider, "example");
    assert_eq!(r.model, "cbai/deepseek-v4.1-flash");
    assert_eq!(r.base_url, "https://router.test/v1");
}

#[test]
fn jsonc_comments_and_trailing_commas_are_stripped() {
    let oc = crate::config::opencode::parse(OPENCODE_JSON).unwrap();
    assert!(oc.providers.contains_key("example"));
}
