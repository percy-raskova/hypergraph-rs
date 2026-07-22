//! Node statistics (XGI `stats/nodestats.py`).

use indexmap::IndexMap;
use serde_json::Value;

use super::Stat;
use crate::Hypergraph;

/// Node degree: number of edges a node belongs to, optionally restricted to
/// edges of a given `order` (size `order + 1`). XGI `nodestats.degree` (unweighted).
pub fn node_degree(h: &Hypergraph, order: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in h.node_ids() {
        let memberships = h.memberships(&nid).unwrap_or_default();
        let count = match order {
            None => memberships.len(),
            Some(o) => memberships
                .iter()
                .filter(|e| h.members(e).map(|m| m.len()) == Some(o + 1))
                .count(),
        };
        out.insert(nid, count);
    }
    Stat::new(out)
}

/// Weighted node degree: sum of an edge-attr weight over the node's edges
/// (default weight 1 when the attr is absent/non-numeric). XGI `degree(weight=...)`.
pub fn node_degree_weighted(h: &Hypergraph, weight: &str, order: Option<usize>) -> Stat<f64> {
    let mut out: IndexMap<String, f64> = IndexMap::new();
    for nid in h.node_ids() {
        let memberships = h.memberships(&nid).unwrap_or_default();
        let mut sum = 0.0f64;
        for e in &memberships {
            if let Some(o) = order {
                if h.members(e).map(|m| m.len()) != Some(o + 1) {
                    continue;
                }
            }
            let w = h
                .edge_attrs(e)
                .and_then(|a| a.get(weight))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            sum += w;
        }
        out.insert(nid, sum);
    }
    Stat::new(out)
}

/// Node attributes: one attr (with `Null` imputed when missing) or the whole
/// attr object per node. XGI `nodestats.attrs`.
pub fn node_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<Value> {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for nid in h.node_ids() {
        let all = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
        let v = match attr {
            None => all,
            Some(a) => all.get(a).cloned().unwrap_or(Value::Null),
        };
        out.insert(nid, v);
    }
    Stat::new(out)
}
