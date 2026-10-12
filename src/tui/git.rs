//! A tiny, read-only peek at the current git branch for the status line.

/// Return the checked-out branch for `start`, walking up to the repo root.
/// Reads `.git/HEAD` directly — no subprocess — so it is safe to call on redraw.
pub(super) fn branch(start: &std::path::Path) -> Option<String> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let head = d.join(".git/HEAD");
        if let Ok(raw) = std::fs::read_to_string(&head) {
            let text = raw.trim();
            if let Some(name) = text.strip_prefix("ref: refs/heads/") {
                return Some(name.to_string());
            }
            if !text.is_empty() {
                return Some(text.chars().take(7).collect());
            }
        }
        dir = d.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::branch;

    #[test]
    fn missing_repo_yields_none() {
        let dir = std::env::temp_dir().join("soracode-git-none");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(branch(&dir), None);
    }
}
