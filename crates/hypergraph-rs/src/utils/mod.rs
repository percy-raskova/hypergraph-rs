//! Pure utility functions ported from `xgi/utils/` (non-Python-machinery only).
//!
//! Ported: `dual_dict`, `powerset`, `subfaces`, `get_network_type`,
//! `convert_labels_to_integers`. See the Phase-6 plan header for the exclusion list.

use indexmap::IndexMap;
use thiserror::Error;

/// Errors raised by utility functions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum UtilError {
    /// `subfaces` was asked for an order larger than the max order of the input edges.
    #[error("order {order} exceeds the maximum edge order {max_order}")]
    OrderTooLarge { order: i64, max_order: i64 },
}

/// Reverse an id→members map into a member→ids map.
///
/// XGI parity: `xgi.dual_dict` (`utils/utilities.py:65-94`). XGI returns
/// `defaultdict(set)` (hash-ordered); we return an insertion-ordered `IndexMap`
/// keyed by node first-seen order (a III.7 determinism improvement). Per-node edge
/// lists preserve the order in which edges were encountered.
pub fn dual_dict(edge_dict: &IndexMap<String, Vec<String>>) -> IndexMap<String, Vec<String>> {
    let mut node_dict: IndexMap<String, Vec<String>> = IndexMap::new();
    for (edge_id, members) in edge_dict {
        for node in members {
            node_dict
                .entry(node.clone())
                .or_default()
                .push(edge_id.clone());
        }
    }
    node_dict
}

/// All `r`-length index-combinations of `items`, in lexicographic-by-index order.
///
/// Matches Python `itertools.combinations` ordering. Bounded: at most `C(n, r)`
/// combinations, each built by a fixed-length inner loop.
fn combinations(items: &[String], r: usize) -> Vec<Vec<String>> {
    let n = items.len();
    let mut out: Vec<Vec<String>> = Vec::new();
    if r == 0 {
        out.push(Vec::new());
        return out;
    }
    if r > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..r).collect();
    loop {
        out.push(idx.iter().map(|&i| items[i].clone()).collect());
        // Advance the index vector (odometer over strictly-increasing indices).
        let mut i = r; // position to bump, scanned from the right
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - r {
                idx[i] += 1;
                for j in (i + 1)..r {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out; // all positions maxed out
            }
        }
    }
}

/// All subsets of `items`, with XGI's option flags.
///
/// XGI parity: `xgi.powerset` (`utils/utilities.py:97-151`).
/// `include_empty` overrides `include_singletons` when true; `max_size` caps the
/// largest subset returned. Order is by ascending size, then lexicographic-by-index.
///
/// **Flag order differs from XGI.** Python's positional signature is
/// `powerset(iterable, include_empty, include_full, include_singletons, max_size)`; this
/// Rust signature is `(items, include_empty, include_singletons, include_full, max_size)`
/// — `include_full` and `include_singletons` are swapped. A future PyO3 binding that
/// mirrors Python's positional call order must remap these two arguments, not pass them
/// straight through.
pub fn powerset(
    items: &[String],
    include_empty: bool,
    include_singletons: bool,
    include_full: bool,
    max_size: Option<usize>,
) -> Vec<Vec<String>> {
    let n = items.len();
    let mut start = if include_singletons { 1 } else { 2 };
    if include_empty {
        start = 0;
    }
    let max = match max_size {
        Some(m) => m.min(n),
        None => {
            if include_full {
                n
            } else {
                n.saturating_sub(1)
            }
        }
    };
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut r = start;
    while r <= max {
        out.extend(combinations(items, r));
        r += 1;
    }
    out
}
