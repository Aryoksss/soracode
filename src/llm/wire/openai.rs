//! OpenAI-compatible request body. Streamed frames are handled by the shared
//! [`super::super::stream::Assembler`], so only the body is built here.

use serde::Serialize;
use serde_json::Value;

use super::super::types::{Message, ToolSpec};

#[derive(Serialize)]
struct Body<'a> {
    model: &'a str,
    messages: &'a [Message],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a [ToolSpec]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

/// Serialize a chat-completions body.
pub(super) fn body(
    model: &str,
    messages: &[Message],
    tools: &[ToolSpec],
    max_tokens: u32,
    temperature: f32,
) -> Value {
    let body = Body {
        model,
        messages,
        tools: if tools.is_empty() { None } else { Some(tools) },
        tool_choice: if tools.is_empty() { None } else { Some("auto") },
        max_tokens,
        temperature,
        stream: true,
    };
    serde_json::to_value(body).unwrap_or(Value::Null)
}
