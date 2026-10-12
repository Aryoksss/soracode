//! Tests for skill discovery and loading.

use super::{catalogue, discover, load};
use std::path::{Path, PathBuf};

fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("soracode-skills-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(name), body).unwrap();
}

fn skill_dir(ws: &Path) -> PathBuf {
    ws.join(".soracode").join("skills")
}

#[test]
fn discovers_packs_and_uses_first_line_as_summary() {
    let ws = workspace("discover");
    let dir = skill_dir(&ws);
    write(
        &dir,
        "ad-attack.md",
        "# Active Directory attacks\n\nKerberoast, AS-REP, delegation.\n",
    );
    write(&dir, "web.md", "Web app testing\n\nInjection, authz.\n");

    let skills = discover(&ws);
    let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["ad-attack", "web"]);
    assert_eq!(skills[0].summary, "Active Directory attacks");
    assert_eq!(skills[1].summary, "Web app testing");
}

#[test]
fn non_markdown_files_are_ignored() {
    let ws = workspace("ignore");
    let dir = skill_dir(&ws);
    write(&dir, "notes.txt", "ignore me");
    write(&dir, "real.md", "Real pack\n");
    let skills = discover(&ws);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "real");
}

#[test]
fn load_returns_full_text_and_errors_on_unknown() {
    let ws = workspace("load");
    let dir = skill_dir(&ws);
    write(&dir, "recon.md", "# Recon\n\nPassive first.\n");

    let text = load(&ws, "recon").unwrap();
    assert!(text.contains("Passive first"));

    let err = load(&ws, "missing").unwrap_err();
    assert!(err.contains("no skill named `missing`"));
    assert!(err.contains("recon"));
}

#[test]
fn catalogue_is_empty_without_packs_and_lists_them_otherwise() {
    let ws = workspace("catalogue");
    assert!(catalogue(&ws).is_empty());

    write(&skill_dir(&ws), "privesc.md", "Linux privesc\n");
    let cat = catalogue(&ws);
    assert!(cat.contains("## Skills"));
    assert!(cat.contains("`privesc`"));
}
