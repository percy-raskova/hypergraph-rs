//! Uniform random hypergraph generators (XGI `generators/uniform.py`).
//!
//! `uniform_HSBM` / `uniform_HPPM` require an m-dimensional probability tensor and
//! are deferred (see the plan's Open questions).

use std::collections::HashSet;

use indexmap::IndexMap;
use serde_json::Value;

use super::classic::{complete_hypergraph, GeneratorError};
use super::rng::{
    choice_without_replacement, comb, geometric, index_to_edge_comb, index_to_edge_prod, seeded_rng,
};
use crate::Hypergraph;

/// How `p` is interpreted in `uniform_erdos_renyi_model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PType {
    /// `p` is the per-edge probability.
    Prob,
    /// `p` is the target mean degree.
    Degree,
}

/// m-uniform Erdős–Rényi hypergraph. XGI `uniform_erdos_renyi_model`.
pub fn uniform_erdos_renyi_model(
    n: usize,
    m: usize,
    p: f64,
    p_type: PType,
    multiedges: bool,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let q = match p_type {
        PType::Degree => {
            if multiedges {
                p / (m as f64 * (n as f64).powi((m - 1) as i32))
            } else {
                p * n as f64 / (m as f64 * comb(n as u64, m as u64) as f64)
            }
        }
        PType::Prob => p,
    };
    if !(0.0..=1.0).contains(&q) {
        return Err(GeneratorError::InvalidParameter(
            "probability not in [0, 1]".into(),
        ));
    }
    if q == 1.0 && !multiedges {
        return complete_hypergraph(n, Some(m - 1), None, false);
    }

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    if q == 0.0 {
        return Ok(h);
    }

    let mut rng = seeded_rng(seed);
    // FINDING 1: when m (the edge/tuple size) exceeds n, the combination count
    // is 0 and there is no valid index at all — XGI's
    // `comb(n, m, exact=True) - 1` is -1 (a signed bigint) so its
    // `while index <= max_index` loop never runs. Skip the whole sampling loop
    // in that case instead of computing a saturated (wrong) max_index of 0,
    // which would otherwise let `index_to_edge_comb`/`index_to_edge_prod` run
    // once on an out-of-range index and overflow `comb`'s accumulator.
    let count: u64 = if multiedges {
        (n as u64).pow(m as u32)
    } else {
        comb(n as u64, m as u64)
    };
    if count > 0 {
        let max_index = count - 1;
        let mut index = geometric(q, &mut rng) - 1;
        // Bounded: index strictly increases by >= 1 each step, capped by max_index.
        while index <= max_index {
            let tuple = if multiedges {
                index_to_edge_prod(index, n as u64, m as u64)
            } else {
                index_to_edge_comb(index, n as u64, m as u64)
            };
            // dedup to a set; keep only if still size m (discard self-loop artifacts)
            let set: HashSet<usize> = tuple.iter().copied().collect();
            if set.len() == m {
                let members: Vec<String> = tuple.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
            }
            index += geometric(q, &mut rng);
        }
    }
    Ok(h)
}

/// m-uniform configuration model from a node-degree sequence.
/// XGI `uniform_configuration_model(k, m)`.
///
/// XGI increases random node degrees when `sum(k) % m != 0`; we instead drop the
/// remainder stubs (documented deviation — still deterministic, avoids mutating
/// the caller's degree map). Loopy/duplicate-collapsed edges are discarded.
pub fn uniform_configuration_model(
    k: &IndexMap<String, usize>,
    m: usize,
    seed: Option<u64>,
) -> Hypergraph {
    let mut rng = seeded_rng(seed);

    // stub list: each node id repeated by its degree, in insertion order
    let mut stubs: Vec<String> = Vec::new();
    for (idx, &deg) in k {
        for _ in 0..deg {
            stubs.push(idx.clone());
        }
    }
    // drop remainder stubs so len is divisible by m (deviation from XGI)
    let rem = stubs.len() % m;
    for _ in 0..rem {
        stubs.pop();
    }

    let mut h = Hypergraph::new();
    for key in k.keys() {
        h.add_node(key, Value::Null);
    }

    // Bounded: each iteration removes exactly m stubs; stubs.len()/m iterations.
    while stubs.len() >= m {
        let picks = choice_without_replacement(&mut rng, stubs.len(), m);
        // Dedup in draw order (NOT via HashSet iteration, whose order is
        // process-randomized) so `members()` is reproducible bit-for-bit given
        // a seed, per the Global Constraint.
        let mut seen: HashSet<String> = HashSet::new();
        let mut members: Vec<String> = Vec::with_capacity(m);
        for &pi in &picks {
            let s = stubs[pi].clone();
            if seen.insert(s.clone()) {
                members.push(s);
            }
        }
        if members.len() == m {
            let _ = h.add_edge(members, None, Value::Null);
        }
        // remove chosen stubs, largest index first
        let mut sorted = picks.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for pi in sorted {
            stubs.remove(pi);
        }
    }
    h
}
