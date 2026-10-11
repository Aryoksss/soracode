//! Filesystem tools: `read`, `write`, `edit`, `list`, `glob`, `grep`.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};

use super::{arg_str, arg_str_opt, arg_u64_opt, Tool, ToolContext};
use crate::error::{Error, Result};
use crate::util::{display_path, truncate_end};

fn resolve(ctx: &ToolContext, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        ctx.workspace.join(path)
    }
}

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
         `limit` to page through large files."
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
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            Error::Tool(format!("cannot read {}: {e}", display_path(&path)))
        })?;

        let offset = arg_u64_opt(&args, "offset").unwrap_or(1).max(1) as usize;
        let limit = arg_u64_opt(&args, "limit").unwrap_or(2000) as usize;

        let total = content.lines().count();
        let mut out = String::new();
        for (i, line) in content.lines().enumerate().skip(offset - 1).take(limit) {
            out.push_str(&format!("{:>6}: {}\n", i + 1, line));
        }
        if out.is_empty() {
            return Ok(format!("(no lines at offset {offset}; file has {total})"));
        }
        if offset - 1 + limit < total {
            out.push_str(&format!(
                "\n[... {} more lines; file has {total} ...]",
                total - (offset - 1 + limit)
            ));
        }
        Ok(out)
    }
}

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

        let re = regex::Regex::new(&pattern)
            .map_err(|e| Error::Tool(format!("invalid regex: {e}")))?;

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

            // Skip large or binary-ish files.
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
                    hits.push(format!("{rel}:{}: {}", i + 1, line.trim()));
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

fn is_noise_dir(p: &Path) -> bool {
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".git" | "node_modules" | "target" | ".venv" | "venv" | "__pycache__" | "dist" | "build"
    )
}

/// Minimal glob supporting `**`, `*`, `?` and literal text.
fn glob_match(pattern: &str, text: &str) -> bool {
    fn inner(p: &[u8], t: &[u8]) -> bool {
        if p.is_empty() {
            return t.is_empty();
        }
        match p[0] {
            b'*' => {
                if p.len() > 1 && p[1] == b'*' {
                    // `**` matches across separators
                    let rest = &p[2..];
                    let rest = rest.strip_prefix(b"/").unwrap_or(rest);
                    for i in 0..=t.len() {
                        if inner(rest, &t[i..]) {
                            return true;
                        }
                    }
                    inner(rest, t)
                } else {
                    let rest = &p[1..];
                    for i in 0..=t.len() {
                        if t[..i].contains(&b'/') {
                            break;
                        }
                        if inner(rest, &t[i..]) {
                            return true;
                        }
                    }
                    false
                }
            }
            b'?' => !t.is_empty() && t[0] != b'/' && inner(&p[1..], &t[1..]),
            c => !t.is_empty() && t[0] == c && inner(&p[1..], &t[1..]),
        }
    }
    inner(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn glob_star_within_segment() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "src/main.rs"));
    }

    #[test]
    fn glob_double_star_crosses_segments() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "main.rs"));
    }

    #[test]
    fn glob_question_mark() {
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "a/c"));
    }
}
