//! Container, cloud, and orchestration tools.

use super::super::{Candidate, Category, Probe};

pub(super) const CLOUD: &[Candidate] = &[
    Candidate {
        bin: "docker",
        about: "container runtime",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "podman",
        about: "daemonless container runtime",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "kubectl",
        about: "Kubernetes control",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "helm",
        about: "Kubernetes package manager",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "aws",
        about: "AWS CLI",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "az",
        about: "Azure CLI",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "gcloud",
        about: "Google Cloud CLI",
        category: Category::Cloud,
        probe: Probe::Version,
    },
    Candidate {
        bin: "terraform",
        about: "infrastructure as code",
        category: Category::Cloud,
        probe: Probe::Version,
    },
];
