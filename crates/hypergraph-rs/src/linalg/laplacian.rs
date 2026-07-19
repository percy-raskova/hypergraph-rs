//! Laplacian matrices (order-`d`, multiorder, normalized) of a hypergraph.

use ndarray::{Array1, Array2};

use super::hypergraph_matrix::{adjacency_matrix, degree_matrix, incidence_matrix};
use super::LinalgError;
use crate::Hypergraph;

/// Build a diagonal matrix from a vector.
fn diag(v: &Array1<f64>) -> Array2<f64> {
    let n = v.len();
    let mut m = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        m[[i, i]] = v[i];
    }
    m
}

/// Order-`d` Laplacian `L = d·diag(K) − A` (A = weighted adjacency at order `d`).
///
/// XGI parity: `xgi.laplacian(H, order, rescale_per_node)`
/// (`laplacian_matrix.py:62-115`). `order = 1` gives the usual graph Laplacian.
pub fn laplacian<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: usize,
    rescale_per_node: bool,
) -> (Array2<f64>, Vec<String>) {
    let (a, node_ids) = adjacency_matrix(h, Some(order), 1, true);
    if a.shape() == [0, 0] {
        return (Array2::zeros((0, 0)), Vec::new());
    }
    let (k, _) = degree_matrix(h, Some(order));
    let mut l = diag(&k) * (order as f64) - &a;
    if rescale_per_node {
        l = l / (order as f64);
    }
    (l, node_ids)
}

/// Multiorder Laplacian `Σ_d L_d · w_d / mean(K_d)`, skipping orders with no edges.
///
/// XGI parity: `xgi.multiorder_laplacian(H, orders, weights, rescale_per_node)`
/// (`laplacian_matrix.py:118-184`). Panics if `orders.len() != weights.len()`
/// (XGI raises `ValueError`).
pub fn multiorder_laplacian<N, E, M>(
    h: &Hypergraph<N, E, M>,
    orders: &[usize],
    weights: &[f64],
    rescale_per_node: bool,
) -> (Array2<f64>, Vec<String>) {
    assert_eq!(
        orders.len(),
        weights.len(),
        "orders and weights must have the same length"
    );
    let n = h.num_nodes();
    let mut l_multi = Array2::<f64>::zeros((n, n));
    for (&d, &w) in orders.iter().zip(weights.iter()) {
        let (k, _) = degree_matrix(h, Some(d));
        let mean_k = if k.is_empty() { 0.0 } else { k.mean().unwrap_or(0.0) };
        // Skip orders with no edges (all-zero K) to avoid divide-by-zero.
        if mean_k == 0.0 {
            continue;
        }
        let (l_d, _) = laplacian(h, d, rescale_per_node);
        if l_d.shape() == [n, n] {
            l_multi = l_multi + l_d * (w / mean_k);
        }
    }
    (l_multi, h.node_ids())
}

/// Normalized hypergraph Laplacian
/// `L = I − Dv^{-1/2}·Inc·W·De^{-1}·Incᵀ·Dv^{-1/2}`.
///
/// XGI parity: `xgi.normalized_hypergraph_laplacian(H, weighted)`
/// (`laplacian_matrix.py:187-254`). Concrete over `N = E = serde_json::Value`
/// because `weighted = true` reads each edge's `"weight"` attribute (default 1.0).
///
/// # Errors
/// [`LinalgError::IsolatedNodes`] if any node has degree 0 (division by zero).
pub fn normalized_hypergraph_laplacian<M>(
    h: &Hypergraph<serde_json::Value, serde_json::Value, M>,
    weighted: bool,
) -> Result<(Array2<f64>, Vec<String>), LinalgError> {
    // Isolate check: every node must belong to at least one edge.
    for nid in h.node_ids() {
        if h.memberships(&nid).map(|m| m.is_empty()).unwrap_or(true) {
            return Err(LinalgError::IsolatedNodes);
        }
    }

    let (inc, node_ids, edge_ids) = incidence_matrix(h, None);
    let n = node_ids.len();

    // Dv = node degrees (incidence row sums); De = edge sizes (col sums).
    let dv = inc.sum_axis(ndarray::Axis(1)); // len n
    let de = inc.sum_axis(ndarray::Axis(0)); // len m

    let weights: Vec<f64> = if weighted {
        edge_ids
            .iter()
            .map(|e| {
                h.edge_attrs(e)
                    .and_then(|a| a.get("weight"))
                    .and_then(|w| w.as_f64())
                    .unwrap_or(1.0)
            })
            .collect()
    } else {
        vec![1.0; edge_ids.len()]
    };

    let dv_invsqrt = diag(&dv.mapv(|x| x.powf(-0.5)));
    let de_inv = diag(&de.mapv(|x| 1.0 / x));
    let w = diag(&Array1::from(weights));
    let eye = Array2::<f64>::eye(n);

    let mid = inc.dot(&w).dot(&de_inv).dot(&inc.t());
    let l = eye - dv_invsqrt.dot(&mid).dot(&dv_invsqrt);
    Ok((l, node_ids))
}
