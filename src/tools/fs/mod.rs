//! Filesystem tools: `read`, `write`, `edit`, `list`, `glob`, `grep`.

mod grep;
mod list;
mod read;
mod write;

use std::path::{Path, PathBuf};

use super::ToolContext;

pub use grep::GrepTool;
pub use list::{GlobTool, ListTool};
pub use read::ReadTool;
pub use write::{EditTool, WriteTool};

/// Resolve a user-supplied path against the workspace unless it is absolute.
fn resolve(ctx: &ToolContext, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        ctx.workspace.join(path)
    }
}

/// Directories that are noise for a source search and are skipped by walkers.
pub(crate) fn is_noise_dir(p: &Path) -> bool {
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
    use super::{glob_match, resolve};
    use crate::config::AgentConfig;
    use crate::tools::notes::Notes;
    use crate::tools::{Tool, ToolContext};
    use std::sync::Arc;

    fn ctx(workspace: &std::path::Path, output_limit: usize) -> ToolContext {
        let cfg = AgentConfig {
            tool_output_limit: output_limit,
            ..Default::default()
        };
        ToolContext {
            config: Arc::new(cfg),
            workspace: workspace.to_path_buf(),
            depth: 0,
            notes: Notes::default(),
            engagement: crate::engagement::Engagement::in_memory().unwrap(),
            spawner: None,
        }
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("soracode-fs-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn read_caps_a_single_giant_line() {
        let dir = scratch("giantline");
        let file = dir.join("one-line.txt");
        std::fs::write(&file, "A".repeat(500_000)).unwrap();
        let budget = 4096;
        let out = super::ReadTool::new(ctx(&dir, budget))
            .run(serde_json::json!({ "path": file.to_string_lossy() }))
            .await
            .unwrap();

        assert!(
            out.len() < budget,
            "giant single line leaked {} bytes into the response",
            out.len()
        );
        assert!(out.contains("truncated"), "no truncation notice: {out}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn read_caps_total_output_across_many_lines() {
        let dir = scratch("manylines");
        let file = dir.join("many.txt");
        let body: String = (0..5000).map(|i| format!("line {i} xxxxxxxx\n")).collect();
        std::fs::write(&file, body).unwrap();
        let budget = 4096;
        let out = super::ReadTool::new(ctx(&dir, budget))
            .run(serde_json::json!({ "path": file.to_string_lossy(), "limit": 5000 }))
            .await
            .unwrap();

        assert!(
            out.len() < budget + 256,
            "many-line file leaked {} bytes",
            out.len()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

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

    #[tokio::test]
    async fn glob_lists_every_match() {
        let dir = scratch("glob-integration");
        for f in ["one.txt", "two.txt", "three.txt", "four.md"] {
            std::fs::write(dir.join(f), "x").unwrap();
        }

        let out = super::GlobTool::new(ctx(&dir, 4096))
            .run(serde_json::json!({ "pattern": "*.txt" }))
            .await
            .unwrap();
        let got: Vec<&str> = out.lines().collect();
        assert_eq!(got.len(), 3, "expected 3 .txt files, got {got:?}");

        assert!(resolve(&ctx(&dir, 4096), "one.txt").is_absolute());
        let _ = std::fs::remove_dir_all(dir);
    }
}
