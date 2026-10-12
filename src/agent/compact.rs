//! Context-budget compaction: eliding old turns so a long session still fits.

use crate::llm::Message;

fn message_len(m: &Message) -> usize {
    match m {
        Message::System { content } | Message::User { content } => content.len(),
        Message::Assistant {
            content,
            tool_calls,
        } => {
            content.as_deref().map_or(0, str::len)
                + tool_calls
                    .as_ref()
                    .map_or(0, |c| c.iter().map(|t| t.function.arguments.len()).sum())
        }
        Message::Tool { content, .. } => content.len(),
    }
}

/// Drop whole older turns until the conversation fits `budget`. Returns how
/// many messages were elided, or `None` when nothing had to be removed.
pub fn compact_messages(messages: &mut Vec<Message>, budget: usize) -> Option<usize> {
    if budget == 0 || messages.len() < 3 {
        return None;
    }
    let mut size: usize = messages.iter().map(message_len).sum();
    if size <= budget {
        return None;
    }

    let leading = messages
        .iter()
        .take_while(|m| matches!(m, Message::System { .. }))
        .count();
    let first_user = messages
        .iter()
        .position(|m| matches!(m, Message::User { .. }))
        .unwrap_or(leading);
    let keep_from = leading.max(first_user + 1);
    if keep_from >= messages.len() {
        return None;
    }

    let mut dropped = 0usize;
    while size > budget {
        let mut candidate = keep_from + dropped + 1;
        while candidate < messages.len() && !matches!(messages[candidate], Message::User { .. }) {
            candidate += 1;
        }
        if candidate >= messages.len() {
            break;
        }
        for m in &messages[keep_from + dropped..candidate] {
            size = size.saturating_sub(message_len(m));
        }
        dropped = candidate - keep_from;
    }

    if dropped == 0 {
        return None;
    }
    let elided = messages.drain(keep_from..keep_from + dropped).count();
    messages.insert(
        keep_from,
        Message::System {
            content: format!("[{elided} earlier messages elided to fit the context budget]"),
        },
    );
    Some(elided)
}
