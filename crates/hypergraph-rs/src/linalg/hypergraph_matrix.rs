//! Incidence, adjacency, intersection-profile, degree, and clique-motif matrices.

use indexmap::IndexMap;
use ndarray::{Array1, Array2, Axis};
use sprs::{CsMat, TriMat};

use super::filtered_edge_ids;
use crate::Hypergraph;

/// Dense incidence matrix `I` (rows = nodes, cols = edges); `I[i,j] = 1`
/// iff node `i` is a member of edge `j`.
///
/// XGI parity: `xgi.incidence_matrix(H, order, sparse=False, index=True)`
/// (`hypergraph_matrix.py:64-135`). Returns a `(0, 0)` matrix with empty id
/// vectors when there are no nodes or no (filtered) edges.
pub fn incidence_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array2<f64>, Vec<String>, Vec<String>) {
    let node_ids = h.node_ids();
    let edge_ids = filtered_edge_ids(h, order);
    if node_ids.is_empty() || edge_ids.is_empty() {
        return (Array2::zeros((0, 0)), Vec::new(), Vec::new());
    }
    let node_index: IndexMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut m = Array2::<f64>::zeros((node_ids.len(), edge_ids.len()));
    for (j, e) in edge_ids.iter().enumerate() {
        for member in h.members(e).unwrap_or_default() {
            if let Some(&i) = node_index.get(member.as_str()) {
                m[[i, j]] = 1.0;
            }
        }
    }
    (m, node_ids, edge_ids)
}

/// Sparse (CSR) incidence matrix — same values and ordering as
/// [`incidence_matrix`], as a `sprs::CsMat<f64>`.
pub fn incidence_matrix_sparse<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (CsMat<f64>, Vec<String>, Vec<String>) {
    let node_ids = h.node_ids();
    let edge_ids = filtered_edge_ids(h, order);
    if node_ids.is_empty() || edge_ids.is_empty() {
        return (CsMat::zero((0, 0)), Vec::new(), Vec::new());
    }
    let node_index: IndexMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut tri = TriMat::<f64>::new((node_ids.len(), edge_ids.len()));
    for (j, e) in edge_ids.iter().enumerate() {
        for member in h.members(e).unwrap_or_default() {
            if let Some(&i) = node_index.get(member.as_str()) {
                tri.add_triplet(i, j, 1.0);
            }
        }
    }
    (tri.to_csr(), node_ids, edge_ids)
}

/// Adjacency matrix `A` (nodes × nodes). `A = I·Iᵀ` with the diagonal zeroed;
/// then thresholded by `s` overlapping edges. If `weighted`, the surviving
/// entries carry the shared-edge count; otherwise they are `1.0`.
///
/// XGI parity: `xgi.adjacency_matrix(H, order, s, weighted)`
/// (`hypergraph_matrix.py:138-204`).
pub fn adjacency_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    s: usize,
    weighted: bool,
) -> (Array2<f64>, Vec<String>) {
    let (inc, node_ids, _edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        // No (filtered) edges: XGI returns an N×N zero matrix over all nodes.
        let n = h.num_nodes();
        return (Array2::zeros((n, n)), h.node_ids());
    }
    let mut a = inc.dot(&inc.t());
    let n = a.nrows();
    for d in 0..n {
        a[[d, d]] = 0.0;
    }
    let s = s as f64;
    let out = a.mapv(|v| {
        if v >= s {
            if weighted {
                v
            } else {
                1.0
            }
        } else {
            0.0
        }
    });
    (out, node_ids)
}

/// Weighted clique-motif matrix — `adjacency_matrix(H, None, 1, true)`.
///
/// XGI parity: `xgi.clique_motif_matrix(H)` (`hypergraph_matrix.py:268-299`).
pub fn clique_motif_matrix<N, E, M>(h: &Hypergraph<N, E, M>) -> (Array2<f64>, Vec<String>) {
    adjacency_matrix(h, None, 1, true)
}

/// Intersection profile `P = Iᵀ·I` (edges × edges). `P[a,b]` is the number of
/// nodes shared by edges `a` and `b`; the diagonal is each edge's size.
///
/// XGI parity: `xgi.intersection_profile(H, order)`
/// (`hypergraph_matrix.py:207-233`). Row ids are edge ids.
pub fn intersection_profile<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array2<f64>, Vec<String>) {
    let (inc, _node_ids, edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        return (Array2::zeros((0, 0)), Vec::new());
    }
    let p = inc.t().dot(&inc);
    (p, edge_ids)
}

/// Degree vector `K` (one entry per node) = incidence row sums.
///
/// XGI parity: `xgi.degree_matrix(H, order)` (`hypergraph_matrix.py:236-265`).
/// When there are no (filtered) edges, returns `zeros(num_nodes)`.
pub fn degree_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array1<f64>, Vec<String>) {
    let (inc, node_ids, _edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        return (Array1::zeros(h.num_nodes()), h.node_ids());
    }
    let k = inc.sum_axis(Axis(1));
    (k, node_ids)
}
