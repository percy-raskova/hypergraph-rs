//! Conversions to/from hyperedge lists/dicts, bipartite edgelists, bipartite petgraph
//! graphs, the 1-skeleton graph, the standardized hypergraph dict, and order-cutting.
//!
//! Ported from `xgi/convert/`. Operates on the default `Hypergraph`
//! (`serde_json::Value` attrs). The `pandas` slice is NOT ported (spec §4.4); HIF,
//! encapsulation-DAG, line-graph, simplex, and incidence conversions are deferred
//! (see the Phase-6 plan open questions).

use indexmap::IndexMap;
use serde_json::Value;
use thiserror::Error;

use crate::Hypergraph;

/// Errors raised by conversion functions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ConvertError {
    /// The input structure was malformed for the requested conversion.
    #[error("malformed input: {0}")]
    Malformed(String),
    /// A supposedly-bipartite graph had an edge joining two same-kind nodes.
    #[error("graph is not bipartite: edge joins two nodes of the same kind")]
    NotBipartite,
    /// A bipartite-graph node lacked a valid `bipartite` marker (0 or 1).
    #[error("node {0} has an invalid bipartite marker (expected 0 or 1)")]
    InvalidMarker(String),
    /// `cut_to_order` was given an order above the network's maximum edge order.
    #[error("order {order} exceeds the maximum edge order {max_order}")]
    OrderTooLarge { order: i64, max_order: i64 },
}

/// Build a hypergraph from a list of hyperedges (auto-generated ids).
///
/// XGI parity: `xgi.from_hyperedge_list` (`convert/hyperedges.py:64-90`).
pub fn from_hyperedge_list(edges: Vec<Vec<String>>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for members in edges {
        // Non-empty edges only; empty inputs are skipped (our core rejects empty edges).
        if !members.is_empty() {
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    h
}

/// Extract the hyperedge list (members per edge, edge insertion order).
///
/// XGI parity: `xgi.to_hyperedge_list` (`convert/hyperedges.py:93-112`).
pub fn to_hyperedge_list(h: &Hypergraph) -> Vec<Vec<String>> {
    h.edge_ids()
        .into_iter()
        .map(|e| h.members(&e).unwrap_or_default())
        .collect()
}

/// Build a hypergraph from an id→members dict (preserving ids).
///
/// XGI parity: `xgi.from_hyperedge_dict` (`convert/hyperedges.py:14-38`).
pub fn from_hyperedge_dict(d: Vec<(String, Vec<String>)>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for (uid, members) in d {
        if !members.is_empty() {
            let _ = h.add_edge(members, Some(uid), Value::Null);
        }
    }
    h
}

/// Extract the id→members dict (edge insertion order).
///
/// XGI parity: `xgi.to_hyperedge_dict` (`convert/hyperedges.py:41-61`).
pub fn to_hyperedge_dict(h: &Hypergraph) -> IndexMap<String, Vec<String>> {
    h.edge_ids()
        .into_iter()
        .map(|e| {
            let members = h.members(&e).unwrap_or_default();
            (e, members)
        })
        .collect()
}
