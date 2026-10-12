//! Database and data querying tools.

use super::super::{Candidate, Category, Probe};

pub(super) const DATA: &[Candidate] = &[
    Candidate {
        bin: "sqlite3",
        about: "SQLite CLI",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "psql",
        about: "PostgreSQL client",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "mysql",
        about: "MySQL client",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "redis-cli",
        about: "Redis client",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "mongosh",
        about: "MongoDB shell",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "mongo",
        about: "legacy MongoDB shell",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "jq",
        about: "JSON processor",
        category: Category::Data,
        probe: Probe::Version,
    },
    Candidate {
        bin: "yq",
        about: "YAML/JSON/XML processor",
        category: Category::Data,
        probe: Probe::Version,
    },
];
