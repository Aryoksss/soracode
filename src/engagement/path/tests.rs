//! Tests for attack-path search.

use super::{Engagement, Path};
use crate::engagement::Edge;

fn with_edges(pairs: &[(&str, &str, &str)]) -> Engagement {
    let e = Engagement::in_memory().unwrap();
    for (from, kind, to) in pairs {
        e.add_edge(&Edge {
            from: (*from).into(),
            kind: (*kind).into(),
            to: (*to).into(),
        })
        .unwrap();
    }
    e
}

fn nodes(p: &Path) -> Vec<&str> {
    p.nodes.iter().map(String::as_str).collect()
}

#[test]
fn finds_a_directed_path_from_root_to_goal() {
    let e = with_edges(&[
        ("creds", "UNLOCKS", "10.0.0.5"),
        ("10.0.0.5", "ADMIN_OF", "dc01"),
    ]);
    let paths = e.attack_paths(Some("creds"), Some("dc01")).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(nodes(&paths[0]), vec!["creds", "10.0.0.5", "dc01"]);
    assert_eq!(paths[0].hops, vec!["UNLOCKS", "ADMIN_OF"]);
}

#[test]
fn no_goal_returns_every_reachable_node() {
    let e = with_edges(&[("a", "TO", "b"), ("b", "TO", "c")]);
    let paths = e.attack_paths(Some("a"), None).unwrap();
    let ends: Vec<&str> = paths
        .iter()
        .map(|p| p.nodes.last().unwrap().as_str())
        .collect();
    assert!(ends.contains(&"b"));
    assert!(ends.contains(&"c"));
}

#[test]
fn unattached_start_yields_no_paths() {
    let e = with_edges(&[("a", "TO", "b")]);
    assert!(e.attack_paths(Some("b"), Some("a")).unwrap().is_empty());
}

#[test]
fn empty_goal_uses_graph_roots() {
    let e = with_edges(&[("root", "TO", "mid"), ("mid", "TO", "leaf")]);
    let paths = e.attack_paths(None, Some("leaf")).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(nodes(&paths[0]), vec!["root", "mid", "leaf"]);
}

#[test]
fn shortest_path_wins_when_two_routes_reach_the_same_node() {
    let e = with_edges(&[("s", "SHORT", "g"), ("s", "MID", "m"), ("m", "LONG", "g")]);
    let paths = e.attack_paths(Some("s"), Some("g")).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(nodes(&paths[0]), vec!["s", "g"]);
    assert_eq!(paths[0].hops, vec!["SHORT"]);
}
