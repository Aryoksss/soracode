use std::sync::{Arc, Mutex};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::{theme, App, SharedSession};
use crate::config::Config;
use crate::session::Session;

/// Recent sessions as `(short id, summary)` pairs; empty when the store is
/// missing or unreadable, so the UI degrades instead of failing to start.
pub(super) fn load_sessions(cfg: &Config) -> Vec<(String, i64, String)> {
    Session::list(&cfg.db_path, 20).unwrap_or_default()
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
            Style::default().fg(theme::text_dim()),
        ))]
    } else {
        app.sessions
            .iter()
            .enumerate()
            .map(|(i, (id, ts, summary))| {
                let selected = i == app.sidebar_index;
                let (bg, marker_fg, id_fg, sum_fg, time_fg) = if selected {
                    (
                        theme::selection_bg(),
                        theme::accent(),
                        theme::text(),
                        theme::text_muted(),
                        theme::text_dim(),
                    )
                } else {
                    (
                        theme::panel_bg(),
                        theme::panel_bg(),
                        theme::text(),
                        theme::text_dim(),
                        theme::text_dim(),
                    )
                };
                let marker = if selected { "▌" } else { " " };
                let short = &id[..id.len().min(8)];
                let name = shorten(summary);
                let when = relative_time(*ts);
                Line::from(vec![
                    Span::styled(format!("{marker} "), Style::default().fg(marker_fg).bg(bg)),
                    Span::styled(
                        format!("{short:<9}"),
                        Style::default()
                            .fg(id_fg)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<9}", truncate(&when, 8)),
                        Style::default().fg(time_fg).bg(bg),
                    ),
                    Span::styled(truncate(&name, 13), Style::default().fg(sum_fg).bg(bg)),
                ])
            })
            .collect()
    };

    let (border, title) = if app.sidebar_focus {
        (
            theme::interactive(),
            " sessions  ↑↓  ⏎ resume  d delete ".to_string(),
        )
    } else {
        (
            theme::border(),
            format!(" sessions · {} ", app.sessions.len()),
        )
    };
    let para = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(theme::panel_bg()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(theme::accent())
                        .add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(para, area);
}

fn shorten(s: &str) -> String {
    s.rsplit('/').next().unwrap_or(s).to_string()
}

/// Compact "how long ago" label for a unix timestamp, newest = "now".
fn relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let secs = now.saturating_sub(ts).max(0);
    match secs {
        0..=59 => "now".into(),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86_399 => format!("{}h", secs / 3600),
        86_400..=2_591_999 => format!("{}d", secs / 86_400),
        _ => format!("{}w", secs / 604_800),
    }
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
        let (id, _, _) = self.sessions.get(self.sidebar_index)?.clone();
        let session = Session::open(&self.db_path, Some(&id)).ok()?;
        self.transcript = super::transcript::rebuild(&session);
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
        let Some((id, _, _)) = self.sessions.get(self.sidebar_index).cloned() else {
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
