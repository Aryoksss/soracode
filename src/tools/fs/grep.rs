//! `grep` — regex content search across a directory tree.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{glob_match, is_noise_dir, resolve};
use crate::error::{Error, Result};
use crate::tools::{arg_str, arg_str_opt, arg_u64_opt, Tool, ToolContext};
use crate::util::truncate_end;

/// Searches file contents by regex.
pub struct GrepTool {
    ctx: ToolContext,
}

impl GrepTool {
    /// Build the tool for the given context.
    #[must_use]
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search file contents with a regular expression and return matching lines with their \
         file and line number. Respects a file glob to narrow the search."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Rust-compatible regex." },
                "path": { "type": "string", "description": "Directory to search. Defaults to the workspace." },
                "file_glob": { "type": "string", "description": "Only search files matching this glob, e.g. `**/*.py`." },
                "limit": { "type": "integer", "description": "Maximum matching lines. Defaults to 200." }
            },
            "required": ["pattern"]
        })
    }

    async fn run(&self, args: Value) -> Result<String> {
        let pattern = arg_str(&args, "pattern")?;
        let root = arg_str_opt(&args, "path")
            .map(|p| resolve(&self.ctx, &p))
            .unwrap_or_else(|| self.ctx.workspace.clone());
        let file_glob = arg_str_opt(&args, "file_glob");
        let limit = arg_u64_opt(&args, "limit").unwrap_or(200) as usize;

        let re =
            regex::Regex::new(&pattern).map_err(|e| Error::Tool(format!("invalid regex: {e}")))?;

        let mut hits: Vec<String> = Vec::new();
        'outer: for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| !is_noise_dir(e.path()))
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            if let Some(g) = &file_glob {
                if !glob_match(g, &rel) {
                    continue;
                }
            }

            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > 4 * 1024 * 1024 {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };

            for (i, line) in text.lines().enumerate() {
                if re.is_match(line) {
                    hits.push(format!(
                        "{rel}:{}: {}",
                        i + 1,
                        truncate_end(line.trim(), 400)
                    ));
                    if hits.len() >= limit {
                        break 'outer;
                    }
                }
            }
        }

        if hits.is_empty() {
            return Ok("(no matches)".into());
        }
        let mut out = hits.join("\n");
        if hits.len() >= limit {
            out.push_str(&format!("\n[truncated at {limit} matching lines]"));
        }
        Ok(out)
    }
}
