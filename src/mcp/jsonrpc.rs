//! JSON-RPC 2.0 framing helpers for the MCP stdio transport.

use serde_json::{json, Value};

/// Build a request frame with a numeric id.
#[must_use]
pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

/// Build a notification frame (no id, no response expected).
#[must_use]
pub fn notification(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": method, "params": params })
}

/// The outcome of decoding one incoming frame.
pub enum Frame {
    /// A response to a known id: the result value.
    Result { id: u64, value: Value },
    /// A response carrying a JSON-RPC error for that id.
    Error { id: u64, message: String },
    /// A frame for another id or a notification, to be ignored.
    Ignore,
    /// The line was not a JSON object.
    Malformed,
}

/// Decode one line of server output.
#[must_use]
pub fn decode(line: &str) -> Frame {
    let Ok(v) = serde_json::from_str::<Value>(line) else {
        return Frame::Malformed;
    };
    let Some(id) = v.get("id").and_then(Value::as_u64) else {
        return Frame::Ignore;
    };
    if let Some(err) = v.get("error") {
        let message = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_string();
        return Frame::Error { id, message };
    }
    Frame::Result {
        id,
        value: v.get("result").cloned().unwrap_or(Value::Null),
    }
}
