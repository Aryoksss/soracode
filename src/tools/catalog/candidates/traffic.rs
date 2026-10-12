//! Packet capture and traffic analysis tools.

use super::super::{Candidate, Category, Probe};

pub(super) const TRAFFIC: &[Candidate] = &[
    Candidate {
        bin: "tcpdump",
        about: "packet capture",
        category: Category::Traffic,
        probe: Probe::Version,
    },
    Candidate {
        bin: "tshark",
        about: "Wireshark CLI",
        category: Category::Traffic,
        probe: Probe::Version,
    },
    Candidate {
        bin: "mitmdump",
        about: "mitmproxy CLI",
        category: Category::Traffic,
        probe: Probe::Version,
    },
    Candidate {
        bin: "bettercap",
        about: "network attack and monitoring",
        category: Category::Traffic,
        probe: Probe::Version,
    },
    Candidate {
        bin: "responder",
        about: "LLMNR/NBT-NS/mDNS poisoner",
        category: Category::Traffic,
        probe: Probe::Skip,
    },
];
