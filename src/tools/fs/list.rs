//! `list` and `glob` — directory listing and glob file search.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{glob_match, is_noise_dir, resolve};
use crate::error::{Error, Result};
use crate::tools::{arg_str, arg_str_opt, arg_u64_opt, Tool, ToolContext};
use crate::util::display_path;

/// Lists a directory.
pub struct ListTool {
    ctx: ToolContext,
}

impl ListTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ListTool {
    fn name(&self) -> &'static str {
        "list"
    }

    fn description(&self) -> &'static str {
        "List the entries of a directory, marking directories with a trailing slash."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path. Defaults to the workspace." }
            }
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let path = arg_str_opt(&args, "path")
            .map(|p| resolve(&self.ctx, &p))
            .unwrap_or_else(|| self.ctx.workspace.clone());

        let mut rd = tokio::fs::read_dir(&path)
            .await
            .map_err(|e| Error::Tool(format!("cannot list {}: {e}", display_path(&path))))?;

        let mut entries: Vec<(String, bool)> = Vec::new();
        while let Some(e) = rd.next_entry().await? {
            let is_dir = e.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            entries.push((e.file_name().to_string_lossy().to_string(), is_dir));
        }
        entries.sort();

        if entries.is_empty() {
            return Ok(format!("{} is empty", display_path(&path)));
        }
        let mut out = format!("{}\n", display_path(&path));
        for (name, is_dir) in entries {
            out.push_str(&format!("  {}{}\n", name, if is_dir { "/" } else { "" }));
        }
        Ok(out)
    }
}

/// Finds files by glob pattern.
pub struct GlobTool {
    ctx: ToolContext,
}

impl GlobTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files whose path matches a glob pattern (e.g. `**/*.php`). Searches from the \
         workspace by default; skip hidden directories and common vendor files."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern, e.g. `**/*.js`." },
                "path": { "type": "string", "description": "Directory to search from. Defaults to the workspace." },
                "limit": { "type": "integer", "description": "Maximum matches. Defaults to 500." }
            },
            "required": ["pattern"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let pattern = arg_str(&args, "pattern")?;
        let root = arg_str_opt(&args, "path")
            .map(|p| resolve(&self.ctx, &p))
            .unwrap_or_else(|| self.ctx.workspace.clone());
        let limit = arg_u64_opt(&args, "limit").unwrap_or(500) as usize;

        let mut matches: Vec<String> = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| !is_noise_dir(e.path()))
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            if glob_match(&pattern, &rel) {
                matches.push(rel);
                if matches.len() >= limit {
                    break;
                }
            }
        }

        if matches.is_empty() {
            return Ok("(no matches)".into());
        }
        let mut out = matches.join("\n");
        if matches.len() >= limit {
            out.push_str(&format!("\n[truncated at {limit} matches]"));
        }
        Ok(out)
    }
}
