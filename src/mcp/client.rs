//! A live stdio MCP session: a child process spoken to over JSON-RPC.

use std::process::Stdio;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::config::McpServer;
use crate::error::{Error, Result};

use super::jsonrpc::{self, Frame};

/// One running MCP server, with its request id counter and pipes.
pub struct McpClient {
    name: String,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    next_id: Mutex<u64>,
    // Kept alive for the process lifetime; dropped on shutdown.
    _child: Mutex<Child>,
}

impl McpClient {
    /// Launch a server, run the `initialize` handshake, and return the client.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the server cannot be started or does not
    /// answer `initialize` with a result.
    pub async fn start(name: &str, cfg: &McpServer) -> Result<Self> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        for (k, v) in &cfg.env {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Tool(format!("mcp `{name}`: spawn `{}`: {e}", cfg.command)))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Tool(format!("mcp `{name}`: no stdin")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Tool(format!("mcp `{name}`: no stdout")))?;

        let client = Self {
            name: name.to_string(),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            next_id: Mutex::new(1),
            _child: Mutex::new(child),
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
    /// Returns [`Error::Tool`] on a broken pipe, a JSON-RPC error, or EOF.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut n = self.next_id.lock().await;
            let id = *n;
            *n += 1;
            id
        };
        let frame = jsonrpc::request(id, method, params);
        self.write_frame(&frame).await?;

        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();
        loop {
            line.clear();
            let n = stdout
                .read_line(&mut line)
                .await
                .map_err(|e| Error::Tool(format!("mcp `{}`: read: {e}", self.name)))?;
            if n == 0 {
                return Err(Error::Tool(format!(
                    "mcp `{}`: server closed the connection during `{method}`",
                    self.name
                )));
            }
            match jsonrpc::decode(line.trim_end()) {
                Frame::Result { id: rid, value } if rid == id => return Ok(value),
                Frame::Error { id: rid, message } if rid == id => {
                    return Err(Error::Tool(format!("mcp `{}`: {message}", self.name)));
                }
                Frame::Result { .. } | Frame::Error { .. } | Frame::Ignore => continue,
                Frame::Malformed => continue,
            }
        }
    }

    /// Write a notification frame; no response is read.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the request cannot be written.
    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.write_frame(&jsonrpc::notification(method, params))
            .await
    }

    async fn write_frame(&self, frame: &Value) -> Result<()> {
        let mut body = serde_json::to_vec(frame)?;
        body.push(b'\n');
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(&body)
            .await
            .map_err(|e| Error::Tool(format!("mcp `{}`: write: {e}", self.name)))?;
        stdin
            .flush()
            .await
            .map_err(|e| Error::Tool(format!("mcp `{}`: flush: {e}", self.name)))?;
        Ok(())
    }

    /// List the server's tools as `(name, description, inputSchema)`.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/list` fails.
    pub async fn list_tools(&self) -> Result<Vec<RemoteTool>> {
        let v = self.call("tools/list", json!({})).await?;
        let items = v
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(items
            .into_iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_string();
                let description = t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let schema = t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object" }));
                Some(RemoteTool {
                    name,
                    description,
                    schema,
                })
            })
            .collect())
    }

    /// Invoke a tool and flatten its content blocks into display text.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/call` fails.
    pub async fn call_tool(&self, tool: &str, args: Value) -> Result<String> {
        let v = self
            .call("tools/call", json!({ "name": tool, "arguments": args }))
            .await?;
        if v.get("isError").and_then(Value::as_bool).unwrap_or(false) {
            return Err(Error::Tool(format!(
                "mcp `{}`: tool `{tool}` reported an error: {}",
                self.name,
                flatten_content(&v)
            )));
        }
        Ok(flatten_content(&v))
    }
}

/// A tool advertised by an MCP server.
pub struct RemoteTool {
    /// Tool name as the server calls it.
    pub name: String,
    /// Human/model description.
    pub description: String,
    /// JSON Schema for the arguments object.
    pub schema: Value,
}

/// Join an MCP `content` array into plain text.
fn flatten_content(v: &Value) -> String {
    let Some(items) = v.get("content").and_then(Value::as_array) else {
        return v.to_string();
    };
    let mut out: Vec<String> = Vec::new();
    for item in items {
        match item.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(t) = item.get("text").and_then(Value::as_str) {
                    out.push(t.to_string());
                }
            }
            Some(other) => out.push(format!("[{other} content omitted]")),
            None => {}
        }
    }
    out.join("\n")
}
