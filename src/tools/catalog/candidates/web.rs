//! Web application probing and fuzzing tools.

use super::super::{Candidate, Category, Probe};

pub(super) const WEB: &[Candidate] = &[
    Candidate {
        bin: "ffuf",
        about: "web fuzzer (paths, params, vhosts)",
        category: Category::Web,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "feroxbuster",
        about: "recursive content discovery",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "gobuster",
        about: "directory, DNS and vhost brute force",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "dirb",
        about: "web content scanner",
        category: Category::Web,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "wfuzz",
        about: "web fuzzer",
        category: Category::Web,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "httpx",
        about: "HTTP probing and tech detection",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "katana",
        about: "crawler and endpoint discovery",
        category: Category::Web,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "nuclei",
        about: "template-based vulnerability scanner",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "nikto",
        about: "web server scanner",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "wpscan",
        about: "WordPress vulnerability scanner",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "whatweb",
        about: "web technology fingerprinting",
        category: Category::Web,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "wafw00f",
        about: "WAF detection",
        category: Category::Web,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "sqlmap",
        about: "SQL injection detection and exploitation",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "dalfox",
        about: "XSS scanner",
        category: Category::Web,
        probe: Probe::Version,
    },
    Candidate {
        bin: "jwt_tool",
        about: "JWT testing and forgery",
        category: Category::Web,
        probe: Probe::Skip,
    },
];

pub(super) const BROWSERS: &[Candidate] = &[
    Candidate {
        bin: "firefox",
        about: "Firefox browser",
        category: Category::Browser,
        probe: Probe::Version,
    },
    Candidate {
        bin: "google-chrome",
        about: "Chrome browser",
        category: Category::Browser,
        probe: Probe::Version,
    },
    Candidate {
        bin: "chromium",
        about: "Chromium browser",
        category: Category::Browser,
        probe: Probe::Version,
    },
    Candidate {
        bin: "chromium-browser",
        about: "Chromium browser",
        category: Category::Browser,
        probe: Probe::Version,
    },
];
