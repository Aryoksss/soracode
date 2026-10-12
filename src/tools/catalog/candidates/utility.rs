//! General purpose utilities.

use super::super::{Candidate, Category, Probe};

pub(super) const UTILITY: &[Candidate] = &[
    Candidate {
        bin: "git",
        about: "version control",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "rg",
        about: "ripgrep fast search",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "grep",
        about: "text search",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "find",
        about: "file search",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "sed",
        about: "stream editor",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "awk",
        about: "text processing",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "base64",
        about: "base64 encode/decode",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "xxd",
        about: "hex dump",
        category: Category::Utility,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "strings",
        about: "extract printable strings from binaries",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "file",
        about: "file type identification",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "objdump",
        about: "binary disassembly",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "readelf",
        about: "ELF header inspection",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "gdb",
        about: "debugger",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "strace",
        about: "syscall tracer",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "ltrace",
        about: "library call tracer",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "zip",
        about: "archive tool",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "unzip",
        about: "archive extraction",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "tar",
        about: "archive tool",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "7z",
        about: "7-Zip archiver",
        category: Category::Utility,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "tmux",
        about: "terminal multiplexer",
        category: Category::Utility,
        probe: Probe::LowerV,
    },
    Candidate {
        bin: "screen",
        about: "terminal multiplexer",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "systemctl",
        about: "service control",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "ps",
        about: "process listing",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "ss",
        about: "socket statistics",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "lsof",
        about: "open file and port listing",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "env",
        about: "environment inspection",
        category: Category::Utility,
        probe: Probe::Version,
    },
    Candidate {
        bin: "timeout",
        about: "run a command with a time limit",
        category: Category::Utility,
        probe: Probe::Version,
    },
];
