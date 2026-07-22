//! Spectral community detection ported from `xgi/communities/spectral.py`.
//!
//! `spectral_clustering` builds the normalized hypergraph Laplacian (formula from the
//! Phase-3 `linalg/laplacian_matrix.py`, inlined here for self-containment), extracts the
//! `k` smallest-eigenvalue eigenvectors via hand-rolled power iteration + Hotelling
//! deflation (the dependency probe found no LAPACK-free ndarray eigensolver), and runs
//! Forgy k-means. Exact XGI parity is impossible (RNG divergence, spec §9.4); assertions
//! are structural, and results are deterministic under a fixed `seed`.

use ndarray::{Array2, Array1};
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;
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

    Ok((l, node_ids))
}

/// Dot product of two column vectors.
#[inline]
fn dot(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    let mut s = 0.0;
    for i in 0..a.len() {
        s += a[i] * b[i];
    }
    s
}

/// Matrix–vector product `m · v` for a dense symmetric matrix.
fn matvec(m: &Array2<f64>, v: &Array1<f64>) -> Array1<f64> {
    let n = m.nrows();
    let mut out = Array1::<f64>::zeros(n);
    for i in 0..n {
        let mut s = 0.0;
        for j in 0..n {
            s += m[[i, j]] * v[j];
        }
        out[i] = s;
    }
    out
}

/// The `k` smallest-eigenvalue eigenvectors of the symmetric matrix `l`, as columns.
///
/// Replaces `scipy` `eigsh(which="SA")` (`communities/spectral.py:61`). Uses power
/// iteration on `B = cI − L` (largest of `B` = smallest of `L`) with Gram–Schmidt
/// deflation. Convergence tolerance `1e-9`; at most 1000 iterations per eigenvector.
/// Eigenvector signs are arbitrary. Deterministic under `seed`.
pub fn smallest_k_eigenvectors(l: &Array2<f64>, k: usize, seed: Option<u64>) -> Array2<f64> {
    let n = l.nrows();
    let k = k.min(n);

    // Gershgorin bound on the spectral radius: c = max_i Σ_j |L[i,j]|.
    let mut c = 0.0_f64;
    for i in 0..n {
        let mut row_abs = 0.0;
        for j in 0..n {
            row_abs += l[[i, j]].abs();
        }
        if row_abs > c {
            c = row_abs;
        }
    }
    // B = cI − L.
    let mut b = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        for j in 0..n {
            b[[i, j]] = if i == j { c - l[[i, j]] } else { -l[[i, j]] };
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed.unwrap_or(0));
    let mut eigvecs: Vec<Array1<f64>> = Vec::with_capacity(k);

    const TOL: f64 = 1e-9;
    const MAX_ITER: usize = 1000;

    for _ in 0..k {
        // Random init.
        let mut v = Array1::<f64>::zeros(n);
        for i in 0..n {
            v[i] = rng.random_range(0.0..1.0) - 0.5;
        }
        // Deflate + normalize the initial vector.
        deflate(&mut v, &eigvecs);
        normalize(&mut v);

        for _ in 0..MAX_ITER {
            let mut w = matvec(&b, &v);
            deflate(&mut w, &eigvecs);
            normalize(&mut w);
            // Fix sign for a stable convergence check (largest-|component| positive).
            align_sign(&mut w);
            let mut prev = v.clone();
            align_sign(&mut prev);
            let mut diff = 0.0;
            for i in 0..n {
                diff += (w[i] - prev[i]).powi(2);
            }
            v = w;
            if diff.sqrt() < TOL {
                break;
            }
        }
        eigvecs.push(v);
    }

    // Pack as columns of an n x k array.
    let mut x = Array2::<f64>::zeros((n, k));
    for (col, vec) in eigvecs.iter().enumerate() {
        for i in 0..n {
            x[[i, col]] = vec[i];
        }
    }
    x
}

/// Gram–Schmidt: remove the components of `v` along each already-found eigenvector.
fn deflate(v: &mut Array1<f64>, basis: &[Array1<f64>]) {
    for u in basis {
        let proj = dot(v, u);
        for i in 0..v.len() {
            v[i] -= proj * u[i];
        }
    }
}

/// Normalize `v` to unit L2 norm (no-op on a zero vector).
fn normalize(v: &mut Array1<f64>) {
    let norm = dot(v, v).sqrt();
    if norm > 1e-300 {
        for i in 0..v.len() {
            v[i] /= norm;
        }
    }
}

/// Flip the sign of `v` so its largest-magnitude component is non-negative
/// (removes the eigenvector sign ambiguity for deterministic comparison).
fn align_sign(v: &mut Array1<f64>) {
    let mut max_abs = 0.0;
    let mut sign = 1.0;
    for i in 0..v.len() {
        if v[i].abs() > max_abs {
            max_abs = v[i].abs();
            sign = if v[i] < 0.0 { -1.0 } else { 1.0 };
        }
    }
    if sign < 0.0 {
        for i in 0..v.len() {
            v[i] = -v[i];
        }
    }
}
