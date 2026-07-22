//! Read-only global views over a hypergraph.

use std::collections::BTreeSet;

use serde_json::Value;

use super::hypergraph::Hypergraph;

/// A frozen induced sub-hypergraph. Keeps `nodes` (default all) and the `edges`
/// (default all) that are subsets of the kept nodes; optionally drops the
/// resulting isolates. XGI parity: `xgi.subhypergraph(H, nodes, edges,
/// keep_isolates)` (globalviews.py:19-74).
pub fn subhypergraph(
    h: &Hypergraph<Value, Value, Value>,
    nodes: Option<&[String]>,
    edges: Option<&[String]>,
    keep_isolates: bool,
) -> Hypergraph<Value, Value, Value> {
    let node_set: BTreeSet<String> = match nodes {
        None => h.node_ids().into_iter().collect(),
        Some(ns) => ns.iter().filter(|n| h.has_node(n)).cloned().collect(),
    };
    let edge_set: BTreeSet<String> = match edges {
        None => h.edge_ids().into_iter().collect(),
        Some(es) => es.iter().filter(|e| h.has_edge(e)).cloned().collect(),
    };

    let mut new = Hypergraph::new();
    for (k, val) in h.graph_attrs().clone() {
        new.set_graph_attr(&k, val);
    }
    // Nodes in insertion order.
    for nid in h.node_ids() {
        if node_set.contains(&nid) {
            let attrs = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
            new.add_node(&nid, attrs);
        }
    }
    // Edges that are subsets of the kept node set, in insertion order.
    for eid in h.edge_ids() {
        if !edge_set.contains(&eid) {
            continue;
        }
        let members = h.members(&eid).unwrap_or_default();
        // D1-ripple: an empty edge is a trivial subset of any node set and is
        // retained (matches XGI subhypergraph, which does not special-case size-0).
        if members.iter().all(|m| node_set.contains(m)) {
            let attrs = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            let _ = new.add_edge(members, Some(eid.clone()), attrs);
        }
    }

    if !keep_isolates {
        let isolates = new.isolate_node_ids();
        new.remove_nodes_from(&isolates);
    }
    new.freeze();
    new
}
