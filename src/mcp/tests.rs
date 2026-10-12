use serde_json::json;

use super::jsonrpc::{decode, Frame};

#[test]
fn decodes_a_result_frame() {
    let line = r#"{"jsonrpc":"2.0","id":3,"result":{"tools":[]}}"#;
    match decode(line) {
        Frame::Result { id, value } => {
            assert_eq!(id, 3);
            assert_eq!(value, json!({ "tools": [] }));
        }
        _ => panic!("expected a result frame"),
    }
}

#[test]
fn decodes_an_error_frame() {
    let line = r#"{"jsonrpc":"2.0","id":7,"error":{"code":-32601,"message":"no such method"}}"#;
    match decode(line) {
        Frame::Error { id, message } => {
            assert_eq!(id, 7);
            assert!(message.contains("no such method"));
        }
        _ => panic!("expected an error frame"),
    }
}

#[test]
fn ignores_notifications_and_other_ids() {
    assert!(matches!(
        decode(r#"{"jsonrpc":"2.0","method":"ping"}"#),
        Frame::Ignore
    ));
    assert!(matches!(
        decode(r#"{"jsonrpc":"2.0","id":9,"result":{}}"#),
        Frame::Result { .. }
    ));
    assert!(matches!(decode("not json at all"), Frame::Malformed));
}

#[test]
fn name_sanitisation_is_stable() {
    assert_eq!(
        super::tool::sanitize_for_test("mcp_Burp Suite_scan.site"),
        "mcp_burp_suite_scan_site"
    );
}

/// A minimal stdio MCP server: reads JSON-RPC lines, answers initialize,
/// tools/list (one `echo` tool) and tools/call.
const FAKE_SERVER: &str = r#"
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    if "id" not in msg:
        continue
    method = msg.get("method")
    if method == "initialize":
        result = {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}},
                  "serverInfo": {"name": "fake", "version": "0"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "echo", "description": "Echo the text back.",
                             "inputSchema": {"type": "object",
                                             "properties": {"text": {"type": "string"}}}}]}
    elif method == "tools/call":
        text = msg["params"]["arguments"].get("text", "")
        result = {"content": [{"type": "text", "text": "echo: " + text}]}
    else:
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg["id"],
                                     "error": {"code": -32601, "message": "no such method"}}) + "\n")
        sys.stdout.flush()
        continue
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg["id"], "result": result}) + "\n")
    sys.stdout.flush()
"#;

#[tokio::test]
async fn round_trips_initialize_list_and_call_over_stdio() {
    let script = std::env::temp_dir().join(format!("soracode-mcp-{}.py", std::process::id()));
    std::fs::write(&script, FAKE_SERVER).expect("write fake server");

    let cfg = crate::config::McpServer {
        command: "python3".to_string(),
        args: vec![script.display().to_string()],
        ..Default::default()
    };

    let client = super::McpClient::start("fake", &cfg).await.expect("start");
    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    let out = client
        .call_tool("echo", json!({ "text": "hi" }))
        .await
        .expect("call");
    assert_eq!(out, "echo: hi");

    let _ = std::fs::remove_file(&script);
}
