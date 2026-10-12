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

/// A minimal SSE MCP server: GET opens the stream and announces a session id,
/// POST `/<sid>` carries JSON-RPC and its responses come back on the stream.
const FAKE_SSE: &str = r#"
import http.server, socketserver, json, queue, uuid, sys

PORT = int(sys.argv[1])
Q = {}

class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        sid = uuid.uuid4().hex
        q = queue.Queue(); Q[sid] = q
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        self.wfile.write(f"data: {sid}\n\n".encode()); self.wfile.flush()
        try:
            while True:
                msg = q.get(timeout=30)
                if msg is None: break
                self.wfile.write(f"data: {json.dumps(msg)}\n\n".encode()); self.wfile.flush()
        except Exception:
            pass
    def do_POST(self):
        sid = self.path.strip("/").split("/")[-1]
        n = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(n) or b"{}")
        self.send_response(202); self.send_header("Content-Length", "0"); self.end_headers()
        q = Q.get(sid)
        if q is None: return
        m = body.get("method"); rid = body.get("id")
        if m == "initialize":
            q.put({"jsonrpc": "2.0", "id": rid, "result": {"protocolVersion": "2024-11-05", "capabilities": {}}})
        elif m == "tools/list":
            q.put({"jsonrpc": "2.0", "id": rid, "result": {"tools": [{"name": "send_http1_request", "description": "raw", "inputSchema": {"type": "object"}}]}})
        elif m == "tools/call":
            a = body.get("params", {}).get("arguments", {})
            q.put({"jsonrpc": "2.0", "id": rid, "result": {"content": [{"type": "text", "text": "OK " + json.dumps(a)}]}})

with socketserver.ThreadingTCPServer(("127.0.0.1", PORT), H) as s:
    s.daemon_threads = True
    print("listening", flush=True)
    s.serve_forever()
"#;

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    l.local_addr().expect("addr").port()
}

#[tokio::test]
async fn round_trips_initialize_list_and_call_over_sse() {
    let port = free_port();
    let script = std::env::temp_dir().join(format!("soracode-mcp-sse-{}.py", std::process::id()));
    std::fs::write(&script, FAKE_SSE).expect("write fake sse server");

    let mut child = std::process::Command::new("python3")
        .arg(&script)
        .arg(port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn fake sse server");
    // wait for the listener to accept
    let url = format!("http://127.0.0.1:{port}");
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let cfg = crate::config::McpServer {
        url: url.clone(),
        ..Default::default()
    };
    let client = super::McpClient::start("burp", &cfg)
        .await
        .expect("start sse");
    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "send_http1_request");

    let out = client
        .call_tool("send_http1_request", json!({ "id": 1 }))
        .await
        .expect("call");
    assert!(out.starts_with("OK "), "got: {out}");

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&script);
    let _ = url;
}

#[tokio::test]
async fn launch_exposes_remote_tools_with_a_prefixed_name() {
    let port = free_port();
    let script =
        std::env::temp_dir().join(format!("soracode-mcp-launch-{}.py", std::process::id()));
    std::fs::write(&script, FAKE_SSE).expect("write fake sse server");
    let mut child = std::process::Command::new("python3")
        .arg(&script)
        .arg(port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let cfg = crate::config::McpServer {
        url: format!("http://127.0.0.1:{port}"),
        ..Default::default()
    };
    let tools = super::launch("burp", &cfg).await.expect("launch");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "mcp_burp_send_http1_request");

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&script);
}
