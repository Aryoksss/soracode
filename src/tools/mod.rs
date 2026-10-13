//! Tool registry and dispatch.

pub mod camouflage;
pub mod catalog;
pub mod fs;
pub mod host;
pub mod http;
pub mod notes;
pub mod pentest;
pub mod shell;
pub mod skill;
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
    /// Shared offensive engagement state.
    pub engagement: crate::engagement::Engagement,
    /// Bridge used by the subagent tool to start nested agents.
    pub spawner: Option<Arc<dyn subagent::Spawner>>,
}

/// Holds the tools available to one agent and dispatches calls by name.
pub struct Registry {
    tools: Vec<Arc<dyn Tool>>,
}

impl Registry {
    /// Build the standard tool set for the given context.
    #[must_use]
    pub fn standard(ctx: &ToolContext) -> Self {
        let mut tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(shell::ShellTool::new(ctx.clone())),
            Arc::new(http::HttpTool::new(ctx.clone())),
            Arc::new(fs::ReadTool::new(ctx.clone())),
            Arc::new(fs::WriteTool::new(ctx.clone())),
            Arc::new(fs::EditTool::new(ctx.clone())),
            Arc::new(fs::ListTool::new(ctx.clone())),
            Arc::new(fs::GlobTool::new(ctx.clone())),
            Arc::new(fs::GrepTool::new(ctx.clone())),
            Arc::new(notes::NoteTool::new(ctx.clone())),
            Arc::new(notes::RecallTool::new(ctx.clone())),
            Arc::new(host::ToolsTool::new(ctx.clone())),
            Arc::new(skill::SkillTool::new(ctx.clone())),
            Arc::new(camouflage::CamouflageTool::new(ctx.clone())),
            Arc::new(pentest::NmapParseTool::new(ctx.clone())),
            Arc::new(pentest::NucleiParseTool::new(ctx.clone())),
            Arc::new(pentest::CmeParseTool::new(ctx.clone())),
            Arc::new(pentest::GobusterParseTool::new(ctx.clone())),
            Arc::new(pentest::SqlmapParseTool::new(ctx.clone())),
            Arc::new(pentest::BloodhoundParseTool::new(ctx.clone())),
            Arc::new(pentest::XssDetectTool::new(ctx.clone())),
            Arc::new(pentest::JwtAnalyzeTool::new(ctx.clone())),
            Arc::new(pentest::StateUpdateTool::new(ctx.clone())),
            Arc::new(pentest::StateQueryTool::new(ctx.clone())),
            Arc::new(pentest::AttackPathTool::new(ctx.clone())),
            Arc::new(pentest::ReportTool::new(ctx.clone())),
        ];

        if ctx.config.subagents_enabled && ctx.depth < ctx.config.subagent_max_depth {
            tools.push(Arc::new(subagent::SubagentTool::new(ctx.clone())));
        }

        tools.extend(crate::mcp::tools());

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

/// Reduce a URL or `host:port` string to its bare host.
///
/// Parsers record the host the way the other tools address it, so a finding
/// from a URL-shaped argument lands on the same host row a scan of that host
/// already created instead of on `http://host/`.
#[must_use]
pub fn host_of(input: &str) -> String {
    let rest = input
        .trim()
        .strip_prefix("https://")
        .or_else(|| input.trim().strip_prefix("http://"))
        .unwrap_or(input.trim());
    rest.split(['/', '?', ':'])
        .next()
        .unwrap_or(rest)
        .to_string()
}
