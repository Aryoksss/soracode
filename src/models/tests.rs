//! Tests for provider/model resolution and the starter file.

use super::*;

fn catalog() -> Catalog {
    let mut c = Catalog::default();
    c.providers.insert(
        "router".into(),
        Provider {
            base_url: "https://example.test/v1".into(),
            api_key: "sk-test".into(),
            wire: Wire::default(),
            headers: BTreeMap::new(),
        },
    );
    c.providers.insert(
        "local".into(),
        Provider {
            base_url: "http://127.0.0.1:1234/v1".into(),
            api_key: String::new(),
            wire: Wire::default(),
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
            wire: Wire::default(),
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
fn starter_file_is_valid_jsonc_and_matches_the_opencode_shape() {
    let text = starter_models_jsonc();
    let dir = std::env::temp_dir().join("soracode-starter-jsonc");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("soracode.jsonc");
    std::fs::write(&path, &text).unwrap();

    let cfg = crate::config::Config::load_with_models(Some(&path)).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    let provider = cfg.providers.get("9router").expect("router provider");
    assert_eq!(
        provider.base_url,
        std::env::var("SORACODE_BASE_URL")
            .unwrap_or_else(|_| "https://9router.aryoks.tech/v1".into())
    );
    assert!(cfg.models.contains_key("9router/gh/gpt-4.1"));
    assert!(cfg.models.contains_key("9router/kr/deepseek-3.2"));
    let m = &cfg.models["9router/gh/gpt-4.1"];
    assert_eq!(m.provider, "9router");
    assert_eq!(m.max_tokens, Some(16384));
}

#[test]
fn wire_names_parse_and_round_trip() {
    assert_eq!(Wire::parse("anthropic"), Some(Wire::Anthropic));
    assert_eq!(Wire::parse("  GEMINI "), Some(Wire::Google));
    assert_eq!(Wire::parse("openai"), Some(Wire::OpenAi));
    assert_eq!(Wire::parse("nope"), None);
    assert_eq!(Wire::default(), Wire::OpenAi);
    assert_eq!(Wire::Google.as_str(), "google");
}

#[test]
fn provider_wire_defaults_to_openai_when_absent() {
    let toml = r#"
        [providers.plain]
        base_url = "https://x.test/v1"
        api_key = "k"
    "#;
    let file: crate::config::ModelsFile = toml::from_str(toml).unwrap();
    assert_eq!(file.providers["plain"].wire, Wire::OpenAi);
}

#[test]
fn provider_wire_is_read_from_config() {
    let toml = r#"
        [providers.anth]
        base_url = "https://api.anthropic.com/v1"
        wire = "anthropic"
        api_key = "k"
    "#;
    let file: crate::config::ModelsFile = toml::from_str(toml).unwrap();
    assert_eq!(file.providers["anth"].wire, Wire::Anthropic);
}
