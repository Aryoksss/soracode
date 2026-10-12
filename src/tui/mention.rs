//! `@` file mentions: parse the active token and list matching workspace files.

/// The active `@` mention token, e.g. `@src/tui` in `look at @src/tui`.
pub(super) fn at_token(input: &str) -> Option<&str> {
    let start = input.rfind('@')?;
    let before_ok = start == 0 || input[..start].ends_with(char::is_whitespace);
    let token = &input[start + 1..];
    let token_ok = !token.contains(char::is_whitespace);
    (before_ok && token_ok).then_some(token)
}

/// Workspace-relative files whose path contains `needle`, capped and sorted.
pub(super) fn matching_files(root: &std::path::Path, needle: &str) -> Vec<String> {
    let needle = needle.to_lowercase();
    walkdir::WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .filter_entry(|e| e.file_type().is_file() || !crate::tools::fs::is_noise_dir(e.path()))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.path().strip_prefix(root).ok().map(|p| p.to_path_buf()))
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| needle.is_empty() || p.to_lowercase().contains(&needle))
        .take(200)
        .collect()
}
