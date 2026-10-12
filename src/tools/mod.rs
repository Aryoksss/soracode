//! Tool registry and dispatch.

pub mod catalog;
pub mod fs;
pub mod host;
pub mod http;
pub mod notes;
pub mod shell;
pub mod subagent;

use std::path::PathBuf;
use std::sync::Arc;

use crate::config::AgentConfig;
use crate::error::{Error, Result};
use crate::llm::ToolSpec;

/// A single unit of work the model can request.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// The tool name exposed to the model.
    fn name(&self) -> &'static str;
    /// Description shown to the model.
    fn description(&self) -> &'static str;
    /// JSON Schema for the arguments object.
    fn schema(&self) -> serde_json::Value;
    /// Execute with raw JSON arguments and return output text.
    async fn run(&self, args: serde_json::Value) -> Result<String>;
}

/// Everything a tool may need at runtime.
#[derive(Clone)]
pub struct ToolContext {
    /// Effective agent configuration.
    pub config: Arc<AgentConfig>,
    /// Workspace root for relative paths.
    pub workspace: PathBuf,
    /// Depth of the current agent (0 = root).
    pub depth: u32,
    /// Shared notes store.
    pub notes: notes::Notes,
    /// Bridge used by the subagent tool to start nested agents.
    pub spawner: Option<Arc<dyn subagent::Spawner>>,
}

/// Holds the tools available to one agent and dispatches calls by name.
pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    /// Build the standard tool set for the given context.
    #[must_use]
    pub fn standard(ctx: &ToolContext) -> Self {
        let mut tools: Vec<Box<dyn Tool>> = vec![
            Box::new(shell::ShellTool::new(ctx.clone())),
            Box::new(http::HttpTool::new(ctx.clone())),
            Box::new(fs::ReadTool::new(ctx.clone())),
            Box::new(fs::WriteTool::new(ctx.clone())),
            Box::new(fs::EditTool::new(ctx.clone())),
            Box::new(fs::ListTool::new(ctx.clone())),
            Box::new(fs::GlobTool::new(ctx.clone())),
            Box::new(fs::GrepTool::new(ctx.clone())),
            Box::new(notes::NoteTool::new(ctx.clone())),
            Box::new(notes::RecallTool::new(ctx.clone())),
            Box::new(host::ToolsTool::new(ctx.clone())),
        ];

        if ctx.config.subagents_enabled && ctx.depth < ctx.config.subagent_max_depth {
            tools.push(Box::new(subagent::SubagentTool::new(ctx.clone())));
        }

        Self { tools }
    }

    /// Tool specs to advertise to the model.
    #[must_use]
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools
            .iter()
            .map(|t| ToolSpec::new(t.name(), t.description(), t.schema()))
            .collect()
    }

    /// Execute a call by name.
    ///
    /// # Errors
    /// Returns [`Error::Tool`] when the tool is unknown or fails.
    pub async fn call(&self, name: &str, args: serde_json::Value) -> Result<String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| Error::Tool(format!("unknown tool: {name}")))?;
        tool.run(args).await
    }

    /// Names of the registered tools.
    #[must_use]
    #[allow(dead_code)] // introspection helper for diagnostics
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}

/// Extract a required string argument, with a helpful error.
///
/// # Errors
/// Returns [`Error::Tool`] when the field is missing or not a string.
pub fn arg_str(args: &serde_json::Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::Tool(format!("missing required string argument `{key}`")))
}

/// Extract an optional string argument.
#[must_use]
pub fn arg_str_opt(args: &serde_json::Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Extract an optional integer argument.
#[must_use]
pub fn arg_u64_opt(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(serde_json::Value::as_u64)
}
