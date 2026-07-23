//! Edge ingest: `kind` dispatch into `Community` / `Extraction` / `Industry`
//! (Phase I, Task 3). See the [`super`] module doc for the normative
//! attribute contract this file implements.

use std::collections::HashMap;

use serde_json::Value;

use crate::raster::algebra::Industry;
use crate::raster::deck::{Community, Extraction};
use crate::Hypergraph;

use super::{
    AttrContract, IngestError, DEFAULT_COMMUNITY_NAME, DEFAULT_LANE, DEFAULT_MASS, KIND_COMMUNITY,
    KIND_EXTRACTION, KIND_INDUSTRY,
};

/// The three edge collections ingest can produce from one `Hypergraph`, plus
/// the edges it could not classify and whether any industry's `lane`
/// defaulted.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestedEdges {
    pub communities: Vec<Community>,
    pub extraction: Vec<Extraction>,
    pub industries: Vec<Industry>,
    /// Edge ids whose `kind` was missing or unrecognised — reported, never
    /// an error (a real hypergraph may carry edges the instruments have no
    /// use for).
    pub skipped: Vec<String>,
    pub lane_defaulted: bool,
}

/// Read a required, finite `f64` edge attribute. `key` absent ⇒
/// [`IngestError::MissingEdgeAttr`]; present but not a finite number ⇒
/// [`IngestError::BadAttrType`].
fn read_required_edge_f64(
    attrs: &Value,
    key: &str,
    edge: &str,
    kind: &str,
) -> Result<f64, IngestError> {
    let v = attrs.get(key).ok_or_else(|| IngestError::MissingEdgeAttr {
        edge: edge.to_string(),
        kind: kind.to_string(),
        key: key.to_string(),
    })?;
    let f = v.as_f64().ok_or_else(|| IngestError::BadAttrType {
        owner: edge.to_string(),
        key: key.to_string(),
    })?;
    if !f.is_finite() {
        return Err(IngestError::BadAttrType {
            owner: edge.to_string(),
            key: key.to_string(),
        });
    }
    Ok(f)
}

/// Read an optional, finite `f64` edge attribute: absent ⇒ `default`;
/// present but not a finite number ⇒ [`IngestError::BadAttrType`].
fn read_optional_edge_f64(
    attrs: &Value,
    key: &str,
    edge: &str,
    default: f64,
) -> Result<f64, IngestError> {
    match attrs.get(key) {
        None => Ok(default),
        Some(v) => {
            let f = v.as_f64().ok_or_else(|| IngestError::BadAttrType {
                owner: edge.to_string(),
                key: key.to_string(),
            })?;
            if !f.is_finite() {
                return Err(IngestError::BadAttrType {
                    owner: edge.to_string(),
                    key: key.to_string(),
                });
            }
            Ok(f)
        }
    }
}

/// Read an optional string edge attribute: absent ⇒ `None`; present but not
/// a string ⇒ [`IngestError::BadAttrType`].
fn read_optional_str<'a>(
    attrs: &'a Value,
    key: &str,
    edge: &str,
) -> Result<Option<&'a str>, IngestError> {
    match attrs.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(Some)
            .ok_or_else(|| IngestError::BadAttrType {
                owner: edge.to_string(),
                key: key.to_string(),
            }),
    }
}

/// Read the required `head` string attribute for an extraction edge.
fn read_head(attrs: &Value, key: &str, edge: &str) -> Result<String, IngestError> {
    let v = attrs.get(key).ok_or_else(|| IngestError::MissingEdgeAttr {
        edge: edge.to_string(),
        kind: KIND_EXTRACTION.to_string(),
        key: key.to_string(),
    })?;
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| IngestError::BadAttrType {
            owner: edge.to_string(),
            key: key.to_string(),
        })
}

/// Read `lane`: absent ⇒ [`DEFAULT_LANE`] (and the caller flags
/// `lane_defaulted`); present ⇒ must be a `u8` in `0..=3`, else
/// [`IngestError::BadAttrType`].
fn read_lane(
    attrs: &Value,
    key: &str,
    edge: &str,
    lane_defaulted: &mut bool,
) -> Result<u8, IngestError> {
    match attrs.get(key) {
        None => {
            *lane_defaulted = true;
            Ok(DEFAULT_LANE)
        }
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| IngestError::BadAttrType {
                owner: edge.to_string(),
                key: key.to_string(),
            })?;
            if n > 3 {
                return Err(IngestError::BadAttrType {
                    owner: edge.to_string(),
                    key: key.to_string(),
                });
            }
            Ok(n as u8)
        }
    }
}

/// Resolve a node id string to its dense render-space index, or
/// [`IngestError::UnknownNode`] if it is not in the hypergraph. This cannot
/// normally trigger for edge members (`add_edge` auto-creates them), but the
/// check is cheap and the alternative is an out-of-bounds index downstream.
fn resolve(index: &HashMap<&str, usize>, edge: &str, node: &str) -> Result<usize, IngestError> {
    index
        .get(node)
        .copied()
        .ok_or_else(|| IngestError::UnknownNode {
            edge: edge.to_string(),
            node: node.to_string(),
        })
}

/// Read `links`: absent ⇒ `[]`; present ⇒ must be an array of 2-element
/// arrays of node-id strings, each drawn from `members` (this edge's own
/// members), else [`IngestError::LinkOutsideCommunity`]. Any other JSON
/// shape ⇒ [`IngestError::BadAttrType`]. Bounded by the number of links
/// present.
fn read_links(
    attrs: &Value,
    key: &str,
    edge: &str,
    members: &[String],
    index: &HashMap<&str, usize>,
) -> Result<Vec<(usize, usize)>, IngestError> {
    let raw = match attrs.get(key) {
        None => return Ok(Vec::new()),
        Some(v) => v,
    };
    let arr = raw.as_array().ok_or_else(|| IngestError::BadAttrType {
        owner: edge.to_string(),
        key: key.to_string(),
    })?;
    let mut links = Vec::with_capacity(arr.len());
    for item in arr {
        let pair = item.as_array().ok_or_else(|| IngestError::BadAttrType {
            owner: edge.to_string(),
            key: key.to_string(),
        })?;
        if pair.len() != 2 {
            return Err(IngestError::BadAttrType {
                owner: edge.to_string(),
                key: key.to_string(),
            });
        }
        let a = pair[0].as_str().ok_or_else(|| IngestError::BadAttrType {
            owner: edge.to_string(),
            key: key.to_string(),
        })?;
        let b = pair[1].as_str().ok_or_else(|| IngestError::BadAttrType {
            owner: edge.to_string(),
            key: key.to_string(),
        })?;
        for node in [a, b] {
            if !members.iter().any(|m| m == node) {
                return Err(IngestError::LinkOutsideCommunity {
                    edge: edge.to_string(),
                    node: node.to_string(),
                });
            }
        }
        links.push((resolve(index, edge, a)?, resolve(index, edge, b)?));
    }
    Ok(links)
}

/// Ingest every edge into `communities` / `extraction` / `industries`,
/// dispatched on `kind`; unrecognised or missing `kind` is skipped and
/// reported, never an error. Order within each output vec follows
/// [`Hypergraph::edge_ids`] relative order — load-bearing for `industries`
/// (`algebra::transform` and `build_sankey` iterate it in slice order).
///
/// Bounded: one pass over `h.edge_ids()` (edge count); each edge's own
/// `members`/`links` loops are bounded by that edge's member/link count.
///
/// **Deviation from the plan's `pub(crate)` sketch**, matching Task 2's
/// `ingest_nodes` precedent: `tests/test_ingest.rs` is an external
/// integration-test crate, which cannot name a `pub(crate)` type or call a
/// `pub(crate)` function, so both `ingest_edges` and [`IngestedEdges`] are
/// `pub` (re-exported from [`super`]) exactly as `ingest_nodes` already is.
pub fn ingest_edges(
    h: &Hypergraph<Value, Value, Value>,
    c: &AttrContract,
    index: &HashMap<&str, usize>,
) -> Result<IngestedEdges, IngestError> {
    let edge_ids = h.edge_ids();
    static EMPTY: Value = Value::Null;

    let mut communities = Vec::new();
    let mut extraction = Vec::new();
    let mut industries = Vec::new();
    let mut skipped = Vec::new();
    let mut lane_defaulted = false;

    for edge in &edge_ids {
        let attrs = h.edge_attrs(edge).unwrap_or(&EMPTY);
        let members = h.members(edge).unwrap_or_default();

        let kind = match attrs.get(&c.kind).and_then(Value::as_str) {
            Some(k) => k,
            None => {
                skipped.push(edge.clone());
                continue;
            }
        };

        match kind {
            KIND_COMMUNITY => {
                let name = read_optional_str(attrs, &c.name, edge)?
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{DEFAULT_COMMUNITY_NAME}{edge}"));
                let links = read_links(attrs, &c.links, edge, &members, index)?;
                let mut member_idxs = Vec::with_capacity(members.len());
                for m in &members {
                    member_idxs.push(resolve(index, edge, m)?);
                }
                communities.push(Community {
                    name,
                    members: member_idxs,
                    links,
                });
            }
            KIND_EXTRACTION => {
                let head = read_head(attrs, &c.head, edge)?;
                if !members.iter().any(|m| m == &head) {
                    return Err(IngestError::HeadNotMember {
                        edge: edge.clone(),
                        node: head,
                    });
                }
                let mass = read_optional_edge_f64(attrs, &c.mass, edge, DEFAULT_MASS)?;
                let head_idx = resolve(index, edge, &head)?;
                let mut tails = Vec::with_capacity(members.len());
                for m in &members {
                    if m == &head {
                        continue;
                    }
                    tails.push(resolve(index, edge, m)?);
                }
                extraction.push(Extraction {
                    tails,
                    head: head_idx,
                    mass,
                });
            }
            KIND_INDUSTRY => {
                let name = read_optional_str(attrs, &c.name, edge)?
                    .map(str::to_string)
                    .unwrap_or_else(|| edge.clone());
                let cval = read_required_edge_f64(attrs, &c.c, edge, KIND_INDUSTRY)?;
                let v = read_required_edge_f64(attrs, &c.v, edge, KIND_INDUSTRY)?;
                let e = read_required_edge_f64(attrs, &c.e, edge, KIND_INDUSTRY)?;
                if v == 0.0 {
                    return Err(IngestError::DegenerateIndustry { edge: edge.clone() });
                }
                let lane = read_lane(attrs, &c.lane, edge, &mut lane_defaulted)?;
                industries.push(Industry {
                    name,
                    lane,
                    c: cval,
                    v,
                    e,
                });
            }
            _ => {
                skipped.push(edge.clone());
            }
        }
    }

    Ok(IngestedEdges {
        communities,
        extraction,
        industries,
        skipped,
        lane_defaulted,
    })
}
