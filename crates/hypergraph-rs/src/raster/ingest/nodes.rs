//! Node ingest: agent extraction, dense-index assignment, contract defaults
//! and typed errors (Phase I, Task 2). See the [`super`] module doc for the
//! normative attribute contract this file implements.

use std::collections::HashMap;

use serde_json::Value;

use crate::raster::deck::Agent;
use crate::Hypergraph;

use super::{AttrContract, IngestError, TensionSource, DEFAULT_BLOC};

/// Build a `key -> dense index` lookup over `ids`, for `edges.rs` (Task 3)
/// to resolve edge members without re-scanning `node_ids()`. Internal only:
/// its `HashMap` iteration order must never reach an output `Vec` (Global
/// Constraints — determinism).
///
/// `#[allow(dead_code)]`: unused until Task 3's `ingest_edges` consumes it —
/// this task (Task 2) only defines it, per the plan's stated interface.
#[allow(dead_code)]
pub(crate) fn index_of(ids: &[String]) -> HashMap<&str, usize> {
    let mut out = HashMap::with_capacity(ids.len());
    // Bounded: one pass over `ids` (node count).
    for (i, id) in ids.iter().enumerate() {
        out.insert(id.as_str(), i);
    }
    out
}

/// Read a required, finite `f64` attribute. `key` absent ⇒
/// [`IngestError::MissingNodeAttr`]; present but not a finite number ⇒
/// [`IngestError::BadAttrType`].
fn read_required_f64(attrs: &Value, key: &str, owner: &str) -> Result<f64, IngestError> {
    let v = attrs.get(key).ok_or_else(|| IngestError::MissingNodeAttr {
        node: owner.to_string(),
        key: key.to_string(),
    })?;
    let f = v.as_f64().ok_or_else(|| IngestError::BadAttrType {
        owner: owner.to_string(),
        key: key.to_string(),
    })?;
    if !f.is_finite() {
        return Err(IngestError::BadAttrType {
            owner: owner.to_string(),
            key: key.to_string(),
        });
    }
    Ok(f)
}

/// Read `bloc`: absent ⇒ [`DEFAULT_BLOC`] (and the caller counts it);
/// present ⇒ must be a `u8` in `0..=2`, else [`IngestError::BadAttrType`].
fn read_bloc(
    attrs: &Value,
    key: &str,
    owner: &str,
    defaulted: &mut usize,
) -> Result<u8, IngestError> {
    match attrs.get(key) {
        None => {
            *defaulted += 1;
            Ok(DEFAULT_BLOC)
        }
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| IngestError::BadAttrType {
                owner: owner.to_string(),
                key: key.to_string(),
            })?;
            if n > 2 {
                return Err(IngestError::BadAttrType {
                    owner: owner.to_string(),
                    key: key.to_string(),
                });
            }
            Ok(n as u8)
        }
    }
}

/// Read the optional `h3_cell`: absent ⇒ `None`; a `u64`-shaped JSON number
/// or a decimal string ⇒ `Some(u64)` (a decimal string is required because
/// JSON numbers lose precision above 2^53 and H3 indices exceed it); any
/// other shape ⇒ [`IngestError::BadAttrType`].
fn read_h3_cell(attrs: &Value, key: &str, owner: &str) -> Result<Option<u64>, IngestError> {
    match attrs.get(key) {
        None => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| IngestError::BadAttrType {
                owner: owner.to_string(),
                key: key.to_string(),
            }),
        Some(Value::String(s)) => {
            s.parse::<u64>()
                .map(Some)
                .map_err(|_| IngestError::BadAttrType {
                    owner: owner.to_string(),
                    key: key.to_string(),
                })
        }
        Some(_) => Err(IngestError::BadAttrType {
            owner: owner.to_string(),
            key: key.to_string(),
        }),
    }
}

/// Result of [`ingest_nodes`]: agents in `node_ids()` order; the parallel
/// original-id vector; the parallel optional H3 cells; the count of nodes
/// whose `bloc` defaulted; and which formula produced `subsistence`.
///
/// A named struct rather than a 5-tuple — the Task 9 clippy leg's preferred
/// fix for `clippy::type_complexity` over an `#[allow]` (the `IngestedEdges`
/// precedent in `edges.rs` follows the same shape).
#[derive(Debug, Clone, PartialEq)]
pub struct IngestedNodes {
    pub agents: Vec<Agent>,
    pub node_ids: Vec<String>,
    pub h3_cells: Vec<Option<u64>>,
    pub bloc_defaulted: usize,
    pub tension_source: TensionSource,
}

/// Ingest one node per agent, in [`Hypergraph::node_ids`] order.
///
/// `repression` is the graph-level repression scalar (read from graph attrs
/// by `to_world`, Task 4) — needed here only to invert a supplied `tension`
/// attr into a synthetic `subsistence` per the D-I1 algebra (module doc,
/// "tension" row). **Deviation from the plan's abbreviated interface
/// sketch**, which omitted this parameter: the inversion `subsistence' =
/// wealth + t/repression` is not computable without it, and no other
/// signature lets `to_world` supply it before agents exist. Recorded in the
/// executor's deviations, not invented silently.
///
/// Bounded: two passes over `h.node_ids()` (node count) — pass 1 detects
/// whether *any* node carries the tension key (which puts every node under
/// the same, stricter, obligation), pass 2 builds the agents.
pub fn ingest_nodes(
    h: &Hypergraph<Value, Value, Value>,
    c: &AttrContract,
    repression: f64,
) -> Result<IngestedNodes, IngestError> {
    let ids = h.node_ids();
    static EMPTY: Value = Value::Null;

    // Pass 1: does any node carry the tension key? If so every node must
    // (a half-populated field is the silent-mixing failure the contract
    // forbids).
    let mut any_tension = false;
    for id in &ids {
        let attrs = h.node_attrs(id).unwrap_or(&EMPTY);
        if attrs.get(&c.tension).is_some() {
            any_tension = true;
            break;
        }
    }
    let tension_source = if any_tension {
        TensionSource::Attr
    } else {
        TensionSource::Derived
    };

    let mut agents = Vec::with_capacity(ids.len());
    let mut h3_cells = Vec::with_capacity(ids.len());
    let mut bloc_defaulted = 0usize;

    // Pass 2: build each agent.
    for (idx, id) in ids.iter().enumerate() {
        let attrs = h.node_attrs(id).unwrap_or(&EMPTY);

        let wealth = read_required_f64(attrs, &c.wealth, id)?;

        let subsistence = if any_tension {
            let t = read_required_f64(attrs, &c.tension, id)?;
            if t < 0.0 {
                return Err(IngestError::BadAttrType {
                    owner: id.clone(),
                    key: c.tension.clone(),
                });
            }
            if repression.is_finite() && repression > 0.0 {
                wealth + t / repression
            } else {
                // `repression` is invalid; `to_world`'s own precondition
                // check rejects this graph before the value below is ever
                // observed. Fall back to a finite, non-fabricated value
                // rather than dividing by a non-positive number.
                wealth
            }
        } else {
            read_required_f64(attrs, &c.subsistence, id)?
        };

        let bloc = read_bloc(attrs, &c.bloc, id, &mut bloc_defaulted)?;
        let h3 = read_h3_cell(attrs, &c.h3_cell, id)?;

        agents.push(Agent {
            id: idx,
            bloc,
            wealth,
            subsistence,
        });
        h3_cells.push(h3);
    }

    Ok(IngestedNodes {
        agents,
        node_ids: ids,
        h3_cells,
        bloc_defaulted,
        tension_source,
    })
}
