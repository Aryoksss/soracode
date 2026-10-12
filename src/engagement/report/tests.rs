//! Tests for the Markdown report.

use super::Engagement;
use crate::engagement::{Access, Cred, Edge, Host, Vuln};

#[test]
fn empty_engagement_still_renders_a_report() {
    let e = Engagement::in_memory().unwrap();
    let md = e.report().unwrap();
    assert!(md.starts_with("# Engagement report"));
    assert!(md.contains("_None recorded._"));
}

#[test]
fn findings_are_ordered_by_descending_severity() {
    let e = Engagement::in_memory().unwrap();
    for (title, sev) in [("low thing", "low"), ("crit thing", "critical")] {
        e.add_vuln(&Vuln {
            host: "h".into(),
            title: title.into(),
            severity: sev.into(),
            status: "suspected".into(),
            evidence: String::new(),
        })
        .unwrap();
    }
    let md = e.report().unwrap();
    let crit = md.find("crit thing").unwrap();
    let low = md.find("low thing").unwrap();
    assert!(crit < low, "critical should precede low");
    assert!(md.contains("[CRITICAL]"));
}

#[test]
fn report_includes_attack_path_when_graph_has_one() {
    let e = Engagement::in_memory().unwrap();
    e.add_host(&Host {
        addr: "dc01".into(),
        hostname: None,
        os: None,
    })
    .unwrap();
    e.add_cred(&Cred {
        username: "svc".into(),
        secret: "pw".into(),
        kind: "password".into(),
        domain: None,
    })
    .unwrap();
    e.add_access(&Access {
        host: "10.0.0.5".into(),
        method: "smb".into(),
        level: "admin".into(),
        username: Some("svc".into()),
    })
    .unwrap();
    e.add_edge(&Edge {
        from: "svc".into(),
        kind: "UNLOCKS".into(),
        to: "10.0.0.5".into(),
    })
    .unwrap();
    e.add_edge(&Edge {
        from: "10.0.0.5".into(),
        kind: "ADMIN_OF".into(),
        to: "dc01".into(),
    })
    .unwrap();

    let md = e.report().unwrap();
    assert!(md.contains("## Attack paths"));
    assert!(md.contains("svc -[UNLOCKS]-> 10.0.0.5 -[ADMIN_OF]-> dc01"));
    assert!(md.contains("| `svc` | `pw` | password | - |"));
}
