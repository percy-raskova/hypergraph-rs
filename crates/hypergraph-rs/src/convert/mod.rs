//! Conversions to/from hyperedge lists/dicts, bipartite edgelists, bipartite petgraph
//! graphs, the 1-skeleton graph, the standardized hypergraph dict, and order-cutting.
//!
//! Ported from `xgi/convert/`. Operates on the default `Hypergraph`
//! (`serde_json::Value` attrs). The `pandas` slice is NOT ported (spec §4.4); HIF,
//! encapsulation-DAG, line-graph, simplex, and incidence conversions are deferred
//! (see the Phase-6 plan open questions).

use indexmap::IndexMap;
use rustworkx_core::petgraph::graph::UnGraph;
use rustworkx_core::petgraph::stable_graph::StableDiGraph;
use rustworkx_core::petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde_json::Map;
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
        let idx = g.add_node(BipartiteNode { id: n.clone(), bipartite: 0 });
        node_idx.insert(n, idx);
    }
    for e in h.edge_ids() {
        let e_idx = g.add_node(BipartiteNode { id: e.clone(), bipartite: 1 });
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

/// Coerce a node/edge attr value to the shape XGI serializes: a JSON object. XGI's
/// `H.nodes[idx]`/`H.edges[idx]` are always dicts (never `None`), so a default
/// `Value::Null` attr (e.g. from auto-created members) must emit `{}`, not `null`, to
/// match the xgi-data contract.
fn attrs_as_object(v: Option<&Value>) -> Value {
    match v.and_then(|v| v.as_object()) {
        Some(obj) => Value::Object(obj.clone()),
        None => Value::Object(Map::new()),
    }
}

/// Serialize to the standardized xgi-data hypergraph dict.
///
/// XGI parity: `xgi.to_hypergraph_dict` (`convert/hypergraph_dict.py:12-72`). `edge-dict`
/// members are sorted (stringwise). `type` is the constant `"hypergraph"`. `node-data`/
/// `edge-data` entries are always JSON objects (a `Value::Null` attr serializes as `{}`,
/// matching XGI's `H.nodes[idx]` dict-always contract — see `attrs_as_object`).
pub fn to_hypergraph_dict(h: &Hypergraph) -> Value {
    let mut root = Map::new();
    root.insert("type".into(), Value::String("hypergraph".into()));
    root.insert("hypergraph-data".into(), Value::Object(h.graph_attrs().clone()));

    let mut node_data = Map::new();
    for n in h.node_ids() {
        node_data.insert(n.clone(), attrs_as_object(h.node_attrs(&n)));
    }
    root.insert("node-data".into(), Value::Object(node_data));

    let mut edge_data = Map::new();
    let mut edge_dict = Map::new();
    for e in h.edge_ids() {
        edge_data.insert(e.clone(), attrs_as_object(h.edge_attrs(&e)));
        let mut members = h.members(&e).unwrap_or_default();
        members.sort();
        edge_dict.insert(
            e.clone(),
            Value::Array(members.into_iter().map(Value::String).collect()),
        );
    }
    root.insert("edge-data".into(), Value::Object(edge_data));
    root.insert("edge-dict".into(), Value::Object(edge_dict));
    Value::Object(root)
}

/// Deserialize from the standardized xgi-data hypergraph dict.
///
/// XGI parity: `xgi.from_hypergraph_dict` (`convert/hypergraph_dict.py:75-165`). Requires
/// `hypergraph-data`, `node-data`, `edge-dict`; `edge-data` attrs are applied if present.
pub fn from_hypergraph_dict(data: &Value) -> Result<Hypergraph, ConvertError> {
    let obj = data
        .as_object()
        .ok_or_else(|| ConvertError::Malformed("top-level value is not an object".into()))?;

    let mut h = Hypergraph::new();

    let hdata = obj
        .get("hypergraph-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing hypergraph-data".into()))?;
    for (k, v) in hdata {
        h.set_graph_attr(k, v.clone());
    }

    let node_data = obj
        .get("node-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing node-data".into()))?;
    for (nid, attrs) in node_data {
        h.add_node(nid, attrs.clone());
    }

    let edge_dict = obj
        .get("edge-dict")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing edge-dict".into()))?;
    for (eid, members_val) in edge_dict {
        let members: Vec<String> = members_val
            .as_array()
            .ok_or_else(|| ConvertError::Malformed(format!("edge {eid} members not an array")))?
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect();
        if !members.is_empty() {
            h.add_edge(members, Some(eid.clone()), Value::Null)
                .map_err(|e| ConvertError::Malformed(format!("edge {eid}: {e}")))?;
        }
    }

    // Optional edge attributes.
    if let Some(edge_data) = obj.get("edge-data").and_then(|v| v.as_object()) {
        let pairs: Vec<(String, Map<String, Value>)> = edge_data
            .iter()
            .filter(|(eid, _)| h.has_edge(eid))
            .filter_map(|(eid, v)| v.as_object().map(|o| (eid.clone(), o.clone())))
            .collect();
        h.set_edge_attributes(pairs);
    }

    Ok(h)
}
