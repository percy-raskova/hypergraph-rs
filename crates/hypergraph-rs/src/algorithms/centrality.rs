//! Hypergraph centralities (`xgi/algorithms/centrality.py`).
//!
//! Phase 3 ships the two non-tensor centralities: `katz_centrality` (power
//! series on the clique-motif matrix) and `clique_eigenvector_centrality`
//! (hand-rolled power iteration — no LAPACK; deviation #5). Tensor-based
//! centralities are deferred (Open Questions).

use indexmap::IndexMap;

use super::connected::is_connected;
use crate::linalg::clique_motif_matrix;
use crate::Hypergraph;

/// L1 norm of a slice.
fn l1(v: &[f64]) -> f64 {
    v.iter().map(|x| x.abs()).sum()
}

/// Katz centrality via the truncated power series
/// `mat = Σ αⁱ Aⁱ⁺¹`, then `c = mat · (1/n · 1)`, L1-normalized.
/// XGI parity: `katz_centrality(H, cutoff)` (`centrality.py:218-296`).
///
/// Deliberate deviation: XGI unconditionally does `c *= 1 / norm(c, 1)`
/// (`centrality.py:294`), which divides by zero (producing inf/nan) when
/// every node's raw score is zero. This function guards on `norm != 0.0`
/// and leaves an all-zero vector unnormalized in that case. This is strictly
/// saner than propagating inf/nan, and the branch is unreachable for any
/// hypergraph with at least one edge (the `m == 0` case is short-circuited
/// above and returns zeros directly; with `m >= 1` and `alpha > 0`, the
/// clique-motif matrix `A` has no negative entries and `mat` accumulates
/// strictly non-negative contributions from `A`, so `c = mat · (1/n · 1)`
/// cannot be identically zero).
pub fn katz_centrality<N, E, M>(h: &Hypergraph<N, E, M>, cutoff: usize) -> IndexMap<String, f64> {
    let n = h.num_nodes();
    let mut result: IndexMap<String, f64> = IndexMap::new();
    if n == 0 {
        return result;
    }
    let node_ids = h.node_ids();
    if h.num_edges() == 0 {
        for nid in node_ids {
            result.insert(nid, 0.0);
        }
        return result;
    }
    let (a, _rows) = clique_motif_matrix(h);
    let alpha = 1.0 / 2f64.powi(n as i32);
    let mut mat = a.clone();
    // Bounded loop: `cutoff` iterations.
    for _ in 1..cutoff {
        mat = mat.dot(&a) * alpha + &a;
    }
    let u = ndarray::Array1::<f64>::from_elem(n, 1.0 / n as f64);
    let mut c = mat.dot(&u);
    let norm = l1(c.as_slice().unwrap());
    if norm != 0.0 {
        c.mapv_inplace(|x| x / norm);
    }
    for (i, nid) in node_ids.into_iter().enumerate() {
        result.insert(nid, c[i]);
    }
    result
}

/// Clique-motif eigenvector centrality via power iteration.
/// XGI parity: `clique_eigenvector_centrality(H, tol)` (`centrality.py:26-67`);
/// deviation #5 (power iteration replaces scipy `eigsh`). Empty ⇒ empty map;
/// disconnected ⇒ all-NaN.
pub fn clique_eigenvector_centrality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    tol: f64,
    max_iter: usize,
) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    if h.num_nodes() == 0 {
        return result;
    }
    let node_ids = h.node_ids();
    if !is_connected(h) {
        for nid in node_ids {
            result.insert(nid, f64::NAN);
        }
        return result;
    }
    let (w, _rows) = clique_motif_matrix(h);
    let n = node_ids.len();
    let mut v = ndarray::Array1::<f64>::from_elem(n, 1.0 / n as f64);
    // Power iteration (bounded by max_iter): v ← W v / ||W v||₂.
    for _ in 0..max_iter {
        let mut next = w.dot(&v);
        let norm2 = next.dot(&next).sqrt();
        if norm2 == 0.0 {
            break;
        }
        next.mapv_inplace(|x| x / norm2);
        let delta = (&next - &v).mapv(|x| x * x).sum().sqrt();
        v = next;
        if delta < tol {
            break;
        }
    }
    // Sign-fix + L1-normalize (XGI: `sign(v[0]) * v / norm(v, 1)`).
    let sign = if v[0] < 0.0 { -1.0 } else { 1.0 };
    let norm1 = l1(v.as_slice().unwrap());
    for (i, nid) in node_ids.into_iter().enumerate() {
        result.insert(nid, sign * v[i] / norm1);
    }
    result
}
