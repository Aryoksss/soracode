//! Turning a `write`/`edit` tool result into colored diff lines.

/// Split a file-change tool result into `(header, diff lines)`. The header is
/// the first line; the diff lines carry a leading `-` (old) or `+` (new).
pub(super) fn file_change(output: &str) -> (String, Vec<String>) {
    let mut lines = output.lines();
    let head = lines.next().unwrap_or("").to_string();
    let rest: Vec<&str> = lines.collect();
    let mut out = Vec::new();
    let old_at = rest.iter().position(|l| l.trim_end() == "--- old ---");
    let new_at = rest.iter().position(|l| l.trim_end() == "--- new ---");
    if let (Some(o), Some(n)) = (old_at, new_at) {
        for l in &rest[o + 1..n] {
            out.push(format!("- {l}"));
        }
        for l in &rest[n + 1..] {
            out.push(format!("+ {l}"));
        }
    }
    (head, out)
}

#[cfg(test)]
mod tests {
    use super::file_change;

    #[test]
    fn splits_old_and_new_into_signed_lines() {
        let out = "edited f.rs (1 replacement)\n--- old ---\nlet x = 1;\n--- new ---\nlet x = 2;";
        let (head, lines) = file_change(out);
        assert_eq!(head, "edited f.rs (1 replacement)");
        assert_eq!(lines, vec!["- let x = 1;", "+ let x = 2;"]);
    }

    #[test]
    fn a_plain_result_has_no_diff_lines() {
        let (head, lines) = file_change("wrote 12 bytes to /tmp/a.txt");
        assert_eq!(head, "wrote 12 bytes to /tmp/a.txt");
        assert!(lines.is_empty());
    }

    #[test]
    fn multiline_bodies_are_preserved() {
        let out = "edited f.rs\n--- old ---\na\nb\n--- new ---\nc\nd";
        let (_, lines) = file_change(out);
        assert_eq!(lines, vec!["- a", "- b", "+ c", "+ d"]);
    }
}
