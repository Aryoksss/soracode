//! Credential and password-cracking tools.

use super::super::{Candidate, Category, Probe};

pub(super) const CREDENTIAL: &[Candidate] = &[
    Candidate {
        bin: "hydra",
        about: "online password brute force",
        category: Category::Credential,
        probe: Probe::Version,
    },
    Candidate {
        bin: "medusa",
        about: "parallel login brute force",
        category: Category::Credential,
        probe: Probe::Version,
    },
    Candidate {
        bin: "john",
        about: "John the Ripper password cracker",
        category: Category::Credential,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "hashcat",
        about: "GPU password cracker",
        category: Category::Credential,
        probe: Probe::Version,
    },
    Candidate {
        bin: "crunch",
        about: "wordlist generator",
        category: Category::Credential,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "cewl",
        about: "wordlist generation from a site",
        category: Category::Credential,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "sshpass",
        about: "non-interactive SSH password auth",
        category: Category::Credential,
        probe: Probe::Skip,
    },
];
