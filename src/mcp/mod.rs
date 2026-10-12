//! Model Context Protocol (MCP) client: launch stdio servers and expose their
//! tools to the agent.
//!
//! A configured server is started once at agent construction, its `tools/list`
//! is read, and each remote tool becomes a local `Tool` named
//! `mcp_<server>_<tool>`. Server frames that are not the awaited response
//! (notifications, other ids) are skipped.

mod client;
mod jsonrpc;
mod sse;
mod tool;

pub use self::client::{RemoteTool, StdioClient};
pub use self::sse::SseClient;

use serde_json::{json, Value};

use crate::config::McpServer;
use crate::error::{Error, Result};

/// A live MCP session over either transport.
pub enum McpClient {
    /// A child process spoken to over stdin/stdout.
    Stdio(Box<StdioClient>),
    /// A remote endpoint over Server-Sent Events + HTTP POST.
    Sse(Box<SseClient>),
}

impl McpClient {
    /// Start a server with the transport its configuration selects.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the server cannot be reached or does not
    /// complete the `initialize` handshake.
    pub async fn start(name: &str, cfg: &McpServer) -> Result<Self> {
        match cfg.transport() {
            crate::config::McpTransport::Stdio => {
                Ok(Self::Stdio(Box::new(StdioClient::start(name, cfg).await?)))
            }
            crate::config::McpTransport::Sse => {
                Ok(Self::Sse(Box::new(SseClient::start(name, &cfg.url).await?)))
            }
        }
    }

    /// The server's name, as configured.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Stdio(c) => c.name(),
            Self::Sse(c) => c.name(),
        }
    }

    /// List the server's tools.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/list` fails.
    pub async fn list_tools(&self) -> Result<Vec<RemoteTool>> {
        match self {
            Self::Stdio(c) => c.list_tools().await,
            Self::Sse(c) => c.list_tools().await,
        }
    }

    /// Invoke a tool and flatten its content blocks into display text.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when `tools/call` fails.
    pub async fn call_tool(&self, tool: &str, args: Value) -> Result<String> {
        match self {
            Self::Stdio(c) => c.call_tool(tool, args).await,
            Self::Sse(c) => c.call_tool(tool, args).await,
        }
    }
}

/// Parse a `tools/list` result into remote tool descriptors.
pub(super) fn remote_tools(v: &Value) -> Result<Vec<RemoteTool>> {
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

/// Flatten a `tools/call` result to text, surfacing server errors.
pub(super) fn flatten_tool_result(server: &str, tool: &str, v: &Value) -> Result<String> {
    if v.get("isError").and_then(Value::as_bool).unwrap_or(false) {
        return Err(Error::Tool(format!(
            "mcp `{server}`: tool `{tool}` reported an error: {}",
            flatten_content(v)
        )));
    }
    Ok(flatten_content(v))
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

pub use self::tool::McpTool;

#[cfg(test)]
mod tests;

use std::sync::{Arc, OnceLock};

use crate::config::Config;
use crate::tools::Tool;

/// MCP tools discovered at startup, shared by every agent's registry.
static TOOLS: OnceLock<Vec<Arc<dyn Tool>>> = OnceLock::new();

/// Launch every configured MCP server and cache their tools.
///
/// A server that fails to start is logged and skipped; it never blocks the
/// session. Calling this twice is a no-op.
pub async fn init(config: &Config) {
    if TOOLS.get().is_some() {
        return;
    }
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    for (name, server) in &config.mcp {
        if server.disabled {
            continue;
        }
        match launch(name, server).await {
            Ok(mut discovered) => tools.append(&mut discovered),
            Err(e) => eprintln!("soracode: mcp `{name}` disabled: {e}"),
        }
    }
    let _ = TOOLS.set(tools);
}

async fn launch(
    name: &str,
    server: &crate::config::McpServer,
) -> crate::error::Result<Vec<Arc<dyn Tool>>> {
    let client = Arc::new(McpClient::start(name, server).await?);
    let remote = client.list_tools().await?;
    Ok(remote
        .into_iter()
        .map(|t| {
            Arc::new(McpTool::new(
                client.clone(),
                t.name,
                t.description,
                t.schema,
            )) as Arc<dyn Tool>
        })
        .collect())
}

/// The MCP tools discovered at startup (empty when none are configured).
#[must_use]
pub fn tools() -> Vec<Arc<dyn Tool>> {
    TOOLS.get().cloned().unwrap_or_default()
}
