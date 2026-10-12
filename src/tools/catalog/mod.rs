//! Host tool discovery.
//!
//! Soracode must know what is actually installed on the machine it runs on.
//! Nothing here is assumed from a hardcoded list of "usual suspects": every
//! entry is resolved through the runtime `PATH`, which is why tools in
//! `~/.cargo/bin`, `~/go/bin`, and snap directories are found without any
//! special handling.
//!
//! Version probing is deliberately conservative. `--version` is not a standard
//! and several tools misbehave on it: `ffuf` and `go` print a usage error to
//! stderr, and `msfconsole --version` starts the Metasploit database. Each
//! entry therefore declares its own probe argument, and entries known to be
//! expensive are marked `no_probe` and reported as present-only.

mod cache;
mod candidates;
mod inventory;
mod probe;

pub use self::candidates::{candidate, candidates};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use self::cache::{cache_path, inventory, load_cache, rescan};
pub use self::inventory::summarize;

/// What category a tool belongs to, used to group the prompt listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Network scanning and mapping.
    Recon,
    /// Web application probing and fuzzing.
    Web,
    /// Exploitation frameworks.
    Exploit,
    /// Credentials and hashing.
    Credential,
    /// Active Directory, SMB, and directory services.
    Directory,
    /// Packet capture and traffic analysis.
    Traffic,
    /// Tunnels, pivots, and network utilities.
    Network,
    /// Databases and data querying.
    Data,
    /// Containers, cloud, and orchestration.
    Cloud,
    /// Language runtimes and compilers.
    Runtime,
    /// General purpose utilities.
    Utility,
    /// Browsers and headless automation.
    Browser,
}

impl Category {
    /// Heading used when rendering this group into the prompt.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            Self::Recon => "Reconnaissance & scanning",
            Self::Web => "Web application",
            Self::Exploit => "Exploitation",
            Self::Credential => "Credentials & cracking",
            Self::Directory => "Directory services",
            Self::Traffic => "Traffic capture",
            Self::Network => "Network & pivoting",
            Self::Data => "Databases & data",
            Self::Cloud => "Containers & cloud",
            Self::Runtime => "Runtimes & compilers",
            Self::Utility => "Utilities",
            Self::Browser => "Browsers",
        }
    }

    fn order() -> [Self; 12] {
        [
            Self::Recon,
            Self::Web,
            Self::Exploit,
            Self::Credential,
            Self::Directory,
            Self::Traffic,
            Self::Network,
            Self::Data,
            Self::Cloud,
            Self::Runtime,
            Self::Utility,
            Self::Browser,
        ]
    }
}

/// How a candidate tool should be probed for a version string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Probe {
    /// Run `--version`.
    Version,
    /// Run `-version`.
    DashVersion,
    /// Run `-V`.
    CapV,
    /// Run `-v`.
    LowerV,
    /// Run `version`.
    Subcommand,
    /// Do not probe; the tool is slow or has side effects.
    Skip,
}

impl Probe {
    fn arg(self) -> Option<&'static str> {
        match self {
            Self::Version => Some("--version"),
            Self::DashVersion => Some("-version"),
            Self::CapV => Some("-V"),
            Self::LowerV => Some("-v"),
            Self::Subcommand => Some("version"),
            Self::Skip => None,
        }
    }
}

/// A tool soracode knows how to look for.
#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    /// Executable name.
    pub bin: &'static str,
    /// Human description shown to the model.
    pub about: &'static str,
    /// Grouping for the prompt listing.
    pub category: Category,
    /// How to ask it for a version.
    pub probe: Probe,
}

/// Result of looking for one candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundTool {
    /// Executable name.
    pub bin: String,
    /// Human description.
    pub about: String,
    /// Grouping.
    pub category: Category,
    /// Absolute path that was resolved through `PATH`.
    pub path: PathBuf,
    /// Version string, when it could be read safely.
    pub version: Option<String>,
}

/// The complete inventory for this host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Tools that were found, grouped for rendering.
    pub found: Vec<FoundTool>,
    /// Candidates that were looked for and not found.
    pub missing: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::probe::{extract_version, which};
    use super::Probe;

    #[test]
    fn extracts_version_from_nmap_banner() {
        assert_eq!(
            extract_version("Nmap version 7.92 ( https://nmap.org )", "nmap"),
            Some("7.92".to_string())
        );
    }

    #[test]
    fn strips_ansi_before_matching() {
        let raw = "[\u{1b}[34mINF\u{1b}[0m] Nuclei Engine Version: v3.11.0";
        assert_eq!(extract_version(raw, "nuclei"), Some("3.11.0".to_string()));
    }

    #[test]
    fn tolerates_usage_error_on_stderr() {
        let err = "flag provided but not defined: -version";
        let out = "Fuzz Faster U Fool - v2.1.0-dev";
        assert_eq!(extract_version(err, "ffuf"), None);
        assert_eq!(extract_version(out, "ffuf"), Some("2.1.0-dev".to_string()));
    }

    #[test]
    fn prefers_own_version_over_dependency() {
        let banner = "OpenSSH_10.2p1, OpenSSL 3.5.9 29 Sep 2026";
        assert_eq!(extract_version(banner, "ssh"), Some("10.2p1".to_string()));
    }

    #[test]
    fn labelled_version_beats_earlier_unlabelled_platform() {
        let banner = "Nmap version 7.92 ( https://nmap.org )\nPlatform: x86_64-redhat-linux-gnu\ncompiled with: liblua-5.3.5";
        assert_eq!(extract_version(banner, "nmap"), Some("7.92".to_string()));
    }

    #[test]
    fn ignores_libcurl_when_curl_is_named() {
        let banner = "curl 8.18.0 (x86_64-redhat-linux-gnu) libcurl/8.18.0 OpenSSL/3.5.9";
        assert_eq!(extract_version(banner, "curl"), Some("8.18.0".to_string()));
    }

    #[test]
    fn finds_a_tool_that_must_exist() {
        assert!(which("sh").is_some());
    }

    #[test]
    fn missing_tool_resolves_to_none() {
        assert!(which("soracode-definitely-not-installed").is_none());
    }

    #[test]
    fn skip_probe_is_never_run() {
        assert_eq!(Probe::Skip.arg(), None);
    }
}
