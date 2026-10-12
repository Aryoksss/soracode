use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme;

pub(super) fn push_markdown(lines: &mut Vec<Line>, text: &str, base: Color) {
    for raw in text.lines() {
        lines.push(render_line(raw, base));
    }
}

fn render_line(raw: &str, base: Color) -> Line<'static> {
    if is_rule(raw) {
        return Line::from(Span::styled(
            "─".repeat(40),
            Style::default().fg(theme::BORDER),
        ));
    }

    let (indent, body) = marker(raw);
    if let Some((level, rest)) = heading(raw) {
        let _ = level;
        return Line::from(vec![
            Span::styled("  ".repeat(level), Style::default().fg(base)),
            Span::styled(
                rest,
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    let mut spans = vec![Span::styled(indent, Style::default().fg(base))];
    spans.extend(inline(body, base));
    Line::from(spans)
}

fn is_rule(raw: &str) -> bool {
    let t = raw.trim();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*'))
}

fn heading(raw: &str) -> Option<(usize, String)> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count().min(6);
    let text = trimmed[level..].trim_start().to_string();
    Some((level, text))
}

/// Returns the leading indent and the body after any list marker.
fn marker(raw: &str) -> (String, &str) {
    let lead = raw.len() - raw.trim_start().len();
    let depth = lead / 2;
    let trimmed = raw.trim_start();
    let pad = "  ".repeat(depth);

    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        return (format!("{pad}  • "), rest);
    }
    if let Some((num, rest)) = numbered(trimmed) {
        return (format!("{pad}  {num}. "), rest);
    }
    (format!("{pad}{}", " ".repeat(lead % 2)), trimmed)
}

fn numbered(s: &str) -> Option<(String, &str)> {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = s[digits.len()..].strip_prefix(". ")?;
    Some((digits, rest))
}

/// Parse inline emphasis and links, splitting on backticks first so code spans
/// never get re-parsed.
fn inline(text: &str, base: Color) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for (i, seg) in text.split('`').enumerate() {
        if seg.is_empty() {
            continue;
        }
        if i % 2 == 1 {
            out.push(Span::styled(
                seg.to_string(),
                Style::default().fg(theme::SUCCESS),
            ));
            continue;
        }
        emphasis(seg, base, &mut out);
    }
    out
}

fn emphasis(text: &str, base: Color, out: &mut Vec<Span<'static>>) {
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((label, url, after)) = link(rest) {
            out.push(Span::styled(
                label.to_string(),
                Style::default()
                    .fg(theme::INTERACTIVE)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            out.push(Span::styled(
                format!(" ({url})"),
                Style::default().fg(theme::TEXT_MUTED),
            ));
            rest = after;
            continue;
        }
        if let Some((label, after)) = delimited(rest, "**") {
            out.push(Span::styled(
                label.to_string(),
                Style::default().fg(base).add_modifier(Modifier::BOLD),
            ));
            rest = after;
            continue;
        }
        if let Some((label, after)) = delimited(rest, "*").or_else(|| delimited(rest, "_")) {
            out.push(Span::styled(
                label.to_string(),
                Style::default().fg(base).add_modifier(Modifier::ITALIC),
            ));
            rest = after;
            continue;
        }
        let take = rest.find(['*', '_', '[']).unwrap_or(rest.len()).max(1);
        out.push(Span::styled(
            rest[..take].to_string(),
            Style::default().fg(base),
        ));
        rest = &rest[take..];
    }
}

fn link(s: &str) -> Option<(&str, &str, &str)> {
    let body = s.strip_prefix('[')?;
    let close = body.find(']')?;
    let after = body[close + 1..].strip_prefix('(')?;
    let end = after.find(')')?;
    Some((&body[..close], &after[..end], &after[end + 1..]))
}

fn delimited<'a>(s: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    let body = s.strip_prefix(delim)?;
    let end = body.find(delim)?;
    if end == 0 {
        return None;
    }
    Some((&body[..end], &body[end + delim.len()..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn heading_is_rendered_without_hashes() {
        let l = render_line("## Title", Color::White);
        assert_eq!(text(&l).trim(), "Title");
    }

    #[test]
    fn nested_bullets_indent_by_depth() {
        let shallow = render_line("- a", Color::White);
        let deep = render_line("    - a", Color::White);
        assert!(text(&deep).starts_with("      • "), "deep: {}", text(&deep));
        assert!(
            text(&shallow).starts_with("  • "),
            "shallow: {}",
            text(&shallow)
        );
    }

    #[test]
    fn numbered_items_keep_their_number() {
        let l = render_line("3. third", Color::White);
        assert!(text(&l).contains("3. third"));
    }

    #[test]
    fn links_show_label_and_url() {
        let l = render_line("see [home](https://x.test) now", Color::White);
        assert!(text(&l).contains("home (https://x.test) now"));
    }

    #[test]
    fn code_spans_are_not_reparsed_for_emphasis() {
        let l = render_line("use `a*b*c` here", Color::White);
        assert!(text(&l).contains("a*b*c"));
    }

    #[test]
    fn rule_becomes_a_line() {
        let l = render_line("---", Color::White);
        assert!(text(&l).starts_with('─'));
    }
}
