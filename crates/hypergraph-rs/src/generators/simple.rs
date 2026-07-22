//! Simple structural hypergraph generators (XGI `xgi/generators/simple.py` port).

use serde_json::Value;

use super::classic::GeneratorError;
use crate::Hypergraph;

/// Enumerate strictly-increasing `k`-combinations of the slice `pool`.
fn combinations_of(pool: &[usize], k: usize) -> Vec<Vec<usize>> {
    let n = pool.len();
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.iter().map(|&i| pool[i]).collect());
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

/// A star network joined to a clique promoted to hyperedges up to `d_max`.
/// XGI `star_clique(n_star, n_clique, d_max)`.
pub fn star_clique(
    n_star: usize,
    n_clique: usize,
    d_max: usize,
) -> Result<Hypergraph, GeneratorError> {
    if n_star == 0 {
        return Err(GeneratorError::InvalidParameter(
            "n_star must be > 0".into(),
        ));
    }
    if n_clique == 0 {
        return Err(GeneratorError::InvalidParameter(
            "n_clique must be > 0".into(),
        ));
    }
    if d_max > n_clique - 1 {
        return Err(GeneratorError::InvalidParameter(
            "d_max must be <= n_clique - 1".into(),
        ));
    }

    let nodes_star: Vec<usize> = (0..n_star).collect();
    let nodes_clique: Vec<usize> = (n_star..(n_star + n_clique)).collect();

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..(n_star + n_clique)).map(|i| (i.to_string(), Value::Null)));

    // star legs (center = nodes_star[0])
    for i in 1..n_star {
        let _ = h.add_edge(
            vec![nodes_star[0].to_string(), nodes_star[i].to_string()],
            None,
            Value::Null,
        );
    }
    // connect star to clique
    let _ = h.add_edge(
        vec![nodes_star[0].to_string(), nodes_clique[0].to_string()],
        None,
        Value::Null,
    );
    // clique hyperedges up to order d_max (sizes 2..=d_max+1)
    for d in 1..=d_max {
        for combo in combinations_of(&nodes_clique, d + 1) {
            let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    Ok(h)
}

/// An m-uniform sunflower hypergraph: `l` petals sharing a `c`-node core.
/// XGI `sunflower(l, c, m)`.
pub fn sunflower(l: usize, c: usize, m: usize) -> Result<Hypergraph, GeneratorError> {
    if m < c {
        return Err(GeneratorError::InvalidParameter(
            "m cannot be smaller than c".into(),
        ));
    }
    let core: Vec<String> = (0..c).map(|i| i.to_string()).collect();
    let mut h = Hypergraph::new();
    let petal = m - c;
    // FINDING 5 (deliberate deviation, documentation only): when m == c,
    // `petal == 0`. XGI's `while start_label + (m - c) <= c + (m - c) * l`
    // (xgi/generators/simple.py:114) becomes `while c <= c`, which is always
    // true and `start_label` never advances — an unbounded loop by
    // construction (read from source, not run — it would not terminate).
    // Our `for p in 0..l` is bounded by construction (Power-of-10 rule 2)
    // and instead terminates after exactly `l` petals, each identical to the
    // `c`-node core (added as `l` distinct parallel edges via auto-id) — a
    // deliberate improvement, not a bug-for-bug port.
    // Bounded: exactly l petals.
    for p in 0..l {
        let mut members = core.clone();
        let start = c + p * petal;
        for i in 0..petal {
            members.push((start + i).to_string());
        }
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}
