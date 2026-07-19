# hypergraph-rs Phase 5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `layout/` and `viz/` modules — the four `LayoutStrategy`
variants (Spring / Bipartite / HyperedgeAware / Radial) that compute deterministic
node positions and per-hyperedge geometry, and the `viz` scene-graph serializer
(`SceneGraph` + glyph structs) with three render entry points (`render`,
`render_to_json`, `render_to_svg`). This is the matplotlib-free replacement for
XGI's `xgi.drawing` (spec §5): **layout is pure math, the scene-graph JSON is the
contract, SVG/JSON are hand-rolled emitters.**

**Architecture:** `compute_layout(&Hypergraph, LayoutStrategy) -> LayoutResult`
produces `node_positions: Vec<(String,(f64,f64))>` (insertion-ordered) plus
`edge_geometries: Vec<(String, HyperedgeGeometry)>` and a `bounding_box`. The
`viz` module wraps a `LayoutResult` into a `SceneGraph` of `NodeGlyph` /
`HyperedgeGlyph` records carrying attrs + style, and emits JSON (serde) or SVG
(hand-rolled string builder). Everything is seeded/deterministic (Constitution
III.7) — no `HashMap` in any public-facing order, no external SVG/geometry crate.

**Tech Stack:** Rust 1.85+ (workspace MSRV), `rustworkx-core` 0.18, `indexmap` 2,
`serde` + `serde_json`, `thiserror` 2, and **used by this phase:** `rand` 0.10.2
+ `rand_pcg` 0.10.2 + `rand_core` 0.10.1. Phase 2 is the sole owner of the
workspace-manifest correction — it already fixed `[workspace.dependencies]` to
these canonical pins (the Phase 0+1 manifest's `rand = "0.8"` / `rand_pcg =
"0.3"` entries were never consumed and are stale) and already wired
`rand.workspace = true` / `rand_pcg.workspace = true` into the core crate's own
`[dependencies]`. Phase 5 assumes that wiring is already present and neither
re-edits `[workspace.dependencies]` nor re-adds the crate-level lines.
BSD-3-Clause.

---

## CRITICAL SPEC CORRECTION — rustworkx-core has NO layout functions

The design spec §5.2/§5.4 repeatedly claims the layouts "delegate to
rustworkx-core's `spring_layout` / `bipartite_layout` / `circular_layout`" and
that "all 14 Rust layout functions are reused." **The 2026-07-18 dependency
probe empirically DISPROVED this:** a `docs.rs` fetch of
`rustworkx_core` 0.18.0 confirms there is **no `layout` module** — the exposed
modules are `centrality`, `connectivity`, `shortest_path`, `generators`,
`traversal`, `max_weight_matching`, `coloring`, `dag_algo`, `graph_ext`. Layout
functions (`spring_layout`, `bipartite_layout`, …) exist only in the **Python**
`rustworkx` package, not in the `rustworkx-core` Rust crate.

**Consequence — this plan supersedes the spec on this point:** all four layout
strategies are **hand-rolled** in `hypergraph-rs::layout` with seeded
`rand_pcg::Pcg64Mcg`. There is no delegation. This mirrors the spec's own
fallback language ("with deterministic seeded fallbacks") — the fallback is the
only path. Position parity with XGI's numpy-based Fruchterman–Reingold is
therefore **structural, not numeric** (see Deviations).

---

## Global Constraints

- **Rust edition 2021**, MSRV 1.85. **License:** BSD-3-Clause.
- **Determinism (III.7 parity):** every layout is a pure function of
  `(hypergraph, strategy)`. Randomised layouts (Spring, HyperedgeAware) seed
  `Pcg64Mcg` from an explicit `u64`; when a strategy carries no seed (or `seed =
  None`) we substitute `DEFAULT_LAYOUT_SEED` — never the wall clock, never a
  global RNG. Node/edge iteration follows the hypergraph's insertion order
  (`node_ids()` / `edge_ids()` from Phase 1). Internal `HashMap`/`HashSet` are
  permitted ONLY for dedup/index lookup whose result order is derived from an
  already-insertion-ordered source (documented per use).
- **No `unsafe`.** **Every loop has a fixed upper bound** (node count, edge
  count, iteration count, or a strictly-decreasing hull stack — noted inline).
- **TDD discipline:** every task is red → green → commit. Integration tests are
  **direct children of `crates/hypergraph-rs/tests/`** (`test_layout.rs`,
  `test_viz.rs`) — cargo discovers targets only there.
- **`assert_eq!`/`assert_ne!` require `Debug`** on operands — every public
  layout/viz type derives `Debug` (and `PartialEq` for equality asserts). Same-
  seed reproduction is **bit-identical** `f64`, so `assert_eq!` on `LayoutResult`
  is valid for determinism tests.
- **serde_json::Value-specific methods** (`.cloned()` of a `&Value`,
  scene-graph attr population) live in a **concrete** `impl<M>
  Hypergraph<serde_json::Value, serde_json::Value, M>` block (the Phase 0+1
  Task-16 lesson) — never in a generic `impl<N,E,M>` block.
- **Commit convention:** conventional commits; end every message with
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Feature gating (spec §10.1):** the new modules are gated `#[cfg(feature =
  "layout")]` / `#[cfg(feature = "viz")]`; `viz` implies `layout`; both are in
  `default`.

## Deliberate deviations from XGI / the spec (each called out where it lands)

1. **No rustworkx-core delegation** (see the CRITICAL SPEC CORRECTION above) —
   all layouts hand-rolled; the spec §5.2/§5.4 delegation table is not
   implementable and is superseded here.
2. **Structural, not numeric, layout parity.** XGI's `pairwise_spring_layout`
   calls `nx.spring_layout` (numpy Fruchterman–Reingold, `layout.py:82-134`);
   our FR uses `rand_pcg`. Different RNG ⇒ different exact coordinates even at
   the same seed. Parity is: every node gets a position, output is deterministic
   per seed, the bounding box is well-formed. XGI's `tests/drawing/test_layout.py`
   position-value assertions are adapted to structural assertions at Phase 7
   (spec §9.3–§9.4 record/replay split applies to layouts too).
3. **`seed = None` is deterministic.** XGI's `seed=None` uses numpy's
   uninitialised global RNG (non-deterministic, `layout.py:92-96`); we substitute
   `DEFAULT_LAYOUT_SEED = 0` — a determinism improvement, not parity.
4. **Non-randomised strategies use a fixed seed too.** Bipartite and Radial are
   fully deterministic *placement* layouts (no RNG). HyperedgeAware reuses the
   Spring FR at `DEFAULT_LAYOUT_SEED` (its enum variant carries no `seed` field,
   spec §5.2).
5. **Convex-hull halo re-implemented.** XGI draws halos with `scipy.spatial.
   ConvexHull` + a `_CCW_sort` (`draw.py:18, 789-800`); we hand-roll Andrew's
   monotone chain (`layout::hull::convex_hull`). XGI's halo "radius margin"
   (default `0.05`, `draw.py:199-200`) maps to our `HyperedgeAware.halo_padding`.
6. **`HyperedgeGeometry` lives in `layout`, not `viz`.** Spec §5.3 lists it under
   viz, but `viz::render` depends on `layout::compute_layout`, so the geometry
   type must sit in `layout` to keep the dependency acyclic (`viz` → `layout`).
   `viz` re-exports it (`pub use crate::layout::HyperedgeGeometry;`) so the
   public path `viz::HyperedgeGeometry` still resolves per the spec.
7. **One geometry variant per strategy** (all four `HyperedgeGeometry` variants
   exercised): Spring → `Spline`, Bipartite → `Stripe`, HyperedgeAware →
   `ConvexHull`, Radial → `Arc`.
8. **Bipartite `Horizontal` align** produces a degenerate (zero-height) `Stripe`
   because the `Stripe` variant models a vertical span; node/edge positions are
   still a valid two-row bipartite arrangement. Documented limitation, not a bug.

---

## File Structure

```
hypergraph-rs/crates/hypergraph-rs/
├── Cargo.toml                         # + rand, rand_pcg deps; + [features]
├── src/
│   ├── lib.rs                         # + pub mod layout; pub mod viz; (cfg-gated)
│   ├── core/ …                        # (Phase 0+1, unchanged)
│   ├── layout/
│   │   ├── mod.rs                     # LayoutStrategy, BipartiteAlign, HyperedgeGeometry,
│   │   │                              #   LayoutResult, DEFAULT_LAYOUT_SEED, bounding_box_of,
│   │   │                              #   compute_layout dispatcher + re-exports
│   │   ├── hull.rs                    # convex_hull (Andrew monotone chain)
│   │   ├── place.rs                   # bipartite_layout, radial_layout (deterministic)
│   │   └── spring.rs                  # skeleton_edges, fruchterman_reingold,
│   │                                  #   spring_layout, hyperedge_aware_layout
│   └── viz/
│       ├── mod.rs                     # NodeStyle, HyperedgeStyle, NodeGlyph,
│       │                              #   HyperedgeGlyph, SceneGraph,
│       │                              #   render/render_to_json/render_to_svg (concrete impl)
│       └── svg.rs                     # scene_to_svg (hand-rolled emitter)
└── tests/
    ├── test_hypergraph.rs             # (Phase 0+1)
    ├── test_layout.rs                 # NEW — Tasks 1-7
    └── test_viz.rs                    # NEW — Tasks 8-10
```

**Note on the `[features]` table:** the Phase 0+1 core `Cargo.toml` had no
`[features]` section. Phases 2-4 are expected to introduce one (spec §10.1:
`algorithms`, `generators`, `stats`, `readwrite`). **Merge** `layout` / `viz`
into the existing table and append them to the existing `default` array —
**never overwrite `default` wholesale**, since that silently drops earlier
phases' features and their gated modules. The literal snippet in Task 1 shows
the fully-merged target (all six features); adapt it to whatever subset of
`algorithms`/`generators`/`stats`/`readwrite` actually landed.

---

## Phase 5: Layout + Viz

### Task 1: Scaffold layout+viz modules, deps, features, and the core public types

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/viz/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::LayoutStrategy`, `layout::BipartiteAlign`,
  `layout::HyperedgeGeometry`, `layout::LayoutResult`,
  `layout::DEFAULT_LAYOUT_SEED`; `viz::{NodeStyle, HyperedgeStyle, NodeGlyph,
  HyperedgeGlyph, SceneGraph}`. All serde-round-trippable, all `Debug + Clone +
  PartialEq`.

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::{
    BipartiteAlign, HyperedgeGeometry, LayoutResult, LayoutStrategy, DEFAULT_LAYOUT_SEED,
};

#[test]
fn test_layout_strategy_variants_construct() {
    let _a = LayoutStrategy::Spring { iterations: 50, seed: Some(7) };
    let _b = LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical };
    let _c = LayoutStrategy::HyperedgeAware { iterations: 100, halo_padding: 0.5 };
    let _d = LayoutStrategy::Radial { rings: 3 };
    assert_eq!(DEFAULT_LAYOUT_SEED, 0);
}

#[test]
fn test_layout_strategy_serde_round_trip() {
    let s = LayoutStrategy::HyperedgeAware { iterations: 100, halo_padding: 0.25 };
    let json = serde_json::to_string(&s).unwrap();
    let back: LayoutStrategy = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}

#[test]
fn test_hyperedge_geometry_variants_serde() {
    let variants = vec![
        HyperedgeGeometry::ConvexHull { points: vec![(0.0, 0.0), (1.0, 1.0)] },
        HyperedgeGeometry::Spline { control_points: vec![(0.0, 0.0)] },
        HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 2.0 },
        HyperedgeGeometry::Arc { center: (0.0, 0.0), radius: 1.0, angle_range: (0.0, 3.14) },
    ];
    for g in variants {
        let json = serde_json::to_string(&g).unwrap();
        let back: HyperedgeGeometry = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}

#[test]
fn test_layout_result_serde_round_trip() {
    let r = LayoutResult {
        node_positions: vec![("a".to_string(), (0.1, 0.2))],
        edge_geometries: vec![(
            "e1".to_string(),
            HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 1.0 },
        )],
        bounding_box: (0.0, 0.0, 1.0, 1.0),
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: LayoutResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
}
```

Also write a viz smoke test into the SAME file (viz types must compile now):

```rust
use hypergraph_rs::viz::{HyperedgeGlyph, HyperedgeStyle, NodeGlyph, NodeStyle, SceneGraph};

#[test]
fn test_scene_graph_serde_round_trip() {
    let scene = SceneGraph {
        nodes: vec![NodeGlyph {
            id: "a".to_string(),
            x: 0.0,
            y: 0.0,
            radius: 0.02,
            attrs: serde_json::Value::Null,
            style: NodeStyle::default(),
        }],
        hyperedges: vec![HyperedgeGlyph {
            id: "e1".to_string(),
            member_ids: vec!["a".to_string()],
            geometry: HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 1.0 },
            attrs: serde_json::Value::Null,
            style: HyperedgeStyle::default(),
        }],
        bounding_box: (0.0, 0.0, 1.0, 1.0),
        metadata: serde_json::Map::new(),
    };
    let json = serde_json::to_string(&scene).unwrap();
    let back: SceneGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `layout` and `viz` modules do not exist (compile error).

- [ ] **Step 3: Wire deps, features, modules, and types**

Edit `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`. **Phase 2 is the sole
owner of the workspace-manifest correction:** it already edited
`[workspace.dependencies]` to the canonical pins (`rand = "0.10.2"`,
`rand_pcg = "0.10.2"`, `rand_core = "0.10.1"`) and already added
`rand.workspace = true` / `rand_pcg.workspace = true` (and `rand_core` if
needed) to the core crate's own `[dependencies]` table. **Phase 5 does NOT
re-edit `[workspace.dependencies]` and does NOT add these lines again** —
verify first with `grep -c '^rand' Cargo.toml` (expect the lines already
present); a duplicate key is a hard `cargo` error (`duplicate key`). If, for
some reason, Phase 2 or Phase 4 has not landed yet and the lines are genuinely
absent, add them once:

```toml
rand.workspace = true
rand_pcg.workspace = true
```

Add a `[features]` section, or — if Phases 2-4 already created one (spec §10.1
default set is `["algorithms", "generators", "stats", "readwrite"]`) — **merge
into it**: add the `layout = []` and `viz = ["layout"]` keys, and append
`"layout", "viz"` to whatever `default` already lists. **Never overwrite an
existing `default` array** — replacing it verbatim silently drops earlier
phases' features and their `#[cfg(feature = ...)]` modules, breaking
`cargo test --workspace`. If no `[features]` table exists yet (no prior phase
added one), write it fresh:

```toml
[features]
default = ["algorithms", "generators", "stats", "readwrite", "layout", "viz"]
layout = []
viz = ["layout"]
```

(Only include the feature names that Phases 2-4 actually declared — this
snippet assumes the full spec §10.1 set landed; if some algorithm/generator
features didn't materialize, merge in only the ones present.) Note
`viz = ["layout"]` (viz implies layout) is intentional and must be preserved
verbatim when merging — it is correct and deliberately stronger than the
spec's own `viz = []`, since `viz::render` depends on `crate::layout`.

Edit `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`, add after `pub mod core;`:

```rust
#[cfg(feature = "layout")]
pub mod layout;
#[cfg(feature = "viz")]
pub mod viz;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`:

```rust
//! Deterministic hypergraph layout: node positions + per-hyperedge geometry.
//!
//! All strategies are hand-rolled — `rustworkx-core` 0.18 exposes NO layout
//! functions (empirically confirmed 2026-07-18). Randomised layouts seed
//! `rand_pcg::Pcg64Mcg`; non-randomised layouts (Bipartite, Radial) are pure
//! placement functions. Positions/geometries are emitted in the hypergraph's
//! insertion order (Constitution III.7 parity).

use serde::{Deserialize, Serialize};

/// The default deterministic seed used when a strategy provides no explicit
/// seed (or `seed = None`). XGI's `seed=None` is non-deterministic (numpy global
/// RNG); we substitute a fixed seed for Constitution III.7 determinism.
pub const DEFAULT_LAYOUT_SEED: u64 = 0;

/// Column/row alignment for the bipartite layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BipartiteAlign {
    /// Agents in a left column, hyperedges in a right column (x separates roles).
    Vertical,
    /// Agents in a bottom row, hyperedges in a top row (y separates roles).
    Horizontal,
}

/// The four layout strategies (spec §5.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutStrategy {
    /// Fruchterman–Reingold force-directed layout on the 1-skeleton.
    Spring { iterations: usize, seed: Option<u64> },
    /// Two-column/row bipartite placement of agents and hyperedge markers.
    Bipartite { align: BipartiteAlign },
    /// Spring positions for agents; each hyperedge is the padded convex hull of
    /// its members (a colored halo).
    HyperedgeAware { iterations: usize, halo_padding: f64 },
    /// Agents on concentric rings by membership count; hyperedges as arcs.
    Radial { rings: usize },
}

/// The geometry of a single hyperedge in a laid-out scene.
///
/// Defined here (not in `viz`) so that `viz` can depend on `layout` acyclically;
/// `viz` re-exports it as `viz::HyperedgeGeometry` (spec §5.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HyperedgeGeometry {
    /// Convex hull around member nodes (halo style).
    ConvexHull { points: Vec<(f64, f64)> },
    /// Closed spline / polyline through members (elliptical style).
    Spline { control_points: Vec<(f64, f64)> },
    /// Vertical stripe (bipartite layout).
    Stripe { x: f64, y_min: f64, y_max: f64 },
    /// Radial arc (radial layout).
    Arc { center: (f64, f64), radius: f64, angle_range: (f64, f64) },
}

/// The result of a layout computation: insertion-ordered node positions and
/// per-hyperedge geometry, plus the overall bounding box (min_x, min_y, max_x,
/// max_y).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutResult {
    pub node_positions: Vec<(String, (f64, f64))>,
    pub edge_geometries: Vec<(String, HyperedgeGeometry)>,
    pub bounding_box: (f64, f64, f64, f64),
}
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/viz/mod.rs`:

```rust
//! Scene-graph serializer: the matplotlib-free rendering contract (spec §5.3).
//!
//! A `SceneGraph` is a `LayoutResult` enriched with attrs + style, ready for
//! JSON export, SVG emit, or consumption by a WASM/React renderer.

use serde::{Deserialize, Serialize};

/// Re-export so `viz::HyperedgeGeometry` resolves (spec §5.3 places it here).
pub use crate::layout::HyperedgeGeometry;

/// Visual style for a node glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeStyle {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill: "#1f77b4".to_string(),
            stroke: "#000000".to_string(),
            stroke_width: 1.0,
        }
    }
}

/// Visual style for a hyperedge glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperedgeStyle {
    pub fill: String,
    pub stroke: String,
    pub opacity: f64,
}

impl Default for HyperedgeStyle {
    fn default() -> Self {
        Self {
            fill: "#ff7f0e".to_string(),
            stroke: "#ff7f0e".to_string(),
            opacity: 0.4,
        }
    }
}

/// A rendered node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeGlyph {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub attrs: serde_json::Value,
    pub style: NodeStyle,
}

/// A rendered hyperedge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperedgeGlyph {
    pub id: String,
    pub member_ids: Vec<String>,
    pub geometry: HyperedgeGeometry,
    pub attrs: serde_json::Value,
    pub style: HyperedgeStyle,
}

/// The scene-graph: the contract between Rust and every renderer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneGraph {
    pub nodes: Vec<NodeGlyph>,
    pub hyperedges: Vec<HyperedgeGlyph>,
    pub bounding_box: (f64, f64, f64, f64),
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 5 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): scaffold layout+viz modules, types, deps, features

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: Convex hull (Andrew's monotone chain)

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/src/layout/hull.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)>` —
  hull vertices in counter-clockwise order; for ≤2 unique points returns those
  points (degenerate hull). Replaces XGI's scipy `ConvexHull` (`draw.py:797`).

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::convex_hull;

#[test]
fn test_convex_hull_square_excludes_interior() {
    let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)];
    let hull = convex_hull(&pts);
    assert_eq!(hull.len(), 4);
    assert!(!hull.contains(&(0.5, 0.5)));
    for corner in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
        assert!(hull.contains(&corner), "hull must contain corner {:?}", corner);
    }
}

#[test]
fn test_convex_hull_triangle() {
    let pts = vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)];
    let hull = convex_hull(&pts);
    assert_eq!(hull.len(), 3);
}

#[test]
fn test_convex_hull_two_points() {
    let hull = convex_hull(&[(0.0, 0.0), (1.0, 1.0)]);
    assert_eq!(hull.len(), 2);
}

#[test]
fn test_convex_hull_single_point() {
    let hull = convex_hull(&[(0.5, 0.5)]);
    assert_eq!(hull, vec![(0.5, 0.5)]);
}

#[test]
fn test_convex_hull_empty() {
    let hull = convex_hull(&[]);
    assert!(hull.is_empty());
}

#[test]
fn test_convex_hull_deterministic() {
    let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.3, 0.7)];
    assert_eq!(convex_hull(&pts), convex_hull(&pts));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `convex_hull` does not exist.

- [ ] **Step 3: Implement the hull**

Write to `hypergraph-rs/crates/hypergraph-rs/src/layout/hull.rs`:

```rust
//! Convex hull via Andrew's monotone chain (replaces XGI's scipy ConvexHull).

/// Compute the convex hull of a set of 2D points. Returns the hull vertices in
/// counter-clockwise order, with collinear points removed. For 0, 1, or 2
/// unique points the "hull" is those points themselves (a point or a segment);
/// callers render degenerate hulls specially.
///
/// Deterministic: the input is sorted by (x, then y) before construction.
pub fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = points.to_vec();
    // Deterministic total order on finite f64 (positions are always finite);
    // NaN would compare Equal, which is acceptable for layout coordinates.
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup(); // exact duplicates are now adjacent after the sort
    let n = pts.len();
    if n <= 2 {
        return pts;
    }

    // 2D cross product of (a-o) x (b-o).
    fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }

    let mut hull: Vec<(f64, f64)> = Vec::with_capacity(2 * n);
    // Lower hull. The inner `while` is bounded: each pop strictly shrinks
    // `hull`, and total pops across the whole build are <= total pushes (2n).
    for &p in pts.iter() {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    // Upper hull.
    let lower_len = hull.len() + 1;
    for &p in pts.iter().rev() {
        while hull.len() >= lower_len
            && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
        {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop(); // the last point duplicates the first point of the lower hull
    hull
}
```

Add to `layout/mod.rs` (near the top, after the `use` line):

```rust
pub mod hull;

pub use hull::convex_hull;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 11 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): convex_hull via Andrew monotone chain

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Bipartite layout + `bounding_box_of` helper

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/src/layout/place.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::bipartite_layout<N,E,M>(h: &Hypergraph<N,E,M>, align:
  BipartiteAlign) -> LayoutResult` (deterministic, no RNG); and the crate-private
  `bounding_box_of` helper used by all layouts.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::bipartite_layout;
use hypergraph_rs::Hypergraph;

fn sample_h() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string(), "d".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_bipartite_positions_every_node() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_bipartite_vertical_agents_in_left_column() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    // Vertical align: all agents share x = 0.0.
    for (_, (x, _)) in &r.node_positions {
        assert_eq!(*x, 0.0);
    }
}

#[test]
fn test_bipartite_insertion_order_preserved() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    let ids: Vec<&str> = r.node_positions.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_bipartite_emits_stripe_geometry() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Stripe { .. }));
    }
}

#[test]
fn test_bipartite_deterministic() {
    let h = sample_h();
    assert_eq!(
        bipartite_layout(&h, BipartiteAlign::Vertical),
        bipartite_layout(&h, BipartiteAlign::Vertical)
    );
}

#[test]
fn test_bipartite_bounding_box_well_formed() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    let (min_x, min_y, max_x, max_y) = r.bounding_box;
    assert!(max_x >= min_x);
    assert!(max_y >= min_y);
}

#[test]
fn test_bipartite_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    assert!(r.node_positions.is_empty());
    assert!(r.edge_geometries.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `bipartite_layout` does not exist.

- [ ] **Step 3: Implement the helper and bipartite layout**

Add to `layout/mod.rs` (after the `LayoutResult` struct):

```rust
/// Fold node positions + geometry extents into a bounding box
/// (min_x, min_y, max_x, max_y). Empty input yields the zero box.
pub(crate) fn bounding_box_of(
    positions: &[(String, (f64, f64))],
    geometries: &[(String, HyperedgeGeometry)],
) -> (f64, f64, f64, f64) {
    let mut bb = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    fn fold(bb: &mut (f64, f64, f64, f64), x: f64, y: f64) {
        if x < bb.0 {
            bb.0 = x;
        }
        if y < bb.1 {
            bb.1 = y;
        }
        if x > bb.2 {
            bb.2 = x;
        }
        if y > bb.3 {
            bb.3 = y;
        }
    }
    for (_, (x, y)) in positions {
        fold(&mut bb, *x, *y);
    }
    for (_, g) in geometries {
        match g {
            HyperedgeGeometry::ConvexHull { points }
            | HyperedgeGeometry::Spline { control_points: points } => {
                for (x, y) in points {
                    fold(&mut bb, *x, *y);
                }
            }
            HyperedgeGeometry::Stripe { x, y_min, y_max } => {
                fold(&mut bb, *x, *y_min);
                fold(&mut bb, *x, *y_max);
            }
            HyperedgeGeometry::Arc { center, radius, .. } => {
                fold(&mut bb, center.0 - radius, center.1 - radius);
                fold(&mut bb, center.0 + radius, center.1 + radius);
            }
        }
    }
    if !bb.0.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    bb
}
```

Also add to `layout/mod.rs` the submodule + re-export:

```rust
pub mod place;

pub use place::bipartite_layout;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/layout/place.rs`:

```rust
//! Deterministic placement layouts (no RNG): bipartite two-column and radial.

use crate::core::hypergraph::Hypergraph;

use super::{bounding_box_of, BipartiteAlign, HyperedgeGeometry, LayoutResult};

/// Evenly spread index `i` of `count` items into [0, 1]; a single item sits at
/// the midpoint 0.5.
fn spread(count: usize, i: usize) -> f64 {
    if count <= 1 {
        0.5
    } else {
        i as f64 / (count as f64 - 1.0)
    }
}

/// Bipartite layout: agents in one column/row, hyperedge markers in the other.
/// Deterministic — no RNG. Each hyperedge emits a `Stripe` spanning the
/// (oriented) y-range of its members at the marker's x. `Horizontal` align
/// swaps the role axis; see the module deviation note (its stripe is degenerate).
pub fn bipartite_layout<N, E, M>(h: &Hypergraph<N, E, M>, align: BipartiteAlign) -> LayoutResult {
    let node_ids = h.node_ids();
    let edge_ids = h.edge_ids();

    let orient = |x: f64, y: f64| -> (f64, f64) {
        match align {
            BipartiteAlign::Vertical => (x, y),
            BipartiteAlign::Horizontal => (y, x),
        }
    };

    let node_positions: Vec<(String, (f64, f64))> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), orient(0.0, spread(node_ids.len(), i))))
        .collect();

    let pos_of = |id: &str| -> (f64, f64) {
        node_positions
            .iter()
            .find(|(n, _)| n == id)
            .map(|(_, p)| *p)
            .unwrap_or((0.0, 0.0))
    };

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = edge_ids
        .iter()
        .enumerate()
        .map(|(i, eid)| {
            let (ex, _ey) = orient(1.0, spread(edge_ids.len(), i));
            let members = h.members(eid).unwrap_or_default();
            let (y_min, y_max) = if members.is_empty() {
                (0.0, 0.0)
            } else {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for m in &members {
                    let (_mx, my) = pos_of(m);
                    if my < lo {
                        lo = my;
                    }
                    if my > hi {
                        hi = my;
                    }
                }
                (lo, hi)
            };
            (eid.clone(), HyperedgeGeometry::Stripe { x: ex, y_min, y_max })
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
```

Note: `place.rs` imports `crate::core::hypergraph::Hypergraph` — the concrete
struct path from Phase 0+1 (`src/core/hypergraph.rs`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 18 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): bipartite_layout + bounding_box_of helper

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Radial layout

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/place.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::radial_layout<N,E,M>(h, rings: usize) -> LayoutResult`
  (deterministic). Agents placed on concentric rings bucketed by membership
  count; each hyperedge emits an `Arc`.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::radial_layout;

#[test]
fn test_radial_positions_every_node() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_radial_emits_arc_geometry() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Arc { .. }));
    }
}

#[test]
fn test_radial_deterministic() {
    let h = sample_h();
    assert_eq!(radial_layout(&h, 3), radial_layout(&h, 3));
}

#[test]
fn test_radial_zero_rings_does_not_panic() {
    let h = sample_h();
    // rings is clamped to at least 1 internally.
    let r = radial_layout(&h, 0);
    assert_eq!(r.node_positions.len(), h.num_nodes());
}

#[test]
fn test_radial_positions_within_unit_disk() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    for (_, (x, y)) in &r.node_positions {
        assert!((x * x + y * y).sqrt() <= 1.0 + 1e-9);
    }
}

#[test]
fn test_radial_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = radial_layout(&h, 3);
    assert!(r.node_positions.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `radial_layout` does not exist.

- [ ] **Step 3: Implement radial layout**

Add to `layout/mod.rs` the re-export (next to the bipartite one):

```rust
pub use place::radial_layout;
```

Add to `layout/place.rs`. First add the import at the top (merge with existing `use`):

```rust
use std::f64::consts::PI;
```

Then append this function:

```rust
/// Radial layout: agents on concentric rings bucketed by membership count
/// (higher membership ⇒ outer ring), evenly spread by angle in insertion order.
/// Each hyperedge emits an `Arc` at the mean radius of its members spanning
/// their angular range. Deterministic — no RNG.
pub fn radial_layout<N, E, M>(h: &Hypergraph<N, E, M>, rings: usize) -> LayoutResult {
    let node_ids = h.node_ids();
    let rings = rings.max(1);
    let n = node_ids.len();

    let degree = |id: &str| -> usize { h.memberships(id).map(|m| m.len()).unwrap_or(0) };
    let max_deg = node_ids.iter().map(|id| degree(id)).max().unwrap_or(0);

    let node_positions: Vec<(String, (f64, f64))> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let ring = if max_deg == 0 {
                0
            } else {
                degree(id) * (rings - 1) / max_deg
            };
            let radius = (ring as f64 + 1.0) / (rings as f64 + 1.0);
            let angle = if n == 0 {
                0.0
            } else {
                2.0 * PI * (i as f64) / (n as f64)
            };
            (id.clone(), (radius * angle.cos(), radius * angle.sin()))
        })
        .collect();

    let pos_of = |id: &str| -> (f64, f64) {
        node_positions
            .iter()
            .find(|(nn, _)| nn == id)
            .map(|(_, p)| *p)
            .unwrap_or((0.0, 0.0))
    };

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let (radius, a_min, a_max) = if members.is_empty() {
                (0.0, 0.0, 0.0)
            } else {
                let mut rsum = 0.0;
                let mut amin = f64::INFINITY;
                let mut amax = f64::NEG_INFINITY;
                for m in &members {
                    let (x, y) = pos_of(m);
                    rsum += (x * x + y * y).sqrt();
                    let ang = y.atan2(x);
                    if ang < amin {
                        amin = ang;
                    }
                    if ang > amax {
                        amax = ang;
                    }
                }
                (rsum / members.len() as f64, amin, amax)
            };
            (
                eid.clone(),
                HyperedgeGeometry::Arc {
                    center: (0.0, 0.0),
                    radius,
                    angle_range: (a_min, a_max),
                },
            )
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 24 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): radial_layout (concentric rings by degree)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Spring layout (Fruchterman–Reingold on the 1-skeleton)

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/src/layout/spring.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::spring_layout<N,E,M>(h, iterations: usize, seed:
  Option<u64>) -> LayoutResult`. Hand-rolled FR seeded by `rand_pcg::Pcg64Mcg`
  (XGI `pairwise_spring_layout`, `layout.py:82-134`, calls numpy `nx.spring_layout`
  — we do NOT reproduce numpy values; parity is structural, see Deviation 2).
  Each hyperedge emits a `Spline` of member positions.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::spring_layout;

#[test]
fn test_spring_positions_every_node() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_spring_same_seed_is_bit_identical() {
    let h = sample_h();
    let a = spring_layout(&h, 50, Some(42));
    let b = spring_layout(&h, 50, Some(42));
    assert_eq!(a, b);
}

#[test]
fn test_spring_different_seed_differs() {
    let h = sample_h();
    let a = spring_layout(&h, 50, Some(1));
    let b = spring_layout(&h, 50, Some(2));
    assert!(a.node_positions != b.node_positions);
}

#[test]
fn test_spring_none_seed_is_deterministic() {
    let h = sample_h();
    // seed=None falls back to DEFAULT_LAYOUT_SEED — still deterministic.
    let a = spring_layout(&h, 50, None);
    let b = spring_layout(&h, 50, None);
    assert_eq!(a, b);
}

#[test]
fn test_spring_emits_spline_geometry() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Spline { .. }));
    }
}

#[test]
fn test_spring_positions_finite() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    for (_, (x, y)) in &r.node_positions {
        assert!(x.is_finite() && y.is_finite());
    }
}

#[test]
fn test_spring_single_node() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("solo", serde_json::Value::Null);
    let r = spring_layout(&h, 50, Some(42));
    assert_eq!(r.node_positions.len(), 1);
}

#[test]
fn test_spring_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = spring_layout(&h, 50, Some(42));
    assert!(r.node_positions.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `spring_layout` does not exist.

- [ ] **Step 3: Implement the skeleton builder, FR, and spring layout**

Add to `layout/mod.rs` the submodule + re-export:

```rust
pub mod spring;

pub use spring::spring_layout;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/layout/spring.rs`:

```rust
//! Fruchterman–Reingold spring layout on the 1-skeleton, plus the
//! hyperedge-aware variant. Hand-rolled (no rustworkx-core layout exists) and
//! seeded by `rand_pcg::Pcg64Mcg` for Constitution III.7 determinism.

use std::collections::{HashMap, HashSet};

use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg64Mcg;

use crate::core::hypergraph::Hypergraph;

use super::hull::convex_hull;
use super::{bounding_box_of, HyperedgeGeometry, LayoutResult, DEFAULT_LAYOUT_SEED};

/// Build the 1-skeleton (pairwise projection) as an edge list over node
/// indices. Every pair of members of each hyperedge becomes one undirected
/// edge (deduplicated). Node indices are positions in `node_ids` (insertion
/// order). Returns `(node_ids, edges)`.
///
/// The internal `HashMap`/`HashSet` are lookup/dedup only — the returned
/// `edges` order is derived from `edge_ids()` (true insertion order) and
/// `members()` (deterministic petgraph adjacency order, NOT insertion order —
/// see Phase 0+1 `members()`), so the result is deterministic though not
/// insertion-ordered end to end.
fn skeleton_edges<N, E, M>(h: &Hypergraph<N, E, M>) -> (Vec<String>, Vec<(usize, usize)>) {
    let node_ids = h.node_ids();
    let mut index_of: HashMap<&str, usize> = HashMap::new();
    for (i, id) in node_ids.iter().enumerate() {
        index_of.insert(id.as_str(), i);
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        for a in 0..members.len() {
            for b in (a + 1)..members.len() {
                let ia = index_of[members[a].as_str()];
                let ib = index_of[members[b].as_str()];
                let key = if ia < ib { (ia, ib) } else { (ib, ia) };
                if seen.insert(key) {
                    edges.push(key);
                }
            }
        }
    }
    (node_ids, edges)
}

/// Fruchterman–Reingold force-directed placement of `n` nodes given the
/// undirected `edges` (index pairs). `iterations` cooling steps, seeded RNG.
/// Every loop is bounded by `n`, `iterations`, or `edges.len()`.
fn fruchterman_reingold(
    n: usize,
    edges: &[(usize, usize)],
    iterations: usize,
    seed: u64,
) -> Vec<(f64, f64)> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|_| (rng.random_range(0.0..1.0), rng.random_range(0.0..1.0)))
        .collect();
    if n <= 1 {
        return pos;
    }
    let k = (1.0f64 / n as f64).sqrt();
    let mut t = 0.1f64;
    let dt = t / (iterations as f64 + 1.0);

    for _ in 0..iterations {
        let mut disp = vec![(0.0f64, 0.0f64); n];
        // Repulsive forces: every ordered pair.
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                let force = k * k / dist;
                disp[i].0 += dx / dist * force;
                disp[i].1 += dy / dist * force;
            }
        }
        // Attractive forces: each skeleton edge.
        for &(a, b) in edges {
            let dx = pos[a].0 - pos[b].0;
            let dy = pos[a].1 - pos[b].1;
            let dist = (dx * dx + dy * dy).sqrt().max(0.01);
            let force = dist * dist / k;
            let ox = dx / dist * force;
            let oy = dy / dist * force;
            disp[a].0 -= ox;
            disp[a].1 -= oy;
            disp[b].0 += ox;
            disp[b].1 += oy;
        }
        // Apply displacement, capped by the cooling temperature.
        for i in 0..n {
            let d = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1).sqrt().max(0.01);
            let capped = d.min(t);
            pos[i].0 += disp[i].0 / d * capped;
            pos[i].1 += disp[i].1 / d * capped;
        }
        t -= dt;
    }
    pos
}

/// Look up a node's position from an ordered position list (linear scan;
/// acceptable at Babylon's ~10^3-node scale).
fn pos_of(positions: &[(String, (f64, f64))], id: &str) -> (f64, f64) {
    positions
        .iter()
        .find(|(n, _)| n == id)
        .map(|(_, p)| *p)
        .unwrap_or((0.0, 0.0))
}

/// Spring layout: FR positions for agents; each hyperedge is a `Spline` through
/// its members' positions, in `members()` traversal order (deterministic
/// petgraph adjacency order — NOT insertion order; only `node_ids()`/
/// `edge_ids()` are true insertion order, see Phase 0+1 `members()`).
/// `seed = None` uses `DEFAULT_LAYOUT_SEED`.
pub fn spring_layout<N, E, M>(
    h: &Hypergraph<N, E, M>,
    iterations: usize,
    seed: Option<u64>,
) -> LayoutResult {
    let (node_ids, edges) = skeleton_edges(h);
    let seed = seed.unwrap_or(DEFAULT_LAYOUT_SEED);
    let raw = fruchterman_reingold(node_ids.len(), &edges, iterations, seed);
    let node_positions: Vec<(String, (f64, f64))> =
        node_ids.iter().cloned().zip(raw.iter().copied()).collect();

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let control_points: Vec<(f64, f64)> =
                members.iter().map(|m| pos_of(&node_positions, m)).collect();
            (eid.clone(), HyperedgeGeometry::Spline { control_points })
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
```

Note: `convex_hull` is imported here but only USED in Task 6 — until then it is
an unused import. That is a warning, not an error; `cargo test` (no
`-D warnings`) still passes. Task 11 runs clippy. If you prefer a clean build
between tasks, add the `use super::hull::convex_hull;` line in Task 6 instead.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 32 tests, 0 failures (warnings about the unused `convex_hull`
import and possible clippy-style lints are acceptable at this step).

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): spring_layout (Fruchterman-Reingold, seeded)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: HyperedgeAware layout (spring positions + convex-hull halos)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/spring.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::hyperedge_aware_layout<N,E,M>(h, iterations: usize,
  halo_padding: f64) -> LayoutResult`. Agent positions from the FR spring
  (seeded at `DEFAULT_LAYOUT_SEED` — the variant carries no seed field); each
  hyperedge is the `halo_padding`-expanded convex hull of its members.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::hyperedge_aware_layout;

#[test]
fn test_hyperedge_aware_positions_every_node() {
    let h = sample_h();
    let r = hyperedge_aware_layout(&h, 50, 0.05);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_hyperedge_aware_emits_convex_hull_geometry() {
    let h = sample_h();
    let r = hyperedge_aware_layout(&h, 50, 0.05);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::ConvexHull { .. }));
    }
}

#[test]
fn test_hyperedge_aware_deterministic() {
    let h = sample_h();
    assert_eq!(
        hyperedge_aware_layout(&h, 50, 0.05),
        hyperedge_aware_layout(&h, 50, 0.05)
    );
}

#[test]
fn test_hyperedge_aware_padding_grows_hull() {
    let h = sample_h();
    let no_pad = hyperedge_aware_layout(&h, 50, 0.0);
    let padded = hyperedge_aware_layout(&h, 50, 0.3);
    // The bounding box must grow (or stay equal) when the halo padding grows.
    let (nx0, ny0, nx1, ny1) = no_pad.bounding_box;
    let (px0, py0, px1, py1) = padded.bounding_box;
    let no_area = (nx1 - nx0) * (ny1 - ny0);
    let pad_area = (px1 - px0) * (py1 - py0);
    assert!(pad_area >= no_area - 1e-9);
}

#[test]
fn test_hyperedge_aware_hull_has_at_least_two_points_for_triangle_edge() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let r = hyperedge_aware_layout(&h, 80, 0.05);
    let (_, geom) = &r.edge_geometries[0];
    if let HyperedgeGeometry::ConvexHull { points } = geom {
        // >= 2, not == 3: FR-placed members could be (near-)collinear, which
        // collapses the hull to a 2-point segment. >= 2 is the safe bound.
        assert!(points.len() >= 2, "a 3-member edge should yield at least a degenerate (segment) hull");
    } else {
        panic!("expected ConvexHull geometry");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `hyperedge_aware_layout` does not exist.

- [ ] **Step 3: Implement hyperedge-aware layout + hull padding**

Add to `layout/mod.rs` the re-export:

```rust
pub use spring::hyperedge_aware_layout;
```

Append to `layout/spring.rs`:

```rust
/// Expand a convex hull outward from its centroid by `padding`. A degenerate
/// hull (single point) is returned unchanged; a 2-point hull is pushed apart
/// along its line.
fn pad_hull(hull: &[(f64, f64)], padding: f64) -> Vec<(f64, f64)> {
    if hull.is_empty() || padding == 0.0 {
        return hull.to_vec();
    }
    let count = hull.len() as f64;
    let cx = hull.iter().map(|p| p.0).sum::<f64>() / count;
    let cy = hull.iter().map(|p| p.1).sum::<f64>() / count;
    hull.iter()
        .map(|&(x, y)| {
            let dx = x - cx;
            let dy = y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 1e-9 {
                (x, y)
            } else {
                (x + dx / dist * padding, y + dy / dist * padding)
            }
        })
        .collect()
}

/// Hyperedge-aware layout: FR agent positions (seeded at `DEFAULT_LAYOUT_SEED`),
/// each hyperedge rendered as the `halo_padding`-expanded convex hull of its
/// members. `halo_padding` corresponds to XGI's halo "radius margin"
/// (`draw.py:199-200`, default 0.05).
pub fn hyperedge_aware_layout<N, E, M>(
    h: &Hypergraph<N, E, M>,
    iterations: usize,
    halo_padding: f64,
) -> LayoutResult {
    let (node_ids, edges) = skeleton_edges(h);
    let raw = fruchterman_reingold(node_ids.len(), &edges, iterations, DEFAULT_LAYOUT_SEED);
    let node_positions: Vec<(String, (f64, f64))> =
        node_ids.iter().cloned().zip(raw.iter().copied()).collect();

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let member_pts: Vec<(f64, f64)> =
                members.iter().map(|m| pos_of(&node_positions, m)).collect();
            let hull = convex_hull(&member_pts);
            let padded = pad_hull(&hull, halo_padding);
            (eid.clone(), HyperedgeGeometry::ConvexHull { points: padded })
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 37 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): hyperedge_aware_layout (spring + convex-hull halos)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: `compute_layout` dispatcher

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/layout/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_layout.rs`

**Interfaces:**
- Produces: `layout::compute_layout<N,E,M>(h, strategy: LayoutStrategy) ->
  LayoutResult` — the exhaustive dispatcher across all four strategies (spec §5.2).

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_layout.rs`:

```rust
use hypergraph_rs::layout::compute_layout;

#[test]
fn test_compute_layout_spring_arm() {
    let h = sample_h();
    let r = compute_layout(&h, LayoutStrategy::Spring { iterations: 50, seed: Some(42) });
    assert_eq!(r, spring_layout(&h, 50, Some(42)));
}

#[test]
fn test_compute_layout_bipartite_arm() {
    let h = sample_h();
    let r = compute_layout(&h, LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical });
    assert_eq!(r, bipartite_layout(&h, BipartiteAlign::Vertical));
}

#[test]
fn test_compute_layout_hyperedge_aware_arm() {
    let h = sample_h();
    let r = compute_layout(&h, LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert_eq!(r, hyperedge_aware_layout(&h, 50, 0.05));
}

#[test]
fn test_compute_layout_radial_arm() {
    let h = sample_h();
    let r = compute_layout(&h, LayoutStrategy::Radial { rings: 3 });
    assert_eq!(r, radial_layout(&h, 3));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: FAIL — `compute_layout` does not exist.

- [ ] **Step 3: Implement the dispatcher**

Add to `layout/mod.rs`, after the `bounding_box_of` helper (needs a
`use crate::core::hypergraph::Hypergraph;` at the top of `mod.rs`):

```rust
use crate::core::hypergraph::Hypergraph;

/// Dispatch a `LayoutStrategy` to its implementation (spec §5.2).
pub fn compute_layout<N, E, M>(h: &Hypergraph<N, E, M>, strategy: LayoutStrategy) -> LayoutResult {
    match strategy {
        LayoutStrategy::Spring { iterations, seed } => spring::spring_layout(h, iterations, seed),
        LayoutStrategy::Bipartite { align } => place::bipartite_layout(h, align),
        LayoutStrategy::HyperedgeAware { iterations, halo_padding } => {
            spring::hyperedge_aware_layout(h, iterations, halo_padding)
        }
        LayoutStrategy::Radial { rings } => place::radial_layout(h, rings),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_layout`
Expected: PASS — 41 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): compute_layout dispatcher over 4 strategies

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: `render()` — build the SceneGraph

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/viz/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_viz.rs`

**Interfaces:**
- Produces: `Hypergraph<Value,Value,M>::render(&self, strategy: LayoutStrategy)
  -> SceneGraph` (spec §5.3). Lives in a **concrete** `impl<M>
  Hypergraph<serde_json::Value, serde_json::Value, M>` block (it clones
  `&serde_json::Value` attrs into glyphs).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_viz.rs`:

```rust
use hypergraph_rs::layout::{BipartiteAlign, HyperedgeGeometry, LayoutStrategy};
use hypergraph_rs::Hypergraph;

fn sample_h() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"heat": 0.5}),
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_render_produces_glyph_per_node_and_edge() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert_eq!(scene.nodes.len(), h.num_nodes());
    assert_eq!(scene.hyperedges.len(), h.num_edges());
}

#[test]
fn test_render_preserves_node_insertion_order() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical });
    let ids: Vec<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}

#[test]
fn test_render_carries_edge_attrs_and_members() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let e1 = scene.hyperedges.iter().find(|g| g.id == "e1").unwrap();
    assert_eq!(e1.attrs["heat"], 0.5);
    // members() is deterministic petgraph adjacency order, not insertion
    // order — compare as a sorted set rather than assert a specific order.
    let mut m = e1.member_ids.clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_render_hyperedge_aware_geometry_is_convex_hull() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    for g in &scene.hyperedges {
        assert!(matches!(g.geometry, HyperedgeGeometry::ConvexHull { .. }));
    }
}

#[test]
fn test_render_bounding_box_well_formed() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let (min_x, min_y, max_x, max_y) = scene.bounding_box;
    assert!(max_x >= min_x && max_y >= min_y);
}

#[test]
fn test_render_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    assert!(scene.nodes.is_empty());
    assert!(scene.hyperedges.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: FAIL — `render` does not exist.

- [ ] **Step 3: Implement `render`**

Add to `viz/mod.rs` (top-of-file imports, merge with existing `use`):

```rust
use crate::core::hypergraph::Hypergraph;
use crate::layout::{compute_layout, LayoutStrategy};
```

Add the default node radius constant and the concrete impl block (append to
`viz/mod.rs`):

```rust
/// Default rendered node radius (layout units).
pub const DEFAULT_NODE_RADIUS: f64 = 0.02;

impl<M> Hypergraph<serde_json::Value, serde_json::Value, M> {
    /// Build a `SceneGraph` from a layout strategy (spec §5.3). Concrete impl:
    /// clones `serde_json::Value` node/edge attrs into the glyphs.
    pub fn render(&self, strategy: LayoutStrategy) -> SceneGraph {
        let layout = compute_layout(self, strategy);

        let nodes: Vec<NodeGlyph> = layout
            .node_positions
            .iter()
            .map(|(id, (x, y))| {
                let attrs = self.node_attrs(id).cloned().unwrap_or(serde_json::Value::Null);
                NodeGlyph {
                    id: id.clone(),
                    x: *x,
                    y: *y,
                    radius: DEFAULT_NODE_RADIUS,
                    attrs,
                    style: NodeStyle::default(),
                }
            })
            .collect();

        let hyperedges: Vec<HyperedgeGlyph> = layout
            .edge_geometries
            .iter()
            .map(|(eid, geom)| {
                let attrs = self.edge_attrs(eid).cloned().unwrap_or(serde_json::Value::Null);
                let member_ids = self.members(eid).unwrap_or_default();
                HyperedgeGlyph {
                    id: eid.clone(),
                    member_ids,
                    geometry: geom.clone(),
                    attrs,
                    style: HyperedgeStyle::default(),
                }
            })
            .collect();

        SceneGraph {
            nodes,
            hyperedges,
            bounding_box: layout.bounding_box,
            metadata: serde_json::Map::new(),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: PASS — 6 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): Hypergraph::render builds the SceneGraph

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 9: `render_to_json()`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/viz/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_viz.rs`

**Interfaces:**
- Produces: `Hypergraph<Value,Value,M>::render_to_json(&self, strategy) ->
  String` — serde_json string of the SceneGraph; round-trips back to `SceneGraph`.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_viz.rs`:

```rust
use hypergraph_rs::viz::SceneGraph;

#[test]
fn test_render_to_json_round_trips() {
    let h = sample_h();
    let json = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let scene: SceneGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(scene.nodes.len(), h.num_nodes());
    assert_eq!(scene.hyperedges.len(), h.num_edges());
}

#[test]
fn test_render_to_json_matches_render() {
    let h = sample_h();
    let strat = LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical };
    let direct = serde_json::to_string(&h.render(strat.clone())).unwrap();
    let via = h.render_to_json(strat);
    assert_eq!(direct, via);
}

#[test]
fn test_render_to_json_deterministic() {
    let h = sample_h();
    let a = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(7) });
    let b = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(7) });
    assert_eq!(a, b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: FAIL — `render_to_json` does not exist.

- [ ] **Step 3: Implement `render_to_json`**

Add inside the concrete `impl<M> Hypergraph<serde_json::Value, serde_json::Value, M>`
block in `viz/mod.rs`:

```rust
    /// Serialize the rendered scene-graph to JSON (spec §5.3).
    pub fn render_to_json(&self, strategy: LayoutStrategy) -> String {
        let scene = self.render(strategy);
        serde_json::to_string(&scene).unwrap_or_else(|_| "{}".to_string())
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: PASS — 9 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): render_to_json (SceneGraph JSON export)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 10: `render_to_svg()` — hand-rolled SVG emitter

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/src/viz/svg.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/viz/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_viz.rs`

**Interfaces:**
- Produces: `viz::svg::scene_to_svg(&SceneGraph) -> String` (hand-rolled, no
  external SVG crate — spec §5.1/§8) and
  `Hypergraph<Value,Value,M>::render_to_svg(&self, strategy) -> String`.

- [ ] **Step 1: Write the failing test**

Append to `crates/hypergraph-rs/tests/test_viz.rs`:

```rust
#[test]
fn test_render_to_svg_is_well_formed() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("viewBox="));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
}

#[test]
fn test_render_to_svg_has_a_circle_per_node() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let circles = svg.matches("<circle").count();
    assert_eq!(circles, h.num_nodes());
}

#[test]
fn test_render_to_svg_hyperedge_aware_has_polygon() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert!(svg.contains("<polygon"));
}

#[test]
fn test_render_to_svg_bipartite_has_line() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical });
    assert!(svg.contains("<line"));
}

#[test]
fn test_render_to_svg_radial_has_path() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Radial { rings: 3 });
    assert!(svg.contains("<path"));
}

#[test]
fn test_render_to_svg_deterministic() {
    let h = sample_h();
    let a = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(3) });
    let b = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(3) });
    assert_eq!(a, b);
}

#[test]
fn test_render_to_svg_empty_hypergraph_is_valid() {
    let h: Hypergraph = Hypergraph::new();
    let svg = h.render_to_svg(LayoutStrategy::Spring { iterations: 10, seed: Some(1) });
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: FAIL — `render_to_svg` / the `svg` module do not exist.

- [ ] **Step 3: Implement the SVG emitter**

Write to `hypergraph-rs/crates/hypergraph-rs/src/viz/svg.rs`:

```rust
//! Hand-rolled minimal SVG emitter for a `SceneGraph` (no external SVG crate).
//!
//! Coordinates are the layout's own units mapped through a `viewBox` with a
//! small margin. Note: SVG's y-axis grows downward while layout y grows upward;
//! consumers that need a "math" orientation can flip y — we emit layout coords
//! directly for a lossless round-trip with the scene-graph JSON.

use super::{HyperedgeStyle, SceneGraph};
use crate::layout::HyperedgeGeometry;

/// Emit a self-contained SVG document for a scene-graph.
pub fn scene_to_svg(scene: &SceneGraph) -> String {
    let (min_x, min_y, max_x, max_y) = scene.bounding_box;
    let span = (max_x - min_x).max(max_y - min_y).max(1.0);
    let margin = 0.1 * span;
    let vx = min_x - margin;
    let vy = min_y - margin;
    let vw = ((max_x - min_x) + 2.0 * margin).max(1.0);
    let vh = ((max_y - min_y) + 2.0 * margin).max(1.0);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">\n",
        fmt(vx),
        fmt(vy),
        fmt(vw),
        fmt(vh)
    ));

    // Hyperedges first (drawn under nodes).
    for he in &scene.hyperedges {
        let g = geometry_to_svg(&he.geometry, &he.style);
        if !g.is_empty() {
            s.push_str(&g);
            s.push('\n');
        }
    }
    // Nodes on top.
    for n in &scene.nodes {
        s.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />\n",
            fmt(n.x),
            fmt(n.y),
            fmt(n.radius),
            escape(&n.style.fill),
            escape(&n.style.stroke),
            fmt(n.style.stroke_width)
        ));
    }
    s.push_str("</svg>\n");
    s
}

fn geometry_to_svg(geom: &HyperedgeGeometry, style: &HyperedgeStyle) -> String {
    match geom {
        HyperedgeGeometry::ConvexHull { points }
        | HyperedgeGeometry::Spline { control_points: points } => {
            if points.is_empty() {
                return String::new();
            }
            let pts: String = points
                .iter()
                .map(|(x, y)| format!("{},{}", fmt(*x), fmt(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "  <polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" fill-opacity=\"{}\" />",
                pts,
                escape(&style.fill),
                escape(&style.stroke),
                fmt(style.opacity)
            )
        }
        HyperedgeGeometry::Stripe { x, y_min, y_max } => format!(
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-opacity=\"{}\" />",
            fmt(*x),
            fmt(*y_min),
            fmt(*x),
            fmt(*y_max),
            escape(&style.stroke),
            fmt(style.opacity)
        ),
        HyperedgeGeometry::Arc { center, radius, angle_range } => {
            let (a0, a1) = *angle_range;
            let (cx, cy) = *center;
            let x0 = cx + radius * a0.cos();
            let y0 = cy + radius * a0.sin();
            let x1 = cx + radius * a1.cos();
            let y1 = cy + radius * a1.sin();
            let large = if (a1 - a0).abs() > std::f64::consts::PI { 1 } else { 0 };
            format!(
                "  <path d=\"M {} {} A {} {} 0 {} 1 {} {}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{}\" />",
                fmt(x0),
                fmt(y0),
                fmt(*radius),
                fmt(*radius),
                large,
                fmt(x1),
                fmt(y1),
                escape(&style.stroke),
                fmt(style.opacity)
            )
        }
    }
}

/// Deterministic fixed-precision formatting (6 decimals, trailing zeros
/// trimmed, `-0` normalised to `0`).
fn fmt(v: f64) -> String {
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s
    }
}

/// Escape XML special characters in attribute values.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

Add to `viz/mod.rs` the submodule declaration (near the top):

```rust
pub mod svg;
```

Add `render_to_svg` inside the concrete `impl<M> Hypergraph<serde_json::Value,
serde_json::Value, M>` block:

```rust
    /// Render the scene-graph to a self-contained SVG string (spec §5.3).
    pub fn render_to_svg(&self, strategy: LayoutStrategy) -> String {
        let scene = self.render(strategy);
        svg::scene_to_svg(&scene)
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_viz`
Expected: PASS — 16 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): render_to_svg (hand-rolled SVG emitter)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 11: Clippy, fmt, feature-matrix build, full suite

**Files:**
- Modify: any files with clippy warnings

- [ ] **Step 1: Run clippy on the core crate (default features)**

Run: `cd hypergraph-rs && cargo clippy -p hypergraph-rs -- -D warnings`
Expected: may report lints. Fix all.

Likely lints and preferred fixes:
- **Unused import** `use super::hull::convex_hull;` in `spring.rs` — it IS used
  by `hyperedge_aware_layout` after Task 6, so no fix needed once Task 6 landed.
- **`clippy::needless_range_loop`** on `for i in 0..n { for j in 0..n {` in
  `fruchterman_reingold` — this indexes two distinct arrays (`pos[i]`, `pos[j]`,
  `disp[i]`) by the same indices, so a direct iterator does not apply. Add
  `#[allow(clippy::needless_range_loop)]` on the function with a one-line
  justification (dual/self-indexed arrays).
- **`clippy::type_complexity`** on `Vec<(String, (f64, f64))>` returns — if it
  fires, prefer an `#[allow(clippy::type_complexity)]` with a comment over
  introducing a type alias that would fragment the public signatures already
  fixed by this plan.
- **`clippy::manual_range_contains` / float-cmp** — the determinism tests
  intentionally compare `f64` for bit-equality (same-seed reproduction); if
  `clippy::float_cmp` fires in TEST code it is a false positive for our
  determinism contract — allow it locally in the test module with a comment.

- [ ] **Step 2: Verify the feature matrix compiles**

Run each and confirm success:

```bash
cd hypergraph-rs
cargo build -p hypergraph-rs --no-default-features --features layout
cargo build -p hypergraph-rs --no-default-features --features viz
cargo build -p hypergraph-rs --no-default-features
cargo build -p hypergraph-rs
```

Expected: all four succeed. `--features viz` must pull `layout` transitively
(the `viz = ["layout"]` dependency). `--no-default-features` (no layout, no viz)
must still compile the Phase 0+1 core.

- [ ] **Step 3: Run clippy across the workspace**

Run: `cd hypergraph-rs && cargo clippy --workspace -- -D warnings`
Expected: PASS — no warnings.

- [ ] **Step 4: Run the full test suite**

Run: `cd hypergraph-rs && cargo test --workspace`
Expected: PASS — Phase 0+1's 64 `test_hypergraph` tests + 41 `test_layout` +
16 `test_viz` = 121 tests in the core crate, 0 failures (the pyo3/wasm/cli stub
crates contribute 0 tests).

- [ ] **Step 5: Run rustfmt**

Run: `cd hypergraph-rs && cargo fmt --all`

- [ ] **Step 6: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs): clippy + fmt + feature-matrix cleanup (layout/viz)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 5 Completion Summary

After all 11 tasks the `hypergraph-rs` core crate has:

- **`layout/` module** (feature `layout`, default-on):
  - `LayoutStrategy` (4 variants), `BipartiteAlign`, `HyperedgeGeometry`
    (4 variants), `LayoutResult`, `DEFAULT_LAYOUT_SEED`.
  - `convex_hull` (Andrew monotone chain), `bounding_box_of`.
  - `spring_layout` (seeded Fruchterman–Reingold on the 1-skeleton),
    `hyperedge_aware_layout` (spring + padded convex-hull halos),
    `bipartite_layout` (deterministic two-column), `radial_layout`
    (deterministic concentric rings).
  - `compute_layout` dispatcher.
- **`viz/` module** (feature `viz` ⇒ `layout`, default-on):
  - `SceneGraph`, `NodeGlyph`, `HyperedgeGlyph`, `NodeStyle`, `HyperedgeStyle`
    (all serde round-trippable), `HyperedgeGeometry` re-export.
  - `Hypergraph<Value,Value,M>::render`, `render_to_json`, `render_to_svg`
    (concrete impl — Value-attr cloning).
  - `svg::scene_to_svg` hand-rolled emitter (circle/polygon/line/path).
- **57 new tests** (41 `test_layout` + 16 `test_viz`), all deterministic; total
  core-crate suite 121 tests. Clippy + fmt clean; feature matrix verified.

**Conformance note (Phase 7):** XGI's `tests/drawing/test_layout.py` position-value
assertions are adapted to STRUCTURAL assertions (see Deviation 2) — exact numpy
coordinate parity is not achievable with `rand_pcg`. `test_draw.py` /
`test_draw_utils.py` (matplotlib image-compare) are replaced by scene-graph tests
(spec §9.3).

**Next: Phase 6** — convert / dynamics / communities / utils.
