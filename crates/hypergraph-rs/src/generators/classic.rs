//! Classic hypergraph generators (XGI `xgi/generators/classic.py` port).

use serde_json::Value;
use thiserror::Error;

use crate::Hypergraph;

/// Error raised by a generator given invalid parameters.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum GeneratorError {
    /// A parameter was outside its valid range.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}

/// The empty hypergraph (0 nodes, 0 edges). XGI `empty_hypergraph()`.
pub fn empty_hypergraph() -> Hypergraph {
    Hypergraph::new()
}

/// A hypergraph with `n` nodes ("0".."n-1") and no edges. XGI `trivial_hypergraph(n)`.
pub fn trivial_hypergraph(n: usize) -> Hypergraph {
    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    h
}

/// Enumerate every strictly-increasing `k`-combination of `0..n` (lexicographic).
/// Bounded: yields exactly `comb(n, k)` combinations.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    if k == 0 || k > n {
        return result;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        result.push(idx.iter().map(|&i| i).collect());
        // advance to next combination
        let mut i = k;
        // Bounded: at most k steps.
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return result;
            }
        }
        if k == 0 {
            return result;
        }
    }
}

/// A complete hypergraph on `n` nodes. Exactly one of `order` / `max_order` must
/// be `Some`. XGI `complete_hypergraph`.
pub fn complete_hypergraph(
    n: usize,
    order: Option<usize>,
    max_order: Option<usize>,
    include_singletons: bool,
) -> Result<Hypergraph, GeneratorError> {
    match (order, max_order) {
        (None, None) => {
            return Err(GeneratorError::InvalidParameter(
                "at least one of order and max_order must be specified".into(),
            ))
        }
        (Some(_), Some(_)) => {
            return Err(GeneratorError::InvalidParameter(
                "order and max_order cannot both be specified".into(),
            ))
        }
        _ => {}
    }

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    let mut edges: Vec<Vec<usize>> = Vec::new();
    if let Some(order) = order {
        edges.extend(combinations(n, order + 1));
    } else if let Some(max_order) = max_order {
        let start = if include_singletons { 1 } else { 2 };
        // Bounded: sizes start..=max_order+1.
        for r in start..=(max_order + 1) {
            edges.extend(combinations(n, r));
        }
    }

    for e in edges {
        let members: Vec<String> = e.iter().map(|i| i.to_string()).collect();
        // Non-empty by construction; auto-id.
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}

/// The complement of `h`: same nodes, every possible edge (size 1..=maxsize)
/// EXCEPT those present in `h`. XGI `complement(H)`.
pub fn complement(h: &Hypergraph) -> Hypergraph {
    let nodes = h.node_ids();
    let n = nodes.len();
    let mut hc = Hypergraph::new();
    if n == 0 {
        return hc;
    }
    hc.add_nodes_from(nodes.iter().map(|nid| (nid.clone(), Value::Null)));

    // node id -> dense index 0..n
    let index: std::collections::HashMap<&str, usize> =
        nodes.iter().enumerate().map(|(i, nid)| (nid.as_str(), i)).collect();

    // existing edges as sorted index-tuples
    let mut existing: std::collections::HashSet<Vec<usize>> = std::collections::HashSet::new();
    let mut max_size = 0usize;
    for e in h.edge_ids() {
        let mut idxs: Vec<usize> = h.members(&e).unwrap().iter().map(|m| index[m.as_str()]).collect();
        idxs.sort_unstable();
        max_size = max_size.max(idxs.len());
        existing.insert(idxs);
    }
    if max_size == 0 {
        max_size = 1;
    }

    // every possible edge of size 1..=max_size not in `existing`
    // Bounded: sizes 1..=max_size, each a finite combination set.
    for size in 1..=max_size {
        for combo in combinations(n, size) {
            if !existing.contains(&combo) {
                let members: Vec<String> = combo.iter().map(|&i| nodes[i].clone()).collect();
                let _ = hc.add_edge(members, None, Value::Null);
            }
        }
    }
    hc
}
