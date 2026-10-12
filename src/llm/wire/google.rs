//! Google Gemini: request body and streamed-frame translation.

use serde_json::{json, Value};

use super::super::stream::Assembler;
use super::super::types::{Message, StreamEvent, ToolSpec};
use super::input_of;

/// Serialize a `streamGenerateContent` body.
pub(super) fn body(
    messages: &[Message],
    tools: &[ToolSpec],
    max_tokens: u32,
    temperature: f32,
) -> Value {
    let mut system = String::new();
    let mut contents: Vec<Value> = Vec::new();

    for m in messages {
        match m {
            Message::System { content } => {
                if !system.is_empty() {
                    system.push_str("\n\n");
                }
                system.push_str(content);
            }
            Message::User { content } => {
                contents.push(json!({"role": "user", "parts": [{"text": content}]}));
            }
            Message::Assistant {
                content,
                tool_calls,
            } => {
                let mut parts: Vec<Value> = Vec::new();
                if let Some(text) = content.as_ref().filter(|t| !t.is_empty()) {
                    parts.push(json!({"text": text}));
                }
                for call in tool_calls.iter().flatten() {
                    parts.push(json!({
                        "functionCall": {
                            "name": call.function.name,
                            "args": input_of(&call.function.arguments),
                        }
                    }));
                }
                contents.push(json!({"role": "model", "parts": parts}));
            }
            Message::Tool { content, .. } => push_tool_result(content, &mut contents),
        }
    }

    let mut body = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": max_tokens,
            "temperature": temperature,
        },
    });
    if !system.is_empty() {
        body["systemInstruction"] = json!({"parts": [{"text": system}]});
    }
    if !tools.is_empty() {
        let decls: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "parameters": t.function.parameters,
                })
            })
            .collect();
        body["tools"] = json!([{ "functionDeclarations": decls }]);
    }
    body
}

/// Google has no tool-call id, so a result is a `functionResponse` part that
/// carries the output object; any id in the payload is dropped.
fn push_tool_result(content: &str, contents: &mut Vec<Value>) {
    let payload: Value =
        serde_json::from_str(content).unwrap_or_else(|_| json!({ "output": content }));
    let response = match payload {
        Value::Object(mut map) => {
            map.remove("id");
            Value::Object(map)
        }
        other => json!({ "output": other }),
    };
    contents.push(json!({
        "role": "user",
        "parts": [{"functionResponse": {"name": "tool", "response": response}}],
    }));
}

/// Translate one Gemini SSE frame into shared stream events.
pub(super) fn absorb(asm: &mut Assembler, data: &str) -> Vec<StreamEvent> {
    let Ok(v) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    let mut events = Vec::new();

    if let Some(usage) = v.get("usageMetadata") {
        asm.set_usage(
            usage.get("promptTokenCount").and_then(Value::as_u64),
            usage.get("candidatesTokenCount").and_then(Value::as_u64),
        );
    }

    let Some(parts) = v
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(Value::as_array)
    else {
        return events;
    };

    let mut tool_index = asm.tool_count();
    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                events.extend(asm.push_text(text));
            }
        }
        if let Some(call) = part.get("functionCall") {
            let name = call.get("name").and_then(Value::as_str).unwrap_or("");
            let args = call.get("args").cloned().unwrap_or_else(|| json!({}));
            asm.start_tool(tool_index, None, Some(name));
            asm.push_tool_args(tool_index, &args.to_string());
            tool_index += 1;
        }
    }
    events
}
