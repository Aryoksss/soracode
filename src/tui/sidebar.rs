use std::path::Path;
use std::sync::{Arc, Mutex};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::{theme, App, Line2, SharedSession};
use crate::config::Config;
use crate::session::Session;

/// Recent sessions as `(short id, summary)` pairs; empty when the store is
/// missing or unreadable, so the UI degrades instead of failing to start.
pub(super) fn load_sessions(cfg: &Config) -> Vec<(String, String)> {
    let path: &Path = &cfg.db_path;
    match Session::list(path, 20) {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, _c, summary)| (id, summary))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Build the shared, swappable session handle used by the TUI and its
/// submission callback.
pub(crate) fn shared(session: Session) -> SharedSession {
    Arc::new(Mutex::new(Arc::new(session)))
}

pub(super) fn draw(app: &App, f: &mut ratatui::Frame<'_>, area: Rect) {
    let lines: Vec<Line> = if app.sessions.is_empty() {
        vec![Line::from(Span::styled(
            " no recent sessions ",
            Style::default().fg(theme::TEXT_MUTED),
        ))]
    } else {
        app.sessions
            .iter()
            .enumerate()
            .map(|(i, (id, summary))| {
                let selected = i == app.sidebar_index;
                let style = if selected {
                    Style::default()
                        .fg(theme::INTERACTIVE)
                        .bg(theme::SELECTION_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };
                let marker = if selected { "▌" } else { " " };
                let name = format!("{} {}", &id[..id.len().min(8)], shorten(summary));
                Line::from(Span::styled(
                    format!("{marker} {}", truncate(&name, 26)),
                    style,
                ))
            })
            .collect()
    };

    let border = if app.sidebar_focus {
        theme::INTERACTIVE
    } else {
        theme::BORDER
    };
    let title = if app.sidebar_focus {
        " sessions — Enter resumes "
    } else {
        " sessions "
    };
    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(theme::BG_ELEMENT))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(para, area);
}

fn shorten(s: &str) -> String {
    s.rsplit('/').next().unwrap_or(s).to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Reload the sidebar list, clamping the highlight when rows disappear.
pub(super) fn refresh(app: &mut App) {
    app.sessions = load_sessions(&app.cfg);
    if app.sidebar_index >= app.sessions.len() {
        app.sidebar_index = app.sessions.len().saturating_sub(1);
    }
}

impl App {
    /// Open the highlighted session and make it the target of the next
    /// submission. Returns the new session id, or `None` when the sidebar is
    /// empty or the store cannot be reopened. The transcript is rebuilt from
    /// stored entries so the operator sees what they are resuming.
    pub(super) fn resume_selected(&mut self) -> Option<String> {
        let (id, _) = self.sessions.get(self.sidebar_index)?.clone();
        let session = Session::open(&self.db_path, Some(&id)).ok()?;
        self.transcript = rebuild(&session);
        *self.shared_session.lock().unwrap() = Arc::new(session);
        self.sessions = load_sessions(&self.cfg);
        self.scroll = 0;
        self.auto_scroll = true;
        self.thinking.clear();
        self.current_stream.clear();
        Some(id)
    }
}

fn rebuild(session: &Session) -> Vec<Line2> {
    let Ok(entries) = session.entries() else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter_map(|e| {
            let content = e.content;
            if content.trim().is_empty() {
                return None;
            }
            Some(match e.role.as_str() {
                "user" => Line2::User(content),
                "tool" => Line2::Tool(content),
                _ => Line2::Assistant(content),
            })
        })
        .collect()
}
