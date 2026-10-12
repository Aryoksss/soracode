//! Tunnels, pivots, and network utilities.

use super::super::{Candidate, Category, Probe};

pub(super) const NETWORK: &[Candidate] = &[
    Candidate {
        bin: "curl",
        about: "HTTP client",
        category: Category::Network,
        probe: Probe::Version,
    },
    Candidate {
        bin: "wget",
        about: "HTTP/FTP downloader",
        category: Category::Network,
        probe: Probe::Version,
    },
    Candidate {
        bin: "nc",
        about: "netcat, raw TCP/UDP",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "ncat",
        about: "Nmap netcat",
        category: Category::Network,
        probe: Probe::Version,
    },
    Candidate {
        bin: "socat",
        about: "bidirectional data relay",
        category: Category::Network,
        probe: Probe::LowerV,
    },
    Candidate {
        bin: "openssl",
        about: "TLS, certificates and crypto",
        category: Category::Network,
        probe: Probe::Version,
    },
    Candidate {
        bin: "ssh",
        about: "remote shell, also usable for tunnels",
        category: Category::Network,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "ssh-keygen",
        about: "SSH key generation",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "proxychains4",
        about: "route traffic through proxies",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "chisel",
        about: "TCP/UDP tunnel over HTTP",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "ligolo-ng",
        about: "reverse tunneling agent",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "tor",
        about: "anonymity network",
        category: Category::Network,
        probe: Probe::Version,
    },
    Candidate {
        bin: "rdesktop",
        about: "RDP client",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "vncviewer",
        about: "VNC client",
        category: Category::Network,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "ping",
        about: "ICMP reachability",
        category: Category::Network,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "traceroute",
        about: "path discovery",
        category: Category::Network,
        probe: Probe::Skip,
    },
];
