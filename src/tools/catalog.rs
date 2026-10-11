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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Result;

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

/// The catalog of tools soracode looks for, covering the common
/// security-testing surface plus the runtimes this harness itself relies on.
pub const CANDIDATES: &[Candidate] = &[
    // Reconnaissance & scanning
    Candidate { bin: "nmap", about: "port scanner and service/version detection", category: Category::Recon, probe: Probe::Version },
    Candidate { bin: "masscan", about: "asynchronous port scanner, very fast", category: Category::Recon, probe: Probe::Version },
    Candidate { bin: "naabu", about: "fast SYN/CONNECT port scanner", category: Category::Recon, probe: Probe::CapV },
    Candidate { bin: "dig", about: "DNS lookup", category: Category::Recon, probe: Probe::LowerV },
    Candidate { bin: "host", about: "DNS lookup", category: Category::Recon, probe: Probe::CapV },
    Candidate { bin: "nslookup", about: "DNS lookup", category: Category::Recon, probe: Probe::Skip },
    Candidate { bin: "whois", about: "registration data lookup", category: Category::Recon, probe: Probe::Version },
    Candidate { bin: "subfinder", about: "passive subdomain enumeration", category: Category::Recon, probe: Probe::CapV },
    Candidate { bin: "amass", about: "attack surface and subdomain mapping", category: Category::Recon, probe: Probe::Skip },
    Candidate { bin: "dnsx", about: "DNS query toolkit", category: Category::Recon, probe: Probe::CapV },
    Candidate { bin: "shodan", about: "Shodan CLI", category: Category::Recon, probe: Probe::Version },
    Candidate { bin: "theHarvester", about: "email, host and name harvesting", category: Category::Recon, probe: Probe::Skip },

    // Web application
    Candidate { bin: "ffuf", about: "web fuzzer (paths, params, vhosts)", category: Category::Web, probe: Probe::CapV },
    Candidate { bin: "feroxbuster", about: "recursive content discovery", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "gobuster", about: "directory, DNS and vhost brute force", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "dirb", about: "web content scanner", category: Category::Web, probe: Probe::Skip },
    Candidate { bin: "wfuzz", about: "web fuzzer", category: Category::Web, probe: Probe::Skip },
    Candidate { bin: "httpx", about: "HTTP probing and tech detection", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "katana", about: "crawler and endpoint discovery", category: Category::Web, probe: Probe::CapV },
    Candidate { bin: "nuclei", about: "template-based vulnerability scanner", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "nikto", about: "web server scanner", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "wpscan", about: "WordPress vulnerability scanner", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "whatweb", about: "web technology fingerprinting", category: Category::Web, probe: Probe::Skip },
    Candidate { bin: "wafw00f", about: "WAF detection", category: Category::Web, probe: Probe::Skip },
    Candidate { bin: "sqlmap", about: "SQL injection detection and exploitation", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "dalfox", about: "XSS scanner", category: Category::Web, probe: Probe::Version },
    Candidate { bin: "jwt_tool", about: "JWT testing and forgery", category: Category::Web, probe: Probe::Skip },

    // Exploitation
    Candidate { bin: "msfconsole", about: "Metasploit framework console", category: Category::Exploit, probe: Probe::Skip },
    Candidate { bin: "msfvenom", about: "Metasploit payload generator", category: Category::Exploit, probe: Probe::Version },
    Candidate { bin: "searchsploit", about: "Exploit-DB local search", category: Category::Exploit, probe: Probe::Version },
    Candidate { bin: "impacket-smbserver", about: "SMB server from impacket", category: Category::Exploit, probe: Probe::Skip },
    Candidate { bin: "impacket-psexec", about: "remote execution via SMB", category: Category::Exploit, probe: Probe::Skip },
    Candidate { bin: "msfdb", about: "Metasploit database management", category: Category::Exploit, probe: Probe::Skip },

    // Credentials & cracking
    Candidate { bin: "hydra", about: "online password brute force", category: Category::Credential, probe: Probe::Version },
    Candidate { bin: "medusa", about: "parallel login brute force", category: Category::Credential, probe: Probe::Version },
    Candidate { bin: "john", about: "John the Ripper password cracker", category: Category::Credential, probe: Probe::Skip },
    Candidate { bin: "hashcat", about: "GPU password cracker", category: Category::Credential, probe: Probe::Version },
    Candidate { bin: "crunch", about: "wordlist generator", category: Category::Credential, probe: Probe::Skip },
    Candidate { bin: "cewl", about: "wordlist generation from a site", category: Category::Credential, probe: Probe::Skip },
    Candidate { bin: "sshpass", about: "non-interactive SSH password auth", category: Category::Credential, probe: Probe::Skip },

    // Directory services
    Candidate { bin: "enum4linux", about: "SMB/NetBIOS enumeration", category: Category::Directory, probe: Probe::Skip },
    Candidate { bin: "smbclient", about: "SMB share access", category: Category::Directory, probe: Probe::Version },
    Candidate { bin: "smbmap", about: "SMB share enumeration", category: Category::Directory, probe: Probe::Skip },
    Candidate { bin: "rpcclient", about: "MS-RPC enumeration", category: Category::Directory, probe: Probe::Skip },
    Candidate { bin: "ldapsearch", about: "LDAP queries", category: Category::Directory, probe: Probe::Skip },
    Candidate { bin: "bloodhound-python", about: "BloodHound AD data collection", category: Category::Directory, probe: Probe::Version },
    Candidate { bin: "nxc", about: "NetExec network service swiss army knife", category: Category::Directory, probe: Probe::Version },
    Candidate { bin: "crackmapexec", about: "legacy NetExec", category: Category::Directory, probe: Probe::Version },
    Candidate { bin: "GetADUsers.py", about: "AD user enumeration", category: Category::Directory, probe: Probe::Skip },

    // Traffic capture
    Candidate { bin: "tcpdump", about: "packet capture", category: Category::Traffic, probe: Probe::Version },
    Candidate { bin: "tshark", about: "Wireshark CLI", category: Category::Traffic, probe: Probe::Version },
    Candidate { bin: "mitmdump", about: "mitmproxy CLI", category: Category::Traffic, probe: Probe::Version },
    Candidate { bin: "bettercap", about: "network attack and monitoring", category: Category::Traffic, probe: Probe::Version },
    Candidate { bin: "responder", about: "LLMNR/NBT-NS/mDNS poisoner", category: Category::Traffic, probe: Probe::Skip },

    // Network & pivoting
    Candidate { bin: "curl", about: "HTTP client", category: Category::Network, probe: Probe::Version },
    Candidate { bin: "wget", about: "HTTP/FTP downloader", category: Category::Network, probe: Probe::Version },
    Candidate { bin: "nc", about: "netcat, raw TCP/UDP", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "ncat", about: "Nmap netcat", category: Category::Network, probe: Probe::Version },
    Candidate { bin: "socat", about: "bidirectional data relay", category: Category::Network, probe: Probe::LowerV },
    Candidate { bin: "openssl", about: "TLS, certificates and crypto", category: Category::Network, probe: Probe::Version },
    Candidate { bin: "ssh", about: "remote shell, also usable for tunnels", category: Category::Network, probe: Probe::CapV },
    Candidate { bin: "ssh-keygen", about: "SSH key generation", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "proxychains4", about: "route traffic through proxies", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "chisel", about: "TCP/UDP tunnel over HTTP", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "ligolo-ng", about: "reverse tunneling agent", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "tor", about: "anonymity network", category: Category::Network, probe: Probe::Version },
    Candidate { bin: "rdesktop", about: "RDP client", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "vncviewer", about: "VNC client", category: Category::Network, probe: Probe::Skip },
    Candidate { bin: "ping", about: "ICMP reachability", category: Category::Network, probe: Probe::CapV },
    Candidate { bin: "traceroute", about: "path discovery", category: Category::Network, probe: Probe::Skip },

    // Databases & data
    Candidate { bin: "sqlite3", about: "SQLite CLI", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "psql", about: "PostgreSQL client", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "mysql", about: "MySQL client", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "redis-cli", about: "Redis client", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "mongosh", about: "MongoDB shell", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "mongo", about: "legacy MongoDB shell", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "jq", about: "JSON processor", category: Category::Data, probe: Probe::Version },
    Candidate { bin: "yq", about: "YAML/JSON/XML processor", category: Category::Data, probe: Probe::Version },

    // Containers & cloud
    Candidate { bin: "docker", about: "container runtime", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "podman", about: "daemonless container runtime", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "kubectl", about: "Kubernetes control", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "helm", about: "Kubernetes package manager", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "aws", about: "AWS CLI", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "az", about: "Azure CLI", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "gcloud", about: "Google Cloud CLI", category: Category::Cloud, probe: Probe::Version },
    Candidate { bin: "terraform", about: "infrastructure as code", category: Category::Cloud, probe: Probe::Version },

    // Runtimes & compilers
    Candidate { bin: "python3", about: "Python 3 interpreter", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "pip3", about: "Python package manager", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "uv", about: "fast Python package manager", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "node", about: "Node.js runtime", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "npm", about: "Node package manager", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "bun", about: "Bun JavaScript runtime", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "go", about: "Go toolchain", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "rustc", about: "Rust compiler", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "cargo", about: "Rust package manager", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "gcc", about: "C compiler", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "clang", about: "Clang compiler", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "make", about: "build automation", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "java", about: "JVM", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "ruby", about: "Ruby interpreter", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "php", about: "PHP interpreter", category: Category::Runtime, probe: Probe::Version },
    Candidate { bin: "perl", about: "Perl interpreter", category: Category::Runtime, probe: Probe::Version },

    // Utilities
    Candidate { bin: "git", about: "version control", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "rg", about: "ripgrep fast search", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "grep", about: "text search", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "find", about: "file search", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "sed", about: "stream editor", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "awk", about: "text processing", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "base64", about: "base64 encode/decode", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "xxd", about: "hex dump", category: Category::Utility, probe: Probe::Skip },
    Candidate { bin: "strings", about: "extract printable strings from binaries", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "file", about: "file type identification", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "objdump", about: "binary disassembly", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "readelf", about: "ELF header inspection", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "gdb", about: "debugger", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "strace", about: "syscall tracer", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "ltrace", about: "library call tracer", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "zip", about: "archive tool", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "unzip", about: "archive extraction", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "tar", about: "archive tool", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "7z", about: "7-Zip archiver", category: Category::Utility, probe: Probe::Skip },
    Candidate { bin: "tmux", about: "terminal multiplexer", category: Category::Utility, probe: Probe::LowerV },
    Candidate { bin: "screen", about: "terminal multiplexer", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "systemctl", about: "service control", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "ps", about: "process listing", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "ss", about: "socket statistics", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "lsof", about: "open file and port listing", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "env", about: "environment inspection", category: Category::Utility, probe: Probe::Version },
    Candidate { bin: "timeout", about: "run a command with a time limit", category: Category::Utility, probe: Probe::Version },

    // Browsers
    Candidate { bin: "firefox", about: "Firefox browser", category: Category::Browser, probe: Probe::Version },
    Candidate { bin: "google-chrome", about: "Chrome browser", category: Category::Browser, probe: Probe::Version },
    Candidate { bin: "chromium", about: "Chromium browser", category: Category::Browser, probe: Probe::Version },
    Candidate { bin: "chromium-browser", about: "Chromium browser", category: Category::Browser, probe: Probe::Version },
];

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

fn is_executable(p: &Path) -> bool {
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
fn extract_version(text: &str, bin: &str) -> Option<String> {
    let clean = strip_ansi(text);
    let re = regex::Regex::new(r"(?i)([a-z][a-z0-9_+.-]*?)[\s_\-]*v?(\d+\.\d+(?:\.\d+)?(?:[a-z]\d+)?(?:[-.][0-9a-z]+)*)").ok()?;

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
            "openssl", "libcurl", "zlib", "brotli", "libidn2", "libpsl", "libssh",
            "nghttp2", "ngtcp2", "nghttp3", "libressl", "gnutls", "mit", "openldap",
            "kernel", "glibc", "runtime", "embedded",
        ];
        if DEPS.contains(&label.as_str()) {
            score -= 200;
        }

        if !labelled {
            score -= 50;
        }

        score -= cap.get(0).map_or(0, |m| m.start() as i32) / 256;

        if best.as_ref().is_none_or(|(bs, _)| score > *bs) {
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

    for c in CANDIDATES {
        let c = *c;
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
            Ok(None) => missing.push(CANDIDATES[i].bin.to_string()),
            Err(_) => missing.push(CANDIDATES[i].bin.to_string()),
        }
    }

    found.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.bin.cmp(&b.bin)));
    missing.sort();

    Inventory { found, missing }
}

impl Inventory {
    /// Render the inventory as prompt text.
    #[must_use]
    pub fn render(&self) -> String {
        if self.found.is_empty() && self.missing.is_empty() {
            return String::new();
        }

        let mut by_cat: BTreeMap<Category, Vec<&FoundTool>> = BTreeMap::new();
        for t in &self.found {
            by_cat.entry(t.category).or_default().push(t);
        }

        let mut s = String::new();
        s.push_str("\n## Tools installed on this host\n\n");
        s.push_str(
            "These were discovered by resolving `PATH` on the machine you are running on. \
             Treat this as the authoritative list of what exists here: use a tool if it is \
             listed, and do not assume a tool exists if it is not.\n\n",
        );

        for cat in Category::order() {
            let Some(tools) = by_cat.get(&cat) else {
                continue;
            };
            s.push_str(&format!("### {}\n", cat.heading()));
            for t in tools {
                match &t.version {
                    Some(v) => s.push_str(&format!("- `{}` ({}) — {}\n", t.bin, v, t.about)),
                    None => s.push_str(&format!("- `{}` — {}\n", t.bin, t.about)),
                }
            }
            s.push('\n');
        }

        if !self.missing.is_empty() {
            s.push_str("### Not installed\n\n");
            s.push_str(
                "Looked for and absent. Do not try to run these; find another route or \
                 install one if the task genuinely needs it.\n\n",
            );
            let names: Vec<String> = self.missing.iter().map(|m| format!("`{m}`")).collect();
            s.push_str(&names.join(", "));
            s.push_str("\n\n");
        }

        s.push_str(&format!(
            "Total: {} tools available, {} absent.\n",
            self.found.len(),
            self.missing.len()
        ));
        s
    }
}

/// A short operator-facing summary, used at startup and by the `tools` tool.
#[must_use]
pub fn summarize(inv: &Inventory) -> String {
    format!(
        "{} tools available on this host, {} absent",
        inv.found.len(),
        inv.missing.len()
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
