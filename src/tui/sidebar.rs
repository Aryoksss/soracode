use std::path::Path;

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
