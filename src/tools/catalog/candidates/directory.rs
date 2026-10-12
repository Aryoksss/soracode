//! Active Directory, SMB, and directory service tools.

use super::super::{Candidate, Category, Probe};

pub(super) const DIRECTORY: &[Candidate] = &[
    Candidate {
        bin: "enum4linux",
        about: "SMB/NetBIOS enumeration",
        category: Category::Directory,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "smbclient",
        about: "SMB share access",
        category: Category::Directory,
        probe: Probe::Version,
    },
    Candidate {
        bin: "smbmap",
        about: "SMB share enumeration",
        category: Category::Directory,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "rpcclient",
        about: "MS-RPC enumeration",
        category: Category::Directory,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "ldapsearch",
        about: "LDAP queries",
        category: Category::Directory,
        probe: Probe::Skip,
    },
    Candidate {
        bin: "bloodhound-python",
        about: "BloodHound AD data collection",
        category: Category::Directory,
        probe: Probe::Version,
    },
    Candidate {
        bin: "nxc",
        about: "NetExec network service swiss army knife",
        category: Category::Directory,
        probe: Probe::Version,
    },
    Candidate {
        bin: "crackmapexec",
        about: "legacy NetExec",
        category: Category::Directory,
        probe: Probe::Version,
    },
    Candidate {
        bin: "GetADUsers.py",
        about: "AD user enumeration",
        category: Category::Directory,
        probe: Probe::Skip,
    },
];
