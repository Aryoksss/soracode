//! Caching a scan result between runs, and the entry points that use it.

use std::path::PathBuf;

use super::probe::{is_executable, scan};
use super::Inventory;
use crate::error::Result;

/// Where the scan result is cached between runs.
#[must_use]
pub fn cache_path() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("soracode");
    p.push("tools.json");
    p
}

/// Load a previously cached inventory, if it is still valid.
///
/// A cached entry is only reused when every recorded path still exists and is
/// still executable, so removing or moving a tool invalidates the cache.
#[must_use]
pub fn load_cache() -> Option<Inventory> {
    let raw = std::fs::read_to_string(cache_path()).ok()?;
    let inv: Inventory = serde_json::from_str(&raw).ok()?;
    let stale = inv.found.iter().any(|t| !is_executable(&t.path));
    (!stale).then_some(inv)
}

/// Persist an inventory for the next run.
///
/// # Errors
/// Propagates filesystem errors.
pub fn save_cache(inv: &Inventory) -> Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(inv)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Rescan and refresh the cache.
pub async fn rescan() -> Inventory {
    let inv = scan().await;
    let _ = save_cache(&inv);
    inv
}

/// Return the cached inventory when valid, otherwise scan once.
pub async fn inventory() -> Inventory {
    if let Some(inv) = load_cache() {
        return inv;
    }
    rescan().await
}
