//! Locating executables on `PATH` and reading their version strings.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{candidate, candidates, Candidate, FoundTool, Inventory, Probe};

/// Locate an executable by walking `PATH` the way the shell would.
#[must_use]
pub fn which(bin: &str) -> Option<PathBuf> {
    if bin.contains('/') {
        let p = PathBuf::from(bin);
        return is_executable(&p).then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.join(bin))
        .find(|c| is_executable(c))
}

/// True when the path is a regular file carrying an execute bit.
pub(super) fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Ask a tool for its version, tolerating the many ways they misbehave.
///
/// Some tools reject the flag and print a usage banner to stderr while still
/// emitting the version on stdout, so both streams are inspected.
async fn probe_version(path: &Path, probe: Probe) -> Option<String> {
    let arg = probe.arg()?;
    let bin = path.file_name()?.to_string_lossy().to_string();

    let mut cmd = tokio::process::Command::new(path);
    cmd.arg(arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let out = tokio::time::timeout(Duration::from_secs(5), cmd.output())
        .await
        .ok()?
        .ok()?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    extract_version(&stdout, &bin).or_else(|| extract_version(&stderr, &bin))
}

/// Pull the version out of a tool's banner, stripping ANSI colour codes
/// because several tools emit them even when not attached to a terminal.
///
/// Taking the first number in the text is wrong often enough to matter:
/// `ssh -V` prints `OpenSSH_10.2p1, OpenSSL 3.5.9` where the first number
/// belongs to the dependency, and `curl --version` names libcurl before any
/// version at all. Candidates are therefore scored, preferring a number that
/// follows the tool's own name or the word `version`, and demoting numbers
/// that follow a known library or dependency marker.
pub(super) fn extract_version(text: &str, bin: &str) -> Option<String> {
    let clean = strip_ansi(text);
    let re = regex::Regex::new(
        r"(?i)([a-z][a-z0-9_+.-]*?)[\s_\-]*v?(\d+\.\d+(?:\.\d+)?(?:[a-z]\d+)?(?:[-.][0-9a-z]+)*)",
    )
    .ok()?;

    let bin_lc = bin.to_lowercase();
    let bin_stem = bin_lc.split('-').next().unwrap_or(&bin_lc).to_string();

    let mut best: Option<(i32, String)> = None;

    for cap in re.captures_iter(&clean) {
        let label = cap.get(1)?.as_str().to_lowercase();
        let ver = cap.get(2)?.as_str().to_string();

        let mut score = 0i32;

        let labelled = label == bin_stem || label == bin_lc || label == "version";
        if label == bin_stem || label == bin_lc {
            score += 200;
        }
        if label.contains(&bin_stem) && bin_stem.len() > 3 {
            score += 120;
        }
        if label == "version" {
            score += 100;
        }

        const DEPS: &[&str] = &[
            "openssl", "libcurl", "zlib", "brotli", "libidn2", "libpsl", "libssh", "nghttp2",
            "ngtcp2", "nghttp3", "libressl", "gnutls", "mit", "openldap", "kernel", "glibc",
            "runtime", "embedded",
        ];
        if DEPS.contains(&label.as_str()) {
            score -= 200;
        }

        if !labelled {
            score -= 50;
        }

        score -= cap.get(0).map_or(0, |m| m.start() as i32) / 256;

        if best.as_ref().map_or(true, |(bs, _)| score > *bs) {
            best = Some((score, ver));
        }
    }

    best.map(|(_, v)| v)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Walk the catalog and report what this host actually has.
///
/// Probes run concurrently so a slow tool cannot hold up the whole sweep.
pub async fn scan() -> Inventory {
    let mut handles = Vec::new();

    for c in candidates() {
        let c: Candidate = *c;
        handles.push(tokio::spawn(async move {
            let path = which(c.bin)?;
            let version = probe_version(&path, c.probe).await;
            Some(FoundTool {
                bin: c.bin.to_string(),
                about: c.about.to_string(),
                category: c.category,
                path,
                version,
            })
        }));
    }

    let mut found = Vec::new();
    let mut missing = Vec::new();

    for (i, h) in handles.into_iter().enumerate() {
        match h.await {
            Ok(Some(t)) => found.push(t),
            Ok(None) | Err(_) => {
                missing.push(candidate(i).map_or(String::new(), |c| c.bin.to_string()))
            }
        }
    }

    found.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.bin.cmp(&b.bin)));
    missing.sort();

    Inventory { found, missing }
}
