//! Tests for rebuilding the transcript from a stored session.

use super::rebuild;
use crate::session::Session;

#[test]
fn rebuild_restores_stored_entries() {
    let dir = std::env::temp_dir().join(format!("soracode-rebuild-{}", std::process::id()));
    let _ = std::fs::remove_file(&dir);
    let session = Session::open(&dir, None).expect("open");
    session.append("user", "scan the host", None).expect("user");
    session
        .append("assistant", "found 22 open", None)
        .expect("assistant");
    session
        .append("tool", "nmap says ...", Some("shell"))
        .expect("tool");
    session.append("assistant", "   ", None).expect("blank");

    let lines = rebuild(&session);
    assert_eq!(lines.len(), 3, "blank content is skipped");
    assert!(matches!(lines[0], super::Line2::User(_)));
    assert!(matches!(lines[1], super::Line2::Assistant(_)));
    assert!(matches!(lines[2], super::Line2::Tool(_)));

    let _ = std::fs::remove_file(&dir);
}
