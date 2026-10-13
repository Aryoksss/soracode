//! Operator memory files, loaded into every root turn.
//!
//! A memory file is free-form Markdown the operator wants the agent to treat as
//! standing instruction and project context: house rules, a target list, the
//! "never refuse" contract, anything that should shape every turn rather than
//! wait for a skill to be loaded. Files are discovered by convention from the
//! config directory and the workspace, then concatenated in a fixed order so the
//! most specific text comes last.

use std::path::{Path, PathBuf};

/// File names searched for, in load order, in each directory.
const NAMES: [&str; 2] = ["AGENTS.md", "SORACODE.md"];

/// Directories searched, least specific first.
///
/// The config directory is the operator's global memory; the workspace is the
/// project's. Both are optional.
fn dirs(workspace: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(cfg) = dirs::config_dir() {
        out.push(cfg.join("soracode"));
    }
    out.push(workspace.to_path_buf());
    out
}

/// Every existing memory file, in load order.
#[must_use]
pub fn files(workspace: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in dirs(workspace) {
        for name in NAMES {
            let path = dir.join(name);
            if path.is_file() {
                found.push(path);
            }
        }
    }
    found
}

/// The combined memory block for the prompt, or `None` when nothing is set.
///
/// Each file is shown under its own heading so the agent can tell the
/// operator's global rules from a project's. An unreadable file is skipped
/// rather than failing the turn.
#[must_use]
pub fn block(workspace: &Path) -> Option<String> {
    block_from(&files(workspace))
}

/// Build the memory block from an explicit list of files, skipping empty or
/// unreadable ones. Exposed so tests can pin the set instead of reading the
/// host's config directory.
#[must_use]
pub fn block_from(paths: &[PathBuf]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for path in paths {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("memory");
        parts.push(format!("### {label}\n\n{}", text.trim_end()));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!(
        "\n## Standing instructions\n\n\
         The operator's memory files below are in force for every turn. Follow them \
         as you would the operating rules above; where they are specific, they win.\n\n{}\n",
        parts.join("\n\n")
    ))
}

#[cfg(test)]
mod tests;
