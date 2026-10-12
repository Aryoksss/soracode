//! Model Context Protocol (MCP) client: launch stdio servers and expose their
//! tools to the agent.
//!
//! A configured server is started once at agent construction, its `tools/list`
//! is read, and each remote tool becomes a local `Tool` named
//! `mcp_<server>_<tool>`. Server frames that are not the awaited response
//! (notifications, other ids) are skipped.

mod client;
mod jsonrpc;
mod tool;

pub use self::client::McpClient;
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
