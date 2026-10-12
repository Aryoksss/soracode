//! Tests for per-dialect request and stream translation.

use super::*;
use crate::llm::types::{FunctionCall, ToolCall};

fn text_tool() -> ToolSpec {
    ToolSpec::new(
        "run",
        "run a thing",
        serde_json::json!({"type": "object", "properties": {}}),
    )
}

fn messages() -> Vec<Message> {
    vec![
        Message::System {
            content: "be brief".into(),
        },
        Message::User {
            content: "hello".into(),
        },
        Message::Assistant {
            content: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".into(),
                kind: "function".into(),
                function: FunctionCall {
                    name: "run".into(),
                    arguments: "{\"a\":1}".into(),
                },
            }]),
        },
        Message::Tool {
            tool_call_id: "call_1".into(),
            content: "{\"ok\":true}".into(),
        },
    ]
}

#[test]
fn openai_body_is_a_chat_completion() {
    let out = build(Wire::OpenAi, "m", &messages(), &[text_tool()], 100, 0.5);
    assert_eq!(out.path, "/chat/completions");
    assert_eq!(out.body["model"], "m");
    assert_eq!(out.body["stream"], true);
    assert_eq!(out.body["tools"][0]["function"]["name"], "run");
    assert_eq!(out.body["messages"][0]["role"], "system");
}

#[test]
fn anthropic_body_lifts_system_and_builds_blocks() {
    let out = build(
        Wire::Anthropic,
        "claude",
        &messages(),
        &[text_tool()],
        100,
        0.5,
    );
    assert_eq!(out.path, "/messages");
    assert_eq!(out.body["system"], "be brief");
    assert_eq!(out.body["messages"][0]["role"], "user");
    assert_eq!(out.body["messages"][1]["content"][0]["type"], "tool_use");
    assert_eq!(out.body["messages"][1]["content"][0]["input"]["a"], 1);
    assert_eq!(out.body["messages"][2]["content"][0]["type"], "tool_result");
    assert_eq!(out.body["tools"][0]["input_schema"]["type"], "object");
    assert!(out.body.get("tool_choice").is_none());
}

#[test]
fn google_body_uses_contents_and_function_declarations() {
    let out = build(
        Wire::Google,
        "gemini",
        &messages(),
        &[text_tool()],
        100,
        0.5,
    );
    assert_eq!(out.path, "/models/gemini:streamGenerateContent?alt=sse");
    assert_eq!(
        out.body["systemInstruction"]["parts"][0]["text"],
        "be brief"
    );
    assert_eq!(out.body["contents"][0]["role"], "user");
    assert_eq!(out.body["contents"][1]["role"], "model");
    assert_eq!(
        out.body["contents"][1]["parts"][0]["functionCall"]["name"],
        "run"
    );
    assert_eq!(
        out.body["tools"][0]["functionDeclarations"][0]["name"],
        "run"
    );
}

#[test]
fn each_wire_uses_its_own_auth_header() {
    assert_eq!(auth_headers(Wire::OpenAi, "k")[0].0, "authorization");
    assert_eq!(auth_headers(Wire::Anthropic, "k")[0].0, "x-api-key");
    assert_eq!(auth_headers(Wire::Anthropic, "k")[1].0, "anthropic-version");
    assert_eq!(auth_headers(Wire::Google, "k")[0].0, "x-goog-api-key");
}

#[test]
fn anthropic_stream_assembles_text_and_a_tool_call() {
    let mut asm = Assembler::new();
    absorb(
        Wire::Anthropic,
        &mut asm,
        r#"{"type":"message_start","message":{"usage":{"input_tokens":7}}}"#,
    );
    absorb(
        Wire::Anthropic,
        &mut asm,
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#,
    );
    absorb(
        Wire::Anthropic,
        &mut asm,
        r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tu_1","name":"run"}}"#,
    );
    absorb(
        Wire::Anthropic,
        &mut asm,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"a\":"}}"#,
    );
    absorb(
        Wire::Anthropic,
        &mut asm,
        r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"1}"}}"#,
    );
    let done = asm.finish();
    assert_eq!(done.content, "hi");
    assert_eq!(done.prompt_tokens, 7);
    assert_eq!(done.tool_calls.len(), 1);
    assert_eq!(done.tool_calls[0].function.name, "run");
    assert_eq!(done.tool_calls[0].function.arguments, "{\"a\":1}");
    assert_eq!(done.tool_calls[0].id, "tu_1");
}

#[test]
fn google_stream_assembles_text_and_function_call() {
    let mut asm = Assembler::new();
    absorb(
        Wire::Google,
        &mut asm,
        r#"{"candidates":[{"content":{"role":"model","parts":[{"text":"hey"}]}}]}"#,
    );
    absorb(
        Wire::Google,
        &mut asm,
        r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"run","args":{"a":1}}}]}}],"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":2}}"#,
    );
    let done = asm.finish();
    assert_eq!(done.content, "hey");
    assert_eq!(done.completion_tokens, 2);
    assert_eq!(done.tool_calls.len(), 1);
    assert_eq!(done.tool_calls[0].function.name, "run");
    assert_eq!(done.tool_calls[0].function.arguments, "{\"a\":1}");
}

#[test]
fn google_tool_result_becomes_a_function_response() {
    let out = build(Wire::Google, "g", &messages(), &[], 100, 0.5);
    let last = out.body["contents"].as_array().unwrap().last().unwrap();
    assert_eq!(last["parts"][0]["functionResponse"]["response"]["ok"], true);
}
