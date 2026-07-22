//! Random (non-uniform) hypergraph generators (XGI `generators/random.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use super::rng::{comb, geometric, index_to_edge_comb, seeded_rng, uniform_f64};
use crate::Hypergraph;

/// Resolve `(ps, order)` per XGI `_check_input_args`: default `order = 1..=len(ps)`;
/// validate every `p in [0, 1]` and lengths match.
fn check_input_args(ps: &[f64], order: Option<&[usize]>) -> Result<Vec<usize>, GeneratorError> {
    let order: Vec<usize> = match order {
        None => (0..ps.len()).map(|i| i + 1).collect(),
        Some(o) => {
            if o.len() != ps.len() {
                return Err(GeneratorError::InvalidParameter(
                    "the length of ps must match the length of order".into(),
                ));
            }
            o.to_vec()
        }
    };
    if ps.iter().any(|&p| !(0.0..=1.0).contains(&p)) {
        return Err(GeneratorError::InvalidParameter(
            "all elements of ps must be between 0 and 1".into(),
        ));
    }
    Ok(order)
}

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

/// Fast random hypergraph via geometric skip-sampling. XGI `fast_random_hypergraph`.
pub fn fast_random_hypergraph(
    n: usize,
    ps: &[f64],
    order: Option<&[usize]>,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let order = check_input_args(ps, order)?;
    let mut rng = seeded_rng(seed);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    for (d, &p) in order.iter().zip(ps.iter()) {
        let d = *d;
        if p == 1.0 {
            for combo in all_combinations(n, d + 1) {
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
            }
        } else if p > 0.0 {
            let max_index = comb(n as u64, (d + 1) as u64).saturating_sub(1);
            // -1 for zero-indexing (geometric >= 1 here)
            let mut index = geometric(p, &mut rng) - 1;
            // Bounded: index strictly increases by >= 1 each step, capped by max_index.
            while index <= max_index {
                let combo = index_to_edge_comb(index, n as u64, (d + 1) as u64);
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
                index += geometric(p, &mut rng);
            }
        }
    }
    Ok(h)
}

/// Random hypergraph via Bernoulli over every combination. XGI `random_hypergraph`.
pub fn random_hypergraph(
    n: usize,
    ps: &[f64],
    order: Option<&[usize]>,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let order = check_input_args(ps, order)?;
    let mut rng = seeded_rng(seed);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    for (d, &p) in order.iter().zip(ps.iter()) {
        for combo in all_combinations(n, *d + 1) {
            if uniform_f64(&mut rng) <= p {
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
            }
        }
    }
    Ok(h)
}
