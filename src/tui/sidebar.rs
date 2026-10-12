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

pub(super) fn draw(app: &mut App, f: &mut ratatui::Frame<'_>, area: Rect) {
    app.sidebar_area = area;
    let lines: Vec<Line> = if app.sessions.is_empty() {
        vec![Line::from(Span::styled(
            "  no recent sessions ",
            Style::default().fg(theme::TEXT_DIM),
        ))]
    } else {
        app.sessions
            .iter()
            .enumerate()
            .map(|(i, (id, summary))| {
                let selected = i == app.sidebar_index;
                let (bg, marker_fg, id_fg, sum_fg) = if selected {
                    (
                        theme::SELECTION_BG,
                        theme::ACCENT,
                        theme::TEXT,
                        theme::TEXT_MUTED,
                    )
                } else {
                    (
                        theme::PANEL_BG,
                        theme::PANEL_BG,
                        theme::TEXT,
                        theme::TEXT_DIM,
                    )
                };
                let marker = if selected { "▌" } else { " " };
                let short = &id[..id.len().min(8)];
                let name = format!("{} ", shorten(summary));
                Line::from(vec![
                    Span::styled(format!("{marker} "), Style::default().fg(marker_fg).bg(bg)),
                    Span::styled(
                        short,
                        Style::default()
                            .fg(id_fg)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {}", truncate(&name, 18)),
                        Style::default().fg(sum_fg).bg(bg),
                    ),
                ])
            })
            .collect()
    };

    let (border, title) = if app.sidebar_focus {
        (
            theme::INTERACTIVE,
            " sessions  ↑↓  ⏎ resume  d delete ".to_string(),
        )
    } else {
        (
            theme::BORDER,
            format!(" sessions · {} ", app.sessions.len()),
        )
    };
    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(theme::PANEL_BG))
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
        self.session_id = id.clone();
        self.sessions = load_sessions(&self.cfg);
        self.scroll = 0;
        self.auto_scroll = true;
        self.thinking.clear();
        self.current_stream.clear();
        Some(id)
    }

    /// Permanently delete the highlighted session. Refuses when it is the one
    /// currently open, because the run loop still holds it. Returns a status
    /// line describing the outcome.
    pub(super) fn delete_selected(&mut self) -> String {
        let Some((id, _)) = self.sessions.get(self.sidebar_index).cloned() else {
            return "no session to delete".into();
        };
        if self.shared_session.lock().unwrap().id() == id {
            return format!("cannot delete the open session {id}; switch first");
        }
        match Session::delete(&self.db_path, &id) {
            Ok(true) => {
                self.sessions = load_sessions(&self.cfg);
                if self.sidebar_index >= self.sessions.len() {
                    self.sidebar_index = self.sessions.len().saturating_sub(1);
                }
                format!("deleted session {id}")
            }
            Ok(false) => "session already gone".into(),
            Err(e) => format!("delete failed: {e}"),
        }
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
