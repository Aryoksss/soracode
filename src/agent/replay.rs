//! Rebuilding prior conversation turns from a stored session.

use crate::llm::Message;
use crate::session::Session;

/// Rebuild prior turns from the session so `--resume` keeps its context.
///
/// Only the tail is replayed. A long session would otherwise push the system
/// prompt past the model's window, and the recent turns are the ones that
/// matter for the next instruction. Tool entries are skipped: their exact
/// request shape is not stored, so a fabricated assistant/tool pair would be
/// malformed. The assistant text that followed them is kept, which preserves
/// what the model concluded.
pub fn replay_history(session: &Session, messages: &mut Vec<Message>) {
    const MAX_REPLAY: usize = 24;

    let Ok(entries) = session.entries() else {
        return;
    };

    let mut replayed = Vec::new();
    for entry in entries {
        match entry.role.as_str() {
            "user" => replayed.push(Message::User {
                content: entry.content,
            }),
            "assistant" if !entry.content.trim().is_empty() => {
                replayed.push(Message::Assistant {
                    content: Some(entry.content),
                    tool_calls: None,
                });
            }
            _ => {}
        }
    }

    if replayed.len() > MAX_REPLAY {
        replayed.drain(0..replayed.len() - MAX_REPLAY);
    }

    // A history that starts mid-turn would open with an assistant reply.
    if matches!(replayed.first(), Some(Message::Assistant { .. })) {
        replayed.remove(0);
    }

    messages.extend(replayed);
}
