//! Lattice hypergraph generators (XGI `generators/lattice.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use crate::Hypergraph;

/// The edges of `ring_lattice(n, d, k, l, ..)` in id order: node i, then the d-1 nodes
/// (start + l + j) % n for j in 0..d-1, for start in i+1 ..= i + k/2. XGI's `_ring_lattice_edges`.
///
/// `n` must be positive (`ring_lattice` checks it); the result has `n * (k / 2)` edges of `d` entries.
#[must_use]
pub fn ring_lattice_edges(n: usize, d: usize, k: usize, l: usize) -> Vec<Vec<usize>> {
    let mut edges: Vec<Vec<usize>> = Vec::with_capacity(n * (k / 2));
    for node in 0..n {
        for start in (node + 1)..=(node + k / 2) {
            let mut members: Vec<usize> = Vec::with_capacity(d);
            members.push(node);
            for j in 0..d.saturating_sub(1) {
                members.push((start + l + j) % n);
            }
            edges.push(members);
        }
    }
    edges
}

/// A d-uniform ring-lattice hypergraph on `n` nodes: each node is in `k` edges,
/// consecutive edges overlap by `d - l`. XGI `ring_lattice(n, d, k, l, with_positions)`.
///
/// With `with_positions`, node `i` gets the attribute `"pos"` = `[cos(2 pi i / n), sin(2 pi i / n)]`
/// (a JSON list: attribute values are JSON, so XGI's tuple comes back as a list).
///
/// `k` is `usize` here (negative k is unrepresentable); XGI's `k < 2` /
/// odd-`k` warnings are elided; the structural output is unchanged.
///
/// # Errors
///
/// `GeneratorError::InvalidParameter` when `n == 0` or `d == 0`.
pub fn ring_lattice(
    n: usize,
    d: usize,
    k: usize,
    l: usize,
    with_positions: bool,
) -> Result<Hypergraph, GeneratorError> {
    if n == 0 {
        return Err(GeneratorError::InvalidParameter("n must be > 0".into()));
    }
    if d == 0 {
        return Err(GeneratorError::InvalidParameter("d must be >= 1".into()));
    }
    let mut h = Hypergraph::new();
    for members in ring_lattice_edges(n, d, k, l) {
        let members: Vec<String> = members.iter().map(ToString::to_string).collect();
        let _ = h.add_edge(members, None, Value::Null);
    }
    h.add_nodes_from((0..n).map(|i| {
        let attrs = if with_positions {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            serde_json::json!({ "pos": [angle.cos(), angle.sin()] })
        } else {
            Value::Null
        };
        (i.to_string(), attrs)
    }));
    Ok(h)
}
