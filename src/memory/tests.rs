//! Tests for operator memory discovery.

use super::{block_from, files};

/// A throwaway directory under the system temp root.
fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("soracode-memory-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn missing_memory_yields_no_block() {
    let ws = tmp("missing");
    assert!(block_from(&[]).is_none());
    let _ = ws;
}

#[test]
fn workspace_memory_is_discovered() {
    let ws = tmp("ws");
    std::fs::write(ws.join("SORACODE.md"), "house rules").unwrap();
    let found = files(&ws);
    assert!(found.iter().any(|p| p == &ws.join("SORACODE.md")));
    let local: Vec<_> = found.into_iter().filter(|p| p.starts_with(&ws)).collect();
    let b = block_from(&local).unwrap();
    assert!(b.contains("house rules"), "got: {b}");
    assert!(b.contains("Standing instructions"), "got: {b}");
}

#[test]
fn both_conventions_load_with_agents_first() {
    let ws = tmp("both");
    std::fs::write(ws.join("AGENTS.md"), "agents text").unwrap();
    std::fs::write(ws.join("SORACODE.md"), "soracode text").unwrap();
    let b = block_from(&files(&ws)).unwrap();
    let a = b.find("agents text").unwrap();
    let s = b.find("soracode text").unwrap();
    assert!(a < s, "AGENTS.md should load before SORACODE.md");
}

#[test]
fn empty_memory_file_is_skipped() {
    let ws = tmp("empty");
    let empty = ws.join("SORACODE.md");
    std::fs::write(&empty, "   \n\n").unwrap();
    assert!(block_from(&[empty]).is_none());
}
