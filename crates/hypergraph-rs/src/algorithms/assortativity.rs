//! Degree assortativity of a hypergraph (`xgi/algorithms/assortativity.py`).
//!
//! Deterministic paths only: `dynamical_assortativity` and the exact
//! `degree_assortativity`. The random-sampling default awaits the Phase-4
//! `rand_pcg` pin.

use thiserror::Error;

use super::properties::{is_uniform, unique_edge_sizes};
use crate::Hypergraph;

/// Errors from assortativity computations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AssortError {
    #[error("hypergraph must contain nodes")]
    NoNodes,
    #[error("hypergraph must contain edges")]
    NoEdges,
    #[error("hypergraph must be uniform")]
    NotUniform,
    #[error("hypergraph must have no singleton edges")]
    HasSingletons,
}

/// The kind of exact degree assortativity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssortKind {
    Uniform,
    Top2,
    TopBottom,
}

fn degrees<N, E, M>(h: &Hypergraph<N, E, M>) -> std::collections::HashMap<String, f64> {
    h.node_ids()
        .into_iter()
        .map(|n| {
            let d = h.node_degree(&n, None).unwrap_or(0) as f64;
            (n, d)
        })
        .collect()
}

/// Dynamical assortativity of a uniform hypergraph.
/// XGI parity: `dynamical_assortativity` (`assortativity.py:13-64`).
pub fn dynamical_assortativity<N, E, M>(h: &Hypergraph<N, E, M>) -> Result<f64, AssortError> {
    if h.num_nodes() == 0 {
        return Err(AssortError::NoNodes);
    }
    if h.num_edges() == 0 {
        return Err(AssortError::NoEdges);
    }
    if is_uniform(h).is_none() {
        return Err(AssortError::NotUniform);
    }
    if unique_edge_sizes(h).contains(&1) {
        return Err(AssortError::HasSingletons);
    }
    let k = degrees(h);
    let vals: Vec<f64> = h.node_ids().iter().map(|n| k[n]).collect();
    let n = vals.len() as f64;
    let k1 = vals.iter().sum::<f64>() / n; // mean degree
    let k2 = vals.iter().map(|x| x * x).sum::<f64>() / n; // 2nd moment
    let mut pair_products: Vec<f64> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        for a in 0..members.len() {
            for b in (a + 1)..members.len() {
                pair_products.push(k[&members[a]] * k[&members[b]]);
            }
        }
    }
    let kk1 = pair_products.iter().sum::<f64>() / pair_products.len() as f64;
    Ok(kk1 * k1 * k1 / (k2 * k2) - 1.0)
}

/// Choose two degrees from an edge per the assortativity kind (exact variants).
fn choose_degrees(
    members: &[String],
    k: &std::collections::HashMap<String, f64>,
    kind: AssortKind,
) -> Vec<(f64, f64)> {
    match kind {
        AssortKind::Uniform => {
            let mut out = Vec::new();
            // permutations of size 2 (ordered) with n1 != n2.
            for a in 0..members.len() {
                for b in 0..members.len() {
                    if a != b {
                        out.push((k[&members[a]], k[&members[b]]));
                    }
                }
            }
            out
        }
        AssortKind::Top2 => {
            let mut ks: Vec<f64> = members.iter().map(|m| k[m]).collect();
            ks.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let top = &ks[ks.len().saturating_sub(2)..];
            // ordered permutations of the two selected degrees
            vec![(top[0], top[1]), (top[1], top[0])]
        }
        AssortKind::TopBottom => {
            let mut ks: Vec<f64> = members.iter().map(|m| k[m]).collect();
            ks.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let lo = ks[0];
            let hi = ks[ks.len() - 1];
            vec![(lo, hi), (hi, lo)]
        }
    }
}

/// Pearson correlation of two equal-length columns; 0 if undefined (nan).
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        cov += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    let denom = (vx * vy).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        cov / denom
    }
}

/// Exact degree assortativity (Pearson corr over the chosen degree pairs).
/// XGI parity: `degree_assortativity(exact=True)` (`assortativity.py:117-161`).
pub fn degree_assortativity_exact<N, E, M>(
    h: &Hypergraph<N, E, M>,
    kind: AssortKind,
) -> Result<f64, AssortError> {
    if h.num_nodes() == 0 {
        return Err(AssortError::NoNodes);
    }
    if h.num_edges() == 0 {
        return Err(AssortError::NoEdges);
    }
    let k = degrees(h);
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        if members.len() > 1 {
            for (a, b) in choose_degrees(&members, &k, kind) {
                xs.push(a);
                ys.push(b);
            }
        }
    }
    Ok(pearson(&xs, &ys))
}
