//! Edge statistics (XGI `stats/edgestats.py`).

use indexmap::IndexMap;
use serde_json::Value;

use super::Stat;
use crate::Hypergraph;

/// Count members whose node-degree equals `degree` (helper for degree-filtered order/size).
fn members_with_degree(h: &Hypergraph, edge_id: &str, degree: usize) -> usize {
    h.members(edge_id)
        .unwrap_or_default()
        .iter()
        .filter(|n| h.memberships(n).map(|m| m.len()) == Some(degree))
        .count()
}

/// Edge order: `size - 1`, optionally counting only members of a given node-degree.
/// XGI `edgestats.order`.
pub fn edge_order(h: &Hypergraph, degree: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in h.edge_ids() {
        let val = match degree {
            None => h.members(&eid).map(|m| m.len()).unwrap_or(0).saturating_sub(1),
            Some(d) => members_with_degree(h, &eid, d).saturating_sub(1),
        };
        out.insert(eid, val);
    }
    Stat::new(out)
}

/// Edge size: number of members. XGI `edgestats.size`.
pub fn edge_size(h: &Hypergraph, degree: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in h.edge_ids() {
        let val = match degree {
            None => h.members(&eid).map(|m| m.len()).unwrap_or(0),
            Some(d) => members_with_degree(h, &eid, d),
        };
        out.insert(eid, val);
    }
    Stat::new(out)
}

/// Edge attributes: one attr (Null-imputed) or the whole object. XGI `edgestats.attrs`.
pub fn edge_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<Value> {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for eid in h.edge_ids() {
        let all = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
        let v = match attr {
            None => all,
            Some(a) => all.get(a).cloned().unwrap_or(Value::Null),
        };
        out.insert(eid, v);
    }
    Stat::new(out)
}
