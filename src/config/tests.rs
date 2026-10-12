//! Tests for configuration resolution.

use super::*;
use crate::models::Model;

fn with_models() -> Config {
    let mut c = Config::default();
    c.llm.api_key = "sk-flat".into();
    c.llm.base_url = "https://flat.test/v1".into();
    c.llm.model = "flat-model".into();
    c.providers.insert(
        "router".into(),
        crate::models::Provider {
            base_url: "https://router.test/v1".into(),
            api_key: "sk-router".into(),
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
    c
}

#[test]
fn named_model_selects_its_provider() {
    let c = with_models();
    let r = c.resolve(Some("fast")).unwrap();
    assert_eq!(r.base_url, "https://router.test/v1");
    assert_eq!(r.model, "gh/gpt-4.1");
    assert_eq!(r.api_key, "sk-router");
}

#[test]
fn slash_name_stays_literal_even_with_models() {
    let c = with_models();
    assert_eq!(
        c.selection(Some("gh/gpt-4o-mini")),
        ModelSelection::Literal("gh/gpt-4o-mini".into())
    );
}

#[test]
fn bare_unknown_name_is_a_typo_not_a_literal() {
    let c = with_models();
    assert_eq!(
        c.selection(Some("fst")),
        ModelSelection::Unknown("fst".into())
    );
    let err = c.resolve(Some("fst")).unwrap_err().to_string();
    assert!(err.contains("unknown model `fst`"));
    assert!(err.contains("fast"));
    assert!(err.contains("literal:fst"));
}

#[test]
fn literal_prefix_forces_verbatim_even_with_models() {
    let c = with_models();
    assert_eq!(
        c.selection(Some("literal:fst")),
        ModelSelection::Literal("fst".into())
    );
    assert_eq!(c.resolve(Some("literal:fst")).unwrap().model, "fst");
}

#[test]
fn without_models_everything_stays_literal() {
    let mut c = Config::default();
    c.llm.api_key = "k".into();
    c.llm.base_url = "https://flat.test/v1".into();
    assert_eq!(
        c.selection(Some("anything")),
        ModelSelection::Literal("anything".into())
    );
    assert!(c.validate(Some("anything")).is_ok());
}

#[test]
fn default_model_drives_a_plain_run() {
    let mut c = with_models();
    c.default_model = Some("fast".into());
    assert_eq!(c.selection(None), ModelSelection::Named("fast".into()));
}

#[test]
fn unknown_default_falls_back_instead_of_failing() {
    let mut c = with_models();
    c.default_model = Some("ghost".into());
    assert_eq!(c.selection(None), ModelSelection::Default);
    assert!(c.resolve(None).is_ok());
}

#[test]
fn named_model_needs_no_flat_api_key() {
    let mut c = with_models();
    c.llm.api_key = String::new();
    assert!(c.validate(Some("fast")).is_ok());
    assert!(c.validate(None).is_err());
}

#[test]
fn model_overrides_beat_flat_settings() {
    let mut c = with_models();
    c.llm.max_tokens = 111;
    c.models.insert(
        "big".into(),
        Model {
            provider: "router".into(),
            model: "m".into(),
            max_tokens: Some(999),
            temperature: Some(0.9),
            timeout_secs: Some(7),
        },
    );
    let r = c.resolve(Some("big")).unwrap();
    assert_eq!(r.max_tokens, 999);
    assert!((r.temperature - 0.9).abs() < f32::EPSILON);
    assert_eq!(r.timeout_secs, 7);
}

#[test]
fn explicit_value_equal_to_default_is_not_treated_as_absent() {
    let mut c = Config::default();
    c.llm.max_tokens = 1;
    let parsed: LlmConfig =
        toml::from_str("base_url = \"u\"\napi_key = \"k\"\nmodel = \"m\"\nmax_tokens = 16384\n")
            .unwrap();
    c.llm.overlay(&parsed);
    assert_eq!(c.llm.max_tokens, 16_384);
}

#[test]
fn requested_prefers_cli_over_environment() {
    let c = with_models();
    assert_eq!(c.requested(Some("fast")).as_deref(), Some("fast"));
}

#[test]
fn a_requested_typo_is_rejected_instead_of_silently_falling_back() {
    let c = with_models();
    assert!(c.resolve(Some("fst")).is_err());
    assert!(c.validate(Some("fst")).is_err());
}

#[test]
fn cli_base_url_overrides_a_named_providers_endpoint() {
    let mut c = with_models();
    c.cli_base_url = Some("http://127.0.0.1:9/v1".into());
    let r = c.resolve(Some("fast")).unwrap();
    assert_eq!(r.base_url, "http://127.0.0.1:9/v1");
    assert_eq!(r.api_key, "sk-router", "key stays the provider's own");
}

#[test]
fn cli_api_key_overrides_a_named_providers_key() {
    let mut c = with_models();
    c.cli_api_key = Some("sk-cli".into());
    let r = c.resolve(Some("fast")).unwrap();
    assert_eq!(r.api_key, "sk-cli");
    assert_eq!(r.base_url, "https://router.test/v1");
}
