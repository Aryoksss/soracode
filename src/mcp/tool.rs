//! Exposes one remote MCP tool as a local [`Tool`].

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::Result;
use crate::tools::Tool;

use super::McpClient;

/// A remote MCP tool wrapped so the agent can call it by name.
pub struct McpTool {
    client: Arc<McpClient>,
    remote_name: String,
    local_name: &'static str,
    description: &'static str,
    schema: Value,
}

impl McpTool {
    /// Wrap `remote` on `client`; the local name is `mcp_<server>_<tool>`,
    /// lower-cased with non-alphanumerics replaced by `_`.
    #[must_use]
    pub fn new(
        client: Arc<McpClient>,
        remote_name: String,
        description: String,
        schema: Value,
    ) -> Self {
        let local = format!("mcp_{}_{}", client.name(), remote_name);
        let local_name = Box::leak(sanitize(&local).into_boxed_str());
        let description = Box::leak(description.into_boxed_str());
        Self {
            client,
            remote_name,
            local_name,
            description,
            schema,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &'static str {
        self.local_name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    async fn run(&self, args: Value) -> Result<String> {
        self.client.call_tool(&self.remote_name, args).await
    }
}

/// Test-only view of the name sanitiser.
#[cfg(test)]
pub(super) fn sanitize_for_test(s: &str) -> String {
    sanitize(s)
}

/// Lower-case and replace every run of non-alphanumerics with a single `_`.
fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_underscore = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}
