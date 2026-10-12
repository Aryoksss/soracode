//! `subagent` — spawn nested agents that run in parallel.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{arg_str, arg_str_opt, Tool, ToolContext};
use crate::error::{Error, Result};

/// Implemented by the agent layer so tools can start nested agents.
#[async_trait]
pub trait Spawner: Send + Sync {
    /// Run a subagent with the given instruction, optional specialist role and
    /// optional label.
    async fn spawn(
        &self,
        instruction: &str,
        role: Option<String>,
        label: Option<String>,
    ) -> Result<String>;
}

/// Spawns one or more subagents and collects their reports.
pub struct SubagentTool {
    ctx: ToolContext,
}

impl SubagentTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &'static str {
        "subagent"
    }

    fn description(&self) -> &'static str {
        "Spawn one or more autonomous subagents that work in parallel and report back. Each \
         subagent has its own tool set and context and cannot see your conversation, so the \
         instruction must be self-contained: state the target and exactly what to find or do. \
         Pass several tasks to run them concurrently. Optionally set a `role` on a task to give \
         the subagent a specialist focus: recon, enum, exploit, ad, web, or report."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "description": "Self-contained instructions, one per subagent, run in parallel.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "instruction": { "type": "string", "description": "Full task for the subagent." },
                            "label": { "type": "string", "description": "Short label used in the report." },
                            "role": {
                                "type": "string",
                                "enum": ["recon", "enum", "exploit", "ad", "web", "report"],
                                "description": "Optional specialist focus for this subagent."
                            }
                        },
                        "required": ["instruction"]
                    }
                },
                "instruction": {
                    "type": "string",
                    "description": "Single task, convenience alternative to `tasks`."
                },
                "role": {
                    "type": "string",
                    "enum": ["recon", "enum", "exploit", "ad", "web", "report"],
                    "description": "Specialist focus for the single-task form."
                },
                "label": { "type": "string", "description": "Label for the single-task form." }
            }
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let spawner = self
            .ctx
            .spawner
            .clone()
            .ok_or_else(|| Error::Tool("subagents are unavailable in this context".into()))?;

        let mut tasks: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        if let Some(arr) = args.get("tasks").and_then(Value::as_array) {
            for t in arr {
                let instruction = arg_str(t, "instruction")?;
                tasks.push((instruction, arg_str_opt(t, "role"), arg_str_opt(t, "label")));
            }
        } else if let Ok(instruction) = arg_str(&args, "instruction") {
            tasks.push((
                instruction,
                arg_str_opt(&args, "role"),
                arg_str_opt(&args, "label"),
            ));
        }

        if tasks.is_empty() {
            return Err(Error::Tool("no tasks given".into()));
        }

        let limit = self.ctx.config.subagent_max_parallel.max(1) as usize;
        let mut reports: Vec<(String, String)> = Vec::new();

        for chunk in tasks.chunks(limit) {
            let mut handles = Vec::new();
            for (instruction, role, label) in chunk {
                let spawner = spawner.clone();
                let instruction = instruction.clone();
                let role = role.clone();
                let label = label.clone().unwrap_or_else(crate::util::short_id);
                handles.push(tokio::spawn(async move {
                    let result = spawner.spawn(&instruction, role, Some(label.clone())).await;
                    (label, result)
                }));
            }
            for h in handles {
                match h.await {
                    Ok((label, Ok(text))) => reports.push((label, text)),
                    Ok((label, Err(e))) => {
                        reports.push((label, format!("[subagent failed: {e}]")));
                    }
                    Err(e) => reports.push(("?".into(), format!("[join error: {e}]"))),
                }
            }
        }

        let mut out = String::new();
        for (label, text) in reports {
            out.push_str(&format!("\n===== subagent `{label}` =====\n{text}\n"));
        }
        Ok(out)
    }
}
