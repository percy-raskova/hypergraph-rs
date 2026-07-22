//! Directed edge statistics (XGI `xgi/stats/diedgestats.py` port).

use indexmap::IndexMap;

use super::Stat;
use crate::DiHypergraph;

/// Edge size: `|head ∪ tail|`. XGI `diedgestats.size`.
///
/// Deviation from the plan's assumed API: the landed `DiHypergraph` already
/// exposes `size(edge_id) -> usize` directly (see `core/dihypergraph.rs`), so
/// this wraps that instead of computing a `HashSet` union over
/// `tail(...)`/`head(...)` (which return `Option<Vec<String>>`, not the
/// plan's assumed `Vec<String>`).
pub fn di_edge_size<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        let sz = d.size(&eid);
        out.insert(eid, sz);
    }
    Stat::new(out)
}

/// Edge order: `|head ∪ tail| - 1`. XGI `diedgestats.order`.
pub fn di_edge_order<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        let ord = d.order(&eid);
        out.insert(eid, ord);
    }
    Stat::new(out)
}

/// Head size: `|head|`. XGI `diedgestats.head_size`.
pub fn di_head_size<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        let sz = d.head_size(&eid);
        out.insert(eid, sz);
    }
    Stat::new(out)
}

/// Tail size: `|tail|`. XGI `diedgestats.tail_size`.
pub fn di_tail_size<N, E, M>(d: &DiHypergraph<N, E, M>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        let sz = d.tail_size(&eid);
        out.insert(eid, sz);
    }
    Stat::new(out)
}
