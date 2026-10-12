//! Tests for session identity and newest-session selection.

use super::Session;

fn temp_db(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("soracode-session-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    (dir.clone(), dir.join("s.db"))
}

#[test]
fn latest_is_none_on_an_empty_database() {
    let (dir, path) = temp_db("empty");
    assert_eq!(Session::latest(&path).unwrap(), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn latest_returns_the_newest_session() {
    let (dir, path) = temp_db("newest");
    let a = Session::open(&path, None).unwrap();
    a.append("user", "first", None).unwrap();
    let b = Session::open(&path, None).unwrap();

    let got = Session::latest(&path).unwrap();
    assert_eq!(got.as_deref(), Some(b.id()));
    let _ = std::fs::remove_dir_all(dir);
}
