//! OpenCode-style config, env substitution, and provider/model selection tests.

use super::*;
use crate::config::env_expand;

const OPENCODE_JSON: &str = r#"
{
  // active model, OpenCode style
  "model": "9router/cbai/deepseek-v4.1-flash",
  "provider": {
    "9router": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "9Router",
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

    assert_eq!(oc.providers["9router"].base_url, "https://router.test/v1");
    assert_eq!(oc.providers["9router"].api_key, "sk-from-env");
    assert_eq!(oc.providers["9router"].wire, Wire::OpenAi);

    let m = &oc.models["9router/cbai/deepseek-v4.1-flash"];
    assert_eq!(m.provider, "9router");
    assert_eq!(m.model, "cbai/deepseek-v4.1-flash");
    assert_eq!(m.max_tokens, Some(384000));
    assert_eq!(
        oc.default_model.as_deref(),
        Some("9router/cbai/deepseek-v4.1-flash")
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
        "9router".into(),
        crate::models::Provider {
            base_url: "https://router.test/v1".into(),
            api_key: "sk".into(),
            wire: Wire::default(),
            headers: BTreeMap::new(),
        },
    );
    c.models.insert(
        "9router/cbai/deepseek-v4.1-flash".into(),
        Model {
            provider: "9router".into(),
            model: "cbai/deepseek-v4.1-flash".into(),
            ..Default::default()
        },
    );
    let r = c.resolve(Some("9router/cbai/deepseek-v4.1-flash")).unwrap();
    assert_eq!(r.provider, "9router");
    assert_eq!(r.model, "cbai/deepseek-v4.1-flash");
    assert_eq!(r.base_url, "https://router.test/v1");
}

#[test]
fn jsonc_comments_and_trailing_commas_are_stripped() {
    let oc = crate::config::opencode::parse(OPENCODE_JSON).unwrap();
    assert!(oc.providers.contains_key("9router"));
}
