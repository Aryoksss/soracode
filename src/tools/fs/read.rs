//! `read` — paged, budget-capped text file reader.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::resolve;
use crate::error::{Error, Result};
use crate::tools::{arg_str, arg_u64_opt, Tool, ToolContext};
use crate::util::{display_path, truncate_end};

/// Reads a UTF-8 text file, with optional offset/limit.
pub struct ReadTool {
    ctx: ToolContext,
}

impl ReadTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a text file and return its contents with 1-based line numbers. Use `offset` and \
         `limit` to page through large files. Very long lines are truncated, and the whole \
         response is capped, so a binary-ish or minified file cannot flood the conversation."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path (absolute or relative to the workspace)." },
                "offset": { "type": "integer", "description": "First line to return (1-based)." },
                "limit": { "type": "integer", "description": "Maximum number of lines." }
            },
            "required": ["path"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let path = resolve(&self.ctx, &arg_str(&args, "path")?);
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| Error::Tool(format!("cannot read {}: {e}", display_path(&path))))?;

        let offset = arg_u64_opt(&args, "offset").unwrap_or(1).max(1) as usize;
        let limit = arg_u64_opt(&args, "limit").unwrap_or(2000) as usize;
        let budget = self.ctx.config.tool_output_limit;

        let total = content.lines().count();
        let mut out = String::new();
        let mut long_lines = 0usize;
        let mut truncated_total = false;

        for (i, line) in content.lines().enumerate().skip(offset - 1).take(limit) {
            let per_line = budget / 4;
            let shown = if line.len() > per_line {
                long_lines += 1;
                truncate_end(line, per_line)
            } else {
                line.to_string()
            };
            if out.len() + shown.len() + 16 > budget {
                truncated_total = true;
                break;
            }
            out.push_str(&format!("{:>6}: {}\n", i + 1, shown));
        }

        if out.is_empty() {
            return Ok(format!("(no lines at offset {offset}; file has {total})"));
        }
        if truncated_total {
            out.push_str(&format!(
                "\n[... output cap reached; continue at offset {} or narrow the range ...]",
                offset + out.lines().count()
            ));
        }
        if long_lines > 0 {
            out.push_str(&format!(
                "\n[... {long_lines} line(s) were individually truncated ...]"
            ));
        }
        let shown_lines = out.lines().count();
        if offset - 1 + shown_lines < total {
            out.push_str(&format!(
                "\n[... {} more lines; file has {total} ...]",
                total - (offset - 1 + shown_lines)
            ));
        }
        Ok(out)
    }
}
