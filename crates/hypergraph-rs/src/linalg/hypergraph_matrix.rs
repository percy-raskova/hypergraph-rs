//! Incidence, adjacency, intersection-profile, degree, and clique-motif matrices.

use indexmap::IndexMap;
use ndarray::Array2;
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
