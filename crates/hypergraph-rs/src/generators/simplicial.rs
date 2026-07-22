//! Simplicial-complex generators (XGI `generators/simplicial_complexes.py`).
//!
//! Only `random_simplicial_complex` is ported here; the `flag_complex` family
//! consumes NetworkX graphs and belongs to the `convert` slice (Phase 6).

use serde_json::Value;

use super::classic::GeneratorError;
use super::rng::{seeded_rng, uniform_f64};
use crate::SimplicialComplex;

/// Enumerate strictly-increasing `k`-combinations of `0..n`.
fn all_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out;
            }
        }
    }
}

/// Random simplicial complex: connect any `d+1` nodes with a simplex with prob
/// `ps[d-1]`; all subfaces are added. XGI `random_simplicial_complex(N, ps)`.
///
/// Note (Phase 4 Task 9): the plan's assumed `SimplicialComplex` API
/// (`add_nodes_from(impl IntoIterator<Item=(String, Value)>)`,
/// `add_simplices_from(impl IntoIterator<Item = Vec<String>>)`) does not
/// match the landed Phase-2 surface exactly — `add_simplices_from` takes
/// 3-tuples `(members, idx, attrs)`, and `add_node`/`add_nodes_from` did not
/// exist at all (added to `core/simplicialcomplex.rs` alongside this task,
/// mirroring `Hypergraph::add_node`/`add_nodes_from`) so that all `N` nodes
/// can be registered up front, matching XGI's `S.add_nodes_from(nodes)`
/// before `S.add_simplices_from(simplices)`.
pub fn random_simplicial_complex(
    n: usize,
    ps: &[f64],
    seed: Option<u64>,
) -> Result<SimplicialComplex, GeneratorError> {
    if ps.iter().any(|&p| !(0.0..=1.0).contains(&p)) {
        return Err(GeneratorError::InvalidParameter(
            "all elements of ps must be between 0 and 1".into(),
        ));
    }
    let mut rng = seeded_rng(seed);

    let mut simplices: Vec<(Vec<String>, Option<String>, Value)> = Vec::new();
    for (i, &p) in ps.iter().enumerate() {
        let d = i + 1; // ps[0] is prob of edges (d=1)
        for combo in all_combinations(n, d + 1) {
            if uniform_f64(&mut rng) <= p {
                simplices.push((
                    combo.iter().map(|x| x.to_string()).collect(),
                    None,
                    Value::Null,
                ));
            }
        }
    }

    let mut s = SimplicialComplex::new();
    s.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    s.add_simplices_from(simplices);
    Ok(s)
}
