# Geometry for weighted sets and fixed placements: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> A fresh implementer builds one task, a fresh reviewer checks it, then the next begins.

**Goal:** Add to the core crate of hypergraph-rs a module `geometry`, behind a new feature `geometry`, that draws the
outline of a hyperedge from the numbers on its memberships, places nodes at a weighted mix of given positions, and
spreads overlapping points apart, all as pure functions that give the same bits on every machine.

**Architecture:** Two layers. The lower layer works on plain points and numbers (`barycentre`, `corner_mix`, `spread`,
`iso_lines`, `weighted_outline`); the upper layer reads a `Hypergraph` (`anchored_placement`, `edge_outline`) through
the reads the core already serves (`node_ids`, `members_with_data`). No core file changes. Nothing is random, nothing
iterates a hash map, and only `+ - * /`, `sqrt`, `abs`, `floor`, `min` and `max` touch a float, so results are bit
for bit the same everywhere. Meshes, hatching, moving marks and the Bevy plugin are the next plan's (step 3).

**Tech Stack:** Rust (pinned toolchain 1.100.0, floor `rust-version = "1.90"`), `indexmap` 2 and `thiserror` 2
(already plain dependencies of the core crate). No new dependency.

**Spec:** the approved design `2026-11-21-field-and-particles-design.md` (link in "Sources" below), sections 6.1,
6.5 to 6.8, 8, 10 (step 2), 11 and 12. The design speaks of a game; this plan and everything it builds speak only of
hypergraphs, nodes, hyperedges, memberships and numbers. The sketch `field-and-particles.html` (functions `contour`,
`drawMembranes`, `tiesStep`, `wellsStep`, `relax`, `wellTargets`) illustrates one working form; where it differs from
this plan (its spreading compares every pair, its jitter uses `Math.sin`, its placement eases over frames), this plan
binds.

**Sources (read-only):** the design, in the first user's repository, and the sketch:

```text
https://github.com/percy-raskova/babylon/blob/a2b9a482b/docs/superpowers/specs/2026-11-21-field-and-particles-design.md
The sketch is kept outside the repositories. The lead hands its path to builders and reviewers.
```

## Global Constraints

Carried by every plan for this library:

- The library is public and must stay usable by itself: nothing of any game in its code, tests, docs or examples.
  Test names, ids and doc examples use neutral words (`a`, `b`, `club`, `team`, `top`, `bottom`, corners `c0..c3`).
- Everything at its newest version: crates, the pinned toolchain, CI's Python, workflow actions. The minimum Rust
  version rises when a dependency needs it; nothing is held back. (This plan adds no dependency.)
- The gates are the repository's own: `mise run rust:check` (fmt, clippy with warnings denied, tests, rustdoc with
  warnings denied, all features, each feature alone), `mise run rust:msrv`, cargo-deny, CodeQL, and for the Python
  bindings `mise run py:gate`. A task ends with the narrowest of these that covers what it touched; the whole set runs
  once at the end of the plan (Task 7).
- One cargo or pytest process at a time on the machine. Never pass `-j` or `--jobs`.
- Commits: the repository has no git identity of its own. The lead supplies the author and committer through the
  environment (`GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`) on every commit. A
  builder never runs `git config` and never commits without that environment. Messages follow the history's style
  (`feat(geometry): ...`), with any trailer the lead names.
- No release to crates.io or PyPI. That is the owner's to order.
- The one known user (a game) takes the library by git revision with default features off and uses only `Hypergraph`
  and `subhypergraph`. No task may break a build with default features off.

This plan's own:

- Work on its own branch (`feat/geometry`), in a git worktree of its own (the lead names the directory). Never push.
  The local `main` and `origin/main` refs of the library's checkout are stale (they predate the merge of pull
  request 4, `4e05b4e6`). Run `git fetch origin` first and branch from the fetched `origin/main`. Wherever this
  plan says "the base", it means `$(git merge-base HEAD origin/main)`, never the local `main`.
- No core file changes: `src/core/**`, `src/layout/**`, `src/viz/**`, the Python and WASM crates are not touched.
  `layout::HyperedgeGeometry` gets no new variant (that enum is not `#[non_exhaustive]`; a variant would break users).
- The public API only grows. No existing signature or test expectation changes.
- No `unsafe`. No `HashMap`, `HashSet`, `rand`, clock or thread in `src/geometry/`. No `sin`, `cos`, `tan`, `exp`,
  `ln`, `powf`, `powi` or `mul_add` there: those are not bit-identical across platforms. Task 1 adds a test that
  enforces this over every file of the module.
- Every listing comes out in a stated order: node and member listings in the hypergraph's order, slice results in the
  input's order. Ties in any sort break by input index.
- A bad number is an error that names the item, never a silent zero: a weight that is negative or not finite, a
  position that is not finite, a parameter out of range.
- MSRV 1.90: no language or std feature newer than that (`f64::total_cmp`, let-else and `is_sorted` are fine).
- Tests are files directly under `crates/hypergraph-rs/tests/`, each starting `#![cfg(feature = "geometry")]`.
  One exception: the crate-private `inside_cell` (Task 4) is tested in a `#[cfg(test)] mod tests` at the end of
  `contour.rs`, because an integration test cannot reach it.
- An expected compile failure names the missing item; whether rustc says "unresolved import" or "cannot find"
  depends on how the test imports it. Either wording is the expected RED.
- Size: every function here must work for 25,000 points without any cost that grows with the square of their number.

## Review Focus

1. **Many points on one spot.** Nodes with the same ties sit at the same weighted mean, often hundreds together. The
   spread must separate them without comparing every pair, and without jumping (Task 3, `a_pile_of_400_...`).
2. **A set with almost nothing in it.** Every member at zero, or one member above zero among many at zero. Expect
   `Ok(None)`, or a ring around the one, and no lookup of a zero member's position: a zero member at `(NaN, NaN)`
   is not an error (Task 5, `a_membership_of_zero_is_outside`; Task 6, `an_edge_outline_reads_...`).
3. **Numbers of very different size.** One membership of 1e12 among ones, or every number scaled by the same
   factor. The shape depends only on the numbers relative to the largest (Task 5, `scaling_every_weight_...`).
4. **Members on the same spot inside a set.** Two members at one point make a capsule with equal ends; duplicates in a
   membrane must not produce empty or zero-area paths (Task 5, `three_members_on_one_spot_make_one_round_membrane`).
5. **Inputs that cannot be satisfied.** Bounds narrower than the spacing, an empty hypergraph, an empty slice, members
   so far apart that the grid's node count overflows `usize`. Expect no panic and no endless loop; the spread reports
   the overlaps it could not remove, the outline reports `GridTooLarge` (Tasks 2, 3 and 5).

## The public API this plan produces

All of it is in `hypergraph_rs::geometry`, behind the feature `geometry`, and re-exported from that module's root.
The next plan (the Bevy crate) is written from this block. Rustdoc on each item states the rules given in the tasks.

```rust
// geometry/mod.rs
pub type Point = (f64, f64);
pub type Rect = (f64, f64, f64, f64);           // (min_x, min_y, max_x, max_y), as layout::LayoutResult uses

// geometry/error.rs
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum GeometryError {
    #[error("weight {value} of {item} is negative or not finite")]
    BadWeight { item: String, value: f64 },
    #[error("position of {item} is not finite")]
    NonFinitePosition { item: String },
    #[error("parameter {name} = {value} is out of range")]
    BadParameter { name: &'static str, value: f64 },
    #[error("{item} has {found} numbers, expected {expected}")]
    CountMismatch { item: String, expected: usize, found: usize },
    #[error("edge {0} is not in the hypergraph")]
    UnknownEdge(String),
    #[error("member {node} of edge {edge} has a weight but no position")]
    NoPosition { edge: String, node: String },
    #[error("a grid of {nodes} nodes exceeds the limit of {limit}")]
    GridTooLarge { nodes: usize, limit: usize },
}
// In slice functions `item` is the index written as "#<index>"; in hypergraph functions it is the node or edge id.

// geometry/positions.rs
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Positions { /* IndexMap<String, Point>: insertion order, lookups by id */ }
impl Positions {
    pub fn new() -> Self;
    pub fn insert(&mut self, id: impl Into<String>, at: Point) -> Option<Point>; // keeps the first insertion's place
    pub fn get(&self, id: &str) -> Option<Point>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = (&str, Point)> + '_;          // insertion order
}
impl FromIterator<(String, Point)> for Positions;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placement {
    pub positions: Positions,   // placed items, in input order
    pub unplaced: Vec<String>,  // items with no positive weight, in input order; never put at the origin
}

// geometry/place.rs
pub fn barycentre(anchors: &[Point], weights: &[f64]) -> Result<Option<Point>, GeometryError>;
pub fn corner_mix(corners: &[Point], shares: &[(String, Vec<f64>)]) -> Result<Placement, GeometryError>;
pub fn anchored_placement<N, E, M>(
    h: &Hypergraph<N, E, M>,
    anchors: &Positions,                 // edge id -> fixed position
    weight: impl Fn(&M) -> f64,          // the number read from a membership's value
) -> Result<Placement, GeometryError>;

// geometry/spread.rs
#[derive(Clone, Debug, PartialEq)]
pub struct SpreadParams { pub min_distance: f64, pub steps: u32, pub max_neighbours: usize, pub bounds: Option<Rect> }
impl SpreadParams { pub fn new(min_distance: f64, steps: u32) -> Self; } // max_neighbours 32, bounds None
pub const DEFAULT_MAX_NEIGHBOURS: usize = 32;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpreadReport { pub comparisons: u64, pub overlaps_left: usize }
pub fn spread(positions: &mut Positions, obstacles: &[Point], params: &SpreadParams)
    -> Result<SpreadReport, GeometryError>;

// geometry/contour.rs
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarGrid { pub origin: Point, pub cell: f64, pub cols: usize, pub rows: usize, pub values: Vec<f64> }
impl ScalarGrid {
    pub fn zeros(origin: Point, cell: f64, cols: usize, rows: usize) -> Result<Self, GeometryError>;
    pub fn get(&self, col: usize, row: usize) -> f64;   // values[row * cols + col]; panics out of range
    pub fn sample(&self, at: Point) -> Option<f64>;     // bilinear; None outside the grid
}
pub type ClosedPath = Vec<Point>;                       // first vertex not repeated at the end
pub fn iso_lines(grid: &ScalarGrid, threshold: f64) -> Vec<ClosedPath>;
pub fn signed_area(path: &[Point]) -> f64;              // counter-clockwise positive
pub fn contains(paths: &[ClosedPath], at: Point) -> bool; // even-odd rule

// geometry/outline.rs
pub const DEFAULT_LEVEL_FRACTION: f64 = 0.5;
pub const DEFAULT_MAX_GRID_NODES: usize = 4_194_304;
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineParams { pub reach: f64, pub cell: f64, pub share: f64, pub level_fraction: f64,
                           pub max_grid_nodes: usize }
impl OutlineParams { pub fn new(reach: f64, cell: f64, share: f64) -> Self; } // the two defaults above
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Outline {
    Ring { center: Point, radius: f64 },
    Capsule { a: Point, radius_a: f64, b: Point, radius_b: f64 },
    Membrane { paths: Vec<ClosedPath> },   // outer paths counter-clockwise, holes clockwise; fill even-odd
}
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedOutline { pub outline: Outline, pub held: Vec<usize>, pub held_weight: f64, pub total_weight: f64 }
pub fn weighted_outline(members: &[(Point, f64)], params: &OutlineParams)
    -> Result<Option<WeightedOutline>, GeometryError>;
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeOutline { pub edge: String, pub members: Vec<String>, pub outline: WeightedOutline }
pub fn edge_outline<N, E, M>(h: &Hypergraph<N, E, M>, edge_id: &str, positions: &Positions,
    weight: impl Fn(&M) -> f64, params: &OutlineParams) -> Result<Option<EdgeOutline>, GeometryError>;
```

**How a caller keeps positions stable from one call to the next.** Nothing is kept between calls: every result is a
function of its input alone (design 6.7, "a measurement, not a simulation"). Stability comes from four rules, each
pinned by a test: results are keyed by id, in the input's order (`Positions`); the push that separates two points on
one spot is chosen from the two ids, never from their indices, so adding or removing a far-away node changes no other
node's bits; a membrane's grid sits on a lattice of `cell` fixed at the origin `(0, 0)`, so a set's outline does not
shimmer when its extent changes; and the same input gives the same bits on every run. Easing between periods belongs
to the caller (the Bevy plan).

**What the core must serve.** Nothing new. The upper layer reads `Hypergraph::node_ids`, `edge_ids`,
`members_with_data` and the membership value type `M` through a caller's `Fn(&M) -> f64`. No new core method is
added, so `src/core/hypergraph.rs`, which the XGI 1.1 plan may touch, stays out of this plan.

**What is left to the Bevy plan (step 3), on purpose.** Meshing of rings, capsules and membranes; hatching and its
thickness; drawing overlapping sets; marks moving along paths by a clock, and the curve of a path; picking which paths
to draw until a share of a stream is shown; easing; telling a change of membership from a change of number; the
fixed spot of each kind of node around a place's centre; the plugin and the example. Each needs a screen or a clock,
or is a choice of picture rather than of geometry.

## File Structure

| File | Responsibility | Task |
| --- | --- | --- |
| `crates/hypergraph-rs/Cargo.toml` | the line `geometry = []`; `geometry` added to `default` and `all` | 1 (shared, see below) |
| `crates/hypergraph-rs/src/lib.rs` | `#[cfg(feature = "geometry")] pub mod geometry;`; one feature-table row | 1, 7 (shared) |
| `crates/hypergraph-rs/src/geometry/mod.rs` (new) | module docs and doctest, `Point`, `Rect`, re-exports | 1, 7 |
| `crates/hypergraph-rs/src/geometry/error.rs` (new) | `GeometryError` | 1 |
| `crates/hypergraph-rs/src/geometry/positions.rs` (new) | `Positions`, `Placement` | 1 |
| `crates/hypergraph-rs/src/geometry/place.rs` (new) | `barycentre`, `corner_mix`, `anchored_placement` | 1, 2 |
| `crates/hypergraph-rs/src/geometry/spread.rs` (new) | `spread` and its parameters and report | 3 |
| `crates/hypergraph-rs/src/geometry/contour.rs` (new) | `ScalarGrid`, `iso_lines`, `signed_area`, `contains`, `inside_cell` and its unit test | 4 |
| `crates/hypergraph-rs/src/geometry/outline.rs` (new) | `weighted_outline`, `edge_outline` and their types | 5, 6 |
| `crates/hypergraph-rs/tests/test_geometry_rules.rs` (new) | forbidden calls and types in `src/geometry/` | 1 |
| `crates/hypergraph-rs/tests/test_geometry_place.rs` (new) | placements | 1, 2 |
| `crates/hypergraph-rs/tests/test_geometry_spread.rs` (new) | spreading | 3 |
| `crates/hypergraph-rs/tests/test_geometry_contour.rs` (new) | iso-lines | 4 |
| `crates/hypergraph-rs/tests/test_geometry_outline.rs` (new) | outlines, points and hypergraph | 5, 6 |
| `README.md`, `CHANGELOG.md` | one feature-table row; one "Added" entry | 7 (shared) |

**Files the XGI 1.1 plan also touches.** `crates/hypergraph-rs/Cargo.toml` (`[features]`), `src/lib.rs` (module list
and feature table), `README.md` (feature table) and `CHANGELOG.md`. This plan only adds lines to them: one feature
line and two list entries, one `mod` line, one table row in each table, one changelog entry under the topmost
unreleased section. Whichever branch lands second rebases and keeps both sides' lines. No other file is shared.

---

### Task 1: The feature, the module, positions and the two plain placements

**Files:**
- Modify: `crates/hypergraph-rs/Cargo.toml` (`[features]`), `crates/hypergraph-rs/src/lib.rs` (module list)
- Create: `src/geometry/mod.rs`, `src/geometry/error.rs`, `src/geometry/positions.rs`, `src/geometry/place.rs`
- Test: `tests/test_geometry_rules.rs`, `tests/test_geometry_place.rs` (Create)

**Interfaces:**
- Consumes: nothing.
- Produces: `Point`, `Rect`, `GeometryError`, `Positions`, `Placement`, `barycentre`, `corner_mix`, exactly as in "The
  public API". `positions.rs` also gives the crate `pub(crate) fn point_mut(&mut self, index: usize) -> &mut Point`
  and `pub(crate) fn id_at(&self, index: usize) -> &str` for Task 3.

**Decisions:**
- *Feature:* `geometry = []`, added to `default` (after `raster`) and to `all`. It needs no other feature and no
  optional crate, so a build with default features off gains nothing.
- *`barycentre`:* `(Σ wᵢ·xᵢ / Σ wᵢ, Σ wᵢ·yᵢ / Σ wᵢ)`, summed in index order. `Ok(None)` when every weight is zero or
  the slices are empty. Different lengths give `CountMismatch { item: "weights", .. }`.
- *`corner_mix`:* each row is one item's shares over the corners, not required to sum to one. Each row goes through
  `barycentre`; a row of zeros is unplaced. A row of the wrong length gives `CountMismatch` with the item's id.
  Validation errors name the item's id. Corners that are not finite give `NonFinitePosition { item: "#<k>" }`.
- *Equality of `Positions`:* the derived `PartialEq` compares the inner `IndexMap`, which ignores order. That is
  kept (it is the map's meaning). Every test of order compares `iter()` listings, never `==`.

- [ ] **Step 1: Write the failing tests.**
  `tests/test_geometry_rules.rs`, one test `geometry_module_uses_only_portable_arithmetic`: walk every `.rs` file
  under `concat!(env!("CARGO_MANIFEST_DIR"), "/src/geometry")` with `std::fs::read_dir`, assert there is at least one,
  and assert no line contains any of `HashMap`, `HashSet`, `rand::`, `rand_pcg`, `rand_core`, `thread::`, `rayon`,
  `.sin(`, `.cos(`, `.tan(`, `.sin_cos(`, `.asin(`, `.acos(`, `.atan(`, `.atan2(`, `.sinh(`, `.cosh(`, `.tanh(`, `.exp(`, `.exp2(`,
  `.exp_m1(`, `.ln(`, `.ln_1p(`, `.log`, `.powf(`, `.powi(`, `.hypot(`, `.cbrt(`, `mul_add`, `Instant`, `SystemTime`,
  `unsafe`. Lines whose trimmed text starts with `//` are skipped, so the module docs may name the rule (Task 7
  writes them). The failure message names the file, the line number and the word.

  `tests/test_geometry_place.rs`:

  ```rust
  #[test] fn barycentre_weighs_each_anchor() {
      assert_eq!(barycentre(&[(0.0, 0.0), (4.0, 0.0)], &[3.0, 1.0]).unwrap(), Some((1.0, 0.0)));
  }
  #[test] fn barycentre_of_nothing_is_none() {
      assert_eq!(barycentre(&[(1.0, 1.0)], &[0.0]).unwrap(), None);
      assert_eq!(barycentre(&[], &[]).unwrap(), None);
  }
  #[test] fn barycentre_refuses_bad_numbers() {
      assert!(matches!(barycentre(&[(0.0, 0.0)], &[-1.0]), Err(GeometryError::BadWeight { .. })));
      assert!(matches!(barycentre(&[(0.0, 0.0)], &[f64::NAN]), Err(GeometryError::BadWeight { .. })));
      assert!(matches!(barycentre(&[(f64::INFINITY, 0.0)], &[1.0]), Err(GeometryError::NonFinitePosition { .. })));
      assert!(matches!(barycentre(&[(0.0, 0.0)], &[1.0, 1.0]), Err(GeometryError::CountMismatch { .. })));
  }
  // corners c0 (0,1), c1 (1,1), c2 (0,0), c3 (1,0)
  #[test] fn a_whole_share_stands_exactly_on_its_corner() {
      // rows: "a" [1,0,0,0]; "b" [0,0,0.5,0.5]; "c" [2,2,0,0]; "d" [0,0,0,0]
      // a.to_bits() of each coordinate == (0.0, 1.0); b == (0.5, 0.0); c == (0.5, 1.0)
      // positions order == ["a", "b", "c"]; unplaced == ["d"]
  }
  #[test] fn corner_mix_refuses_a_row_of_the_wrong_length() {
      // row "x" with 3 numbers over 4 corners -> CountMismatch { item: "x", expected: 4, found: 3 }
  }
  #[test] fn positions_keep_insertion_order_and_first_place() {
      // insert "b", "a", then "b" again with a new point: iter order ["b", "a"], get("b") == the new point,
      // insert returned the old point
  }
  ```

- [ ] **Step 2: Add the feature line, then run and see them fail.** First add `geometry = []` to `[features]` and
  to `default` and `all` (see Decisions), nothing else. Without it, `#![cfg(feature = "geometry")]` removes both
  test files and the run passes with zero tests, which proves nothing.
  Run: `cargo test -p hypergraph-rs --locked --test test_geometry_place --test test_geometry_rules`
  Expected: FAIL to compile, "unresolved import `hypergraph_rs::geometry`".

- [ ] **Step 3: Implement** the `mod` line in `lib.rs`, and the types and two functions of "Produces" in their
  files. `mod.rs` gets a short module doc (what the module is for, the determinism rule, the
  stability rules); Task 7 completes it.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Prove the default-off build is untouched.**
  Run, in order: `cargo check -p hypergraph-rs --no-default-features --locked`;
  `cargo check -p hypergraph-rs --no-default-features --features geometry --locked`;
  `git diff "$(git merge-base HEAD origin/main)" -- Cargo.lock` (expected: empty);
  `git diff "$(git merge-base HEAD origin/main)" -- crates/hypergraph-rs/Cargo.toml` (expected: only the
  `geometry` line and the two list entries in `[features]`; `[dependencies]` untouched). Expected: both checks pass.

- [ ] **Step 6: Scoped gate, then commit.** `cargo fmt --all -- --check`;
  `cargo clippy -p hypergraph-rs --all-targets --all-features --locked -- -D warnings`;
  `RUSTDOCFLAGS='-D warnings' cargo doc -p hypergraph-rs --all-features --no-deps --locked`. Then
  `git add` the files above and commit `feat(geometry): weighted mixes of fixed points behind a new feature`.

---

### Task 2: Placement among anchored hyperedges

**Files:**
- Modify: `crates/hypergraph-rs/src/geometry/place.rs`
- Test: `crates/hypergraph-rs/tests/test_geometry_place.rs`

**Interfaces:**
- Consumes: `Positions`, `Placement`, `GeometryError` (Task 1); `Hypergraph::node_ids`, `Hypergraph::edge_ids`,
  `Hypergraph::members_with_data(&self, edge_id: &str) -> Option<Vec<(String, &M)>>` (core, unchanged).
- Produces: `anchored_placement`, as in "The public API".

**Decisions:**
- *An anchor is a hyperedge with a fixed position.* A node stands at the mean of the positions of the anchored
  hyperedges it belongs to, weighted by `weight(&membership value)`. This is design 6.7's "between its employers and
  whoever holds a claim on it, nearer the larger share", in hypergraph terms. Memberships of hyperedges that are not
  anchors are ignored.
- *Order:* sums run over `anchors` in their order, then over each anchor's members in `members_with_data` order.
  `positions` lists placed nodes in `node_ids()` order; `unplaced` lists the rest in `node_ids()` order.
- *Errors:* an anchor id not in `h` gives `UnknownEdge`; an anchor position not finite gives `NonFinitePosition`
  with the edge id; a bad weight gives `BadWeight` with the node id.
- *Cost:* one pass over the anchored memberships; the per-node sums live in a `Vec` indexed by the node's position in
  `node_ids()`, found through an `IndexMap<&str, usize>` built once (lookups only).

- [ ] **Step 1: Write the failing tests** in `tests/test_geometry_place.rs`, on `Hypergraph<(), (), f64>`:

  ```rust
  #[test] fn a_node_stands_nearer_its_heavier_anchor() {
      // edges added in this order: "top" {a: 1.0}, "bottom" {a: 3.0, c: 0.0}, "loose" {b: 5.0}
      // (so node order is a, c, b); anchors: "top" -> (0.0, 10.0), "bottom" -> (0.0, 0.0)
      // positions == [("a", (0.0, 2.5))]; unplaced == ["c", "b"]
  }
  #[test] fn anchored_placement_follows_node_order() {
      // nodes added z, y, x, each in "top" with weight 1 -> positions ids == ["z", "y", "x"]
  }
  #[test] fn an_unknown_anchor_is_refused() {
      // anchors has "nope" -> Err(UnknownEdge("nope"));
      // anchors "top" -> (f64::NAN, 0.0) -> Err(NonFinitePosition { item: "top" })
  }
  #[test] fn a_bad_membership_number_names_the_node() {
      // "top" {a: -2.0} -> Err(BadWeight { item: "a", value: -2.0 })
  }
  #[test] fn an_empty_hypergraph_places_nothing() {
      // empty h, empty anchors -> Placement::default()
  }
  #[test] fn the_same_input_gives_the_same_bits_twice() {
      // build the same 200-node, 12-anchor hypergraph twice from a fixed linear congruential generator in the test
      // (seed 20261121, weights k/7 for small integers k); compare every coordinate with to_bits()
  }
  ```

- [ ] **Step 2: Run and see them fail.**
  Run: `cargo test -p hypergraph-rs --locked --test test_geometry_place`
  Expected: FAIL to compile, "cannot find function `anchored_placement`".

- [ ] **Step 3: Implement `anchored_placement`** in `place.rs`, through `barycentre`'s rules for each node.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 1, Step 6), then commit** `feat(geometry): place nodes among anchored
  hyperedges by the numbers on their memberships`.

---

### Task 3: Spreading by fixed steps

**Files:**
- Create: `crates/hypergraph-rs/src/geometry/spread.rs`
- Modify: `crates/hypergraph-rs/src/geometry/mod.rs` (re-exports)
- Test: `crates/hypergraph-rs/tests/test_geometry_spread.rs` (Create)

**Interfaces:**
- Consumes: `Positions` with `point_mut` and `id_at` (Task 1), `Point`, `Rect`, `GeometryError`.
- Produces: `SpreadParams`, `SpreadParams::new`, `DEFAULT_MAX_NEIGHBOURS`, `SpreadReport`, `spread`.

**Decisions (the algorithm; the builder writes the body):**
- *Validation:* `min_distance` finite and above zero; `max_neighbours` at least 1; bounds finite with
  `min <= max`; every position finite (`NonFinitePosition` with the id); obstacles finite (`"#<k>"`). `steps = 0`
  changes nothing.
- *One step:* put every point in a square cell of side `D = min_distance`, keyed `(floor(y/D), floor(x/D))` as
  `i64`, in a `Vec<(i64, i64, usize)>` sorted by that key then by index. For each point `i` in index order, visit the
  nine cells around its own (rows −1..=1, then columns −1..=1), and in each the points `j > i` in index order, until
  `max_neighbours` candidates have been examined for `i`. Each examined pair counts one comparison.
- *A close pair* (`d < D`) is pushed apart by `(D − d) / 2` each, along the line between them. When `d == 0` the
  direction `u` is
  `DIRECTIONS[(fnv1a(smaller_id).wrapping_mul(31) ^ fnv1a(larger_id)) & 15]` ("smaller" by byte order of the ids):
  the larger id moves `+u · D/2` and the smaller `−u · D/2`.
- *Indices* are positions in `Positions` (insertion order). `bounds`, when invalid, is `BadParameter` with
  `name: "bounds"`.
- *Obstacles* use their own sorted cell list. A point closer than `D` to an obstacle, at the start of the step, is
  pushed out to `D` from it; on the obstacle exactly it moves along `DIRECTIONS[fnv1a(id) & 15]`. Obstacles never
  move. Obstacle checks also count as comparisons and are not capped.
- *Applying:* pair displacements are summed in the order above and each point's sum is cut to length `D / 2` at
  most; the obstacle pushes are then added in full; all points move at once after the pass. Then `bounds`, if any,
  clamps each point.
- *Report:* `comparisons` over all steps; `overlaps_left` is the number of pairs examined after the last step (with the
  same cap) that are still closer than `D * (1.0 - 1e-9)`. That final count is not added to `comparisons`.
- *Helpers, private:* `fnv1a(bytes) -> u64` (64-bit FNV-1a: offset 0xcbf29ce484222325, prime 0x100000001b3) and the
  sixteen unit vectors at multiples of 22.5°, written as literals: `(1.0, 0.0)`, `(0.9238795325112867,
  0.3826834323650898)`, `(0.7071067811865476, 0.7071067811865476)`, `(0.3826834323650898, 0.9238795325112867)`,
  `(0.0, 1.0)`, and so on round the circle with signs.
- *Why:* a fixed number of steps in a fixed order is design 6.7; the cell grid and the neighbour cap keep the cost
  at `n × max_neighbours × steps`, never `n²`; choosing the push from ids is the stability rule.

- [ ] **Step 1: Write the failing tests** in `tests/test_geometry_spread.rs`:

  ```rust
  #[test] fn a_close_pair_ends_one_distance_apart() {
      // "a" (0,0), "b" (0.2,0); SpreadParams::new(1.0, 1) -> distance between them == 1.0 within 1e-12,
      // midpoint unchanged (0.1, 0.0) within 1e-12, overlaps_left == 0
  }
  #[test] fn points_on_one_spot_part_by_their_ids_not_their_order() {
      // ["x","y"] both at (0,0) vs ["y","x"] both at (0,0), new(1.0, 4): per id, both coordinates equal by to_bits()
      // and the two are at least 1.0 - 1e-9 apart
  }
  #[test] fn a_pile_of_400_on_one_spot_spreads_without_comparing_every_pair() {
      // ids "n000".."n399" at (5,5), new(0.1, 60): report.comparisons <= 400 * 32 * 60;
      // no two points share a position (sort by (x, y) bits, check neighbours differ);
      // every point moved at most 60 * 0.05 from (5,5)
  }
  #[test] fn a_far_node_changes_no_other_nodes_bits() {
      // 50 points on a line 0.3 apart from a fixed table; spread with new(1.0, 10);
      // again with "far" at (1000,1000) inserted FIRST: every one of the 50 equal by to_bits()
  }
  #[test] fn obstacles_push_points_off_and_never_move() {
      // "a" at (2,2), obstacle (2,2); new(1.0, 1) -> distance from (2,2) == 1.0 within 1e-12
  }
  #[test] fn bounds_too_narrow_are_reported_not_looped() {
      // 10 points at (0,0), bounds (0,0,0.5,0.5), new(1.0, 20) -> Ok; all inside bounds; overlaps_left > 0
  }
  #[test] fn zero_steps_and_empty_input_change_nothing() { /* both Ok, positions equal before and after */ }
  #[test] fn bad_parameters_are_refused() {
      // min_distance 0.0, NaN; max_neighbours 0; bounds (1, 0, 0, 1) -> BadParameter;
      // a position (NaN, 0) -> NonFinitePosition { item: id }; an obstacle (NaN, 0) -> NonFinitePosition { item: "#0" }
  }
  #[test] fn twenty_five_thousand_points_stay_within_the_cost_bound() {
      // 25_000 points on a 0.05 lattice (158 per row), new(0.1, 10): comparisons <= 25_000 * 32 * 10 + obstacles(0)
  }
  #[test] fn the_same_input_gives_the_same_bits_twice() { /* 500 points from a fixed LCG, run twice, to_bits() */ }
  ```

- [ ] **Step 2: Run and see them fail.**
  Run: `cargo test -p hypergraph-rs --locked --test test_geometry_spread`
  Expected: FAIL to compile, "cannot find function `spread`".

- [ ] **Step 3: Implement `spread`** in `spread.rs` by the decisions above, and re-export it from `mod.rs`.

- [ ] **Step 4: Run and see them pass.** The same command, then
  `cargo test -p hypergraph-rs --locked --test test_geometry_rules`. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 1, Step 6), then commit** `feat(geometry): spread points apart in fixed steps`.

---

### Task 4: Iso-lines of a scalar grid

**Files:**
- Create: `crates/hypergraph-rs/src/geometry/contour.rs`
- Modify: `crates/hypergraph-rs/src/geometry/mod.rs` (re-exports)
- Test: `crates/hypergraph-rs/tests/test_geometry_contour.rs` (Create)

**Interfaces:**
- Consumes: `Point`, `GeometryError` (Task 1).
- Produces: `ScalarGrid` (`zeros`, `get`, `sample`), `ClosedPath`, `iso_lines`, `signed_area`, `contains`.
  Crate-private for Task 5: `pub(crate) fn inside_cell(grid: &ScalarGrid, threshold: f64, at: Point) -> bool`, true
  exactly when `contains(&iso_lines(grid, threshold), at)` would be, decided from the one cell holding `at`.

**Decisions:**
- *Grid:* node `(col, row)` stands at `origin + (col·cell, row·cell)` and holds `values[row·cols + col]`. `zeros`
  refuses `cell` not finite or not above zero, an origin not finite, and `cols` or `rows` of zero (`BadParameter`).
- *Inside means strictly above:* a node is inside when its value is `> threshold`.
- *Marching squares* over every cell, with one virtual ring of nodes around the grid that is always outside. A
  crossing on an edge between two real nodes is placed by linear interpolation; a crossing on an edge to a virtual
  node sits at that edge's midpoint (half a cell outside the grid). So every path closes.
- *Saddles* (two opposite corners inside): the cell's centre value is the mean of its four corners; if it is
  `> threshold` the two inside corners are joined, otherwise kept apart.
- *Output order and orientation:* segments are joined into closed paths; each path starts at its vertex that comes
  first in row-major order of crossing edges (horizontal edges of a row before its vertical edges); paths are listed
  in that order. Each path is turned so that the inside is on its left: outer boundaries counter-clockwise (positive
  `signed_area`), holes clockwise. No vertex repeats; no path has fewer than three vertices.
- *Joining cost:* segment ends are matched through a `Vec` indexed by crossing-edge number (each edge of the grid,
  virtual ring included, has a fixed number), never by searching the segments, so `iso_lines` is linear in the
  number of grid nodes.
- *A malformed grid:* the fields are public, so `iso_lines`, `sample` and `inside_cell` panic when
  `values.len() != cols * rows`, and say so under `# Panics`. A `NaN` value is outside (it is not `> threshold`).
- `contains` is the even-odd rule over all paths; `signed_area` is the shoelace formula.
- *`inside_cell`* computes the cell's crossings with the same function `iso_lines` uses, so it agrees with
  `contains` bit for bit except for a point lying exactly on a path.

- [ ] **Step 1: Write the failing tests** in `tests/test_geometry_contour.rs`:

  ```rust
  #[test] fn one_node_above_gives_one_diamond() {
      // grid of cols 1, rows 1 at origin (0,0), cell 2.0, value 1.0, threshold 0.5:
      // one path of 4 vertices {(1,0),(0,1),(-1,0),(0,-1)} (any start), signed_area == 2.0
  }
  #[test] fn crossings_are_interpolated() {
      // grid of cols 2, rows 1, cell 1.0, values [1.0, 0.0], threshold 0.25: the path has the vertex (0.75, 0.0)
  }
  #[test] fn a_saddle_is_joined_when_its_centre_is_inside() {
      // 2x2 grid, cell 1.0, values [1.0, 0.0, 0.0, 1.0], threshold 0.5 -> 2 paths
      // values [1.0, 0.2, 0.2, 1.0], threshold 0.5 -> 1 path
  }
  #[test] fn a_hole_runs_clockwise() {
      // 5x5 grid, cell 1.0, every value 1.0 except the centre (2,2) at 0.0, threshold 0.5:
      // 2 paths, one positive area, one negative; contains(paths, (2.0, 2.0)) == false; contains(.., (0.5, 0.5))
  }
  #[test] fn sample_is_bilinear_and_none_outside() {
      // 2x2 cell 1.0 values [0,1,2,3]: sample((0.5,0.5)) == Some(1.5); sample((-0.1, 0.0)) == None
  }
  #[test] fn the_same_grid_gives_the_same_bits_twice() { /* 40x30 grid from a fixed LCG, to_bits() on all vertices */ }
  #[test] fn zeros_refuses_a_bad_grid() { /* cell 0.0, cell NaN, cols 0 -> BadParameter */ }
  ```

  And at the end of `src/geometry/contour.rs`, `#[cfg(test)] mod tests` with one test:

  ```rust
  #[test] fn inside_cell_agrees_with_contains() {
      // the 40x30 LCG grid above, threshold 0.5; 2_000 points from a second LCG stream, each strictly inside the
      // grid's extent: for every point, inside_cell(&grid, 0.5, p) == contains(&iso_lines(&grid, 0.5), p)
  }
  ```

- [ ] **Step 2: Run and see them fail.**
  Run: `cargo test -p hypergraph-rs --locked --test test_geometry_contour`
  Expected: FAIL to compile, "cannot find type `ScalarGrid`". (The unit test is written with `contour.rs` in Step 3
  and runs with `cargo test -p hypergraph-rs --locked --lib geometry::contour`.)

- [ ] **Step 3: Implement** the items of "Produces" in `contour.rs`, and re-export the public ones.

- [ ] **Step 4: Run and see them pass.** The same command, then
  `cargo test -p hypergraph-rs --locked --lib geometry::contour`, then the rules test. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 1, Step 6), then commit** `feat(geometry): iso-lines of a scalar grid`.

---

### Task 5: The weighted outline of a set of points

**Files:**
- Create: `crates/hypergraph-rs/src/geometry/outline.rs`
- Modify: `crates/hypergraph-rs/src/geometry/mod.rs` (re-exports)
- Test: `crates/hypergraph-rs/tests/test_geometry_outline.rs` (Create)

**Interfaces:**
- Consumes: `ScalarGrid`, `iso_lines`, `inside_cell`, `ClosedPath` (Task 4); `Point`, `GeometryError` (Task 1).
- Produces: `DEFAULT_LEVEL_FRACTION`, `DEFAULT_MAX_GRID_NODES`, `OutlineParams`, `OutlineParams::new`, `Outline`,
  `WeightedOutline`, `weighted_outline`.

**Decisions (the rules of design 6.7, in the library's terms):**
- *Zero is outside.* Only members with weight `> 0` take part. Their positions must be finite; a zero member's
  position is never read. With no positive member the result is `Ok(None)`. `total_weight` is the sum of the positive
  weights, in index order.
- *Relative weight:* `ŵᵢ = wᵢ / w_max`, so only the numbers relative to the largest count.
- *Validation:* `reach` finite and `> 0`; `cell` finite, `> 0` and `<= reach / 2`; `share` in `(0, 1]`;
  `level_fraction` in `(0, 1]`; each a `BadParameter` naming the field.
- *The field:* `f(x) = Σ ŵᵢ · k(|x − pᵢ| / reach)` with `k(q) = (1 − q²)²` for `q < 1`, else 0, sampled on a
  `ScalarGrid` whose nodes lie on the lattice of `cell` through `(0, 0)` and which covers every positive member's
  position widened by `reach + cell` on each side. If `cols × rows > max_grid_nodes` the result is `GridTooLarge`,
  checked before anything is allocated. `cols` and `rows` are first worked out in `f64`; a count that is not finite,
  does not fit a `usize`, or whose product overflows (`checked_mul`) is also `GridTooLarge` (with `nodes:
  usize::MAX`), never a panic. Splatting visits only the nodes within `reach` of each member.
- *A member's level* is `grid.sample(pᵢ)`.
- *Which members the outline must hold:* sort positive members by level, highest first, ties by index; take the
  shortest prefix whose weights sum to at least `share × total_weight`. Its length `k` chooses the form (design 6.7):
  - `k == 1`: `Ring` round that member, radius `reach × (0.5 + 0.5 × ŵ)`.
  - `k == 2`: `Capsule` with `a` the lower index of the two, each end's radius `reach × (0.5 + 0.5 × ŵ)`.
  - `k >= 3`: `Membrane { paths: iso_lines(&grid, T) }` with `T = level_fraction × (level of the k-th member)`.
- *`held` is measured, not assumed:* the indices (ascending) of every positive member inside the outline as drawn: in
  the circle, in the capsule (distance to the segment `ab` at most the radius interpolated along it), or by
  `inside_cell` for a membrane. `held_weight` sums their weights. If a membrane holds less than
  `share × total_weight`, `T` steps down: the candidates are `T_j = level_fraction × (level of the j-th member)` for
  `j = k..=n` in the sorted order. Measuring `held` at one `T_j` is one pass of `inside_cell` over the positive
  members (no drawing). The chosen `j` is the smallest whose `held_weight` reaches the share, found by bisection over
  `j` (held is taken to grow as `T` falls); if even `T_n` falls short, `T_n` is used and `held` reports what it holds.
  `iso_lines` runs once, at the chosen `T`. So the search costs `O(n log n)` plus one drawing, never one drawing per
  member (that would be `n ×` grid size, square in `n`). This keeps the stated share true of the picture.
- *Thicker where the number is larger:* in a ring or capsule through the radius rule; in a membrane because the
  field reaches further around a heavier member.

- [ ] **Step 1: Write the failing tests** in `tests/test_geometry_outline.rs`, with `OutlineParams::new(1.0, 0.125,
  share)` unless stated:

  ```rust
  #[test] fn one_member_is_a_ring() {
      // [((3,4), 2.0)], share 0.9 -> Ring { center: (3,4), radius: 1.0 }; held [0]; held_weight 2.0
  }
  #[test] fn two_members_are_a_capsule_thicker_at_the_heavier_end() {
      // [((0,0), 2.0), ((5,0), 1.0)], share 1.0 -> Capsule { a: (0,0), radius_a: 1.0, b: (5,0), radius_b: 0.75 }
  }
  #[test] fn two_members_on_one_spot_are_a_capsule_with_equal_ends() {
      // [((1,1), 1.0), ((1,1), 1.0)], share 1.0 -> Capsule { a: (1,1), radius_a: 1.0, b: (1,1), radius_b: 1.0 };
      // held == [0, 1]
  }
  #[test] fn three_members_on_one_spot_make_one_round_membrane() {
      // three times ((1,1), 1.0), share 1.0 -> Membrane with exactly one path; signed_area > 0 and within 8% of
      // pi * (1 - sqrt(0.5)) (field 3k(r) against T = 1.5); no two consecutive vertices equal; held == [0, 1, 2]
  }
  #[test] fn three_or_more_are_a_membrane_that_need_not_be_convex() {
      // a U: points every 0.5 on x=0 and x=4 for y in 0..=4, and on y=0 for x in 0.5..=3.5, weight 1, share 1.0:
      // Membrane; contains(paths, (2.0, 3.0)) == false (the convex hull would contain it); every member held
  }
  #[test] fn far_apart_groups_make_separate_pieces() {
      // 3x3 points 0.25 apart around (0,0) and the same around (20,0), weight 1, share 1.0 -> 2 paths, both positive
  }
  #[test] fn a_ring_of_members_leaves_a_hole() {
      // 24 members on the circle of radius 3 round (0,0), weight 1, share 1.0 -> one positive and one negative path;
      // contains(paths, (0,0)) == false
  }
  #[test] fn the_outline_holds_the_stated_share_and_leaves_the_stray_out() {
      // 3x3 points 0.25 apart round (0,0) weight 1, plus ((10,0), 1.0); share 0.9:
      // held == (0..9).collect::<Vec<_>>(); held_weight == 9.0; total_weight == 10.0;
      // contains(paths, (10,0)) == false
  }
  #[test] fn a_membership_of_zero_is_outside() {
      // share 0.9: the 3x3 square at weight 1, plus ((0.125, 0.0), 0.0) inside it (index 9), plus
      // ((f64::NAN, f64::NAN), 0.0) (index 10): Ok; 9 and 10 not in held; total_weight == 9.0;
      // contains(paths, (0.125, 0.0)) == true (it lies inside, and is still not held)
      // all members at zero -> Ok(None)
  }
  #[test] fn the_heavier_member_has_the_wider_margin() {
      // ((0,0),1.0), ((4,0),1.0), ((8,0),0.6), share 1.0: three pieces; the piece round (0,0) has area greater than
      // 1.3 times the piece round (8,0); each piece's area within 8% of pi * r^2 with
      // r = sqrt(1 - sqrt(T / w)) and T = 0.3 (w = 1.0 or 0.6)
  }
  #[test] fn scaling_every_weight_by_a_power_of_two_changes_nothing() {
      // the U above with weights cycling 1, 2, 3 over its members; again with every weight times 1024.0:
      // outline equal by to_bits() on every vertex; held equal
  }
  #[test] fn moving_the_set_by_whole_cells_moves_the_outline_exactly() {
      // the U shifted by (3 * 0.125, -5 * 0.125): same number of paths and vertices; each vertex shifted within 1e-9
  }
  #[test] fn held_agrees_with_the_even_odd_test() {
      // 300 members from a fixed LCG in a 10x10 square, weights k/5, share 0.9: the outline is a Membrane, and for
      // every positive member,
      // (index in held) == contains(paths, p)
  }
  #[test] fn bad_input_is_refused() {
      // weight -1 -> BadWeight { item: "#0" }; NaN weight; position (NaN,0) with weight 1 -> NonFinitePosition;
      // share 0.0 and 1.5, cell 0.6 with reach 1.0, level_fraction 0.0 -> BadParameter;
      // two members 1e6 apart with max_grid_nodes 1000 -> GridTooLarge;
      // members at (-1e300, 0) and (1e300, 0) with the default limit -> GridTooLarge, no panic
  }
  #[test] fn twenty_five_thousand_members_outline_in_one_call() {
      // 25_000 members on a 0.2 lattice (158 per row), weight 1, OutlineParams::new(1.0, 0.25, 0.9):
      // Ok(Some); held_weight >= 0.9 * total_weight
  }
  #[test] fn the_same_input_gives_the_same_bits_twice() { /* the LCG set above, outline compared by to_bits() */ }
  ```

- [ ] **Step 2: Run and see them fail.**
  Run: `cargo test -p hypergraph-rs --locked --test test_geometry_outline`
  Expected: FAIL to compile, "cannot find function `weighted_outline`".

- [ ] **Step 3: Implement** the items of "Produces" in `outline.rs` by the decisions above, and re-export them.

- [ ] **Step 4: Run and see them pass.** The same command, then the rules test. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 1, Step 6), then commit** `feat(geometry): outline a set of points from the
  numbers on its members`.

---

### Task 6: The outline of a hyperedge

**Files:**
- Modify: `crates/hypergraph-rs/src/geometry/outline.rs`, `src/geometry/mod.rs` (re-exports)
- Test: `crates/hypergraph-rs/tests/test_geometry_outline.rs`

**Interfaces:**
- Consumes: `weighted_outline`, `OutlineParams`, `WeightedOutline` (Task 5); `Positions` (Task 1);
  `Hypergraph::members_with_data` (core, unchanged).
- Produces: `EdgeOutline`, `edge_outline`.

**Decisions:**
- Members are read with `members_with_data(edge_id)`, in that order. A member whose weight is zero is skipped before
  its position is looked up. `EdgeOutline::members` lists the members with weight above zero, in that order, and
  `outline.held` indexes into it.
- An unknown edge gives `UnknownEdge`; a member with a weight above zero and no position gives
  `NoPosition { edge, node }`; a bad weight gives `BadWeight` with the node id; a position not finite gives
  `NonFinitePosition` with the node id. No positive member gives `Ok(None)`.

- [ ] **Step 1: Write the failing tests**, on `Hypergraph<(), (), f64>`, with `OutlineParams::new(1.0, 0.125, 0.9)`:

  ```rust
  #[test] fn an_edge_outline_reads_the_numbers_on_its_memberships() {
      // edge "club" {a: 2.0, b: 1.0, z: 0.0}; positions a (0,0), b (5,0), no z:
      // Ok(Some); members == ["a","b"]; outline.outline == Capsule { a: (0,0), radius_a: 1.0, b: (5,0), radius_b: 0.75 }
  }
  #[test] fn a_weighted_member_without_a_position_is_refused() {
      // "club" {a: 1.0, b: 1.0}; positions only a -> Err(NoPosition { edge: "club", node: "b" })
  }
  #[test] fn an_unknown_edge_is_refused() { /* "nope" -> Err(UnknownEdge("nope")) */ }
  #[test] fn an_edge_of_zeros_has_no_outline() { /* "club" {a: 0.0, b: 0.0}, empty positions -> Ok(None) */ }
  ```

- [ ] **Step 2: Run and see them fail.** Run: `cargo test -p hypergraph-rs --locked --test test_geometry_outline`
  Expected: FAIL to compile, "cannot find function `edge_outline`".

- [ ] **Step 3: Implement `edge_outline`** in `outline.rs` and re-export it with `EdgeOutline`.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 1, Step 6), then commit** `feat(geometry): outline a hyperedge from its
  memberships`.

---

### Task 7: Documents, and every gate

**Files:**
- Modify: `crates/hypergraph-rs/src/geometry/mod.rs` (module docs), `crates/hypergraph-rs/src/lib.rs` (feature table
  row and the "What is in it" module list), `README.md` (feature table row), `CHANGELOG.md` (one entry)

**Interfaces:**
- Consumes: everything above. Produces: no code.

- [ ] **Step 1: Module docs.** `geometry/mod.rs` gets a guide: what the module is for; the two layers; the
  determinism rule (which float operations, no randomness, no hash-map order); the four stability rules from "How a
  caller keeps positions stable"; and one doctest that builds a `Hypergraph<(), (), f64>` with three edges, two
  anchors `top` and `bottom` and one set `club` of three weighted members, each member also in `top` or `bottom`
  with a weight above zero (so every member is placed), places the nodes with `anchored_placement` (anchors
  `top -> (0, 10)`, `bottom -> (0, 0)`), spreads them with `spread`, and asserts that `edge_outline(&h, "club", ..)`
  returns an outline whose `held_weight` is at least the stated share of `total_weight`.
  Neutral words only.

- [ ] **Step 2: The tables and the changelog.** `lib.rs` and `README.md` each get the row
  ``| `geometry` | outlines of a weighted set (ring, capsule, membrane), placement at a weighted mix of fixed points,
  spreading in fixed steps; the same bits on every machine |`` after the `raster-png` row, and `geometry` joins the
  list of default features in both. `CHANGELOG.md` gets one "Added" line under the topmost unreleased section naming
  the feature and its public items.

- [ ] **Step 3: Every gate, one at a time.** `mise run rust:check`; `mise run rust:msrv`; `cargo deny check`; if
  `.venv` exists, `.venv/bin/maturin develop` then `mise run py:gate` (the Python crate is untouched; this shows it).
  Expected: each passes. Then repeat Task 1, Step 5 (the default-off proof) against the base. Expected: unchanged.

- [ ] **Step 4: Commit** `docs(geometry): guide, feature tables and changelog`.

## Self-review notes

- Design 6.7 sets: weighted outline with zero outside and a stated share (Task 5), three forms by size (Task 5),
  placement between fixed anchors nearer the larger share (Task 2), spreading by fixed steps in a fixed order
  (Task 3), same picture on every machine (determinism tests in Tasks 2 to 5, the rules test in Task 1). Design 6.8:
  placement at the mix of corners, all of one share on the corner itself (Task 1). Design 6.1: 25,000 points, no
  square-cost (Tasks 3 and 5). Design 8: core, no Bevy; open set of outline shapes (`#[non_exhaustive]`). Design 11:
  tests with and without each feature and at the floor (Task 7).
- Design 6.7's groupings that are not hyperedges (read from pairwise ties) are outlined by the lower layer,
  `weighted_outline` on plain points; drawing them in a different line is the Bevy plan's.
- Not here, by design 8 and 10: meshes, hatching, marks, plugin, example (step 3); the scene, wells figures, easing
  and labels (the game's client).
