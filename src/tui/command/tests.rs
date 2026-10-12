//! Tests for the slash-command registry.

use super::*;

#[test]
fn every_command_has_a_distinct_name() {
    let mut names: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), COMMANDS.len());
}

#[test]
fn names_and_descriptions_are_non_empty() {
    for c in COMMANDS {
        assert!(!c.name.is_empty(), "empty command name");
        assert!(!c.desc.is_empty(), "empty description for {}", c.name);
        assert!(
            !c.name.contains('/'),
            "name must not contain a slash: {}",
            c.name
        );
    }
}

#[test]
fn matching_accepts_prefix_with_and_without_slash() {
    let bare = matching("model");
    let slashed = matching("/model");
    assert_eq!(bare.len(), 1);
    assert_eq!(bare[0].name, "model");
    assert_eq!(slashed.len(), 1);
    assert_eq!(slashed[0].name, "model");
}

#[test]
fn matching_falls_back_to_the_description() {
    let names: Vec<&str> = matching("graph").into_iter().map(|c| c.name).collect();
    assert!(names.contains(&"matrix"), "description match for graph");
    let creds: Vec<&str> = matching("cred").into_iter().map(|c| c.name).collect();
    assert_eq!(creds.first(), Some(&"creds"), "name match sorts first");
}

#[test]
fn matching_returns_every_prefix_candidate() {
    let names: Vec<&str> = matching("mod").into_iter().map(|c| c.name).collect();
    assert_eq!(names, vec!["model", "mode"]);
}

#[test]
fn pause_resume_and_mode_are_registered() {
    assert_eq!(by_name("pause").map(|c| c.action), Some(Action::Pause));
    assert_eq!(by_name("resume").map(|c| c.action), Some(Action::Resume));
    assert_eq!(by_name("mode").map(|c| c.action), Some(Action::SetMode));
}

#[test]
fn matching_includes_all_commands_for_bare_slash() {
    assert_eq!(matching("/").len(), COMMANDS.len());
}

#[test]
fn matching_is_empty_for_unknown_prefix() {
    assert!(matching("zzz").is_empty());
}

#[test]
fn by_name_resolves_and_rejects() {
    assert_eq!(by_name("quit").map(|c| c.action), Some(Action::Quit));
    assert_eq!(by_name("model").map(|c| c.action), Some(Action::PickModel));
    assert!(by_name("quit ").is_none());
    assert!(by_name("/quit").is_none());
}

#[test]
fn dashboard_commands_resolve() {
    assert_eq!(by_name("status").map(|c| c.action), Some(Action::Status));
    assert_eq!(by_name("targets").map(|c| c.action), Some(Action::Targets));
    assert_eq!(
        by_name("services").map(|c| c.action),
        Some(Action::Services)
    );
    assert_eq!(by_name("matrix").map(|c| c.action), Some(Action::Matrix));
    assert_eq!(by_name("vulns").map(|c| c.action), Some(Action::Vulns));
    assert_eq!(by_name("creds").map(|c| c.action), Some(Action::Creds));
}
