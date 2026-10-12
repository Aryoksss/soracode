//! Tests for the `@` file-mention helpers in `keys.rs`.

use super::mention::{at_token, matching_files};

#[test]
fn token_is_the_last_at_word() {
    assert_eq!(at_token("see @src/ma"), Some("src/ma"));
    assert_eq!(at_token("x @"), Some(""));
    assert_eq!(at_token("@top"), Some("top"));
}

#[test]
fn token_needs_a_word_boundary() {
    assert_eq!(at_token("a@b"), None);
    assert_eq!(at_token("nope"), None);
    assert_eq!(at_token("done @a b"), None);
}

#[test]
fn lists_workspace_files_by_substring() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = matching_files(root, "main.rs");
    assert!(files.iter().any(|f| f.ends_with("main.rs")), "{files:?}");
}

#[test]
fn skips_noise_directories() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = matching_files(root, "target");
    assert!(!files.iter().any(|f| f.starts_with("target/")), "{files:?}");
}

#[test]
fn parses_hex_colours() {
    use super::theme::parse_hex;
    use ratatui::style::Color;
    let d = Color::Rgb(1, 2, 3);
    assert_eq!(parse_hex("#ff0000", d), Color::Rgb(255, 0, 0));
    assert_eq!(parse_hex("00ff00", d), Color::Rgb(0, 255, 0));
    assert_eq!(parse_hex("#f00", d), Color::Rgb(255, 0, 0));
    assert_eq!(parse_hex("nope", d), d);
    assert_eq!(parse_hex("#12", d), d);
}
