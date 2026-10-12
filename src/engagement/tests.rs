//! Tests for the engagement store.

use super::{Access, Cred, Edge, Engagement, Host, Service, Vuln};

fn store() -> Engagement {
    Engagement::in_memory().unwrap()
}

#[test]
fn host_upsert_keeps_newest_hostname() {
    let e = store();
    e.add_host(&Host {
        addr: "10.0.0.1".into(),
        hostname: None,
        os: None,
    })
    .unwrap();
    e.add_host(&Host {
        addr: "10.0.0.1".into(),
        hostname: Some("web01".into()),
        os: Some("Linux".into()),
    })
    .unwrap();
    let hosts = e.hosts().unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].hostname.as_deref(), Some("web01"));
}

#[test]
fn service_upsert_is_keyed_by_host_proto_port() {
    let e = store();
    for name in ["http", "nginx"] {
        e.add_service(&Service {
            host: "10.0.0.1".into(),
            proto: "tcp".into(),
            port: 80,
            name: Some(name.into()),
            version: None,
        })
        .unwrap();
    }
    let svcs = e.services(None).unwrap();
    assert_eq!(svcs.len(), 1);
    assert_eq!(svcs[0].name.as_deref(), Some("nginx"));
}

#[test]
fn records_vulns_creds_access_and_edges() {
    let e = store();
    e.add_vuln(&Vuln {
        host: "10.0.0.1".into(),
        title: "AS-REP roastable".into(),
        severity: "high".into(),
        status: "confirmed".into(),
        evidence: "hash captured".into(),
    })
    .unwrap();
    e.add_cred(&Cred {
        username: "svc".into(),
        secret: "hash".into(),
        kind: "hash".into(),
        domain: Some("CORP".into()),
    })
    .unwrap();
    e.add_access(&Access {
        host: "10.0.0.1".into(),
        method: "winrm".into(),
        level: "user".into(),
        username: Some("svc".into()),
    })
    .unwrap();
    e.add_edge(&Edge {
        from: "svc".into(),
        kind: "CREDENTIAL_FROM".into(),
        to: "10.0.0.1".into(),
    })
    .unwrap();

    assert_eq!(e.vulns().unwrap().len(), 1);
    assert_eq!(e.creds().unwrap().len(), 1);
    assert_eq!(e.access().unwrap().len(), 1);
    assert_eq!(e.edges().unwrap().len(), 1);

    let rendered = e.render().unwrap();
    assert!(rendered.contains("AS-REP roastable"));
    assert!(rendered.contains("CREDENTIAL_FROM"));
}

#[test]
fn empty_store_renders_a_placeholder() {
    let e = store();
    assert_eq!(e.render().unwrap(), "(engagement state empty)");
}
