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

#[test]
fn exists_is_false_for_an_unknown_id() {
    let (dir, path) = temp_db("exists-unknown");
    assert!(!Session::exists(&path, "deadbeef").unwrap());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn exists_is_true_after_open() {
    let (dir, path) = temp_db("exists-known");
    let s = Session::open(&path, None).unwrap();
    assert!(Session::exists(&path, s.id()).unwrap());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn delete_removes_the_session_and_its_entries() {
    let (dir, path) = temp_db("delete");
    let a = Session::open(&path, None).unwrap();
    a.append("user", "hi", None).unwrap();
    let id = a.id().to_string();
    drop(a);

    assert!(Session::delete(&path, &id).unwrap());
    assert!(!Session::exists(&path, &id).unwrap());
    assert_eq!(Session::latest(&path).unwrap(), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn delete_of_an_unknown_id_reports_false() {
    let (dir, path) = temp_db("delete-unknown");
    let s = Session::open(&path, None).unwrap();
    assert!(!Session::delete(&path, "deadbeef").unwrap());
    drop(s);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn undo_last_turn_removes_the_instruction_and_its_replies() {
    let (dir, path) = temp_db("undo");
    let s = Session::open(&path, None).unwrap();
    s.append("user", "first", None).unwrap();
    s.append("assistant", "one", None).unwrap();
    s.append("user", "second", None).unwrap();
    s.append("tool", "out", Some("shell")).unwrap();
    s.append("assistant", "two", None).unwrap();

    assert_eq!(s.undo_last_turn().unwrap().as_deref(), Some("second"));
    let roles: Vec<String> = s.entries().unwrap().into_iter().map(|e| e.role).collect();
    assert_eq!(roles, vec!["user", "assistant"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn undo_last_turn_is_none_without_a_user_turn() {
    let (dir, path) = temp_db("undo-empty");
    let s = Session::open(&path, None).unwrap();
    assert_eq!(s.undo_last_turn().unwrap(), None);
    let _ = std::fs::remove_dir_all(dir);
}
