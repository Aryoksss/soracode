//! Tests for the transcript markdown renderer.

use ratatui::style::Color;
use ratatui::text::Line;

use super::markdown::{push_markdown_indented, render_line};

fn text(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn render(md: &str) -> Vec<String> {
    let mut lines = Vec::new();
    push_markdown_indented(&mut lines, md, Color::White, "");
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
        .collect()
}

#[test]
fn heading_is_rendered_without_hashes() {
    let l = render_line("## Title", Color::White);
    assert_eq!(text(&l).trim(), "Title");
}

#[test]
fn nested_bullets_indent_by_depth() {
    let deep = text(&render_line("    - a", Color::White));
    let shallow = text(&render_line("- a", Color::White));
    assert!(deep.starts_with("      • "), "deep: {deep}");
    assert!(shallow.starts_with("  • "), "shallow: {shallow}");
}

#[test]
fn numbered_items_keep_their_number() {
    assert!(text(&render_line("3. third", Color::White)).contains("3. third"));
}

#[test]
fn links_show_label_and_url() {
    let l = render_line("see [home](https://x.test) now", Color::White);
    assert!(text(&l).contains("home (https://x.test) now"));
}

#[test]
fn code_spans_are_not_reparsed_for_emphasis() {
    assert!(text(&render_line("use `a*b*c` here", Color::White)).contains("a*b*c"));
}

#[test]
fn rule_becomes_a_line() {
    assert!(text(&render_line("---", Color::White)).starts_with('─'));
}

#[test]
fn pipe_table_renders_header_and_rows() {
    let out = render("| a | b |\n| --- | --- |\n| 1 | 2 |");
    assert!(out[0].contains("a │ b"), "header: {}", out[0]);
    assert!(out.iter().any(|l| l.contains('─')), "separator missing");
    assert!(out.iter().any(|l| l.contains("1 │ 2")), "row missing");
}

#[test]
fn blockquote_gets_a_bar() {
    let out = render("> quoted text");
    assert!(out[0].starts_with("  ▏ quoted"), "got: {}", out[0]);
}

#[test]
fn plain_pipe_text_is_not_a_table() {
    let out = render("a | b");
    assert_eq!(out.len(), 1);
    assert!(!out[0].contains('─'));
}

#[test]
fn intraword_underscores_are_not_emphasis() {
    let text = render("see my_under_score_file.txt now").join("\n");
    assert!(text.contains("my_under_score_file.txt"), "got: {text}");
}

#[test]
fn underscore_emphasis_still_works_at_word_boundaries() {
    let text = render("a _note_ here").join("\n");
    assert!(text.contains("note"), "got: {text}");
    assert!(!text.contains("_note_"), "got: {text}");
}
