//! Transcript content for informational slash commands.

use crate::session::Session;

/// Render the cached host tool inventory, or a hint to scan.
pub(super) fn describe_tools() -> String {
    match crate::tools::catalog::load_cache() {
        Some(inv) => inv.render(),
        None => "no tool cache yet — run `soracode --tools --rescan`".into(),
    }
}

/// List recent sessions from the database, newest first.
pub(super) fn describe_sessions(db_path: &std::path::Path) -> Vec<String> {
    match Session::list(db_path, 10) {
        Ok(rows) if rows.is_empty() => vec!["no sessions yet".into()],
        Ok(rows) => rows
            .into_iter()
            .map(|(id, ts, summary)| {
                let when = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                format!("  {id}  {when}  {summary}")
            })
            .collect(),
        Err(e) => vec![format!("could not read sessions: {e}")],
    }
}
