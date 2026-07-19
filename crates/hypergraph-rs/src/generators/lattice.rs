//! Lattice hypergraph generators (XGI `generators/lattice.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use crate::Hypergraph;

/// A d-uniform ring-lattice hypergraph on `n` nodes: each node is in `k` edges,
/// consecutive edges overlap by `d - l`. XGI `ring_lattice(n, d, k, l)`.
///
/// `k` is `usize` here (negative k is unrepresentable); XGI's `k < 2` /
/// odd-`k` warnings are elided — the structural output is unchanged.
pub fn ring_lattice(n: usize, d: usize, k: usize, l: usize) -> Result<Hypergraph, GeneratorError> {
    if n == 0 {
        return Err(GeneratorError::InvalidParameter("n must be > 0".into()));
    }
    if d == 0 {
        return Err(GeneratorError::InvalidParameter("d must be >= 1".into()));
    }
    let mut h = Hypergraph::new();
    // Bounded: n * (k/2) edges, each with d members.
    for node in 0..n {
        for start in (node + 1)..=(node + k / 2) {
            let mut members: Vec<String> = Vec::with_capacity(d);
            members.push(node.to_string());
            for i in 0..(d - 1) {
                members.push(((start + l + i) % n).to_string());
            }
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    Ok(h)
}
