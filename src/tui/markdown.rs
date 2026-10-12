use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::table;
use super::theme;

pub(super) fn push_markdown(lines: &mut Vec<Line>, text: &str, base: Color) {
    let rows: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < rows.len() {
        if let Some(end) = table::table_end(&rows, i) {
            table::push_table(lines, &rows[i..end], base);
            i = end;
            continue;
        }
        lines.push(render_line(rows[i], base));
        i += 1;
    }
}

pub(super) fn render_line(raw: &str, base: Color) -> Line<'static> {
    if is_rule(raw) {
        return Line::from(Span::styled(
            "─".repeat(40),
            Style::default().fg(theme::BORDER),
        ));
    }

    if let Some(rest) = raw.trim_start().strip_prefix('>') {
        let mut spans = vec![Span::styled(
            "  ▏ ".to_string(),
            Style::default().fg(theme::ACCENT),
        )];
        spans.extend(inline(rest.trim_start(), base));
        return Line::from(spans);
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
        if let Some((label, after)) = delimited(rest, "*").or_else(|| underscore_em(rest)) {
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

/// Underscore emphasis, but never intraword: `my_under_score_file.txt` keeps
/// its underscores, while `_note_` is italic. Follows CommonMark's rule that a
/// `_` surrounded by alphanumerics neither opens nor closes emphasis.
fn underscore_em(s: &str) -> Option<(&str, &str)> {
    let body = s.strip_prefix('_')?;
    if body.starts_with('_') {
        return None;
    }
    let end = body.find('_')?;
    if end == 0
        || body[end + 1..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
    {
        return None;
    }
    Some((&body[..end], &body[end + 1..]))
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
