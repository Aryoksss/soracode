//! Anthropic Messages API: request body and streamed-frame translation.

use serde_json::{json, Value};

use super::super::stream::Assembler;
use super::super::types::{Message, StreamEvent, ToolSpec};
use super::input_of;

/// Serialize a `/v1/messages` body, lifting system turns out of the array.
pub(super) fn body(
    model: &str,
    messages: &[Message],
    tools: &[ToolSpec],
    max_tokens: u32,
    temperature: f32,
) -> Value {
    let mut system = String::new();
    let mut turns: Vec<Value> = Vec::new();

    for m in messages {
        match m {
            Message::System { content } => {
                if !system.is_empty() {
                    system.push_str("\n\n");
                }
                system.push_str(content);
            }
            Message::User { content } => {
                turns.push(json!({"role": "user", "content": content}));
            }
            Message::Assistant {
                content,
                tool_calls,
            } => {
                let mut blocks: Vec<Value> = Vec::new();
                if let Some(text) = content.as_ref().filter(|t| !t.is_empty()) {
                    blocks.push(json!({"type": "text", "text": text}));
                }
                for call in tool_calls.iter().flatten() {
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.function.name,
                        "input": input_of(&call.function.arguments),
                    }));
                }
                turns.push(json!({"role": "assistant", "content": blocks}));
            }
            Message::Tool {
                tool_call_id,
                content,
            } => {
                turns.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content,
                    }],
                }));
            }
        }
    }

    let mut body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "messages": turns,
        "stream": true,
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if !tools.is_empty() {
        let defs: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "input_schema": t.function.parameters,
                })
            })
            .collect();
        body["tools"] = json!(defs);
    }
    body
}

/// Translate one Anthropic SSE event into shared stream events.
pub(super) fn absorb(asm: &mut Assembler, data: &str) -> Vec<StreamEvent> {
    let Ok(v) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    match v.get("type").and_then(Value::as_str) {
        Some("message_start") => {
            let usage = &v["message"]["usage"];
            asm.set_usage(
                usage.get("input_tokens").and_then(Value::as_u64),
                usage.get("output_tokens").and_then(Value::as_u64),
            );
            Vec::new()
        }
        Some("content_block_start") => {
            let block = &v["content_block"];
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                let index = v["index"].as_u64().unwrap_or(0) as usize;
                asm.start_tool(
                    index,
                    block.get("id").and_then(Value::as_str),
                    block.get("name").and_then(Value::as_str),
                );
            }
            Vec::new()
        }
        Some("content_block_delta") => {
            let index = v["index"].as_u64().unwrap_or(0) as usize;
            let delta = &v["delta"];
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => {
                    let t = delta.get("text").and_then(Value::as_str).unwrap_or("");
                    asm.push_text(t)
                }
                Some("thinking_delta") => {
                    let t = delta.get("thinking").and_then(Value::as_str).unwrap_or("");
                    asm.push_reasoning(t)
                }
                Some("input_json_delta") => {
                    let partial = delta
                        .get("partial_json")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    asm.push_tool_args(index, partial);
                    Vec::new()
                }
                _ => Vec::new(),
            }
        }
        Some("message_delta") => {
            let usage = &v["usage"];
            asm.set_usage(None, usage.get("output_tokens").and_then(Value::as_u64));
            Vec::new()
        }
        _ => Vec::new(),
    }
}
