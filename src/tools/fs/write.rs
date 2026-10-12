//! `write` and `edit` — file creation and exact-substring replacement.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::resolve;
use crate::error::{Error, Result};
use crate::tools::{arg_str, Tool, ToolContext};
use crate::util::{display_path, truncate_end};

/// Writes a file, creating parent directories.
pub struct WriteTool {
    ctx: ToolContext,
}

impl WriteTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Write text to a file, replacing it entirely and creating parent directories as needed."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path." },
                "content": { "type": "string", "description": "Full file contents." }
            },
            "required": ["path", "content"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let path = resolve(&self.ctx, &arg_str(&args, "path")?);
        let content = arg_str(&args, "content")?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, &content).await?;
        Ok(format!(
            "wrote {} bytes to {}",
            content.len(),
            display_path(&path)
        ))
    }
}

/// Replaces an exact substring in a file.
pub struct EditTool {
    ctx: ToolContext,
}

impl EditTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Replace an exact substring in a file. `old` must appear exactly once unless \
         `replace_all` is true. Shows a short diff of the change."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path." },
                "old": { "type": "string", "description": "Exact text to replace." },
                "new": { "type": "string", "description": "Replacement text." },
                "replace_all": { "type": "boolean", "description": "Replace every occurrence." }
            },
            "required": ["path", "old", "new"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let path = resolve(&self.ctx, &arg_str(&args, "path")?);
        let old = arg_str(&args, "old")?;
        let new = arg_str(&args, "new")?;
        let all = args
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| Error::Tool(format!("cannot read {}: {e}", display_path(&path))))?;

        let count = content.matches(&old).count();
        if count == 0 {
            return Err(Error::Tool(format!(
                "`old` text not found in {}",
                display_path(&path)
            )));
        }
        if count > 1 && !all {
            return Err(Error::Tool(format!(
                "`old` appears {count} times in {}; pass replace_all or give more context",
                display_path(&path)
            )));
        }

        let updated = if all {
            content.replace(&old, &new)
        } else {
            content.replacen(&old, &new, 1)
        };
        tokio::fs::write(&path, &updated).await?;

        Ok(format!(
            "edited {} ({} replacement{})\n--- old ---\n{}\n--- new ---\n{}",
            display_path(&path),
            if all { count } else { 1 },
            if all { "s" } else { "" },
            truncate_end(&old, 500),
            truncate_end(&new, 500)
        ))
    }
}
