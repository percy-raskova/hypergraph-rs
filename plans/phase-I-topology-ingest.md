# hypergraph-rs Phase I Implementation Plan — Topology Ingest (real `Hypergraph` → instruments)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Each `### Task N` section is self-contained: an agent picks it up cold, reads this preamble + that task, and executes its RED→GREEN→commit steps.

**Goal:** Let the three Phase-R instruments render a **real `Hypergraph`** instead of only the seeded `DeckWorld` fixture. Today `build(inst, &DeckWorld, &opts)` is the only entry point, and no raster code path has ever read a single node or edge attribute — `deck.rs` builds `Hypergraph<Value,Value,Value>` with `Value::Null` everywhere and consumes only graph *structure*. Phase I adds (a) a documented, versioned **attribute contract** naming which attr keys supply each scalar the instruments need, (b) an **adapter** `(&Hypergraph, &AttrContract) -> Result<DeckWorld, IngestError>` that reads those attrs, (c) **typed graceful degradation** so a graph missing required attrs errors or renders empty but never panics and never fabricates a number, (d) a **PyO3 surface** so babylon can reach ingest → rasterize → `PixelBuffer`. Phase I adds no new mathematics and no new instrument: it is a data path, and the existing frozen render path must come out the other side **byte-identical**.

**Architecture:** `Hypergraph<Value,Value,Value>` --(`ingest::to_world`)--> `DeckWorld` --(existing frozen path: `instruments::build` → `rasterize` / `render_pixels`)--> `CellGrid` / `PixelBuffer`. The adapter is the *only* new code on the way in. Nothing downstream of `DeckWorld` changes — not one signature, not one line — which is what makes the byte-identity guarantee structural rather than merely tested.

**Tech stack:** Rust 1.85+ (workspace MSRV, edition 2021), BSD-3-Clause, no new dependencies. `thiserror` is already a workspace dep (`crates/hypergraph-rs/Cargo.toml:17`) and is the error-enum idiom in this crate (`communities::SpectralError`, `communities/mod.rs:21-27`). **No new Cargo feature is introduced** — see Global Constraints.

---

## CRITICAL DESIGN RULING — adapter into `DeckWorld`, NOT a `WorldSource` trait

The crate-reuse scout proposed a `trait InstrumentWorld { fn agents(&self) -> &[Agent]; … }` implemented by both `DeckWorld` and a new `HypergraphWorld<'a>` borrow-adapter, arguing it keeps `DeckWorld` conceptually pinned to the deck fixture. **This plan rejects that and takes the adapter-into-struct instead.** The reasoning, stated plainly because the tradeoff is real:

- **Byte-identity must be structural, not tested.** The trait requires editing `instruments::build`, `cylinder::build_cylinder`, `terrain::build_terrain`, and `cylinder::spectral_positions` — i.e. **every** function on the frozen golden path — from `&DeckWorld` to `&impl InstrumentWorld`. Those edits are *very probably* value-preserving. "Very probably" is the wrong standard for a path with 4 scene goldens and 7 frame goldens. The adapter touches **zero** frozen-path lines: `to_world` returns an ordinary `DeckWorld` and hands it to the untouched `build`. Task 6's byte-identity proof then confirms a guarantee that already holds by construction, instead of chasing one.
- **The trait's own argument is weaker than it looks.** Its stated benefit is naming hygiene — keeping the fixture type out of the ingest path. But `DeckWorld` is a plain data bundle (`agents`/`communities`/`extraction`/`industries`/`repression`/`zoom` + two unread fields); nothing in it is deck-specific except its name and its `seed`/`tick` provenance fields, which the crate scout confirmed **no instrument builder reads** (`deck_hypergraph` at `deck.rs:279` touches only `agents`/`communities`/`extraction`; `seed`/`tick` are read only by `build_world`/`do_tick`). Naming hygiene is worth a doc comment, not a refactor of the golden path.
- **Provenance is handled without mutating the struct.** The trait's other claim — that the adapter "can't express no-seed provenance" — is true but cheap to solve: `to_world` returns `(DeckWorld, IngestReport)`, and the report carries provenance (contract version, node/edge counts, defaults applied, dropped edges). `DeckWorld.seed`/`.tick` get documented sentinel `0`. **No field is added to `DeckWorld`** — adding one would change its `PartialEq`/`Debug` and every construction site, which is exactly the frozen-path churn we are avoiding.

**Consequence:** `DeckWorld` is hereby re-documented (Task 1) as "the instrument input bundle, of which `build_world(seed)` is the seeded deck *fixture* producer and `ingest::to_world` is the real-graph producer". If the owner later wants a rename, it is a mechanical `s/DeckWorld/InstrumentWorld/` with a `pub type DeckWorld = InstrumentWorld;` alias — cheaper after Phase I than before it.

## CRITICAL PRECONDITION FINDING — three real panic/NaN routes in the frozen path

The frozen path is safe for the deck fixture and **not** safe for an arbitrary graph. Verified by reading the shipped code; the adapter must validate all three, because we cannot fix them downstream without breaking byte-identity:

1. **Dense agent ids are load-bearing.** `spectral_positions` indexes `pos[a.id]` (`cylinder.rs:203`) and `deck_hypergraph` indexes `degree[a.id]` (`deck.rs:318`). Both assume `Agent.id` values are exactly `0..agents.len()`. A sparse or non-zero-based id set is an **out-of-bounds panic**. → the adapter assigns dense indices and keeps the string ids in a side map (`IngestReport.node_ids`).
2. **`periphery_mean_wealth` divides by zero.** `deck.rs:416-419` computes `sum / per.len() as f64` over the `bloc == 2` agents. With no bloc-2 agent that is `0.0/0.0 = NaN`, which flows into `psa` → the cylinder's `metadata` → the glyph banner → a golden frame containing `NaN`. `psa` is computed whenever `communities.len() >= 2` (`cylinder.rs`). → the adapter enforces: if the ingested world has ≥2 communities, at least one agent must carry the periphery bloc. Typed error, never a default.
3. **`< 2` agents breaks the eigensolver.** `spectral_positions` calls `smallest_k_eigenvectors(&l, 2, …)` and `.expect(…)`s the Laplacian. → the adapter requires ≥2 agent nodes for the Cylinder/Terrain instruments.

A fourth, milder one: `algebra::transform(&[])` on zero industries yields `r = S/(C+V) = 0/0 = NaN`. → Sankey with zero industry edges returns an **explicitly empty scene**, not a NaN-bannered one (Task 5).

`deck_hypergraph`'s isolated-node handling is already generic — `deck.rs:317-321` gives any degree-0 agent an inert singleton self-edge, so an ingested graph with isolated agents is safe *provided* ids are dense. That one is a genuine reuse, not a gap.

---

## Global Constraints

- **Rust edition 2021**, MSRV 1.85. **License:** BSD-3-Clause. **No `unsafe`.**
- **Determinism is sacred.** `to_world` is a pure function of `(&Hypergraph, &AttrContract)` — no wall-clock, no entropy, no `HashMap`/`HashSet` iteration order in any output. All graph walks go through `node_ids()` / `edge_ids()` (`core/hypergraph.rs:292,298`), which are `IndexMap`-backed insertion order. Where the adapter needs a lookup map it may use `HashMap` **internally**, but must never let its iteration order reach an output `Vec`. Float compares in validation are NaN-safe: reject non-finite attr values with a typed error rather than `partial_cmp().unwrap()`.
- **Every loop has a fixed upper bound** (node count, edge count, member count per edge, industry count) — Power-of-10 rule 2, noted inline per task.
- **Explicit error handling only** (Power-of-10 rule 4). No `unwrap()`, no `expect()`, no catch-all in `ingest`. Every failure is an `IngestError` variant with the offending node/edge id and key in the message.
- **Stringly-typed ID boundary preserved (divergence D7).** `Hypergraph` ids stay `String` end to end. The adapter's dense `usize` indices are an **internal render-space index**, never a new public id domain; the `String` ↔ `usize` map is exposed read-only on `IngestReport` so callers can map a rendered node back to their own id.
- **Never invent data.** Every scalar the contract cannot find has exactly one of two dispositions, chosen per key and documented in the contract's own doc comment: an **explicit named `DEFAULT_*` constant** (for keys whose absence is a legitimate state) or an **`IngestError`** (for keys whose absence means the caller has the wrong graph). There is no third option. Silent zeros are forbidden.
- **Feature gating: no new feature.** `raster/ingest/` is gated `#[cfg(feature = "raster")]` alongside `deck.rs` (it produces a `DeckWorld`, which lives under `raster`); the instrument-dispatching entry point `ingest::build_scene` is additionally `#[cfg(feature = "cells3d")]`, exactly like the `instruments` module it calls. **`cells3d` and `raster-png` stay OUT of `default`** (the S9 lane gate; current table verified at `crates/hypergraph-rs/Cargo.toml`: `default = ["algorithms","generators","stats","readwrite","layout","viz","raster"]`, `all = ["default","dynamics","communities","convert","network","cells3d","raster-png"]`). The five-way feature matrix stays five-way — do not add a sixth leg.
- **TDD discipline:** every task is red → green → commit. Integration tests are **direct children of `crates/hypergraph-rs/tests/`** — cargo discovers targets only there. New file: `tests/test_ingest.rs`, opening with a crate-level `#![cfg(feature = "cells3d")]` header (it reaches `instruments`), matching the shipped precedent in `tests/test_instruments.rs`.
- **`assert_eq!` requires `Debug`**; every public ingest type derives `Debug + Clone + PartialEq`. `AttrContract` additionally derives `Serialize`/`Deserialize` so a contract can be pinned as a golden artifact.
- **Commit convention:** conventional commits; end every message with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **No publishing** (no `cargo publish`). **cargo single-flight** — one cargo invocation at a time, never parallelized.

---

## REUSE LEDGER (DRY is a project law — every entry is shipped code a task reuses)

Verified against the live tree at `b9435eb`. **Do not re-implement any of these; call them.**

| Shipped API | Signature (abbrev.) | file:line | Reused by |
|---|---|---|---|
| `DeckWorld` | `{seed,agents,communities,extraction,industries,repression,tick,zoom}` | `raster/deck.rs:78-88` | Task 4 (the adapter's output type — **unmodified**) |
| `Agent` | `{id: usize, bloc: u8, wealth: f64, subsistence: f64}` | `raster/deck.rs:49-55` | Task 2 |
| `Community` | `{name: String, members: Vec<usize>, links: Vec<(usize,usize)>}` | `raster/deck.rs:59-64` | Task 3 |
| `Extraction` | `{tails: Vec<usize>, head: usize, mass: f64}` | `raster/deck.rs:68-73` | Task 3 |
| `Industry` | `{name: String, lane: u8, c: f64, v: f64, e: f64}` | `raster/algebra.rs:7-13` | Task 3 |
| `instruments::build` | `fn(Instrument, &DeckWorld, &InstrumentOpts) -> SceneGraph3D` | `raster/instruments/mod.rs:48` | Task 5 (**called, never edited**) |
| `Instrument` / `InstrumentOpts` / `CylinderHeight` | 3-variant enum / opts struct / height enum (`Default`) | `raster/instruments/mod.rs:15,32,41` | Task 5 |
| `rasterize` | `fn(&SceneGraph3D, &Camera, u16, u16) -> CellGrid` | `raster/mod.rs:296` | Task 7, Task 8 |
| `CellGrid::to_text` | `fn(&self) -> String` | `raster/mod.rs:109` | Task 7 (density measurement) |
| `render_pixels` / `PixelBuffer` | `fn(&SceneGraph3D,&Camera,u32,u32) -> PixelBuffer` / `{w,h,px}` | `raster/png.rs:37,19` | Task 8 |
| `render_png` | `fn(&SceneGraph3D,&Camera,u32,u32) -> Vec<u8>` | `raster/png.rs:61` | Task 8 |
| `Camera` | `{ry,rx,dist,fov}` (`Serialize`/`Deserialize`) | `raster/camera.rs:28` | Tasks 7, 8 |
| `SceneGraph3D::empty()` | `fn() -> Self` (all collections empty, zero bbox) | `raster/mod.rs:196` | Task 5 (the explicitly-empty scene) |
| `SceneGraph3D::labeled_scalars()` | `fn(&self) -> Vec<String>` (key-sorted, `fmt6` floats) | `raster/mod.rs:216` | Task 7 (banner/glyph-floor parity) |
| `deck_hypergraph` | `fn(&DeckWorld) -> Hypergraph<Value,Value,Value>` — incl. degree-0 singleton self-edge guard | `raster/deck.rs:279,317-321` | Task 4 (reached via `build`; the isolated-node guard is why isolated ingested agents are safe) |
| `org_degree` | `fn(&Community) -> f64`, `1.0` when `\|C\|<2` | `raster/deck.rs:336` | Task 3 (why `links` may legitimately be empty) |
| `agent_tension` | `fn(&Agent, f64) -> f64` = `max(0, repression·(subsistence−wealth))` | `raster/instruments/terrain.rs:33` | Task 2 (the tension-scalar contract's target formula) |
| `periphery_mean_wealth` / `psa` / `psr` | deck metadata scalars | `raster/deck.rs:416,424,438` | Task 4 (the three preconditions) |
| `node_ids` / `edge_ids` / `members` / `node_attrs` / `edge_attrs` / `graph_attr` | insertion-ordered accessors, `Option`-returning | `core/hypergraph.rs:292,298,272,366,384,402` | Tasks 2–4 (the entire read surface) |
| `SpectralError` (thiserror idiom) | `#[error("…")]` enum | `communities/mod.rs:21-27` | Task 1 (`IngestError` follows this shape) |
| `json_to_py` | `fn(Python, &Value) -> PyResult<Bound<PyAny>>` | `hypergraph-rs-python/src/pyid.rs` | Task 8 (scene → Python dict, same pattern as `PyHypergraph::render`, `hypergraph.rs:1078-1084`) |
| `PyHypergraph::render` | `serde_json::to_value(&scene)` → `json_to_py` → `PyDict` | `hypergraph-rs-python/src/hypergraph.rs:1078-1084` | Task 8 (the exact idiom to copy for `SceneGraph3D`) |

**Confirmed gaps Phase I BUILDS new:** `AttrContract` + its key constants + version, `IngestError`, `IngestReport`, `to_world`, `build_scene`, the PyO3 raster module. Nothing else. **Do not invent crate APIs** — if a task seems to need an accessor not in this ledger, stop and report rather than adding one.

---

## THE ATTRIBUTE CONTRACT (normative — Task 1 implements this table verbatim)

Attrs are `serde_json::Value` (the crate default, `Hypergraph<Value,Value,Value>`). `ATTR_CONTRACT_VERSION: u32 = 1`. Every key name is a `pub const &str` with the default spelling below; `AttrContract` holds owned `String`s so a caller may rename any key without forking the crate.

### Node attrs (one node per agent)

| Contract field | Default key | Type | Missing ⇒ | Rationale / babylon source |
|---|---|---|---|---|
| `identity` | *(the node id itself)* | `String` | n/a | The `Hypergraph` node id **is** the agent identity (D7 stringly-typed boundary). Never read from attrs. Babylon: `SocialClass.id`, `^C[0-9]{3,}$`. |
| `wealth` | `"wealth"` | finite `f64` | **`IngestError::MissingNodeAttr`** | Feeds `agent_tension` and `psa`. No honest default exists — a default wealth is a fabricated economic fact. Babylon: `SocialClass.wealth`. |
| `subsistence` | `"subsistence"` | finite `f64` | **`IngestError::MissingNodeAttr`** | Same. Babylon: `SocialClass.subsistence_threshold`. |
| `bloc` | `"bloc"` | `u8` in `0..=2` | **`DEFAULT_BLOC: u8 = 1`** | Bloc selects a marker colour (`terrain.rs:31` `BLOC_COLORS`) and the periphery filter. `1` is the neutral bone-coloured middle bloc. Babylon has **no** 0/1/2 bloc — `SocialRole` is a different vocabulary — so mapping is the caller's job and the default is explicitly "unclassified", not "core" or "periphery". **Constrained by precondition 2:** with ≥2 communities, at least one node must carry `bloc == PERIPHERY_BLOC (2)` or ingest errors. |
| `tension` | `"tension"` | finite `f64 ≥ 0` | **absent ⇒ derive** | *Optional override.* When present it is used verbatim as the terrain's per-agent scalar; when absent the frozen `agent_tension(a, repression)` formula applies. This is the seam for babylon's `contradiction_fields["exploitation"]`, which is **not** numerically equal to the formula (state scout). Which source is in play is recorded in `IngestReport.tension_source` — never silently ambiguous. |
| `h3_cell` | `"h3_cell"` | `u64` or decimal-`String` | absent ⇒ `None` | *Optional, carried not consumed.* Phase I stores it in `IngestReport.h3_cells` for callers; no instrument reads it (the Sankey grid derives its own cells). Accepting a decimal string is required because JSON numbers lose precision above 2^53 and H3 indices exceed it — a `u64` H3 id **must not** round-trip through `f64`. Reject any other shape with `IngestError::BadAttrType`. |

### Edge attrs (edge `kind` dispatches the shape)

`kind` default key `"kind"`. An edge whose `kind` is missing or unrecognised is **skipped** and counted in `IngestReport.skipped_edges` — a hypergraph may legitimately carry edges the instruments have no use for, and erroring on them would make ingest unusable against a real babylon graph. Skipping is reported, never silent.

| `kind` | Extra keys | Missing ⇒ | Maps to |
|---|---|---|---|
| `"community"` | `name` (`String`, default `DEFAULT_COMMUNITY_NAME` + edge id), `links` (array of 2-element id arrays, default `[]`) | name defaults; `links` defaults to empty | `Community { name, members = edge members, links }`. Empty `links` is legitimate: `org_degree` returns `1.0` for `\|C\|<2` and `1.0 - (\|C\|-1)/(\|C\|-1) = 0.0` for a fully disconnected larger community — a real reading, not a fabrication. Any link id not in this edge's own members ⇒ `IngestError::LinkOutsideCommunity`. |
| `"extraction"` | `head` (node id `String`, **required**), `mass` (`f64`, default `DEFAULT_MASS = 1.0`) | missing `head` ⇒ **`IngestError::MissingEdgeAttr`** | `Extraction { tails = members minus head, head, mass }`. `mass` defaults because the crate scout confirmed **no instrument reads it** (`deck.rs`: `mass` is used only by `do_tick`) — defaulting an unread field is safe and is documented as such. `head` not among the edge's members ⇒ `IngestError::HeadNotMember`. |
| `"industry"` | `name` (`String`, default = edge id), `c`, `v`, `e` (finite `f64`, **all required**), `lane` (`u8` in `0..=3`) | `c`/`v`/`e` missing ⇒ **`IngestError::MissingEdgeAttr`**; `lane` missing ⇒ **`DEFAULT_LANE: u8 = 0`** *and* `IngestReport.lane_defaulted = true` | `Industry { name, lane, c, v, e }`. **Lane is the one honest hole.** The state scout established that babylon has *no* industry-level periphery/internal-colony/settler-garrison/apex classification anywhere; `TerritoryType` classifies territories and `CommunityType` classifies populations. Synthesising a lane from either is inference dressed as data. Phase I therefore defaults every unlabelled industry into a single lane and **flags it**, so a lane-coloured Sankey built from a lane-less graph is visibly one-laned rather than plausibly-four-laned. `v == 0.0` ⇒ `IngestError::DegenerateIndustry` (`SankeyEntity::e()` is `s/v`, `sankey.rs:76`). |

### Graph attrs

| Contract field | Default key | Missing ⇒ | Notes |
|---|---|---|---|
| `repression` | `"repression"` | **`DEFAULT_REPRESSION: f64 = 1.0`** | Scales `agent_tension` linearly and appears in `psr`'s denominator. `1.0` is the identity scaling — the one value that adds no information. Must be finite and `> 0.0` (it divides in `psr`, `deck.rs:445`); `<= 0.0` or non-finite ⇒ `IngestError::BadGraphAttr`. |
| `zoom` | `"zoom"` | **`DEFAULT_ZOOM: u8 = 0`** (fine) | `instruments::build` maps `zoom == 1` → `Zoom::Coarse`, else `Zoom::Fine` (`instruments/mod.rs:53-57`). Fine is the un-aggregated reading. |

### Not in the contract, deliberately

`seed` and `tick` are set to `0` on the produced `DeckWorld` and are **never read by any instrument** (crate scout: `deck_hypergraph` touches only `agents`/`communities`/`extraction`; `seed`/`tick` are read only by `build_world`/`do_tick`, neither of which Phase I calls). Real provenance lives on `IngestReport`. Do not attempt to encode a babylon tick into `DeckWorld.tick` — it would be write-only state and an invitation to later confusion.

---

## File Structure

```
hypergraph-rs/crates/hypergraph-rs/
├── src/raster/
│   ├── mod.rs                  # + pub mod ingest;  (cfg raster)
│   ├── deck.rs                 # docs only: DeckWorld re-described as the instrument input bundle
│   └── ingest/
│       ├── mod.rs              # AttrContract, key consts, DEFAULT_*, ATTR_CONTRACT_VERSION,
│       │                       #   IngestError, IngestReport, TensionSource, to_world,
│       │                       #   build_scene (cells3d-gated)
│       ├── nodes.rs            # agent extraction + dense-index assignment
│       └── edges.rs            # kind dispatch: community / extraction / industry
└── tests/
    ├── test_ingest.rs          # Tasks 2-5, 7   (#![cfg(feature = "cells3d")])
    └── fixtures/               # UNCHANGED by Phase I — Task 6 proves it
crates/hypergraph-rs-python/
├── Cargo.toml                  # + [features] cells3d / raster-png forwarding (default = [])
└── src/
    ├── lib.rs                  # + mod raster; (cfg cells3d) + raster::register(m)?
    └── raster.rs               # PyO3: ingest + rasterize + PixelBuffer
```

---

### Task 1: Ingest module scaffold — versioned `AttrContract`, key constants, `IngestError`, `IngestReport`

**Files:** Create `src/raster/ingest/mod.rs`; modify `src/raster/mod.rs` (add `pub mod ingest;` under the existing `raster` gate); modify `src/raster/deck.rs` (**doc comments only** — re-describe `DeckWorld` per the design ruling; do not touch a single line of executable code); create `tests/test_ingest.rs` with a `#![cfg(feature = "cells3d")]` header.

**Interfaces:**

```rust
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

pub const DEFAULT_BLOC: u8 = 1;
pub const PERIPHERY_BLOC: u8 = 2;
pub const DEFAULT_LANE: u8 = 0;
pub const DEFAULT_MASS: f64 = 1.0;
pub const DEFAULT_REPRESSION: f64 = 1.0;
pub const DEFAULT_ZOOM: u8 = 0;
pub const DEFAULT_COMMUNITY_NAME: &str = "community-";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttrContract { /* one owned String per key above + pub version: u32 */ }
impl Default for AttrContract { /* the default spellings + ATTR_CONTRACT_VERSION */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensionSource { Derived, Attr }

#[derive(Debug, Clone, PartialEq)]
pub struct IngestReport {
    pub contract_version: u32,
    pub node_ids: Vec<String>,          // dense index -> original String id
    pub agent_count: usize,
    pub community_count: usize,
    pub extraction_count: usize,
    pub industry_count: usize,
    pub skipped_edges: Vec<String>,     // edge ids with missing/unknown kind
    pub bloc_defaulted: usize,
    pub lane_defaulted: bool,
    pub repression_defaulted: bool,
    pub tension_source: TensionSource,
    pub h3_cells: Vec<Option<u64>>,     // parallel to node_ids
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum IngestError {
    #[error("node `{node}` is missing required attribute `{key}`")]
    MissingNodeAttr { node: String, key: String },
    #[error("edge `{edge}` (kind `{kind}`) is missing required attribute `{key}`")]
    MissingEdgeAttr { edge: String, kind: String, key: String },
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
    #[error("a world with {communities} communities computes psa over the periphery bloc ({bloc}), but no node carries it")]
    NoPeripheryBloc { communities: usize, bloc: u8 },
}
```

- [ ] **Step 1: Write the failing test** in `tests/test_ingest.rs`:
```rust
#![cfg(feature = "cells3d")]
use hypergraph_rs::raster::ingest::{AttrContract, ATTR_CONTRACT_VERSION, K_WEALTH, DEFAULT_BLOC};

#[test]
fn test_contract_default_spellings_and_version() {
    let c = AttrContract::default();
    assert_eq!(c.version, ATTR_CONTRACT_VERSION);
    assert_eq!(c.wealth, K_WEALTH);
    assert_eq!(DEFAULT_BLOC, 1);
}

#[test]
fn test_contract_roundtrips_through_json() {
    let c = AttrContract::default();
    let s = serde_json::to_string(&c).expect("contract serializes");
    assert_eq!(serde_json::from_str::<AttrContract>(&s).expect("deserializes"), c);
}

#[test]
fn test_contract_keys_are_renameable() {
    let mut c = AttrContract::default();
    c.wealth = "babylon_wealth".to_string();
    assert_ne!(c.wealth, K_WEALTH);
}
```
- [ ] **Step 2: Run** `cargo test -p hypergraph-rs --features cells3d --test test_ingest` — Expected: FAIL (module absent).
- [ ] **Step 3: Implement** `ingest/mod.rs` with the constants, `AttrContract` (+ `Default`, derive `Serialize`/`Deserialize`), `TensionSource`, `IngestReport`, `IngestError`. Add `#[cfg(feature = "raster")] pub mod ingest;` to `raster/mod.rs`. The module doc comment MUST reproduce the normative contract table from this plan's preamble — that doc comment is the contract's canonical home; this plan is its rationale, not its spec. Re-doc `DeckWorld` in `deck.rs` per the design ruling (comment text only).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): attribute contract v1 + typed ingest errors for topology ingest`.

---

### Task 2: Node ingest — agents, dense index assignment, defaults vs typed errors

**Files:** Create `src/raster/ingest/nodes.rs`; modify `src/raster/ingest/mod.rs`; modify `tests/test_ingest.rs`.

**Interfaces:** `pub(crate) fn ingest_nodes(h: &Hypergraph<Value,Value,Value>, c: &AttrContract) -> Result<(Vec<Agent>, Vec<String>, Vec<Option<u64>>, usize, TensionSource), IngestError>` — returning agents in `node_ids()` order, the parallel original-id vector, the parallel optional H3 cells, the bloc-default count, and the tension source. Dense ids: `Agent.id` is the position in `h.node_ids()` (precondition 1). Also `pub(crate) fn index_of(ids: &[String]) -> HashMap<&str, usize>` — internal only, its iteration order never reaches an output.

**Tension handling:** if *any* node carries the tension key, `TensionSource::Attr`; then every node must carry it (a half-populated field is exactly the silent-mixing failure the contract forbids) — a node missing it in that mode ⇒ `MissingNodeAttr`. Under `Attr`, the value is stored by inverting the frozen formula so the untouched `agent_tension` reproduces it exactly: set `subsistence' = wealth + t/repression` when `repression > 0`. **This inversion is the one subtle step in Phase I** — document it in-line with its algebra, and note that it makes `psa` (which reads `subsistence`) a function of the supplied tension. Record `TensionSource::Attr` in the report so any downstream consumer knows `subsistence` is synthetic in that mode.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn test_nodes_missing_wealth_is_typed_error_not_default() {
    // node with subsistence but no wealth -> Err(MissingNodeAttr{key:"wealth"}), never 0.0
}

#[test]
fn test_nodes_missing_bloc_defaults_to_one_and_is_counted() {
    // agent.bloc == DEFAULT_BLOC, report.bloc_defaulted == 1
}

#[test]
fn test_nodes_agent_ids_are_dense_and_follow_node_ids_order() {
    // ids == 0..n and report.node_ids matches h.node_ids() elementwise
}

#[test]
fn test_nodes_reject_non_finite_wealth() {
    // wealth: 1e400 / null / "3.0" -> BadAttrType, no NaN reaches an Agent
}

#[test]
fn test_h3_cell_accepts_u64_string_above_2_pow_53() {
    // "622236723497533439" round-trips exactly; a float-shaped value is BadAttrType
}
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** `nodes.rs`. Walk `h.node_ids()` (bounded by node count); per node read `h.node_attrs(&id)` and match on `Value` shape explicitly — `Value::Number` with `as_f64()` + `is_finite()` for floats, `as_u64()` with a `<= u8::MAX` and range check for `bloc`/`lane`, `as_u64()` **or** `as_str().parse::<u64>()` for `h3_cell`. No `unwrap`. Two passes over nodes are allowed (pass 1 detects the tension mode, pass 2 builds agents) — both bounded by node count.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): node ingest — agents, dense indices, contract defaults and typed errors`.

---

### Task 3: Edge ingest — community / extraction / industry dispatch

**Files:** Create `src/raster/ingest/edges.rs`; modify `src/raster/ingest/mod.rs`; modify `tests/test_ingest.rs`.

**Interfaces:** `pub(crate) fn ingest_edges(h: &…, c: &AttrContract, index: &HashMap<&str, usize>) -> Result<IngestedEdges, IngestError>` where `IngestedEdges { communities: Vec<Community>, extraction: Vec<Extraction>, industries: Vec<Industry>, skipped: Vec<String>, lane_defaulted: bool }`. Walk `h.edge_ids()` (bounded); per edge read `h.edge_attrs(&id)` and `h.members(&id)`; dispatch on the `kind` string. Unknown/missing kind ⇒ push the edge id to `skipped`, continue. Every member id is resolved through `index`; an unresolvable one ⇒ `UnknownNode` (this cannot normally happen — `add_edge` auto-creates members — but the check is cheap and the alternative is an OOB panic).

Ordering: `communities`, `extraction`, and `industries` each preserve `edge_ids()` relative order. `Community.members` preserves `h.members(edge)` order; `Extraction.tails` is `members` order with the head removed. **`Industry` slice order is load-bearing** — `algebra::transform` iterates the slice in order and `build_sankey` lays out from it — so it must be `edge_ids()` order, deterministically, never a sorted-by-name or `HashMap`-derived order.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn test_edges_unknown_kind_is_skipped_and_reported_not_an_error() {}

#[test]
fn test_community_links_outside_members_is_typed_error() {}

#[test]
fn test_community_with_no_links_ingests_and_org_degree_is_real() {
    // 3 members, links: [] -> org_degree == 0.0 (a reading, not a fabrication)
}

#[test]
fn test_extraction_head_must_be_a_member() {}

#[test]
fn test_extraction_mass_defaults_because_no_instrument_reads_it() {}

#[test]
fn test_industry_missing_c_v_or_e_is_typed_error_not_zero() {}

#[test]
fn test_industry_missing_lane_defaults_and_sets_lane_defaulted_flag() {}

#[test]
fn test_industry_zero_v_is_degenerate_industry_error() {}

#[test]
fn test_industry_slice_order_follows_edge_ids_order() {}
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** `edges.rs`. Every loop bounded (edge count, members per edge, links per community). Explicit `Value` shape matching as in Task 2 — `links` must be an array of 2-element arrays of strings, anything else ⇒ `BadAttrType`.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): edge ingest — community/extraction/industry kind dispatch with reported skips`.

---

### Task 4: `to_world` — graph attrs, precondition validation, `IngestReport`

**Files:** Modify `src/raster/ingest/mod.rs`; modify `tests/test_ingest.rs`.

**Interfaces:** `pub fn to_world(h: &Hypergraph<Value,Value,Value>, c: &AttrContract) -> Result<(DeckWorld, IngestReport), IngestError>`. Reads `h.graph_attr(&c.repression)` / `(&c.zoom)` with the documented defaults, composes Tasks 2+3, then validates the three CRITICAL PRECONDITIONS **before** returning:

1. `agents.len() >= 2` else `TooFewAgents`.
2. If `communities.len() >= 2` and no agent has `bloc == PERIPHERY_BLOC` ⇒ `NoPeripheryBloc`. (This is precisely the `0.0/0.0` route in `periphery_mean_wealth`, `deck.rs:416-419`.)
3. `repression` finite and `> 0.0` else `BadGraphAttr` (it divides in `psr`, `deck.rs:445`).

`DeckWorld { seed: 0, tick: 0, .. }` — documented sentinels, never read (see "Not in the contract, deliberately").

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn test_to_world_is_pure_and_deterministic() {
    // two calls on the same graph produce equal DeckWorld and equal IngestReport
}

#[test]
fn test_two_communities_without_periphery_bloc_errors_rather_than_emitting_nan() {
    // the guard for periphery_mean_wealth's 0/0 — assert Err(NoPeripheryBloc), and
    // separately assert that the accepted path's psa(&world).is_finite()
}

#[test]
fn test_single_agent_graph_is_too_few_agents() {}

#[test]
fn test_repression_defaults_to_one_and_zero_repression_is_rejected() {}

#[test]
fn test_report_records_contract_version_counts_and_id_map() {}
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** `to_world`. Validation is a straight-line sequence of explicit checks, each with its own error variant — no combined "invalid world" catch-all (Power-of-10 rule 4).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): to_world adapter — graph attrs, precondition validation, ingest report`.

---

### Task 5: Graceful degradation — `build_scene`, explicitly-empty scenes, no panic ever

**Files:** Modify `src/raster/ingest/mod.rs`; modify `tests/test_ingest.rs`.

**Interfaces (`#[cfg(feature = "cells3d")]`):**
```rust
pub fn build_scene(
    h: &Hypergraph<Value, Value, Value>,
    c: &AttrContract,
    inst: Instrument,
    opts: &InstrumentOpts,
) -> Result<(SceneGraph3D, IngestReport), IngestError>;
```
Calls `to_world`, then dispatches to the **untouched** `instruments::build`. Degradation policy, stated exhaustively so no case is left to judgement:

- **Contract violation** (any `IngestError`) ⇒ `Err`. The caller learns which node/edge and which key.
- **`Instrument::Sankey` with zero industry edges** ⇒ `Ok((SceneGraph3D::empty(), report))`. Rationale: `algebra::transform(&[])` computes `r = S/(C+V) = 0/0 = NaN`, which would reach the glyph banner. An empty scene is the honest rendering of "this graph has no industries"; a NaN banner is a lie with a number in it. `report.industry_count == 0` tells the caller why.
- **`Cylinder`/`Terrain` with zero community edges** ⇒ proceed normally. `build_cylinder` handles an empty community list (rings and the agent disc still render; the `psa`/`psr` metadata is gated on `communities.len() >= 2` at its own call site). This is a legitimate degraded-but-real scene, not an empty one.
- **Never a panic.** Not on empty graphs, not on isolated agents (`deck_hypergraph`'s degree-0 singleton guard, `deck.rs:317-321`), not on absent attrs. A panic-freedom test sweeps a set of adversarial graphs across all three instruments.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn test_sankey_with_no_industries_returns_explicitly_empty_scene() {
    let (s, r) = build_scene(&h, &AttrContract::default(), Instrument::Sankey, &Default::default()).unwrap();
    assert_eq!(s, SceneGraph3D::empty());
    assert_eq!(r.industry_count, 0);
}

#[test]
fn test_no_scene_scalar_is_nan_or_infinite() {
    // every labeled_scalars() token of every instrument parses finite (NaN-safe check)
}

#[test]
fn test_isolated_agents_do_not_panic_the_spectral_embedding() {
    // 4 agents, 1 community over 2 of them, 2 agents in no edge at all
}

#[test]
fn test_all_three_instruments_are_panic_free_over_adversarial_graphs() {
    // empty-ish, isolated-only, single-community, duplicate-member, zero-link graphs
}

#[test]
fn test_missing_required_attr_is_err_not_a_default_scene() {}
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** `build_scene`. It is a thin dispatcher: validate, branch on the one empty-scene case, delegate. **Do not edit `instruments/mod.rs` or any builder.** If a builder appears to need a change, stop and report — that would break Task 6's guarantee.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): graceful degradation — build_scene with typed errors and explicitly-empty scenes`.

---

### Task 6: THE SAFETY PROPERTY — every existing golden fixture is BYTE-IDENTICAL

**Files:** none expected to change. This task's entire job is proving that claim; if it cannot be proven, Phase I stops here and the design ruling is revisited.

**Why this task exists:** Phase I's whole architecture (adapter, not trait) was chosen so byte-identity holds *by construction* — no frozen-path line is edited. This task converts that reasoning into evidence. A regression caught here means someone edited the frozen path; the fix is to revert that edit, **not** to regenerate the fixture.

The protected set (verified present at `crates/hypergraph-rs/tests/fixtures/`): `scene3d/{cylinder,terrain,sankey_fine,sankey_coarse}.json`, `frames/{cylinder_front,cylinder_3q,terrain_front,terrain_3q,sankey_front,sankey_3q}.txt`, `frames/cylinder_front.ansi.txt`, and every file under `png/`.

- [ ] **Step 1: Capture the pre-state.** From the repo root, on a clean tree:
```bash
cd crates/hypergraph-rs
find tests/fixtures -type f | sort | xargs sha256sum > /tmp/phase-I-fixtures-before.txt
git status --porcelain tests/fixtures   # MUST be empty
```
- [ ] **Step 2: Re-run the full five-way feature matrix** (the Phase-R Task-15 matrix, unchanged — Phase I adds no sixth leg):
```bash
cargo build -p hypergraph-rs --no-default-features --features raster
cargo build -p hypergraph-rs --no-default-features --features cells3d
cargo build -p hypergraph-rs --no-default-features --features raster-png
cargo build -p hypergraph-rs
cargo build -p hypergraph-rs --no-default-features
```
Then the golden-bearing test legs: `cargo test -p hypergraph-rs --features cells3d` and `cargo test -p hypergraph-rs --features raster-png`. Confirm `default` still does **not** enable `cells3d` or `raster-png` (the S9 gate).
- [ ] **Step 3: Diff the fixtures.**
```bash
find tests/fixtures -type f | sort | xargs sha256sum > /tmp/phase-I-fixtures-after.txt
diff /tmp/phase-I-fixtures-before.txt /tmp/phase-I-fixtures-after.txt   # MUST be empty
git diff --exit-code -- tests/fixtures                                  # MUST exit 0
```
- [ ] **Step 4: Add the standing regression test** in `tests/test_ingest.rs` — an assertion that the deck path is untouched *in value*, so a future refactor cannot drift it without a red test:
```rust
#[test]
fn test_deck_path_frames_still_match_the_committed_goldens() {
    // build_world(1917) -> build(inst,…) -> rasterize(…, 80, 40).to_text()
    // == include_str!("fixtures/frames/<inst>_front.txt"), for all three instruments.
    // Byte-exact string equality. This is the safety property in executable form.
}
```
- [ ] **Step 5: Record the result** in the commit body: matrix legs green, fixture digest diff empty, test count on each leg.
- [ ] **Step 6: Commit** — `test(hypergraph-rs): prove Phase I leaves every golden fixture byte-identical (five-way matrix)`.

---

### Task 7: Real-`Hypergraph` render acceptance — non-degenerate scene AND non-degenerate frame

**Files:** Modify `tests/test_ingest.rs`.

**This is the acceptance criterion for "babylon topology can render."** Building a small `Hypergraph` with contract attrs, pushing it through all three instruments, and asserting a scene comes out is *necessary and nowhere near sufficient*. **The Phase R lesson (post-review corrections, commit `67b1c17`): a golden that freezes a collapsed render is worse than no golden.** Phase R shipped frames at 34, 83, and 389 lit subcells that all "passed" and were all badly wrong; the fix took them to 363, 152, and 1177. So this task asserts **spread and coverage**, not merely non-emptiness — and it freezes **no** golden. A golden for the ingest path may only be frozen after a human has looked at the rendered frame.

**Measured deck baselines** (committed frames, 80×40 = 3200 cells, non-blank cell counts): `cylinder_front` 359 (11.2%), `terrain_front` 148 (4.6%), `sankey_front` 1158 (36.2%). These bound "the same order of magnitude".

**The fixture graph:** ~12 agent nodes across all three blocs (≥1 with `bloc == 2`), 3 community edges with real `links`, 2 extraction edges, 5 industry edges with distinct `lane`/`c`/`v`/`e`. Built by a `fn contract_fixture() -> Hypergraph<Value,Value,Value>` helper in the test file with **literal, hand-written attrs** — no generator, no RNG, no loop-generated values.

- [ ] **Step 1: Write the failing test:**
```rust
fn lit_fraction(text: &str) -> f64 {
    let cells: Vec<char> = text.chars().filter(|c| *c != '\n').collect();
    let lit = cells.iter().filter(|c| **c != ' ' && **c != '\u{2800}').count();
    lit as f64 / cells.len() as f64
}

#[test]
fn test_real_hypergraph_scene_is_non_degenerate_for_every_instrument() {
    // per instrument: nodes+faces+struts > 0; bounding_box has non-zero extent on
    // >= 2 axes; labeled_scalars() non-empty and every value finite.
}

#[test]
fn test_real_hypergraph_frame_has_deck_comparable_lit_density() {
    // rasterize(scene, &front_camera(), 80, 40).to_text()
    // 1. lit_fraction >= 0.02  (absolute floor, below the deck terrain's 4.6%)
    // 2. lit_fraction is within [0.25x, 4.0x] of the SAME instrument's deck frame
    //    density, measured in-test by rasterizing build_world(1917) with the same
    //    camera and size — self-calibrating, so it cannot rot against the fixtures.
}

#[test]
fn test_real_hypergraph_frame_is_spread_not_collapsed() {
    // lit cells span >= 40% of columns and >= 30% of rows (bounding box of lit
    // cells, banner rows excluded) — the assertion Phase R's collapsed frames
    // would have failed.
}

#[test]
fn test_real_hypergraph_render_is_deterministic_across_repeated_runs() {
    // identical to_text() over 3 runs; and identical across a fresh process is
    // covered by there being no RNG/time input at all.
}
```
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Make it pass.** The likely failure is a *real* one: the fixture graph is too small or too tightly clustered for a visible spread. Fix it by adjusting the **fixture** (more agents, wider wealth spread, more community links) — never by loosening a threshold. If a threshold genuinely cannot be met with a plausible graph, **stop and report**: that is a finding about the instruments, not a test to weaken.
- [ ] **Step 4: Run** — Expected: PASS. Then **print one frame to the terminal and read it** before committing. Record in the commit body: per-instrument lit fraction, deck baseline, and ratio.
- [ ] **Step 5: Commit** — `test(hypergraph-rs): real-Hypergraph render acceptance — non-degenerate scene, spread and density bands`.

---

### Task 8: PyO3 exposure — ingest + rasterize + `PixelBuffer` reachable from Python

**Files:** Modify `crates/hypergraph-rs-python/Cargo.toml`; create `crates/hypergraph-rs-python/src/raster.rs`; modify `crates/hypergraph-rs-python/src/lib.rs`.

**Feature gating (consistent with the crate's S9 lane gate).** The python crate currently has **no `[features]` table** and depends on `hypergraph-rs` with an explicit feature list that omits `raster`/`cells3d` — but does *not* set `default-features = false`, so `raster` is already on transitively via the core crate's `default`. Add:
```toml
[features]
default = []
cells3d = ["hypergraph-rs/cells3d"]
raster-png = ["cells3d", "hypergraph-rs/raster-png"]
```
`default = []` keeps `cells3d` **off** in the default Python build — the same owner gate the core crate applies. The new `mod raster;` and its `register` call are `#[cfg(feature = "cells3d")]`; `render_png` is additionally `#[cfg(feature = "raster-png")]`.

**Interfaces (module `_hypergraph_rs_core.raster`):**
- `build_scene(h: Hypergraph, instrument: str, contract: dict | None = None, cylinder_height: str | None = None) -> tuple[dict, dict]` — `(scene, report)` as plain dicts via `serde_json::to_value` + `json_to_py`, the exact idiom of `PyHypergraph::render` (`hypergraph.rs:1078-1084`). `instrument` parses `"cylinder"|"terrain"|"sankey"`; anything else raises. `contract` overrides key spellings; absent ⇒ `AttrContract::default()`.
- `rasterize_text(h, instrument, cols, rows, camera: dict | None, contract=None) -> str` — the Tier-0 glyph frame.
- `render_pixels(h, instrument, width, height, camera=None, contract=None) -> tuple[int, int, bytes]` — `(w, h, rgb8_bytes)` from `PixelBuffer` (`png.rs:19,37`), flattened row-major RGB. `#[cfg(feature = "raster-png")]`: `render_png(...) -> bytes` via `png.rs:61`.
- Every `IngestError` maps to a Python exception through the crate's existing error idiom (`crates/hypergraph-rs-python/src/errors.rs`) — **the message must carry the offending node/edge id and key verbatim**, because that message is the only debugging signal babylon gets when its attrs are wrong.

- [ ] **Step 1: Write the failing test.** A `#[cfg(test)]` Rust unit test covering the pure conversion helpers (str→`Instrument`, dict→`AttrContract`, dict→`Camera`, `PixelBuffer`→bytes length `w*h*3`) — these are the parts testable without a Python interpreter. Assert the round-trip of an unknown instrument string produces an error, not a default.
- [ ] **Step 2: Run** `cargo test -p hypergraph-rs-python --features cells3d` — Expected: FAIL.
- [ ] **Step 3: Implement** `raster.rs` + the `[features]` table + `lib.rs` registration (`#[cfg(feature = "cells3d")] raster::register(m)?;`). No `unwrap` at the FFI boundary.
- [ ] **Step 4: Run** — Expected: PASS. Then verify the gate: `cargo build -p hypergraph-rs-python` (no `raster` module compiled in), `cargo build -p hypergraph-rs-python --features cells3d`, `cargo build -p hypergraph-rs-python --features raster-png`.
- [ ] **Step 5: Smoke-test from Python** if a maturin build is available in the subrepo `.venv`: build a 12-node contract graph, call `rasterize_text`, print the frame. Record the lit fraction in the commit body. If the toolchain is unavailable, say so explicitly in the commit body rather than implying it was run.
- [ ] **Step 6: Commit** — `feat(hypergraph-rs-python): expose topology ingest, glyph rasterize and pixel plate (cells3d-gated)`.

---

### Task 9: Final gate — clippy, fmt, five-way matrix, full suite, fixture re-diff

**Files:** any files with lints.

- [ ] **Step 1: Clippy both legs:** `cargo clippy -p hypergraph-rs --features cells3d -- -D warnings` and `--features raster-png`, plus `cargo clippy -p hypergraph-rs-python --features raster-png -- -D warnings`. Fix all. Expected lints and preferred fixes:
  - `clippy::type_complexity` on `ingest_nodes`'s 5-tuple return — prefer a named `pub(crate) struct IngestedNodes` over `#[allow]`.
  - `clippy::too_many_arguments` on the PyO3 entry points — group `(cols, rows, camera)` into a small struct or accept the `#[allow]` with a written reason (PyO3 signatures are the Python API surface and grouping degrades it).
  - `clippy::float_cmp` in determinism tests — the bit-equality contract is intentional; `#[allow]` in the test module with a comment.
- [ ] **Step 2: Five-way matrix** — exactly the Task 6 Step 2 commands. Confirm no sixth leg was introduced and `default` still excludes `cells3d`/`raster-png`.
- [ ] **Step 3: Full gate** — `mise run rust:check` (fmt --check + clippy --workspace --all-targets --locked -D warnings + test --workspace --locked + doc -D warnings). This is the canonical composite gate.
- [ ] **Step 4: Re-diff the fixtures one final time** — `git diff --exit-code -- crates/hypergraph-rs/tests/fixtures` must exit 0. The safety property is re-proven at the end of the phase, not only in the middle of it.
- [ ] **Step 5: Commit** — `chore(hypergraph-rs): clippy + fmt + feature-matrix cleanup for topology ingest`.

---

## Phase I Completion Summary

After all 9 tasks the crate has:

- **`raster/ingest/`** (feature `raster`; `build_scene` additionally `cells3d`): a versioned `AttrContract` (v1) whose module doc is the canonical contract, `IngestError` (11 explicit variants), `IngestReport` (provenance, id map, every default that was applied, the tension source), `to_world` (pure `(&Hypergraph, &AttrContract) -> Result<(DeckWorld, IngestReport), IngestError>`), and `build_scene` (typed errors, one documented explicitly-empty case, panic-free).
- **Zero frozen-path edits.** `deck.rs` changed in comments only; `instruments/`, `rasterize`, `png.rs`, `camera.rs`, `grid.rs`, `algebra.rs` untouched. Byte-identity of all 11+ golden fixtures holds by construction and is proven twice (Tasks 6 and 9).
- **A real-`Hypergraph` acceptance test** asserting non-degenerate scenes and frames with deck-comparable lit density (self-calibrating band) and spread across ≥40% of columns / ≥30% of rows — the assertion Phase R's collapsed frames would have failed. **No new golden is frozen**; a human reads a frame before Phase I closes.
- **A PyO3 surface** (`_hypergraph_rs_core.raster`, `cells3d`-gated, out of the Python default build) exposing `build_scene`, `rasterize_text`, `render_pixels`, and `render_png`.

**What Phase I does NOT do**, said plainly so nobody assumes otherwise:
- It does not give babylon a way to *emit* a contract-shaped `Hypergraph`. The state scout found **no export/serialization path** from `BabylonGraph` (a live in-process rustworkx structure) to anything Rust can consume. Phase I defines the shape of what must arrive; producing it is babylon-side work.
- It does not solve **scale**. `DeckWorld` is a 28-agent/13-industry fixture; babylon's graph is national-scale. No subsetting or aggregation policy exists anywhere (which county, which tick, top-N by what) and inventing one is a design decision, not a code task.
- It does not invent an **industry lane**. Babylon has no per-sector periphery/internal-colony/settler-garrison/apex classification; Phase I defaults to one lane and raises `lane_defaulted`.
- It does not adjudicate babylon's **two non-equivalent tension sources** (`repression_faced·(subsistence−wealth)` vs `contradiction_fields["exploitation"]`) or its **two "organization" quantities** (`SocialClass.organization` vs a Π₀ degree over community links). The contract makes the choice *explicit and reported*; it does not make it for the caller.
- It does not touch the **C9 swap gate** (`plans/C9-exclusion-crosscheck.md`) — Phase I is a rendering path, not the xgi swap.

## OPEN OWNER DECISIONS

- **D-I1 — tension inversion (Task 2).** Supplying `tension` as an attr is implemented by inverting the frozen `agent_tension` formula into a synthetic `subsistence`, which makes `psa` a function of the supplied tension. The alternative is editing `terrain.rs` to read a tension field directly — which breaks the "no frozen-path edits" guarantee. **Default: the inversion**, reported via `TensionSource::Attr`. Owner may later prefer a frozen-path change once the goldens are re-frozen for other reasons.
- **D-I2 — H3 carried, not consumed.** `h3_cell` is ingested onto `IngestReport` but no instrument reads it (the Sankey grid derives its own cells). **Default: carry it.** Making the Sankey grid honour supplied H3 cells is a real feature and a real golden change; it is not in Phase I.
- **D-I3 — contract v1 stability.** `ATTR_CONTRACT_VERSION` is bumped on any change to a key's *meaning*, default, or error disposition — not on adding a new optional key. **Default: this rule.** Flagged so the first babylon integration does not silently pin against an unversioned shape.
- **D-I4 — `DeckWorld` rename.** Deferred by design (see the CRITICAL DESIGN RULING). Mechanically cheap after Phase I; owner call, not an executor's.

## Constraints footer

- **No publishing** (no `cargo publish`, no crate release).
- **cargo single-flight** — one cargo invocation at a time; never parallelize cargo.
- **`cells3d` and `raster-png` stay OUT of `default`** in both crates (S9 lane gate, #287 pending).
- **Stringly-typed at the ID boundary** (divergence D7) — the dense `usize` index is internal render space only.
- **Commit trailer:** every commit ends with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Do not invent crate APIs.** Every API this plan calls appears in the REUSE LEDGER with a file:line. Anything else: stop and report.
