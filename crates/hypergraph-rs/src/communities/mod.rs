//! Spectral community detection ported from `xgi/communities/spectral.py`.
//!
//! `spectral_clustering` builds the normalized hypergraph Laplacian (formula from the
//! Phase-3 `linalg/laplacian_matrix.py`, inlined here for self-containment), extracts the
//! `k` smallest-eigenvalue eigenvectors via hand-rolled power iteration + Hotelling
//! deflation (the dependency probe found no LAPACK-free ndarray eigensolver), and runs
//! Forgy k-means. Exact XGI parity is impossible (RNG divergence, spec §9.4); assertions
//! are structural, and results are deterministic under a fixed `seed`.

use ndarray::{Array2, Array1};
use thiserror::Error;

use crate::Hypergraph;

/// Errors raised by spectral clustering.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SpectralError {
    /// More clusters were requested than there are nodes.
    #[error("cannot request {k} clusters from a hypergraph with {n} nodes")]
    TooManyClusters { k: usize, n: usize },
    /// The hypergraph has an isolated node (the normalized Laplacian divides by degree).
    #[error("every node must belong to an edge (isolated node breaks the normalized Laplacian)")]
    IsolatedNode,
}

/// The dense normalized hypergraph Laplacian and its row→node-id map.
///
/// XGI parity: `normalized_hypergraph_laplacian` (`linalg/laplacian_matrix.py:187-254`),
/// unweighted. `L = I − Dv^{-1/2} H De^{-1} Hᵀ Dv^{-1/2}`, with `Dv` = node degrees,
/// `De` = edge sizes. Errors on any isolated node.
pub fn normalized_laplacian(
    h: &Hypergraph,
) -> Result<(Array2<f64>, Vec<String>), SpectralError> {
    let node_ids = h.node_ids();
    let edge_ids = h.edge_ids();
    let n = node_ids.len();
    let m = edge_ids.len();

    // Node index map.
    let mut node_pos = std::collections::HashMap::new();
    for (i, id) in node_ids.iter().enumerate() {
        node_pos.insert(id.clone(), i);
    }

    // Incidence (n x m), edge sizes De (m), node degrees Dv (n).
    let mut incidence = Array2::<f64>::zeros((n, m));
    let mut de = vec![0.0_f64; m];
    let mut dv = vec![0.0_f64; n];
    for (j, e) in edge_ids.iter().enumerate() {
        let members = h.members(e).unwrap_or_default();
        for member in &members {
            let i = node_pos[member];
            incidence[[i, j]] = 1.0;
            de[j] += 1.0;
            dv[i] += 1.0;
        }
    }

    // Isolated-node guard.
    for &d in &dv {
        if d == 0.0 {
            return Err(SpectralError::IsolatedNode);
        }
    }

    // M[i][k] = Σ_j (H[i][j] H[k][j] / De[j]) · Dv[i]^{-1/2} Dv[k]^{-1/2}
    // L = I − M.
    let dv_invsqrt: Vec<f64> = dv.iter().map(|&d| d.powf(-0.5)).collect();
    let mut l = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        for k in 0..n {
            let mut acc = 0.0_f64;
            for j in 0..m {
                let hij = incidence[[i, j]];
                let hkj = incidence[[k, j]];
                if hij != 0.0 && hkj != 0.0 {
                    acc += hij * hkj / de[j];
                }
            }
            let m_ik = acc * dv_invsqrt[i] * dv_invsqrt[k];
            l[[i, k]] = if i == k { 1.0 - m_ik } else { -m_ik };
        }
    }

    // Silence the unused-import lint until the eigensolver (Task 14) uses Array1.
    let _ = Array1::<f64>::zeros(0);

    Ok((l, node_ids))
}
