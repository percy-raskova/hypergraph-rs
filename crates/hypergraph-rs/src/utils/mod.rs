//! Pure utility functions ported from `xgi/utils/` (non-Python-machinery only).
//!
//! Ported: `dual_dict`, `powerset`, `subfaces`, `get_network_type`,
//! `convert_labels_to_integers`. See the Phase-6 plan header for the exclusion list.

use crate::Hypergraph;
use indexmap::IndexMap;
use serde_json::{json, Value};
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

/// Subfaces of a list of hyperedges.
///
/// XGI parity: `xgi.subfaces` (`utils/utilities.py:310-374`). Faces are NOT deduplicated
/// (a face may appear once per parent edge), matching XGI. Singleton and empty edges
/// contribute nothing.
pub fn subfaces(edges: &[Vec<String>], order: Option<i64>) -> Result<Vec<Vec<String>>, UtilError> {
    let max_order: i64 = edges.iter().map(|e| e.len() as i64 - 1).max().unwrap_or(-1);
    if let Some(o) = order {
        if o > max_order {
            return Err(UtilError::OrderTooLarge {
                order: o,
                max_order,
            });
        }
    }
    let mut faces: Vec<Vec<String>> = Vec::new();
    for edge in edges {
        let size = edge.len();
        if size <= 1 {
            continue;
        }
        let to_add = match order {
            None => powerset(edge, false, true, false, None),
            Some(-1) => combinations(edge, size - 1),
            Some(o) if o >= 0 => combinations(edge, (o + 1) as usize),
            Some(_) => Vec::new(), // order < -1 is undefined in XGI; treat as empty
        };
        faces.extend(to_add);
    }
    Ok(faces)
}

/// The lowercased network type name.
///
/// XGI parity: `xgi.get_network_type` (`utils/utilities.py:560-561`). For the undirected
/// `Hypergraph` this is the constant `"hypergraph"` (matching XGI's
/// `type(H).__name__.lower()` for `Hypergraph`).
pub fn get_network_type(_h: &Hypergraph) -> String {
    "hypergraph".to_string()
}

/// Relabel node and edge ids to sequential integer strings, stashing the old ids.
///
/// XGI parity: `xgi.convert_labels_to_integers` (`utils/utilities.py:377-452`). Nodes and
/// edges are mapped to `"0".."n-1"` / `"0".."m-1"` in insertion order; the old id is
/// written to `label_attribute` on each. Graph-level attrs are carried over. Returns a
/// new `Hypergraph` (we rebuild rather than clear-in-place; see the Phase-6 plan header).
pub fn convert_labels_to_integers(h: &Hypergraph, label_attribute: &str) -> Hypergraph {
    let node_index: IndexMap<String, usize> = h
        .node_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let edge_index: IndexMap<String, usize> = h
        .edge_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    let mut out = Hypergraph::new();

    // Carry over graph-level attributes.
    for (k, v) in h.graph_attrs() {
        out.set_graph_attr(k, v.clone());
    }

    // Nodes: new integer id + old id under label_attribute.
    for (old_id, &i) in &node_index {
        let mut obj = h
            .node_attrs(old_id)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.insert(label_attribute.to_string(), json!(old_id));
        out.add_node(&i.to_string(), Value::Object(obj));
    }

    // Edges: remap members, new integer id + old id under label_attribute.
    for (old_edge, &j) in &edge_index {
        let members: Vec<String> = h
            .members(old_edge)
            .unwrap_or_default()
            .iter()
            .map(|m| node_index[m].to_string())
            .collect();
        let mut obj = h
            .edge_attrs(old_edge)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.insert(label_attribute.to_string(), json!(old_edge));
        out.add_edge(members, Some(j.to_string()), Value::Object(obj))
            .expect("relabeled edge has non-empty members and a fresh unique id");
    }

    out
}
