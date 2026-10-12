//! Tests for conversation replay and system-prompt selection.

use super::{base_prompt, replay_history};
use crate::config::AgentConfig;
use crate::llm::Message;
use crate::session::Session;

#[test]
fn base_prompt_prefers_the_operator_file_when_readable() {
    let path = std::env::temp_dir().join("soracode-agent-prompt-override.txt");
    std::fs::write(&path, "OPERATOR PROMPT").unwrap();
    let cfg = AgentConfig {
        system_prompt_file: Some(path.clone()),
        ..AgentConfig::default()
    };
    assert_eq!(base_prompt(&cfg, false), "OPERATOR PROMPT");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn base_prompt_falls_back_when_the_file_is_missing() {
    let cfg = AgentConfig {
        system_prompt_file: Some("/nonexistent/soracode-prompt-xyz.txt".into()),
        ..AgentConfig::default()
    };
    assert_eq!(base_prompt(&cfg, false), crate::prompt::for_role(false));
}

#[test]
fn base_prompt_ignores_the_override_for_subagents() {
    let path = std::env::temp_dir().join("soracode-agent-prompt-subagent.txt");
    std::fs::write(&path, "OPERATOR PROMPT").unwrap();
    let cfg = AgentConfig {
        system_prompt_file: Some(path.clone()),
        ..AgentConfig::default()
    };
    assert_eq!(base_prompt(&cfg, true), crate::prompt::for_role(true));
    let _ = std::fs::remove_file(&path);
}

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

#[test]
fn compaction_leaves_a_small_conversation_untouched() {
    let mut msgs = vec![
        Message::User {
            content: "hi".into(),
        },
        Message::Assistant {
            content: Some("hello".into()),
            tool_calls: None,
        },
    ];
    assert_eq!(super::compact_messages(&mut msgs, 10_000), None);
    assert_eq!(msgs.len(), 2);
}

#[test]
fn compaction_elides_oldest_turns_but_keeps_the_first_and_last() {
    let big = "x".repeat(500);
    let mut msgs = vec![
        Message::System {
            content: "sys".into(),
        },
        Message::User {
            content: "first instruction".into(),
        },
        Message::Assistant {
            content: Some(big.clone()),
            tool_calls: None,
        },
        Message::User {
            content: "second".into(),
        },
        Message::Assistant {
            content: Some(big.clone()),
            tool_calls: None,
        },
        Message::User {
            content: "third".into(),
        },
        Message::Assistant {
            content: Some(big.clone()),
            tool_calls: None,
        },
    ];
    let elided = super::compact_messages(&mut msgs, 1200).expect("should compact");
    assert!(elided >= 1);
    assert!(matches!(msgs[0], Message::System { .. }));
    assert!(matches!(&msgs[1], Message::User { content } if content == "first instruction"));
    assert!(matches!(&msgs[msgs.len() - 1], Message::Assistant { .. }));
}

#[test]
fn compaction_never_splits_a_tool_call_from_its_result() {
    let big = "y".repeat(400);
    let mut msgs = vec![
        Message::System {
            content: "sys".into(),
        },
        Message::User {
            content: "first".into(),
        },
        Message::Assistant {
            content: None,
            tool_calls: Some(vec![serde_json::from_value(serde_json::json!({
                "id": "c1",
                "type": "function",
                "function": { "name": "shell", "arguments": "{}" }
            }))
            .unwrap()]),
        },
        Message::Tool {
            tool_call_id: "c1".into(),
            content: big.clone(),
        },
        Message::User {
            content: "second".into(),
        },
        Message::Assistant {
            content: Some(big.clone()),
            tool_calls: None,
        },
        Message::User {
            content: "third".into(),
        },
        Message::Assistant {
            content: Some(big.clone()),
            tool_calls: None,
        },
    ];
    super::compact_messages(&mut msgs, 900);
    for (i, m) in msgs.iter().enumerate() {
        if let Message::Tool { tool_call_id, .. } = m {
            assert!(
                msgs[..i].iter().any(
                    |p| matches!(p, Message::Assistant { tool_calls: Some(tc), .. }
                    if tc.iter().any(|c| &c.id == tool_call_id))
                ),
                "tool result {tool_call_id} lost its assistant call"
            );
        }
    }
}
