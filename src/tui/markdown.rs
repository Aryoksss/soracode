use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::table;
use super::theme;

/// Render `text` as markdown, prefixing every emitted line with `indent`.
pub(super) fn push_markdown_indented(lines: &mut Vec<Line>, text: &str, base: Color, indent: &str) {
    let rows: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let mut fence = false;
    while i < rows.len() {
        if rows[i].trim_start().starts_with("```") {
            fence = !fence;
            i += 1;
            continue;
        }
        if fence {
            let mut line = code_line(rows[i]);
            if !indent.is_empty() {
                line.spans
                    .insert(0, Span::styled(indent.to_string(), Style::default()));
            }
            lines.push(line);
            i += 1;
            continue;
        }
        if let Some(end) = table::table_end(&rows, i) {
            let start = lines.len();
            table::push_table(lines, &rows[i..end], base);
            if !indent.is_empty() {
                prepend_indent(&mut lines[start..], indent);
            }
            i = end;
            continue;
        }
        let mut line = render_line(rows[i], base);
        if !indent.is_empty() {
            line.spans
                .insert(0, Span::styled(indent.to_string(), Style::default()));
        }
        lines.push(line);
        i += 1;
    }
}

/// One line inside a fenced code block: a dim gutter and code-coloured text.
fn code_line(raw: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  │ ", Style::default().fg(theme::border())),
        Span::styled(raw.to_string(), Style::default().fg(theme::info())),
    ])
}

/// Prefix each line with an unstyled indent span.
pub(super) fn prepend_indent(lines: &mut [Line], indent: &str) {
    for line in lines {
        line.spans
            .insert(0, Span::styled(indent.to_string(), Style::default()));
    }
}

pub(super) fn render_line(raw: &str, base: Color) -> Line<'static> {
    if is_rule(raw) {
        return Line::from(Span::styled(
            "─".repeat(40),
            Style::default().fg(theme::border()),
        ));
    }

    if let Some(rest) = raw.trim_start().strip_prefix('>') {
        let mut spans = vec![Span::styled(
            "  ▏ ".to_string(),
            Style::default().fg(theme::accent()),
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
                    .fg(theme::accent())
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
                Style::default().fg(theme::success()),
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
                    .fg(theme::interactive())
                    .add_modifier(Modifier::UNDERLINED),
            ));
            out.push(Span::styled(
                format!(" ({url})"),
                Style::default().fg(theme::text_muted()),
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
