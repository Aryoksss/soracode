//! Tests for provider/model resolution and the starter file.

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
