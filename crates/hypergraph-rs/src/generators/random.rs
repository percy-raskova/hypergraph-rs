//! Random (non-uniform) hypergraph generators (XGI `generators/random.py`).

use indexmap::IndexMap;
use serde_json::Value;

use super::classic::GeneratorError;
use super::lattice::ring_lattice;
use super::rng::{comb, geometric, index_to_edge_comb, random_below, seeded_rng, uniform_f64};
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
            let count = comb(n as u64, (d + 1) as u64);
            // FINDING 1: when the requested edge size (d+1) exceeds n, comb == 0
            // and there is no valid combination index at all — XGI's
            // `comb(n, d+1, exact=True) - 1` is -1 (a signed bigint) so its
            // `while index <= max_index` loop never runs. Our `max_index` is
            // unsigned, so skip the order entirely instead of computing a
            // saturated (wrong) max_index of 0, which would otherwise let
            // `index_to_edge_comb` run once on an out-of-range index and drive
            // `comb`'s accumulator into overflow (see rng.rs `comb`).
            if count > 0 {
                let max_index = count - 1;
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

/// Chung–Lu hypergraph from node-degree and edge-size sequences.
/// XGI `chung_lu_hypergraph(k1, k2)`.
///
/// `k1`: node id -> degree; `k2`: edge id -> size. Determinism: nodes/edges are
/// sorted by value descending with a stable tiebreak on the (insertion-ordered)
/// key, so the traversal is reproducible.
pub fn chung_lu_hypergraph(
    k1: &IndexMap<String, f64>,
    k2: &IndexMap<String, f64>,
    seed: Option<u64>,
) -> Hypergraph {
    let mut rng = seeded_rng(seed);

    // sort by value desc; stable tiebreak preserves IndexMap insertion order
    let mut node_labels: Vec<&String> = k1.keys().collect();
    node_labels.sort_by(|a, b| k2_cmp(k1[*b], k1[*a]));
    let mut edge_labels: Vec<&String> = k2.keys().collect();
    edge_labels.sort_by(|a, b| k2_cmp(k2[*b], k2[*a]));

    let m = k2.len();
    let s: f64 = k1.values().sum();

    let mut h = Hypergraph::new();
    for &nid in &node_labels {
        h.add_node(nid, Value::Null);
    }

    for &u in &node_labels {
        let mut j = 0usize;
        if m == 0 {
            continue;
        }
        let mut v = edge_labels[0];
        let mut p = ((k1[u] * k2[v]) / s).min(1.0);
        // Bounded: j strictly increases toward m.
        while j < m {
            if p != 1.0 {
                if p <= 0.0 {
                    // geometric(p<=0) returns the u64::MAX "infinity" sentinel;
                    // XGI relies on Python float `j += inf` to end the loop for
                    // this node. Break instead of adding the sentinel, which
                    // would overflow `j` (a real panic risk when j > 0).
                    break;
                }
                j += geometric(p, &mut rng) as usize;
            }
            if j < m {
                v = edge_labels[j];
                let q = ((k1[u] * k2[v]) / s).min(1.0);
                let r = uniform_f64(&mut rng);
                if p > 0.0 && r < q / p {
                    let _ = h.add_node_to_edge(v, u);
                }
                p = q;
                j += 1;
            }
        }
    }
    h
}

/// Total order on f64 for descending sort (NaN sinks last). Not exported.
fn k2_cmp(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

/// Watts–Strogatz hypergraph: a ring lattice with each edge rewired with prob `p`.
/// XGI `watts_strogatz_hypergraph(n, d, k, l, p)`.
pub fn watts_strogatz_hypergraph(
    n: usize,
    d: usize,
    k: usize,
    l: usize,
    p: f64,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let mut rng = seeded_rng(seed);
    let mut h = ring_lattice(n, d, k, l, false)?;

    let mut to_remove: Vec<String> = Vec::new();
    let mut to_add: Vec<Vec<String>> = Vec::new();

    for e in h.edge_ids() {
        if uniform_f64(&mut rng) < p {
            let members = h.members(&e).unwrap();
            // Keep the numeric-min member (node ids are decimal integers here),
            // matching XGI's `min(H.edges.members(e))` over integer node ids.
            // A lexicographic `str` min would diverge for n >= 11 (e.g. "10" < "2").
            let keep = members
                .iter()
                .min_by_key(|s| s.parse::<u64>().unwrap_or(u64::MAX))
                .cloned()
                .unwrap();
            let mut new_edge: Vec<String> = Vec::with_capacity(d);
            for _ in 0..(d - 1) {
                let idx = random_below(&mut rng, n as u64) as usize;
                new_edge.push(idx.to_string());
            }
            new_edge.push(keep);
            to_remove.push(e);
            to_add.push(new_edge);
        }
    }
    for e in to_remove {
        let _ = h.remove_edge(&e);
    }
    for members in to_add {
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}
