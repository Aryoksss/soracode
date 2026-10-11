//! Small shared helpers.

/// Truncate `s` to at most `max` bytes, keeping the head and tail.
#[must_use]
pub fn truncate_middle(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max < 64 {
        return truncate_end(s, max);
    }
    let head_len = max * 2 / 3;
    let tail_len = max - head_len - 32;

    let head = safe_slice(s, 0, head_len);
    let tail_start = s.len().saturating_sub(tail_len);
    let tail = safe_slice(s, tail_start, s.len());

    format!(
        "{head}\n\n[... {} bytes omitted ...]\n\n{tail}",
        s.len() - head.len() - tail.len()
    )
}

/// Truncate `s` to at most `max` bytes, keeping only the head.
#[must_use]
pub fn truncate_end(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!(
        "{}\n[... {} bytes omitted ...]",
        safe_slice(s, 0, max),
        s.len() - max
    )
}

/// Slice `s` between byte offsets, snapping to char boundaries.
#[must_use]
pub fn safe_slice(s: &str, start: usize, end: usize) -> &str {
    let mut a = start.min(s.len());
    let mut b = end.min(s.len());
    while a > 0 && !s.is_char_boundary(a) {
        a -= 1;
    }
    while b > 0 && !s.is_char_boundary(b) {
        b -= 1;
    }
    if b < a {
        b = a;
    }
    &s[a..b]
}

/// Shorten a path for display, replacing the home prefix with `~`.
#[must_use]
pub fn display_path(p: &std::path::Path) -> String {
    let s = p.display().to_string();
    if let Some(home) = dirs::home_dir() {
        let h = home.display().to_string();
        if let Some(rest) = s.strip_prefix(&h) {
            return format!("~{rest}");
        }
    }
    s
}

/// A stable, short id derived from a UUID.
#[must_use]
pub fn short_id() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    id.split('-').next().unwrap_or("0000").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_middle_keeps_both_ends() {
        let s = "a".repeat(1000);
        let out = truncate_middle(&s, 100);
        assert!(out.len() < 300);
        assert!(out.contains("omitted"));
    }

    #[test]
    fn truncate_short_is_unchanged() {
        assert_eq!(truncate_middle("hello", 100), "hello");
    }

    #[test]
    fn safe_slice_handles_multibyte() {
        let s = "héllo wörld";
        // Slicing anywhere must not panic, even mid-codepoint.
        for i in 0..s.len() + 2 {
            let _ = safe_slice(s, 0, i);
        }
    }
}
