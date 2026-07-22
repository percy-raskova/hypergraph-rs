//! JSON read/write (XGI `xgi/readwrite/json.py` port).
//! Also implements `convert/hypergraph_dict.py`'s to/from_hypergraph_dict.

use std::path::Path;

use serde_json::{json, Map, Value};

use super::ReadWriteError;
use crate::Hypergraph;

/// Sort edge members the way XGI's `sorted(H.edges.members(idx))` sorts them:
/// by the members' *native* Python type. Our node ids are always `String`
/// (no int/str distinction survives at the Rust type level), so we
/// approximate: if EVERY member parses as an integer, sort numerically
/// (matching XGI's common case of integer node ids from generators);
/// otherwise fall back to lexicographic (`str`) order, matching XGI's
/// behavior for string node ids or mixed types (FINDING 4(b), verified
/// against the oracle: `xgi.Hypergraph` with all-numeric-string ids sorts
/// numerically; with a mixed numeric/non-numeric id it sorts lexicographically).
fn sort_members_native_type(members: &mut [String]) {
    let all_numeric = members.iter().all(|m| m.parse::<i64>().is_ok());
    if all_numeric {
        members.sort_by_key(|m| m.parse::<i64>().expect("checked by all_numeric"));
    } else {
        members.sort();
    }
}

/// Serialize a hypergraph to the xgi-data dict schema. XGI `to_hypergraph_dict`.
pub fn to_hypergraph_dict(h: &Hypergraph) -> Value {
    let mut node_data = Map::new();
    for nid in h.node_ids() {
        node_data.insert(
            nid.clone(),
            h.node_attrs(&nid).cloned().unwrap_or(Value::Null),
        );
    }
    let mut edge_data = Map::new();
    let mut edge_dict = Map::new();
    for eid in h.edge_ids() {
        edge_data.insert(
            eid.clone(),
            h.edge_attrs(&eid).cloned().unwrap_or(Value::Null),
        );
        let mut members = h.members(&eid).unwrap_or_default();
        sort_members_native_type(&mut members);
        edge_dict.insert(eid.clone(), json!(members));
    }
    // H.graph dict — copy the FULL graph-attr map (XGI:
    // `data["hypergraph-data"].update(H._net_attr)`), not just "name".
    let hgraph_data = h.graph_attrs().clone();

    json!({
        "type": "hypergraph",
        "hypergraph-data": Value::Object(hgraph_data),
        "node-data": Value::Object(node_data),
        "edge-data": Value::Object(edge_data),
        "edge-dict": Value::Object(edge_dict),
    })
}

/// Rebuild a hypergraph from the xgi-data dict schema. XGI `from_hypergraph_dict`.
pub fn from_hypergraph_dict(data: &Value) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();

    if let Some(hd) = data.get("hypergraph-data").and_then(|v| v.as_object()) {
        for (k, v) in hd {
            h.set_graph_attr(k, v.clone());
        }
    }

    let node_data = data
        .get("node-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ReadWriteError::Format("missing node-data".into()))?;
    for (nid, attrs) in node_data {
        h.add_node(nid, attrs.clone());
    }

    let edge_dict = data
        .get("edge-dict")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ReadWriteError::Format("missing edge-dict".into()))?;
    let edge_data = data.get("edge-data").and_then(|v| v.as_object());
    for (eid, members) in edge_dict {
        let arr = members
            .as_array()
            .ok_or_else(|| ReadWriteError::Format(format!("edge {eid} members not an array")))?;
        let member_ids: Vec<String> = arr
            .iter()
            .map(|m| match m {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        // Pass the edge's attrs straight into add_edge rather than seeding
        // Value::Null and patching via set_edge_attributes afterward: that
        // merge path calls `.as_object_mut()` on the existing attrs, which is
        // `None` for `Value::Null` and silently no-ops (P4-01 fix).
        let attrs = edge_data
            .and_then(|ed| ed.get(eid))
            .cloned()
            .unwrap_or(Value::Null);
        h.add_edge(member_ids, Some(eid.clone()), attrs)
            .map_err(|e| ReadWriteError::Format(format!("edge {eid}: {e}")))?;
    }

    Ok(h)
}

/// Write a hypergraph to a pretty-printed JSON file. XGI `write_json`.
pub fn write_json(h: &Hypergraph, path: &Path) -> Result<(), ReadWriteError> {
    let data = to_hypergraph_dict(h);
    let s = serde_json::to_string_pretty(&data)?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Read a hypergraph from a JSON file. XGI `read_json`.
pub fn read_json(path: &Path) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&s)?;
    from_hypergraph_dict(&data)
}
