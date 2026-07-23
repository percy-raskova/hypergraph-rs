//! Attribute contract + typed ingest errors for real-`Hypergraph` topology
//! ingest (Phase I, Task 1). This module doc is the **canonical, normative**
//! home of the attribute contract; `plans/phase-I-topology-ingest.md` is its
//! rationale, not its spec.
//!
//! `ATTR_CONTRACT_VERSION` is bumped whenever a key's *meaning*, default, or
//! error disposition changes — never for adding a new optional key (D-I3).
//!
//! ## Node attrs (one node per agent)
//!
//! | Contract field | Default key | Type | Missing ⇒ | Rationale |
//! |---|---|---|---|---|
//! | `identity` | *(the node id itself)* | `String` | n/a | The `Hypergraph` node id **is** the agent identity (D7 stringly-typed boundary). Never read from attrs. |
//! | `wealth` | `"wealth"` | finite `f64` | [`IngestError::MissingNodeAttr`] | Feeds `agent_tension` and `psa`. No honest default exists. |
//! | `subsistence` | `"subsistence"` | finite `f64` | [`IngestError::MissingNodeAttr`] | Same. |
//! | `bloc` | `"bloc"` | `u8` in `0..=2` | [`DEFAULT_BLOC`] | Bloc selects a marker colour and the periphery filter. `1` is the neutral unclassified middle bloc. With ≥2 communities, at least one node must carry `bloc == PERIPHERY_BLOC (2)` or ingest errors. |
//! | `tension` | `"tension"` | finite `f64 ≥ 0` | absent ⇒ derive | Optional override. When present, used verbatim as the terrain's per-agent scalar (via a `subsistence` inversion); when absent the frozen `agent_tension(a, repression)` formula applies. Recorded in [`IngestReport::tension_source`]. |
//! | `h3_cell` | `"h3_cell"` | `u64` or decimal `String` | absent ⇒ `None` | Optional, carried not consumed. Stored in [`IngestReport::h3_cells`]; no instrument reads it. A decimal string is accepted because JSON numbers lose precision above 2^53 and H3 ids exceed it. |
//!
//! ## Edge attrs (edge `kind` dispatches the shape)
//!
//! `kind` default key `"kind"`. An edge whose `kind` is missing or
//! unrecognised is **skipped** and counted in [`IngestReport::skipped_edges`]
//! — never a silent success, never an error.
//!
//! | `kind` | Extra keys | Missing ⇒ | Maps to |
//! |---|---|---|---|
//! | `"community"` | `name` (default [`DEFAULT_COMMUNITY_NAME`] + edge id), `links` (array of 2-element id arrays, default `[]`) | name defaults; `links` defaults to empty | `Community { name, members = edge members, links }`. Any link id not among this edge's own members ⇒ [`IngestError::LinkOutsideCommunity`]. |
//! | `"extraction"` | `head` (node id, **required**), `mass` (default [`DEFAULT_MASS`]) | missing `head` ⇒ [`IngestError::MissingEdgeAttr`] | `Extraction { tails = members minus head, head, mass }`. `head` not among members ⇒ [`IngestError::HeadNotMember`]. `mass` defaults because no instrument reads it. |
//! | `"industry"` | `name` (default = edge id), `c`, `v`, `e` (finite `f64`, **all required**), `lane` (`u8` in `0..=3`) | `c`/`v`/`e` missing ⇒ [`IngestError::MissingEdgeAttr`]; `lane` missing ⇒ [`DEFAULT_LANE`] and [`IngestReport::lane_defaulted`] set | `Industry { name, lane, c, v, e }`. `v == 0.0` ⇒ [`IngestError::DegenerateIndustry`]. |
//!
//! ## Graph attrs
//!
//! | Contract field | Default key | Missing ⇒ | Notes |
//! |---|---|---|---|
//! | `repression` | `"repression"` | [`DEFAULT_REPRESSION`] | Scales `agent_tension` and appears in `psr`'s denominator. Must be finite and `> 0.0`, else [`IngestError::BadGraphAttr`]. |
//! | `zoom` | `"zoom"` | [`DEFAULT_ZOOM`] (fine) | `instruments::build` maps `zoom == 1` → coarse, else fine. |
//!
//! `seed` and `tick` are **not** in the contract: they are set to `0` on the
//! produced `DeckWorld` (documented sentinels, never read by any
//! instrument); real provenance lives on [`IngestReport`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::raster::deck::DeckWorld;
#[cfg(feature = "cells3d")]
use crate::raster::instruments::{self, Instrument, InstrumentOpts};
#[cfg(feature = "cells3d")]
use crate::raster::SceneGraph3D;
use crate::Hypergraph;

mod edges;
mod nodes;

pub use edges::{ingest_edges, IngestedEdges};
pub(crate) use nodes::index_of;
pub use nodes::{ingest_nodes, IngestedNodes};

/// Version of the attribute contract's key spellings, defaults, and error
/// dispositions. Bumped only on a meaning-changing edit (D-I3).
pub const ATTR_CONTRACT_VERSION: u32 = 1;

pub const K_WEALTH: &str = "wealth";
pub const K_SUBSISTENCE: &str = "subsistence";
pub const K_BLOC: &str = "bloc";
pub const K_TENSION: &str = "tension";
pub const K_H3_CELL: &str = "h3_cell";
pub const K_KIND: &str = "kind";
pub const K_NAME: &str = "name";
pub const K_LINKS: &str = "links";
pub const K_HEAD: &str = "head";
pub const K_MASS: &str = "mass";
pub const K_LANE: &str = "lane";
pub const K_C: &str = "c";
pub const K_V: &str = "v";
pub const K_E: &str = "e";
pub const K_REPRESSION: &str = "repression";
pub const K_ZOOM: &str = "zoom";

pub const KIND_COMMUNITY: &str = "community";
pub const KIND_EXTRACTION: &str = "extraction";
pub const KIND_INDUSTRY: &str = "industry";

/// Default bloc for a node with no `bloc` attr: the neutral, unclassified
/// middle bloc — never "core" (0) or "periphery" (2).
pub const DEFAULT_BLOC: u8 = 1;
/// The bloc read by `periphery_mean_wealth` (`deck.rs:416-419`).
pub const PERIPHERY_BLOC: u8 = 2;
/// Default lane for an industry edge with no `lane` attr. Babylon has no
/// per-sector lane classification (see module doc); every unlabelled
/// industry lands here and [`IngestReport::lane_defaulted`] is raised.
pub const DEFAULT_LANE: u8 = 0;
/// Default `mass` for an extraction edge: safe because no instrument reads
/// `mass` (only `do_tick` does, which Phase I never calls).
pub const DEFAULT_MASS: f64 = 1.0;
/// Default `repression`: the identity scaling, the one value that adds no
/// information.
pub const DEFAULT_REPRESSION: f64 = 1.0;
/// Default `zoom`: fine (un-aggregated).
pub const DEFAULT_ZOOM: u8 = 0;
/// Prefix used to synthesize a community name when `name` is absent; the
/// edge id is appended.
pub const DEFAULT_COMMUNITY_NAME: &str = "community-";

/// Named spellings for every contract-governed attribute key, plus the
/// contract version. Owned `String`s so a caller may rename any key
/// (e.g. to match babylon's own vocabulary) without forking the crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttrContract {
    pub version: u32,
    pub wealth: String,
    pub subsistence: String,
    pub bloc: String,
    pub tension: String,
    pub h3_cell: String,
    pub kind: String,
    pub name: String,
    pub links: String,
    pub head: String,
    pub mass: String,
    pub lane: String,
    pub c: String,
    pub v: String,
    pub e: String,
    pub repression: String,
    pub zoom: String,
}

impl Default for AttrContract {
    fn default() -> Self {
        Self {
            version: ATTR_CONTRACT_VERSION,
            wealth: K_WEALTH.to_string(),
            subsistence: K_SUBSISTENCE.to_string(),
            bloc: K_BLOC.to_string(),
            tension: K_TENSION.to_string(),
            h3_cell: K_H3_CELL.to_string(),
            kind: K_KIND.to_string(),
            name: K_NAME.to_string(),
            links: K_LINKS.to_string(),
            head: K_HEAD.to_string(),
            mass: K_MASS.to_string(),
            lane: K_LANE.to_string(),
            c: K_C.to_string(),
            v: K_V.to_string(),
            e: K_E.to_string(),
            repression: K_REPRESSION.to_string(),
            zoom: K_ZOOM.to_string(),
        }
    }
}

/// Which formula produced a `DeckWorld`'s per-agent tension: the frozen
/// `agent_tension` derivation, or a caller-supplied `tension` attr (inverted
/// into a synthetic `subsistence` — see [`crate::raster::ingest`] module
/// doc, D-I1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensionSource {
    Derived,
    Attr,
}

/// Provenance and every default applied while ingesting one `Hypergraph`:
/// what was read, what was defaulted, and how a rendered node maps back to
/// the caller's own string id.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestReport {
    pub contract_version: u32,
    /// Dense render-space index -> original `Hypergraph` node id.
    pub node_ids: Vec<String>,
    pub agent_count: usize,
    pub community_count: usize,
    pub extraction_count: usize,
    pub industry_count: usize,
    /// Edge ids whose `kind` was missing or unrecognised.
    pub skipped_edges: Vec<String>,
    pub bloc_defaulted: usize,
    pub lane_defaulted: bool,
    pub repression_defaulted: bool,
    pub tension_source: TensionSource,
    /// Parallel to `node_ids`.
    pub h3_cells: Vec<Option<u64>>,
}

/// Every way ingest can fail. Each variant carries the offending node/edge
/// id and key so a caller can find the exact place their graph disagrees
/// with the contract (Power-of-10 rule 4: no catch-all).
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum IngestError {
    #[error("node `{node}` is missing required attribute `{key}`")]
    MissingNodeAttr { node: String, key: String },
    #[error("edge `{edge}` (kind `{kind}`) is missing required attribute `{key}`")]
    MissingEdgeAttr {
        edge: String,
        kind: String,
        key: String,
    },
    #[error("attribute `{key}` on `{owner}` has the wrong shape or is not finite")]
    BadAttrType { owner: String, key: String },
    #[error("graph attribute `{key}` is invalid: {reason}")]
    BadGraphAttr { key: String, reason: String },
    #[error("community edge `{edge}` links node `{node}`, which is not one of its members")]
    LinkOutsideCommunity { edge: String, node: String },
    #[error("extraction edge `{edge}` names head `{node}`, which is not one of its members")]
    HeadNotMember { edge: String, node: String },
    #[error("industry edge `{edge}` has v == 0 (rate of exploitation would divide by zero)")]
    DegenerateIndustry { edge: String },
    #[error("edge `{edge}` references node `{node}`, which is not in the hypergraph")]
    UnknownNode { edge: String, node: String },
    #[error("at least 2 agent nodes are required for a spectral embedding, found {found}")]
    TooFewAgents { found: usize },
    #[error(
        "a world with {communities} communities computes psa over the periphery bloc ({bloc}), but no node carries it"
    )]
    NoPeripheryBloc { communities: usize, bloc: u8 },
}

/// Read the `repression` graph attr: absent ⇒ [`DEFAULT_REPRESSION`] (and
/// `defaulted` is set); present ⇒ must be a finite `f64` (its `> 0.0`
/// requirement is checked by the caller, since the message differs from a
/// bad-shape message). Bad shape ⇒ [`IngestError::BadGraphAttr`].
fn read_repression(
    h: &Hypergraph<Value, Value, Value>,
    key: &str,
    defaulted: &mut bool,
) -> Result<f64, IngestError> {
    match h.graph_attr(key) {
        None => {
            *defaulted = true;
            Ok(DEFAULT_REPRESSION)
        }
        Some(v) => v.as_f64().ok_or_else(|| IngestError::BadGraphAttr {
            key: key.to_string(),
            reason: "must be a finite number".to_string(),
        }),
    }
}

/// Read the `zoom` graph attr: absent ⇒ [`DEFAULT_ZOOM`]; present ⇒ must be
/// a `u8`, else [`IngestError::BadGraphAttr`].
fn read_zoom(h: &Hypergraph<Value, Value, Value>, key: &str) -> Result<u8, IngestError> {
    match h.graph_attr(key) {
        None => Ok(DEFAULT_ZOOM),
        Some(v) => {
            let n = v.as_u64().ok_or_else(|| IngestError::BadGraphAttr {
                key: key.to_string(),
                reason: "must be an unsigned integer".to_string(),
            })?;
            u8::try_from(n).map_err(|_| IngestError::BadGraphAttr {
                key: key.to_string(),
                reason: "must fit in a u8".to_string(),
            })
        }
    }
}

/// Ingest a real `Hypergraph` into a `DeckWorld` the frozen render path
/// (`instruments::build` → `rasterize` / `render_pixels`) can consume
/// unchanged, per `c`'s attribute contract. Pure and deterministic: two
/// calls on the same `(h, c)` produce equal outputs (Global Constraints —
/// determinism).
///
/// Validates the three CRITICAL PRECONDITIONS (module doc, "Global
/// Constraints" / plan's "CRITICAL PRECONDITION FINDING") before returning,
/// as a straight-line sequence of explicit checks — no combined
/// "invalid world" catch-all (Power-of-10 rule 4):
///
/// 1. `agents.len() >= 2`, else [`IngestError::TooFewAgents`] (the spectral
///    embedding's `smallest_k_eigenvectors` requirement).
/// 2. If `communities.len() >= 2` and no agent carries [`PERIPHERY_BLOC`],
///    [`IngestError::NoPeripheryBloc`] — the `0.0/0.0` route in
///    `periphery_mean_wealth` (`deck.rs:416-419`).
/// 3. `repression` finite and `> 0.0`, else [`IngestError::BadGraphAttr`]
///    (it divides in `psr`, `deck.rs:445`).
pub fn to_world(
    h: &Hypergraph<Value, Value, Value>,
    c: &AttrContract,
) -> Result<(DeckWorld, IngestReport), IngestError> {
    let mut repression_defaulted = false;
    let repression = read_repression(h, &c.repression, &mut repression_defaulted)?;
    if !repression.is_finite() || repression <= 0.0 {
        return Err(IngestError::BadGraphAttr {
            key: c.repression.clone(),
            reason: "must be finite and > 0.0".to_string(),
        });
    }
    let zoom = read_zoom(h, &c.zoom)?;

    let IngestedNodes {
        agents,
        node_ids,
        h3_cells,
        bloc_defaulted,
        tension_source,
    } = ingest_nodes(h, c, repression)?;

    if agents.len() < 2 {
        return Err(IngestError::TooFewAgents {
            found: agents.len(),
        });
    }

    let index = index_of(&node_ids);
    let edges = ingest_edges(h, c, &index)?;

    if edges.communities.len() >= 2 && !agents.iter().any(|a| a.bloc == PERIPHERY_BLOC) {
        return Err(IngestError::NoPeripheryBloc {
            communities: edges.communities.len(),
            bloc: PERIPHERY_BLOC,
        });
    }

    let world = DeckWorld {
        seed: 0,
        agents,
        communities: edges.communities,
        extraction: edges.extraction,
        industries: edges.industries,
        repression,
        tick: 0,
        zoom,
    };

    let report = IngestReport {
        contract_version: c.version,
        agent_count: world.agents.len(),
        community_count: world.communities.len(),
        extraction_count: world.extraction.len(),
        industry_count: world.industries.len(),
        skipped_edges: edges.skipped,
        bloc_defaulted,
        lane_defaulted: edges.lane_defaulted,
        repression_defaulted,
        tension_source,
        node_ids,
        h3_cells,
    };

    Ok((world, report))
}

/// Ingest `h` per `c`, then dispatch to the **untouched**
/// `instruments::build` (Task 5, graceful degradation). This is a thin
/// wrapper: it never edits, wraps, or reimplements a builder — a change
/// here can never be the reason a builder's byte-identity guarantee (Task
/// 6) breaks.
///
/// Degradation policy (exhaustive, no case left to judgement):
///
/// - Any [`IngestError`] from `to_world` ⇒ `Err` — the caller learns which
///   node/edge and which key.
/// - [`Instrument::Sankey`] with zero industry edges ⇒
///   `Ok((SceneGraph3D::empty(), report))`. `algebra::transform(&[])`
///   computes `0.0/0.0 = NaN`, which would otherwise reach the glyph
///   banner; an empty scene is the honest rendering of "no industries".
/// - Every other `(instrument, world)` pair ⇒ `Ok((instruments::build(..),
///   report))` — including zero-community `Cylinder`/`Terrain`, which is a
///   legitimate degraded-but-real scene handled by the builders themselves.
#[cfg(feature = "cells3d")]
pub fn build_scene(
    h: &Hypergraph<Value, Value, Value>,
    c: &AttrContract,
    inst: Instrument,
    opts: &InstrumentOpts,
) -> Result<(SceneGraph3D, IngestReport), IngestError> {
    let (world, report) = to_world(h, c)?;

    if inst == Instrument::Sankey && world.industries.is_empty() {
        return Ok((SceneGraph3D::empty(), report));
    }

    let scene = instruments::build(inst, &world, opts);
    Ok((scene, report))
}
