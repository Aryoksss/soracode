//! Tests for conversation replay from a stored session.

use super::replay_history;
use crate::llm::Message;
use crate::session::Session;

fn temp_session(tag: &str) -> (Session, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("soracode-agent-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s.db");
    (Session::open(&path, None).unwrap(), dir)
}

#[test]
fn replay_restores_user_and_assistant_text() {
    let (s, dir) = temp_session("replay-basic");
    s.append("user", "remember MANGO42", None).unwrap();
    s.append("assistant", "Noted.", None).unwrap();

    let mut msgs: Vec<Message> = Vec::new();
    replay_history(&s, &mut msgs);

    assert_eq!(msgs.len(), 2);
    assert!(matches!(&msgs[0], Message::User { content } if content == "remember MANGO42"));
    assert!(matches!(&msgs[1], Message::Assistant { content: Some(c), .. } if c == "Noted."));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn replay_skips_tool_entries_and_a_leading_reply() {
    let (s, dir) = temp_session("replay-tools");
    s.append("tool", "file contents", Some("read")).unwrap();
    s.append("assistant", "I read it.", None).unwrap();
    s.append("user", "and now?", None).unwrap();

    let mut msgs: Vec<Message> = Vec::new();
    replay_history(&s, &mut msgs);

    // The tool entry is dropped, so the first kept turn would be a lone
    // assistant reply; it must be dropped too, leaving the user turn.
    assert_eq!(msgs.len(), 1);
    assert!(matches!(&msgs[0], Message::User { content } if content == "and now?"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn replay_keeps_only_the_recent_tail() {
    let (s, dir) = temp_session("replay-tail");
    for i in 0..20 {
        s.append("user", &format!("u{i}"), None).unwrap();
        s.append("assistant", &format!("a{i}"), None).unwrap();
    }

    let mut msgs: Vec<Message> = Vec::new();
    replay_history(&s, &mut msgs);

    assert_eq!(msgs.len(), 24);
    assert!(matches!(&msgs[0], Message::User { content } if content == "u8"));
    let _ = std::fs::remove_dir_all(dir);
}
