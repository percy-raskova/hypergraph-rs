//! Structural properties of a hypergraph (`xgi/algorithms/properties.py`).

use std::collections::BTreeSet;

use crate::Hypergraph;

/// Number of edges of order `d` (size `d+1`); all edges if `d` is `None`.
/// XGI parity: `num_edges_order` (`properties.py:92-117`).
pub fn num_edges_order<N, E, M>(h: &Hypergraph<N, E, M>, d: Option<usize>) -> usize {
    match d {
        None => h.num_edges(),
        Some(order) => h
            .edge_ids()
            .iter()
            .filter(|e| h.edge_size(e) == Some(order + 1))
            .count(),
    }
}

/// Maximum edge order (max size − 1). `None` for an empty hypergraph with no
/// nodes; `Some(0)` when there are nodes but no edges. XGI parity:
/// `max_edge_order` (`properties.py:120-141`), with one documented deviation
/// (see below).
pub fn max_edge_order<N, E, M>(h: &Hypergraph<N, E, M>) -> Option<usize> {
    let sizes: Vec<usize> = h.edge_ids().iter().filter_map(|e| h.edge_size(e)).collect();
    if sizes.is_empty() {
        // No edges at all: XGI's `H._edge` is falsy, so `d_max = 0 if
        // H._node else None`.
        return if h.num_nodes() > 0 { Some(0) } else { None };
    }
    // D1-ripple: size-0 edges (constructible after D1) carry no order and are
    // excluded from the order-stat, matching XGI's order = size - 1 semantics
    // (a size-0 edge would otherwise underflow this usize subtraction).
    //
    // REV-2: if that filter leaves nothing, at least one edge exists, but
    // every edge is size-0 (the largest edge is empty). XGI computes
    // `max(len(edge) for edge in H._edge.values()) - 1 == -1` here
    // (oracle-verified 2026-07-22: xgi 0.10.2, H.add_edge([]) ->
    // xgi.max_edge_order(H) == -1, and this holds whether or not other nodes
    // exist). usize cannot represent -1; None is the closest sane Rust
    // rendering (deviation, oracle-verified 2026-07-22). This is distinct
    // from the "no edges at all" case above, which XGI (and we) render as
    // Some(0) when nodes are present.
    sizes.into_iter().filter(|&s| s > 0).max().map(|s| s - 1)
}

/// Whether `d` is a possible order (0 ≤ d ≤ max order).
/// XGI parity: `is_possible_order` (`properties.py:144-161`).
pub fn is_possible_order<N, E, M>(h: &Hypergraph<N, E, M>, d: usize) -> bool {
    matches!(max_edge_order(h), Some(dmax) if d <= dmax)
}

/// Unique edge sizes in ascending order. XGI parity: `unique_edge_sizes`
/// (`properties.py:321-334`).
pub fn unique_edge_sizes<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<usize> {
    let set: BTreeSet<usize> = h.edge_ids().iter().filter_map(|e| h.edge_size(e)).collect();
    set.into_iter().collect()
}

/// The order of uniformity if all non-singleton edges share one size, else
/// `None`. XGI parity: `is_uniform` (`properties.py:164-197`) — returns
/// `d = size − 1` on success, `False` otherwise (mapped to `None`), with one
/// documented deviation (see below).
pub fn is_uniform<N, E, M>(h: &Hypergraph<N, E, M>) -> Option<usize> {
    let mut sizes: BTreeSet<usize> = unique_edge_sizes(h).into_iter().collect();
    sizes.remove(&1); // discard singletons
    if sizes.len() != 1 {
        return None;
    }
    let s = *sizes.iter().next().expect("len checked == 1 above");
    if s == 0 {
        // REV-1: the only non-singleton edge size is 0 (constructible after
        // D1). XGI computes `int(edge_sizes.pop() - 1) == -1` here — a
        // truthy-but-negative "order" (oracle-verified 2026-07-22: xgi
        // 0.10.2, H.add_edge([]) -> xgi.is_uniform(H) == -1). usize cannot
        // represent -1; None ("not uniform") is the closest sane Rust
        // rendering (deviation, oracle-verified 2026-07-22) — this is also
        // what previously underflowed `s - 1` and panicked.
        return None;
    }
    Some(s - 1)
}

/// Occurrences of each degree value, index = degree, from 0 to max degree.
/// XGI parity: `degree_counts` (`properties.py:244-282`).
pub fn degree_counts<N, E, M>(h: &Hypergraph<N, E, M>, order: Option<usize>) -> Vec<usize> {
    let degs: Vec<usize> = h
        .node_ids()
        .iter()
        .filter_map(|n| h.node_degree(n, order))
        .collect();
    let max_d = degs.iter().copied().max().unwrap_or(0);
    let mut counts = vec![0usize; max_d + 1];
    for d in degs {
        counts[d] += 1;
    }
    counts
}

/// Observed degrees (ascending) and their counts.
/// XGI parity: `degree_histogram` (`properties.py:285-318`).
pub fn degree_histogram<N, E, M>(h: &Hypergraph<N, E, M>) -> (Vec<usize>, Vec<usize>) {
    let mut counts: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for n in h.node_ids() {
        let d = h.node_degree(&n, None).unwrap_or(0);
        *counts.entry(d).or_insert(0) += 1;
    }
    let degrees: Vec<usize> = counts.keys().copied().collect();
    let heights: Vec<usize> = counts.values().copied().collect();
    (degrees, heights)
}

/// The edge neighbourhood of a node: the members of each edge containing it,
/// excluding the node itself (or including it if `include_self`).
/// XGI parity: `edge_neighborhood` (`properties.py:200-241`).
pub fn edge_neighborhood<N, E, M>(
    h: &Hypergraph<N, E, M>,
    n: &str,
    include_self: bool,
) -> Vec<Vec<String>> {
    let memberships = match h.memberships(n) {
        Some(m) => m,
        None => return Vec::new(),
    };
    memberships
        .iter()
        .map(|e| {
            let members = h.members(e).unwrap_or_default();
            if include_self {
                members
            } else {
                members.into_iter().filter(|m| m != n).collect()
            }
        })
        .collect()
}

/// Binomial coefficient `n choose k` (exact, u128 to resist overflow at scale).
fn comb(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num *= (n - i) as u128;
        den *= (i + 1) as u128;
    }
    (num / den) as f64
}

/// Hypergraph density = existing edges / possible edges.
/// XGI parity: `density` (`properties.py:337-425`). `order`/`max_order` are
/// mutually exclusive (order wins); `ignore_singletons` drops order-0 edges.
pub fn density<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> f64 {
    let n = h.num_nodes();
    assert!(n >= 1, "density is not defined for an empty hypergraph");
    if h.num_edges() < 1 {
        return 0.0;
    }
    let count_order = |d: usize| num_edges_order(h, Some(d));
    let count_leq = |dmax: usize| (0..=dmax).map(count_order).sum::<usize>();

    let (numer, denom): (f64, f64) = match (order, max_order) {
        (None, None) => {
            let mut numer = h.num_edges() as f64;
            let mut denom = 2f64.powi(n as i32) - 1.0;
            if ignore_singletons {
                numer -= count_order(0) as f64;
                denom -= n as f64;
            }
            (numer, denom)
        }
        (None, Some(mo)) => {
            assert!(mo < n, "max_order must be smaller than the number of nodes");
            let mut numer = count_leq(mo) as f64;
            let mut denom: f64 = (0..=mo).map(|o| comb(n, o + 1)).sum();
            if ignore_singletons {
                numer -= count_order(0) as f64;
                denom -= n as f64;
            }
            (numer, denom)
        }
        (Some(o), _) => {
            assert!(o < n, "order must be smaller than the number of nodes");
            if ignore_singletons && o == 0 {
                return 0.0;
            }
            (count_order(o) as f64, comb(n, o + 1))
        }
    };
    if denom == 0.0 {
        0.0
    } else {
        numer / denom
    }
}

/// Density of the incidence matrix = nnz / (n · num_edges_considered).
/// XGI parity: `incidence_density` (`properties.py:428-494`).
pub fn incidence_density<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> f64 {
    let n = h.num_nodes();
    assert!(
        n >= 1,
        "incidence density is not defined for an empty hypergraph"
    );
    if h.num_edges() < 1 {
        return 0.0;
    }
    if let Some(mo) = max_order {
        if order.is_none() {
            assert!(mo < n, "max_order must be smaller than the number of nodes");
        }
    }
    if let Some(o) = order {
        assert!(o < n, "order must be smaller than the number of nodes");
    }
    let keep: Vec<String> = h
        .edge_ids()
        .into_iter()
        .filter(|e| {
            let size = h.edge_size(e).unwrap_or(0);
            let ord = size.saturating_sub(1);
            match (order, max_order) {
                (Some(o), _) => {
                    if ignore_singletons && o == 0 {
                        false
                    } else {
                        ord == o
                    }
                }
                (None, Some(mo)) => ord <= mo && (!ignore_singletons || ord > 0),
                (None, None) => !ignore_singletons || ord > 0,
            }
        })
        .collect();
    let denom = (n * keep.len()) as f64;
    let numer: f64 = keep.iter().filter_map(|e| h.edge_size(e)).sum::<usize>() as f64;
    if denom == 0.0 {
        0.0
    } else {
        numer / denom
    }
}
