//! The catalog of tools soracode looks for, grouped by category.
//!
//! Each category is a separate constant so the data stays reviewable. The flat
//! `candidates()` iterator and `candidate(i)` indexer present the whole set in a
//! stable, compile-time order.

mod cloud;
mod credential;
mod data;
mod directory;
mod exploit;
mod network;
mod recon;
mod runtime;
mod traffic;
mod utility;
mod web;

use super::Candidate;

/// Every category slice, in discovery order.
const GROUPS: &[&[Candidate]] = &[
    recon::RECON,
    web::WEB,
    web::BROWSERS,
    exploit::EXPLOIT,
    credential::CREDENTIAL,
    directory::DIRECTORY,
    traffic::TRAFFIC,
    network::NETWORK,
    data::DATA,
    cloud::CLOUD,
    runtime::RUNTIME,
    utility::UTILITY,
];

/// Iterate every candidate across all categories, in order.
pub fn candidates() -> impl Iterator<Item = &'static Candidate> {
    GROUPS.iter().flat_map(|g| g.iter())
}

/// The candidate at flat position `i`, matching the order of `candidates()`.
pub fn candidate(i: usize) -> Option<&'static Candidate> {
    let mut n = i;
    for g in GROUPS {
        if n < g.len() {
            return Some(&g[n]);
        }
        n -= g.len();
    }
    None
}
