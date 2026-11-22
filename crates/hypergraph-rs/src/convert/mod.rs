//! Conversions to/from hyperedge lists/dicts, bipartite edgelists, bipartite petgraph
//! graphs, the 1-skeleton graph, and order-cutting. (XGI 1.0 dropped its hypergraph dict; so did this
//! module.)
//!
//! Ported from `xgi/convert/`. Operates on the default `Hypergraph`
//! (`serde_json::Value` attrs). The `pandas` slice is NOT ported (spec §4.4); HIF,
//! encapsulation-DAG, line-graph, simplex, and incidence conversions are deferred
//! (see the Phase-6 plan open questions).

use indexmap::IndexMap;
use rustworkx_core::petgraph::graph::UnGraph;
use rustworkx_core::petgraph::stable_graph::StableDiGraph;
use rustworkx_core::petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde_json::Value;
use std::collections::HashSet;
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

/// Build a hypergraph from a bipartite edgelist of `(node, edge)` pairs.
///
/// XGI parity: `xgi.from_bipartite_edgelist` (`convert/bipartite_edges.py:9-42`,
/// undirected branch — each pair calls `add_node_to_edge(edge, node)`).
pub fn from_bipartite_edgelist(edges: Vec<(String, String)>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for (node, edge) in edges {
        // add_node_to_edge auto-creates both; infallible on the default Value types.
        let _ = h.add_node_to_edge(&edge, &node);
    }
    h
}

/// Extract a bipartite edgelist of `(node, edge)` pairs (edge order, then member order).
///
/// XGI parity: `xgi.to_bipartite_edgelist` (`convert/bipartite_edges.py:45-73`).
pub fn to_bipartite_edgelist(h: &Hypergraph) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for e in h.edge_ids() {
        for n in h.members(&e).unwrap_or_default() {
            out.push((n, e.clone()));
        }
    }
    out
}

/// A vertex of a bipartite representation: an id plus a layer marker.
///
/// `bipartite == 0` -> a hypergraph node; `bipartite == 1` -> a hyperedge. Mirrors XGI's
/// `bipartite=0/1` NetworkX node attribute (`convert/bipartite_graph.py`).
#[derive(Clone, Debug, PartialEq)]
pub struct BipartiteNode {
    /// The original node or edge id.
    pub id: String,
    /// Layer marker: 0 = node, 1 = hyperedge.
    pub bipartite: u8,
}

/// Build a bipartite petgraph graph: node-layer + edge-layer vertices, one arc per
/// membership (node -> hyperedge).
///
/// XGI parity: `xgi.to_bipartite_graph` (`convert/bipartite_graph.py:110-185`). Vertices
/// are added in node-insertion order first, then edge-insertion order.
pub fn to_bipartite_graph(h: &Hypergraph) -> StableDiGraph<BipartiteNode, ()> {
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    let mut node_idx = IndexMap::new();
    for n in h.node_ids() {
        let idx = g.add_node(BipartiteNode {
            id: n.clone(),
            bipartite: 0,
        });
        node_idx.insert(n, idx);
    }
    for e in h.edge_ids() {
        let e_idx = g.add_node(BipartiteNode {
            id: e.clone(),
            bipartite: 1,
        });
        for m in h.members(&e).unwrap_or_default() {
            g.add_edge(node_idx[&m], e_idx, ());
        }
    }
    g
}

/// Reconstruct a hypergraph from a bipartite petgraph graph.
///
/// XGI parity: `xgi.from_bipartite_graph` (`convert/bipartite_graph.py:9-97`). Validates
/// that every arc joins a node-layer (0) and an edge-layer (1) vertex.
pub fn from_bipartite_graph(
    g: &StableDiGraph<BipartiteNode, ()>,
) -> Result<Hypergraph, ConvertError> {
    // Validate markers.
    for w in g.node_weights() {
        if w.bipartite > 1 {
            return Err(ConvertError::InvalidMarker(w.id.clone()));
        }
    }
    let mut h = Hypergraph::new();
    // Add all node-layer vertices first (so isolated nodes survive).
    for w in g.node_weights() {
        if w.bipartite == 0 {
            h.add_node(&w.id, Value::Null);
        }
    }
    // Each arc: one endpoint node-layer, one edge-layer.
    for edge in g.edge_references() {
        let src = g.node_weight(edge.source()).expect("valid source");
        let dst = g.node_weight(edge.target()).expect("valid target");
        let (node, hyperedge) = match (src.bipartite, dst.bipartite) {
            (0, 1) => (&src.id, &dst.id),
            (1, 0) => (&dst.id, &src.id),
            _ => return Err(ConvertError::NotBipartite),
        };
        let _ = h.add_node_to_edge(hyperedge, node);
    }
    Ok(h)
}

/// The unweighted 1-skeleton (graph projection) of the hypergraph.
///
/// XGI parity: `xgi.to_graph` (`convert/graph.py:8-28`). Two nodes are linked iff they
/// share at least one hyperedge. Node weights are the node ids (node insertion order);
/// duplicate pairs are added once. Built directly from memberships (no linalg dependency).
pub fn to_graph(h: &Hypergraph) -> UnGraph<String, ()> {
    let mut g: UnGraph<String, ()> = UnGraph::new_undirected();
    let mut node_idx = IndexMap::new();
    for n in h.node_ids() {
        let idx = g.add_node(n.clone());
        node_idx.insert(n, idx);
    }
    // Dedup pairs by their (min, max) positional indices — deterministic, no HashMap order.
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a = node_idx[&members[i]];
                let b = node_idx[&members[j]];
                let key = if a.index() <= b.index() {
                    (a.index(), b.index())
                } else {
                    (b.index(), a.index())
                };
                if seen.insert(key) {
                    g.add_edge(a, b, ());
                }
            }
        }
    }
    g
}

/// A copy of the hypergraph keeping only edges of order ≤ `order`.
///
/// XGI parity: `xgi.cut_to_order` (`convert/higher_order_network.py:219-244`). Edge order
/// is `len(members) - 1`. Errors if `order` exceeds the network's maximum edge order.
/// Nodes are preserved even when their edges are cut.
pub fn cut_to_order(h: &Hypergraph, order: i64) -> Result<Hypergraph, ConvertError> {
    let max_order: i64 = h
        .edge_ids()
        .into_iter()
        .map(|e| h.members(&e).map(|m| m.len() as i64 - 1).unwrap_or(0))
        .max()
        .unwrap_or(-1);
    if order > max_order {
        return Err(ConvertError::OrderTooLarge { order, max_order });
    }
    let mut cut = h.copy();
    let to_remove: Vec<String> = h
        .edge_ids()
        .into_iter()
        .filter(|e| h.members(e).map(|m| m.len() as i64 - 1).unwrap_or(0) > order)
        .collect();
    for e in to_remove {
        let _ = cut.remove_edge(&e);
    }
    Ok(cut)
}
