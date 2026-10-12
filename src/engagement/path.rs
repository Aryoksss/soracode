//! Attack-path search over the relationship graph.
//!
//! The graph is the set of `eng_edges` the operator and the parsers recorded:
//! who owns whom, who can reach what, which credential unlocked which host.
//! Paths are found with a breadth-first search, so each result is a shortest
//! hop chain and the output stays deterministic.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::Engagement;
use crate::error::Result;

/// One candidate path through the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// Nodes in order, from the start to the goal.
    pub nodes: Vec<String>,
    /// Edge kinds traversed, one fewer than `nodes`.
    pub hops: Vec<String>,
}

impl Engagement {
    /// Find shortest attack paths, optionally anchored to a start and a goal.
    ///
    /// With a `goal`, every start node that can reach it is reported. With only
    /// a `start`, every reachable node is reported. With neither, all roots
    /// (nodes with no incoming edge) are used. Results are ordered by hop count
    /// then by node name so repeated calls agree.
    ///
    /// # Errors
    /// Propagates database errors.
    pub fn attack_paths(&self, start: Option<&str>, goal: Option<&str>) -> Result<Vec<Path>> {
        let edges = self.edges()?;
        let mut adj: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
        let mut all: BTreeSet<String> = BTreeSet::new();
        let mut has_incoming: BTreeSet<String> = BTreeSet::new();
        for e in &edges {
            adj.entry(e.from.clone())
                .or_default()
                .insert((e.to.clone(), e.kind.clone()));
            has_incoming.insert(e.to.clone());
            all.insert(e.from.clone());
            all.insert(e.to.clone());
        }

        let starts: Vec<String> = match start {
            Some(s) => vec![s.to_string()],
            None => {
                let roots: Vec<String> = all.difference(&has_incoming).cloned().collect();
                if roots.is_empty() {
                    all.iter().cloned().collect()
                } else {
                    roots
                }
            }
        };

        let mut out = Vec::new();
        for s in starts {
            for path in bfs(&adj, &s) {
                let ends_ok = match goal {
                    None => true,
                    Some(g) => path.nodes.last().is_some_and(|n| n == g),
                };
                if ends_ok {
                    out.push(path);
                }
            }
        }
        out.sort_by(|a, b| {
            a.nodes
                .len()
                .cmp(&b.nodes.len())
                .then_with(|| a.nodes.cmp(&b.nodes))
        });
        Ok(out)
    }
}

/// Breadth-first shortest paths from `start` to every reachable node.
fn bfs(adj: &BTreeMap<String, BTreeSet<(String, String)>>, start: &str) -> Vec<Path> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(start.to_string());
    let mut queue: VecDeque<Path> = VecDeque::new();
    queue.push_back(Path {
        nodes: vec![start.to_string()],
        hops: Vec::new(),
    });
    let mut out = Vec::new();
    while let Some(path) = queue.pop_front() {
        let last = path.nodes.last().cloned().unwrap_or_default();
        if let Some(neighbors) = adj.get(&last) {
            for (to, kind) in neighbors {
                if !seen.insert(to.clone()) {
                    continue;
                }
                let mut next = path.clone();
                next.nodes.push(to.clone());
                next.hops.push(kind.clone());
                out.push(next.clone());
                queue.push_back(next);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
