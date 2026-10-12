//! Language runtimes and compilers.

use super::super::{Candidate, Category, Probe};

pub(super) const RUNTIME: &[Candidate] = &[
    Candidate {
        bin: "python3",
        about: "Python 3 interpreter",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "pip3",
        about: "Python package manager",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "uv",
        about: "fast Python package manager",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "node",
        about: "Node.js runtime",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "npm",
        about: "Node package manager",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "bun",
        about: "Bun JavaScript runtime",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "go",
        about: "Go toolchain",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "rustc",
        about: "Rust compiler",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "cargo",
        about: "Rust package manager",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "gcc",
        about: "C compiler",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "clang",
        about: "Clang compiler",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "make",
        about: "build automation",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "java",
        about: "JVM",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "ruby",
        about: "Ruby interpreter",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "php",
        about: "PHP interpreter",
        category: Category::Runtime,
        probe: Probe::Version,
    },
    Candidate {
        bin: "perl",
        about: "Perl interpreter",
        category: Category::Runtime,
        probe: Probe::Version,
    },
];
