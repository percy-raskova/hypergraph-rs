//! Read/write the HIF standard (XGI `readwrite/hif.py` + `convert/hif_dict.py`).
//! Undirected only; directed HIF requires DiHypergraph (deferred).

use std::path::Path;

use serde_json::{json, Map, Value};

use super::ReadWriteError;
use crate::Hypergraph;

/// True if a serde object is present and non-empty.
fn nonempty_obj(v: &Value) -> bool {
    v.as_object().map(|o| !o.is_empty()).unwrap_or(false)
}

/// Serialize a hypergraph to the HIF dict. XGI `to_hif_dict` (undirected branch).
pub fn to_hif_dict(h: &Hypergraph) -> Value {
    // metadata from graph attrs — the FULL map (XGI: `data["metadata"].update(H._net_attr)`),
    // not just "name" (see the Task 13 core-API note re: `graph_attrs()`).
    let metadata = h.graph_attrs().clone();

    // nodes: isolates OR nodes with attrs
    let mut nodes = Vec::new();
    for nid in h.node_ids() {
        let attrs = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
        let is_isolate = h.memberships(&nid).map(|m| m.is_empty()).unwrap_or(true);
        if is_isolate || nonempty_obj(&attrs) {
            let mut rec = Map::new();
            rec.insert("node".to_string(), json!(nid));
            if nonempty_obj(&attrs) {
                rec.insert("attrs".to_string(), attrs);
            }
            nodes.push(Value::Object(rec));
        }
    }

    // edges: empty edges OR edges with attrs
    let mut edges = Vec::new();
    let mut incidences = Vec::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        let attrs = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
        if members.is_empty() || nonempty_obj(&attrs) {
            let mut rec = Map::new();
            rec.insert("edge".to_string(), json!(eid));
            if nonempty_obj(&attrs) {
                rec.insert("attrs".to_string(), attrs);
            }
            edges.push(Value::Object(rec));
        }
        for node in members {
            incidences.push(json!({"edge": eid, "node": node}));
        }
    }

    json!({
        "metadata": Value::Object(metadata),
        "network-type": "undirected",
        "nodes": nodes,
        "edges": edges,
        "incidences": incidences,
    })
}

/// Rebuild a hypergraph from a HIF dict. XGI `from_hif_dict` (undirected/asc branch).
pub fn from_hif_dict(data: &Value) -> Result<Hypergraph, ReadWriteError> {
    let network_type = data
        .get("network-type")
        .and_then(|v| v.as_str())
        .unwrap_or("undirected");
    if network_type == "directed" {
        return Err(ReadWriteError::Format(
            "directed HIF requires DiHypergraph (not supported in Phase 4)".into(),
        ));
    }

    let mut h = Hypergraph::new();

    if let Some(md) = data.get("metadata").and_then(|v| v.as_object()) {
        for (k, v) in md {
            h.set_graph_attr(k, v.clone());
        }
    }

    let incidences = data
        .get("incidences")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("missing incidences".into()))?;
    for rec in incidences {
        let node = rec
            .get("node")
            .and_then(as_id)
            .ok_or_else(|| ReadWriteError::Format("incidence missing node".into()))?;
        let edge = rec
            .get("edge")
            .and_then(as_id)
            .ok_or_else(|| ReadWriteError::Format("incidence missing edge".into()))?;
        h.add_node_to_edge(&edge, &node)
            .map_err(|e| ReadWriteError::Format(format!("incidence: {e}")))?;
    }

    if let Some(nodes) = data.get("nodes").and_then(|v| v.as_array()) {
        for rec in nodes {
            if let Some(nid) = rec.get("node").and_then(as_id) {
                let attrs = rec.get("attrs").cloned();
                if !h.has_node(&nid) {
                    h.add_node(&nid, attrs.unwrap_or(Value::Null));
                } else if let Some(a) = attrs {
                    // Assign the attrs object wholesale rather than merging via
                    // set_node_attributes: that merge calls `.as_object_mut()`
                    // on the existing (Value::Null, from add_node_to_edge's
                    // default) attrs, which returns None and silently no-ops
                    // (P4-01 fix).
                    if let Some(slot) = h.node_attrs_mut(&nid) {
                        *slot = a;
                    }
                }
            }
        }
    }

    if let Some(edges) = data.get("edges").and_then(|v| v.as_array()) {
        for rec in edges {
            if let Some(eid) = rec.get("edge").and_then(as_id) {
                let attrs = rec.get("attrs").cloned();
                if h.has_edge(&eid) {
                    if let Some(a) = attrs {
                        // Same direct-assignment fix as above (P4-01).
                        if let Some(slot) = h.edge_attrs_mut(&eid) {
                            *slot = a;
                        }
                    }
                }
                // empty edges (in `edges` but no incidences) can't be represented
                // by our core (add_edge rejects empty members) — skipped, documented.
            }
        }
    }

    Ok(h)
}

/// Coerce a JSON id (string or number) to a String key.
fn as_id(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Write a hypergraph to a HIF JSON file. XGI `write_hif`.
pub fn write_hif(h: &Hypergraph, path: &Path) -> Result<(), ReadWriteError> {
    let data = to_hif_dict(h);
    std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

/// Read a hypergraph from a HIF JSON file. XGI `read_hif`.
pub fn read_hif(path: &Path) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&s)?;
    from_hif_dict(&data)
}
