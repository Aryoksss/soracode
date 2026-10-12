use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme;

/// Returns the exclusive end index when a pipe table starts at `start`,
/// i.e. a header row followed by a `---|---` separator.
pub(super) fn table_end(rows: &[&str], start: usize) -> Option<usize> {
    if start + 1 >= rows.len() || cells(rows[start]).len() < 2 {
        return None;
    }
    if !is_separator(rows[start + 1]) {
        return None;
    }
    let mut end = start + 2;
    while end < rows.len() && rows[end].contains('|') && !rows[end].trim().is_empty() {
        end += 1;
    }
    Some(end)
}

pub(super) fn push_table(lines: &mut Vec<Line>, block: &[&str], base: Color) {
    let header = cells(block[0]);
    let width = header.len().max(1);
    lines.push(table_row(&header, base, true));
    lines.push(Line::from(Span::styled(
        "  ".to_string() + &"─".repeat(width * 14),
        Style::default().fg(theme::border()),
    )));
    for row in &block[2..] {
        lines.push(table_row(&cells(row), base, false));
    }
}

fn cells(row: &str) -> Vec<String> {
    row.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn is_separator(row: &str) -> bool {
    let cols = cells(row);
    !cols.is_empty()
        && cols
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
}

fn table_row(cells: &[String], base: Color, bold: bool) -> Line<'static> {
    let mut spans = vec![Span::styled("  ", Style::default().fg(base))];
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme::border())));
        }
        let mut style = Style::default().fg(base);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        spans.push(Span::styled(cell.clone(), style));
    }
    Line::from(spans)
}
