//! Per-dialect request bodies and streamed-frame translation.
//!
//! soracode's own [`Message`]/[`ToolSpec`] shapes stay provider-neutral; this
//! module and its children are the only place that knows how OpenAI, Anthropic,
//! and Google each spell a conversation, a tool, and a token of streamed output.

mod anthropic;
mod google;
mod openai;

use serde_json::Value;

use super::stream::Assembler;
use super::types::{Message, StreamEvent, ToolSpec};
use crate::models::Wire;

/// The HTTP request a wire wants, before transport adds auth.
pub(super) struct Outgoing {
    /// Path appended to the provider base URL.
    pub(super) path: String,
    /// Serialized request body.
    pub(super) body: Value,
}

/// Build the request body for one turn in the given dialect.
pub(super) fn build(
    wire: Wire,
    model: &str,
    messages: &[Message],
    tools: &[ToolSpec],
    max_tokens: u32,
    temperature: f32,
) -> Outgoing {
    match wire {
        Wire::OpenAi => Outgoing {
            path: "/chat/completions".into(),
            body: openai::body(model, messages, tools, max_tokens, temperature),
        },
        Wire::Anthropic => Outgoing {
            path: "/messages".into(),
            body: anthropic::body(model, messages, tools, max_tokens, temperature),
        },
        Wire::Google => Outgoing {
            path: format!("/models/{model}:streamGenerateContent?alt=sse"),
            body: google::body(messages, tools, max_tokens, temperature),
        },
    }
}

/// Auth header(s) for a wire, given the provider's credential.
pub(super) fn auth_headers(wire: Wire, api_key: &str) -> Vec<(&'static str, String)> {
    match wire {
        Wire::OpenAi => vec![("authorization", format!("Bearer {api_key}"))],
        Wire::Anthropic => vec![
            ("x-api-key", api_key.to_string()),
            ("anthropic-version", "2023-06-01".to_string()),
        ],
        Wire::Google => vec![("x-goog-api-key", api_key.to_string())],
    }
}

/// Fold one SSE `data:` payload into the assembler for the given dialect.
pub(super) fn absorb(wire: Wire, asm: &mut Assembler, data: &str) -> Vec<StreamEvent> {
    match wire {
        Wire::OpenAi => asm.push(data),
        Wire::Anthropic => anthropic::absorb(asm, data),
        Wire::Google => google::absorb(asm, data),
    }
}

/// Parse a tool-call argument string into the object a tool schema expects.
pub(super) fn input_of(arguments: &str) -> Value {
    serde_json::from_str(arguments).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(test)]
mod tests;
