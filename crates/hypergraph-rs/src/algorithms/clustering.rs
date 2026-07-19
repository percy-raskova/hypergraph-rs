//! Nodal clustering coefficients (`xgi/algorithms/clustering.py`).

use std::collections::HashSet;

use indexmap::IndexMap;
use ndarray::Axis;

use crate::linalg::adjacency_matrix;
use crate::Hypergraph;

/// Clustering coefficient via the unweighted pairwise projection:
/// `c_i = 0.5 · (A³)_ii / [k_i (k_i − 1) / 2]`, 0 when undefined.
/// XGI parity: `clustering_coefficient` (`clustering.py:17-74`).
pub fn clustering_coefficient<N, E, M>(h: &Hypergraph<N, E, M>) -> IndexMap<String, f64> {
    let (adj, node_ids) = adjacency_matrix(h, None, 1, false);
    let mut result: IndexMap<String, f64> = IndexMap::new();
    // Nodes absent from the adjacency (isolated) get 0.
    for n in h.node_ids() {
        result.insert(n, 0.0);
    }
    if adj.shape() == [0, 0] {
        return result;
    }
    let k = adj.sum_axis(Axis(1));
    let a3 = adj.dot(&adj).dot(&adj);
    for (i, n) in node_ids.iter().enumerate() {
        let denom = k[i] * (k[i] - 1.0) / 2.0;
        let c = if denom != 0.0 {
            0.5 * a3[[i, i]] / denom
        } else {
            0.0
        };
        result.insert(n.clone(), c);
    }
    result
}

/// Local clustering coefficient based on edge overlap.
/// XGI parity: `local_clustering_coefficient` (`clustering.py:77-164`).
pub fn local_clustering_coefficient<N, E, M>(h: &Hypergraph<N, E, M>) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        let ev = h.memberships(&n).unwrap_or_default();
        let dv = ev.len();
        if dv <= 1 {
            result.insert(n, 0.0);
            continue;
        }
        let mut total_eo = 0.0;
        // Bounded: |ev| choose 2.
        for a in 0..ev.len() {
            for b in (a + 1)..ev.len() {
                let edge1: HashSet<String> =
                    h.members(&ev[a]).unwrap_or_default().into_iter().collect();
                let edge2: HashSet<String> =
                    h.members(&ev[b]).unwrap_or_default().into_iter().collect();
                let d1: HashSet<String> = edge1.difference(&edge2).cloned().collect();
                let d2: HashSet<String> = edge2.difference(&edge1).cloned().collect();
                let union: HashSet<String> = d1.union(&d2).cloned().collect();
                let eo = if union.is_empty() {
                    0.0
                } else {
                    let neigh_d1: HashSet<String> = d1
                        .iter()
                        .flat_map(|d| h.node_neighbors(d).unwrap_or_default())
                        .collect();
                    let neigh_d2: HashSet<String> = d2
                        .iter()
                        .flat_map(|d| h.node_neighbors(d).unwrap_or_default())
                        .collect();
                    let a1 = neigh_d1.intersection(&d2).count();
                    let a2 = neigh_d2.intersection(&d1).count();
                    (a1 + a2) as f64 / union.len() as f64
                };
                total_eo += eo;
            }
        }
        result.insert(n, 2.0 * total_eo / (dv as f64 * (dv as f64 - 1.0)));
    }
    result
}

/// Two-node clustering coefficient averaged over a node's neighbours.
/// `kind` ∈ {"union", "min", "max"}. XGI parity:
/// `two_node_clustering_coefficient` (`clustering.py:167-278`).
pub fn two_node_clustering_coefficient<N, E, M>(
    h: &Hypergraph<N, E, M>,
    kind: &str,
) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        let neighbors = h.node_neighbors(&n).unwrap_or_default();
        if neighbors.is_empty() {
            result.insert(n, 0.0);
            continue;
        }
        let m_u: HashSet<String> = h.memberships(&n).unwrap_or_default().into_iter().collect();
        let mut acc = 0.0;
        let count = neighbors.len() as f64;
        for v in &neighbors {
            let m_v: HashSet<String> = h.memberships(v).unwrap_or_default().into_iter().collect();
            let num = m_u.intersection(&m_v).count() as f64;
            let denom = match kind {
                "union" => m_u.union(&m_v).count() as f64,
                "min" => m_u.len().min(m_v.len()) as f64,
                "max" => m_u.len().max(m_v.len()) as f64,
                _ => panic!("invalid kind of clustering: {kind}"),
            };
            // Deviation #10: XGI's `_uv_cc` returns NaN here; we skip the term
            // instead (unreachable in practice — `v` is a neighbour of `n`, so
            // they share >=1 edge and every `kind`'s denominator is >=1).
            if denom != 0.0 {
                acc += (num / denom) / count;
            }
        }
        result.insert(n, acc);
    }
    result
}
