//! Directed node statistics (XGI `xgi/stats/dinodestats.py` port).

use indexmap::IndexMap;

use super::Stat;
use crate::DiHypergraph;

/// In-degree: number of edges where the node is a head (receiver). XGI
/// `dinodestats.in_degree`.
///
/// Deviation from the plan's assumed API: the landed `DiHypergraph` already
/// exposes `in_degree(node_id) -> usize` directly (see
/// `core/dihypergraph.rs`), so this wraps that instead of an
/// `in_edges(...).len()` call (no such method exists on the landed type).
pub fn di_in_degree<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        let deg = d.in_degree(&nid);
        out.insert(nid, deg);
    }
    Stat::new(out)
}

/// Out-degree: number of edges where the node is a tail (sender). XGI
/// `dinodestats.out_degree`.
pub fn di_out_degree<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        let deg = d.out_degree(&nid);
        out.insert(nid, deg);
    }
    Stat::new(out)
}

/// Degree: membership in either head or tail (deduplicated). XGI
/// `dinodestats.degree`.
pub fn di_degree<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        let deg = d.degree(&nid);
        out.insert(nid, deg);
    }
    Stat::new(out)
}
