//! A remote MCP session over Server-Sent Events + HTTP POST.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::error::{Error, Result};

use super::jsonrpc;

mod endpoint;

use endpoint::read_endpoint;

/// One remote MCP server reached at an SSE endpoint.
pub struct SseClient {
    name: String,
    post_url: String,
    http: reqwest::Client,
    next_id: Mutex<u64>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
}

impl SseClient {
    /// Connect, learn the POST endpoint, and start the response reader.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the endpoint cannot be reached or does not
    /// announce a POST target within the handshake window.
    pub async fn start(name: &str, url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| Error::Tool(format!("mcp `{name}`: http client: {e}")))?;

        let opened = tokio::time::timeout(
            Duration::from_secs(5),
            http.get(url).header("Accept", "text/event-stream").send(),
        )
        .await;
        let resp = match opened {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                return Err(Error::Tool(format!("mcp `{name}`: connect `{url}`: {e}")));
            }
            Err(_) => {
                return Err(Error::Tool(format!(
                    "mcp `{name}`: connect `{url}`: timed out"
                )));
            }
        };
        if !resp.status().is_success() {
            return Err(Error::Tool(format!(
                "mcp `{name}`: `{url}` returned {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream();
        let post_url = read_endpoint(name, url, &mut stream).await?;
        let (tx, rx) = mpsc::unbounded_channel::<Value>();
        tokio::spawn(reader(stream, tx));

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(dispatch(rx, pending.clone()));

        let client = Self {
            name: name.to_string(),
            post_url,
            http,
            next_id: Mutex::new(1),
            pending,
        };
        client
            .call(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "soracode", "version": env!("CARGO_PKG_VERSION") }
                }),
            )
            .await?;
        client
            .notify("notifications/initialized", json!({}))
            .await?;
        Ok(client)
    }

    /// The server's name, as configured.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send a request and await the matching response result.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] on a transport error, a JSON-RPC error, or a
    /// dropped response.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut n = self.next_id.lock().await;
            let id = *n;
            *n += 1;
            id
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let frame = jsonrpc::request(id, method, params);
        let sent = self
            .http
            .post(&self.post_url)
            .timeout(Duration::from_secs(600))
            .header("Accept", "application/json, text/event-stream")
            .json(&frame)
            .send()
            .await;
        if let Err(e) = sent {
            self.pending.lock().await.remove(&id);
            return Err(Error::Tool(format!("mcp `{}`: post: {e}", self.name)));
        }

        let msg = rx
            .await
            .map_err(|_| Error::Tool(format!("mcp `{}`: response dropped", self.name)))?;
        if let Some(err) = msg.get("error") {
            let text = err
                .get("message")
                .and_then(Value::as_str)
                .map_or_else(|| err.to_string(), str::to_string);
            return Err(Error::Tool(format!("mcp `{}`: {text}", self.name)));
        }
        Ok(msg.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Write a notification frame; no response is read.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the POST fails.
    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let frame = jsonrpc::notification(method, params);
        self.http
            .post(&self.post_url)
            .timeout(Duration::from_secs(30))
            .header("Accept", "application/json, text/event-stream")
            .json(&frame)
            .send()
            .await
            .map_err(|e| Error::Tool(format!("mcp `{}`: notify: {e}", self.name)))?;
        Ok(())
    }

    /// List the server's tools as `(name, description, inputSchema)`.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/list` fails.
    pub async fn list_tools(&self) -> Result<Vec<super::RemoteTool>> {
        super::remote_tools(&self.call("tools/list", json!({})).await?)
    }

    /// Invoke a tool and flatten its content blocks into display text.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/call` fails.
    pub async fn call_tool(&self, tool: &str, args: Value) -> Result<String> {
        let v = self
            .call("tools/call", json!({ "name": tool, "arguments": args }))
            .await?;
        super::flatten_tool_result(&self.name, tool, &v)
    }
}

/// Read SSE bytes into JSON values, forwarding each complete `data:` object.
async fn reader<S>(mut stream: S, tx: mpsc::UnboundedSender<Value>)
where
    S: futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let Ok(bytes) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let Some(data) = line.trim_end().strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if !data.starts_with('{') {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                if tx.send(v).is_err() {
                    return;
                }
            }
        }
    }
}

/// Route each response to the caller waiting on its id.
async fn dispatch(
    mut rx: mpsc::UnboundedReceiver<Value>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
) {
    while let Some(v) = rx.recv().await {
        if let Some(id) = v.get("id").and_then(Value::as_u64) {
            if let Some(tx) = pending.lock().await.remove(&id) {
                let _ = tx.send(v);
            }
        }
    }
}
