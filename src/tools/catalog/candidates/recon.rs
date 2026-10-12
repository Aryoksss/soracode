//! Reconnaissance and scanning tools.

use super::super::{Candidate, Category, Probe};

pub(super) const RECON: &[Candidate] = &[
    Candidate {
        bin: "nmap",
        about: "port scanner and service/version detection",
        category: Category::Recon,
        probe: Probe::Version,
    },
    Candidate {
        bin: "masscan",
        about: "asynchronous port scanner, very fast",
        category: Category::Recon,
        probe: Probe::Version,
    },
    Candidate {
        bin: "naabu",
        about: "fast SYN/CONNECT port scanner",
        category: Category::Recon,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "dig",
        about: "DNS lookup",
        category: Category::Recon,
        probe: Probe::LowerV,
    },
    Candidate {
        bin: "host",
        about: "DNS lookup",
        category: Category::Recon,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "nslookup",
        about: "DNS lookup",
        category: Category::Recon,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "whois",
        about: "registration data lookup",
        category: Category::Recon,
        probe: Probe::Version,
    },
    Candidate {
        bin: "subfinder",
        about: "passive subdomain enumeration",
        category: Category::Recon,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "amass",
        about: "attack surface and subdomain mapping",
        category: Category::Recon,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "dnsx",
        about: "DNS query toolkit",
        category: Category::Recon,
        probe: Probe::CapV,
    },
    Candidate {
        bin: "shodan",
        about: "Shodan CLI",
        category: Category::Recon,
        probe: Probe::Version,
    },
    Candidate {
        bin: "theHarvester",
        about: "email, host and name harvesting",
        category: Category::Recon,
        probe: Probe::Skip,
    },
];
