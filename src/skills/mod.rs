//! On-demand knowledge packs.
//!
//! A skill is a Markdown file that teaches one technique in depth. Packs are
//! discovered from a user directory and the workspace, listed compactly in the
//! prompt, and loaded in full only when the agent asks for one — so a deep
//! library never bloats the context window.

use std::path::{Path, PathBuf};

/// One discovered skill pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    /// Stable name used to load the pack (its file stem).
    pub name: String,
    /// One-line summary taken from the first heading or paragraph.
    pub summary: String,
    /// Path the pack was read from.
    pub path: PathBuf,
}

/// Directories searched for packs, most specific last.
#[must_use]
pub fn search_dirs(workspace: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(cfg) = dirs::config_dir() {
        dirs.push(cfg.join("soracode").join("skills"));
    }
    dirs.push(workspace.join(".soracode").join("skills"));
    dirs
}

/// Discover every pack across the search directories, de-duplicated by name
/// with the most specific directory winning.
#[must_use]
pub fn discover(workspace: &Path) -> Vec<Skill> {
    discover_in(&search_dirs(workspace))
}

/// Discover packs across an explicit, ordered set of directories.
///
/// Later directories win on a name clash. Exposed so callers and tests can
/// pin the search set instead of inheriting the host's config directory.
#[must_use]
pub fn discover_in(dirs: &[PathBuf]) -> Vec<Skill> {
    let mut found: Vec<Skill> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            found.retain(|s| s.name != name);
            let summary = summary_of(&path);
            found.push(Skill {
                name,
                summary,
                path,
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Read a pack's full text by name.
///
/// # Errors
/// Returns an error message string when no pack matches or it cannot be read.
pub fn load(workspace: &Path, name: &str) -> Result<String, String> {
    let skill = discover(workspace)
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| {
            let names: Vec<String> = discover(workspace).into_iter().map(|s| s.name).collect();
            if names.is_empty() {
                format!("no skill named `{name}`; no packs are installed")
            } else {
                format!("no skill named `{name}`; available: {}", names.join(", "))
            }
        })?;
    std::fs::read_to_string(&skill.path).map_err(|e| format!("could not read skill `{name}`: {e}"))
}

/// A compact catalogue for the prompt, or an empty string when no packs exist.
#[must_use]
pub fn catalogue(workspace: &Path) -> String {
    let skills = discover(workspace);
    if skills.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n## Skills\n\nLoad a pack with the `skill` tool when a task calls for depth on one of these techniques:\n\n");
    for sk in skills {
        s.push_str(&format!("- `{}` — {}\n", sk.name, sk.summary));
    }
    s
}

/// First meaningful line of a pack, stripped of heading markers.
fn summary_of(path: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    for line in text.lines() {
        let line = line.trim().trim_start_matches('#').trim();
        if !line.is_empty() {
            return line.chars().take(120).collect();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests;
