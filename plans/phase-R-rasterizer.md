# hypergraph-rs Phase R Implementation Plan — Cell Rasterizer + Lawverian 3D Instruments

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Each `### Task N` section is self-contained: an agent picks it up cold, reads this preamble + that task, and executes its RED→GREEN→commit steps.

**Goal:** Implement the `raster/` module — the one genuinely new organ of the crate: a deterministic 3D-scene → terminal-text rasterizer, plus the three Lawverian instruments the Babylon Topology Deck already computes. The pipeline is `state → SceneGraph3D → CellGrid` (glyph text). Everything above the graph is *re-projection of quantities the engine already computes* (NORTH_STAR: the formalism surface is closed); Phase R adds no new mathematics, only a camera, a depth buffer, and a glyph floor. Backbone: `plans/PHASE-R-MAP.md` (owner-approved 2026-07-22). Re-scope authority: `plans/PROPOSAL-raster-replan.md` (approved 2026-07-22). Reference implementation: `plans/reference/topology-deck.html` (502 lines, seed 1917) — Rust must reproduce its computed **numbers** before any pixel is judged.

**Architecture:** `rasterize(scene: &SceneGraph3D, camera: &Camera, cols: u16, rows: u16) -> CellGrid`. `SceneGraph3D` is the Phase-5 `SceneGraph` shape (glyphs + bbox + metadata, `viz/mod.rs:74-80`) lifted to `(x,y,z)` with triangle faces and line struts. `CellGrid` is a row-major `Vec<Cell>` (`cols*rows`), each `Cell { ch: char, fg: Rgb, bg: Rgb }` — Tier-0 fully informative by construction. Determinism (PHASE-R-MAP §4): every frame is a pure function of `(state, camera, cols, rows)` — no wall-clock, no entropy, no `HashMap` iteration order leaking into output; frames golden-test byte-exactly. Instrument builders are pure `state → SceneGraph3D`, one per instrument, direct transcriptions of the deck's draw functions minus canvas calls.

**Tech Stack:** Rust 1.85+ (workspace MSRV, edition 2021), existing Phase-5 `layout`/`viz` deps, Phase-6 `communities` eigensolver, Phase-3 `algorithms`, and **new to this phase:** `h3o = "0.10"` for u64-native H3 indices. BSD-3-Clause. Task 1 is the sole owner of the `h3o` `[workspace.dependencies]` pin; later tasks only add `h3o.workspace = true` after a `grep -c '^h3o' Cargo.toml` pre-check (the Phase-5 single-owner-of-manifest discipline, phase-5-layout-viz.md:21-30).

---

## CRITICAL REFERENCE CORRECTION — the technique reference does NOT ship braille

The re-plan (`PROPOSAL-raster-replan.md:44-50`) and PHASE-R-MAP §3 point at `ludvigsandberg/tui-3d-renderer` (MIT, Rust) as the rasterizer technique reference. A 2026-07-22 read of all five source files (`src/{main,char,clip,raster,vertex,scene}.rs`) **empirically DISPROVED** the README's "UTF-8 Unicode shading" claim: that repo implements **half-blocks ONLY** (`▀` U+2580 / `▄` U+2584, 2 vertical subcells), no braille, no quadrants, no sextants, and its line routine is a **floating-point DDA, not Bresenham**. **Consequence — this plan supersedes the reference on these points:** Phase R's mandated 2×4 braille glyph floor (8 subcells/cell) and integer Bresenham are **designed fresh here** (Task 3), mining only the depth-buffer + edge-function triangle-fill + perspective-divide techniques from that repo. Copying its DDA line walk would inject FP-accumulation nondeterminism and break the byte-exact golden contract.

## CRITICAL REFERENCE CORRECTION — mulberry32 ≠ Pcg64Mcg (deck-number parity)

PHASE-R-MAP row 17 marks the deck's `mulberry32(1917)` RNG as "shipped (P4)" via `generators::rng::seeded_rng` (`Pcg64Mcg`). This is true for the *determinism pattern* but **false for byte-exact deck-number parity**: `Pcg64Mcg::seed_from_u64(1917)` and JS `mulberry32(1917)` are different generators emitting different sequences. The economic algebra numbers (`r`, `T`, `Φ`, `rentImperial`, `rentInternal`) never touch RNG and are fully portable under any generator; but the deck's **Π₀ (=13), mean periphery wealth (=1.825), and psa (=0.2359)** are RNG-derived and will NOT match under `Pcg64Mcg`. `mulberry32` is pure 32-bit wrapping integer arithmetic (zero float-tolerance concern, unlike libm transcendentals — user CLAUDE.md contract §4). **Resolution (this plan): port `mulberry32` verbatim as a deck-fixture-only RNG (Task 6)** so `Π₀`/`pw` are true byte-exact golden numbers (pure wrapping-integer + basic-arithmetic paths). **`psa` is the exception (Finding 3):** it is `sigmoid(pw−3.0)`, routing through `exp` — a libm transcendental that does NOT reproduce bit-for-bit across implementations (user CLAUDE.md §4), so `psa` is a **tolerance-bounded** golden (abs < 1e-12), NOT byte-exact (see the derivation in Task 10 Step 1). `Pcg64Mcg` remains the crate's canonical generator for all non-deck use; `mulberry32` is a fixture RNG that never enters the general layout/community API. (Resolves the scouts' top gap.)

---

## Global Constraints

- **Rust edition 2021**, MSRV 1.85. **License:** BSD-3-Clause. **No `unsafe`.**
- **Determinism (III.13 / raster contract):** `rasterize` and every instrument builder are pure functions of `(state, camera, cols, rows)` — no wall-clock, no thread-nondeterminism, no global RNG, no `HashMap`/`HashSet` iteration order in any public-facing output. Every frame is byte-exact given identical inputs (golden-testable). The deck's two impurities are **explicitly dropped** (see Deviations): the ribbon flow-dot animation (`topology-deck.html:425`, wall-clock `time`) and `repress()`'s unseeded `Math.random()` (`:269`).
- **Glyph floor (mandatory, PHASE-R-MAP §4):** every emitted `Cell` is Tier-0 fully informative — it carries `ch + fg + bg` **unconditionally** (no cell relies on an adjacent cell or an external legend for meaning). Every *labeled scalar* an instrument computes (`r`, `Φ`, `o(C)`, per-cell `T`, `psa`/`psr`) MUST appear as literal text somewhere in the `CellGrid` (the deck's own conservation banner `PHI=… r=… — invariant under zoom`, `:430-433`, is the canonical example). Stage 3 property tests assert this parity.
- **H3 boundary (u64 end-to-end, PHASE-R-MAP §4 / divergence D7):** `h3o::CellIndex` (u64-backed `NonZeroU64`) is the spatial index domain type; it is stringified only when it crosses the graph's stringly-typed ID boundary (`core/hypergraph.rs:292` etc.). Parent = bit-masking (`CellIndex::parent`), children = contiguous integer range (`CellIndex::children`). All bitwise spatial algebra lives inside `raster/h3.rs`, never in graph semantics. `to_local_ij` MUST handle `LocalIjError::Pentagon` explicitly (12 of 122 base cells are pentagons) — never `.unwrap()` on H3 math (Power-of-10 rule 4).
- **S9 (assert-the-source / lane discipline, PHASE-R-MAP §4):** the 2D H3-snapped base plane is an **index**, not a layout — S9-clean as-is. 3D spatial embeddings (spectral cylinder heights, terrain, occY altitude) are **library/CLI capability only**, gated behind `render --format cells3d`, until the owner issues the #287 lane-extension ruling for babylon-side consumption. The `cells3d` feature carries this gate. `SceneGraph3D` JSON is a **golden artifact class** (assert-the-source); cell frames are **also** golden because they are text.
- **Every loop has a fixed upper bound** (cols×rows, subcell count, node count, edge count, ring count, the deck's fixed 90 power-iterations, hull-stack shrink — noted inline; Power-of-10 rule 2).
- **TDD discipline:** every task is red → green → commit. Integration tests are **direct children of `crates/hypergraph-rs/tests/`** (`test_raster.rs`, `test_instruments.rs`, `test_deck_fixture.rs`) — cargo discovers targets only there.
- **`assert_eq!`/`assert_ne!` require `Debug`** — every public raster type derives `Debug` (+ `PartialEq` for equality asserts, `Serialize`/`Deserialize` for the JSON golden class). Float→text emission for golden fixtures uses the **exact Phase-5 deterministic formatter pattern** (`viz/svg.rs:109-117`: `format!("{:.6}", v)`, trailing zeros trimmed, `-0`→`0`) — that fn is private, so Task 1 re-copies it into `raster/fmt.rs` as `pub(crate) fn fmt6(v: f64) -> String`.
- **Commit convention:** conventional commits; end every message with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Feature gating:** new modules gated `#[cfg(feature = "raster")]`; `raster = ["viz"]` (it consumes the `SceneGraph`/glyph/style/`HyperedgeGeometry` pattern from `viz`, which implies `layout`). The 3D-embedding instruments additionally require `cells3d = ["raster", "communities", "dep:h3o"]` (the S9 lane gate + the eigensolver + the H3 index dep — `h3o` is consumed ONLY by the `cells3d`-gated `raster/h3.rs` + `sankey.rs`, so it hangs off `cells3d`, never off `raster`; Finding 7). The Tier-1 PNG pixel plate (Task 14) adds `raster-png = ["cells3d", "dep:png"]`: it rasterizes the same 3D `SceneGraph3D` the instruments build, so it rides on the `cells3d` 3D-lane gate plus the optional pure-Rust `png` encoder dep (`png` is consumed ONLY by the `raster-png`-gated `raster/png.rs`, so `dep:png` hangs off `raster-png`, never off `raster`/`cells3d` — narrowest gate). **Merge** all three into the existing `[features]` table and append `"raster"` to the existing `default` array; leave `cells3d` AND `raster-png` OUT of `default` (both are owner-gated capabilities). **Never overwrite `default` wholesale** (phase-5 lesson, phase-5-layout-viz.md:149-157).

---

## REUSE LEDGER (DRY is a project law — every entry is shipped code a task reuses)

Verified by the crate-reuse scout against the live tree. **Do not re-implement any of these; call them.** Gaps below the ledger are the only things Phase R builds new.

| Shipped API | Signature (abbrev.) | file:line | Reused by |
|---|---|---|---|
| `SceneGraph` / `NodeGlyph` / `HyperedgeGlyph` | structs: glyphs + bbox + `serde_json::Map` metadata | `viz/mod.rs:74-80, 52-61, 63-71` | Task 1 (`SceneGraph3D` mirrors this shape + z/faces) |
| `NodeStyle` / `HyperedgeStyle` (+ `Default`) | `{fill,stroke,stroke_width}` / `{fill,stroke,opacity}` | `viz/mod.rs:17-22, 35-40` (defaults `24-32`,`42-50`) | Task 1 (style carried onto 3D glyphs) |
| deterministic float formatter `fmt(v)` | `format!("{:.6}",v)`, trim zeros, `-0`→`0` | `viz/svg.rs:109-117` (private) | Task 1 copies as `raster::fmt6` |
| XML/text escaper `escape(s)` | private | `viz/svg.rs:120-125` | Task 11 (JSON string safety reference) |
| `convex_hull` (Andrew monotone chain) | `fn(&[(f64,f64)]) -> Vec<(f64,f64)>` | `layout/hull.rs:9` | Task 7 (community hull on projected 2D points) |
| `compute_layout` + `LayoutResult` + `HyperedgeGeometry` | dispatcher / result / 4-variant geometry | `layout/mod.rs:145, 80-85, 61-75` | Task 6 (2D substrate under the z lift) |
| `smallest_k_eigenvectors` | `fn(&Array2<f64>, k, seed:Option<u64>) -> Array2<f64>` | `communities/mod.rs:120` | Task 7 (spectral disc positions; **pin seed**) |
| `normalized_laplacian` | `fn(&Hypergraph) -> Result<(Array2,Vec<String>),SpectralError>` | `communities/mod.rs:35` | Task 7 (Laplacian for the eigensolver) |
| `spectral_clustering` / `kmeans` | seeded cluster entry points | `communities/mod.rs:356, 248` | available if an instrument needs cluster ids |
| `connected_components` | `fn<N,E,M>(&Hypergraph) -> Vec<Vec<String>>` (BFS, `node_ids()` order) | `algorithms/connected.rs:36` | Task 6 (o(C) organization degree) |
| `number_connected_components` | delegates to `rustworkx_core` on `skeleton()` | `algorithms/connected.rs:68` | Task 6 (global Π₀ cross-check) |
| `subhypergraph` (induced) | `fn(&Hypergraph<Value,Value,Value>, nodes, edges, keep_isolates)` | `core/globalviews.rs:13` | Task 6 (per-community induced subgraph for o(C)) |
| `seeded_rng` / `uniform_f64` / `choice_without_replacement` | `Pcg64Mcg` helpers (determinism **pattern**, not deck bit-parity) | `generators/rng.rs:15,21,36` | Task 6 (pattern only; mulberry32 is the deck RNG) |
| `node_ids`/`edge_ids`/`members`/`memberships`/`node_attrs`/`edge_attrs` | insertion-ordered ID/attr accessors (D7 String boundary) | `core/hypergraph.rs:292,298,272,251,366,384` | Tasks 6–9 (walk the state deterministically) |
| `skeleton` (1-skeleton `UnGraph`) | `fn(&self) -> UnGraph<String,f64>` | `core/hypergraph.rs:846` | Task 6 (component counting substrate) |
| `paoh_ordering` / `levi_ego_tree` | Phase-5 Task-12 ordering providers | `layout/orderings.rs:127, 41` | Task 8/9 (depth-sorted render orderings, S9) |
| `incidence_matrix` / `normalized_hypergraph_laplacian` | linalg builders | `linalg/hypergraph_matrix.rs:16`, `laplacian.rs:86` | Task 7 (if spectral needs raw incidence) |

**h3o (new dep, confirmed by the h3o scout — pin `"0.10"`, `max_stable_version` 2026-05-24; core API needs NO feature flag):** `CellIndex::try_from(u64) -> Result<_,InvalidCellIndex>`, `CellIndex::parent(Resolution) -> Option<CellIndex>` *(verify Option vs Result against docs.rs 0.10 before binding — the scout flagged signature ambiguity)*, `CellIndex::children(Resolution) -> impl Iterator`, `CellIndex::children_count(Resolution) -> u64`, `CellIndex::resolution() -> Resolution`, `CellIndex::to_local_ij(origin) -> Result<LocalIJ,LocalIjError>` (handle `Pentagon`), and its **inverse** `LocalIJ::new(anchor, i, j)` + `CellIndex::try_from(LocalIJ) -> Result<_,LocalIjError>` (constructs a cell FROM axial offsets — used by `industry_cell` in Task 9; **verify the exact `LocalIJ`/`try_from` signature and the `LocalIjError::Pentagon` arm against docs.rs 0.10**, same as the flagged `parent()` Option-vs-Result check — Finding 10), `CellIndex::boundary() -> Boundary`, `LatLng::new/to_cell/from`. Do **not** pull the `geo` feature (polygon interop, out of scope).

**Confirmed gaps Phase R BUILDS new** (crate scout `rg` proved absent): `SceneGraph3D`/`Camera`/projection/depth-buffer/`CellGrid` (all 3D + terminal types), the braille 2×4 glyph floor, integer Bresenham, `mulberry32` port, the h3o integration layer, and a 3D hull (Task 7 projects to 2D then reuses `convex_hull`, matching the deck).

---

## Deliberate deviations from the deck / the spec (each called out where it lands)

1. **`mulberry32` ported, not `Pcg64Mcg`, for the deck fixture** (see CRITICAL CORRECTION 2). Lands in Task 6 as a fixture-only RNG; the crate's canonical `Pcg64Mcg` is untouched.
2. **Spectral disc positions are structural-parity, not deck-byte-exact.** The deck's `spectral()` fixes exactly 90 power-iterations with no convergence check, mean-projection each step, `mulberry32(seed^0x5EED)=mulberry32(22928)` init (`topology-deck.html:214-240`); the crate's `smallest_k_eigenvectors` uses `Pcg64Mcg` + `TOL=1e-9`/`MAX_ITER=1000` early-exit (`communities/mod.rs:120,146-147`). Iteration counts diverge bit-for-bit. **Resolution:** the deck's golden **numbers** (Stage 0) never include spectral (x,y) — `o(C)`, `Π₀`, `r`, `T` are graph/algebra, not spectral. So Task 7 reuses `smallest_k_eigenvectors` (pinned `seed`) for disc placement and Rust's own output is self-golden at Stage 1; deck-position parity is not asserted. (Mirrors phase-5 Deviation 2.)
3. **Ribbon flow-dot animation dropped.** `drawSankey`'s traveling dot (`:425-428`) is driven by wall-clock `time` — the one deck behavior that is not a pure fn of `(state,camera)`. Phase R omits it entirely (no decorative animation in a golden-testable frame).
4. **`repress()` re-seeded.** The deck's `repress()` uses unseeded `Math.random()` (`:269`) — a determinism bug against its own footer claim. Rust's repress (if/when a verb layer lands) draws from a seeded source advancing a world counter, never `Math.random()`. Phase R does not port verbs (state mutation is babylon's job); this is noted so a future port does not copy the bug.
5. **Ribbon conservation tested on the UNRENDERED allocation.** The deck skips sub-`.4` flows from *rendering* (`:417`) but the `ΣT≡0` / Σflow=Φ conservation holds on the full `flow = j.T*(-i.T)/negSum` allocation. Stage 0/3 assert conservation on the unrendered allocation, never on "sum of drawn ribbons".
6. **`Φ == rentImperial + rentInternal` is a seed-1917 empirical property, not a structural law.** It holds only because all lane-0/1 industries are net donors and all lane-2/3 are net absorbers at these parameters (scout-verified tick1–tick6). Tagged as a fixture property; a mutation test (Task 13) confirms it can break under perturbed c/v/e.
7. **Coarse-zoom cell partition is h3o-derived, not deck-copied.** The deck's `aggregateCells` uses synthetic `key=(hq>>1,hr>>1)` producing **6** parent cells at seed 1917 (NOT lane-uniform — lane 2 splits into 3 singletons; scout-verified). Phase R snaps industries to **real** `h3o::CellIndex` and aggregates via `parent()`. The **partition-independent** invariants (`net-T ≡ 0`, `r =` ratio-of-sums) are asserted as deck-golden and survive any partition; the per-cell `T` decomposition becomes an h3o-derived golden, re-derived, not copied from the deck's 6 cells. (See OPEN DECISION D-R5.)
8. **Deck dead code ignored.** `hexXZ`/`occY`/`aggregateCells` are duplicated verbatim in the reference (`:361-370` and `:371-380`); the `v-zoom` button + listener are doubly-registered (dead). The Rust port defines each once. Not a semantic difference — flagged so the port does not infer two zoom toggles.

---

## File Structure

```
hypergraph-rs/crates/hypergraph-rs/
├── Cargo.toml                          # + h3o dep; + png dep; + [features] raster, cells3d, raster-png
├── src/
│   ├── lib.rs                          # + pub mod raster; (cfg-gated)
│   └── raster/
│       ├── mod.rs                      # Cell, Rgb, CellGrid, SceneGraph3D, Vertex3, Face,
│       │                               #   Strut, rasterize dispatcher, re-exports
│       ├── fmt.rs                      # fmt6 (copy of viz::svg::fmt) — golden float text
│       ├── camera.rs                   # Camera, project() (rotY∘rotX + perspective divide),
│       │                               #   depth_sort() (painter's / z-order)
│       ├── grid.rs                     # subcell depth buffer, integer Bresenham,
│       │                               #   edge-function scanline triangle fill,
│       │                               #   braille 2×4 glyph selection, Cell composition
│       ├── color.rs                    # Rgb, heat() (deck gold→bone→crimson), truecolor +
│       │                               #   256-palette deterministic quantization, fg/bg pick
│       ├── png.rs                      # ADR097 Tier-1 pixel plate: render_pixels/render_png
│       │                               #   (shares project/depth_sort/fill from camera+grid; png crate)
│       ├── h3.rs                       # h3o u64 <-> raster boundary; parent aggregation
│       ├── algebra.rs                  # transformation algebra (s=v·e, r, T, Φ) — pure ~30 lines
│       ├── deck.rs                     # mulberry32 port + buildWorld fixture + o(C) org degree
│       └── instruments/
│           ├── mod.rs                  # Instrument enum + dispatch to the 3 builders
│           ├── cylinder.rs             # Instrument I — adjoint cylinder
│           ├── terrain.rs              # Instrument II — contradiction terrain
│           └── sankey.rs               # Instrument III — OCC hyper-Sankey over H3
└── tests/
    ├── test_raster.rs                  # Tasks 1-4 (types, camera, grid, color)
    ├── test_deck_fixture.rs            # Tasks 5, 6, 10 (algebra, buildWorld, Stage 0)
    ├── test_instruments.rs             # Tasks 7-9 (three builders)
    ├── test_png.rs                     # Task 14 (Tier-1 PNG pixel plate; #![cfg(feature="raster-png")])
    └── fixtures/                       # Task 11/12/14 golden artifacts
        ├── scene3d/*.json              # SceneGraph3D JSON goldens (Stage 1)
        ├── frames/*.txt                # CellGrid text goldens (Stage 2)
        └── png/*.png                   # platform-pinned PNG plate goldens (Task 14)
```

**Note on the `[features]` table:** the current table (verified) is `default = ["algorithms","generators","stats","readwrite","layout","viz"]` with `all = ["default","dynamics","communities","convert","network"]`. **Merge** `raster = ["viz"]`, `cells3d = ["raster","communities","dep:h3o"]`, and `raster-png = ["cells3d","dep:png"]` in (Finding 7: h3o rides on `cells3d`, not `raster`; likewise `png` rides on `raster-png`, not `raster`/`cells3d`); append `"raster"` to `default`; add both `"cells3d"` and `"raster-png"` to `all` (not `default`). Task 1 shows the fully-merged target.

---

## Phase R: Rasterizer + Lawverian Instruments

### Task 1: Scaffold raster module, h3o dep, features, and core public types

**Files:**
- Modify: `crates/hypergraph-rs/Cargo.toml`, `crates/hypergraph-rs/src/lib.rs`
- Create: `src/raster/mod.rs`, `src/raster/fmt.rs`, `crates/hypergraph-rs/tests/test_raster.rs`
- Modify (workspace): `crates/hypergraph-rs/../../Cargo.toml` (workspace root — h3o pin + png pin)

**Interfaces:** `raster::{Rgb, Cell, CellGrid, Vertex3, Face, Strut, SceneGraph3D}` (all `Debug + Clone + PartialEq + Serialize + Deserialize`); `raster::fmt::fmt6(v: f64) -> String`. `Cell { ch: char, fg: Rgb, bg: Rgb }` (glyph floor: all three always present). `CellGrid { cols: u16, rows: u16, cells: Vec<Cell> }` (row-major, `cells.len() == cols*rows`) with `fn to_text(&self) -> String` (glyphs only, newline-joined — the diffable byte-exact form) and `fn to_ansi(&self) -> String` (with fg/bg escapes). `SceneGraph3D { nodes: Vec<Node3>, faces: Vec<Face>, struts: Vec<Strut>, bounding_box: (f64,f64,f64,f64,f64,f64), metadata: serde_json::Map<String,Value> }` mirroring `viz::SceneGraph` (`viz/mod.rs:74-80`) with a 3D bbox and triangle faces.

- [ ] **Step 1: Write the failing test** — append to `tests/test_raster.rs`:
```rust
use hypergraph_rs::raster::{Rgb, Cell, CellGrid, SceneGraph3D};

#[test]
fn test_cell_glyph_floor_always_complete() {
    // Glyph floor: a Cell ALWAYS carries ch+fg+bg (no Option).
    let c = Cell { ch: '⠿', fg: Rgb(196, 58, 46), bg: Rgb(0, 0, 0) };
    assert_eq!(c.ch, '⠿');
    assert_eq!(c.fg, Rgb(196, 58, 46));
}

#[test]
fn test_cellgrid_row_major_len_and_text() {
    let g = CellGrid::filled(3, 2, Cell { ch: ' ', fg: Rgb(0,0,0), bg: Rgb(0,0,0) });
    assert_eq!(g.cells.len(), 6);
    assert_eq!(g.to_text(), "   \n   "); // 3 cols, 2 rows, glyphs only
}

#[test]
fn test_scenegraph3d_serde_round_trip() {
    let s = SceneGraph3D::empty();
    let j = serde_json::to_string(&s).unwrap();
    let back: SceneGraph3D = serde_json::from_str(&j).unwrap();
    assert_eq!(s, back);
}

#[test]
fn test_fmt6_matches_phase5_formatter() {
    use hypergraph_rs::raster::fmt::fmt6;
    assert_eq!(fmt6(0.4608359187088229), "0.460836");
    assert_eq!(fmt6(-0.0), "0");
    assert_eq!(fmt6(198.3), "198.3");
}
```

- [ ] **Step 2: Run test to verify it fails** — `cd hypergraph-rs && cargo test -p hypergraph-rs --features raster --test test_raster`. Expected: FAIL — module `raster` does not exist.

- [ ] **Step 3: Implement.**
  - **Workspace `Cargo.toml`:** `grep -c '^h3o' Cargo.toml` (expect 0), then add to `[workspace.dependencies]`: `h3o = "0.10"`. Likewise, single-owner-of-the-pin: `grep -c '^png' Cargo.toml` (expect 0), then add `png = "0.18"` (the image-rs `png` crate — pure Rust, deterministic; current stable line, `max_stable_version` 0.18.1 as of 2026-07-22). Verify the exact `Encoder`/`Compression`/`Filter` setter API against docs.rs 0.18 before binding in Task 14 (same docs.rs-first discipline as h3o's `parent()` Option-vs-Result check).
  - **Crate `Cargo.toml`:** add `h3o = { workspace = true, optional = true }` and `png = { workspace = true, optional = true }` to `[dependencies]`; merge features: `raster = ["viz"]`, `cells3d = ["raster", "communities", "dep:h3o"]`, `raster-png = ["cells3d", "dep:png"]` (h3o is used only by the `cells3d`-gated `raster/h3.rs` + `sankey.rs`, so `dep:h3o` hangs off `cells3d`, NOT `raster` — otherwise the Task-15 `--no-default-features --features raster` leg compiles h3o unused and muddies the S9 gate; Finding 7; and `png` is used only by the `raster-png`-gated `raster/png.rs`, so `dep:png` hangs off `raster-png`, never off `raster`/`cells3d`); append `"raster"` to `default`; add both `"cells3d"` and `"raster-png"` to `all` (both stay OUT of `default` — owner-gated capabilities).
  - **`lib.rs`:** `#[cfg(feature = "raster")] pub mod raster;`.
  - **`raster/fmt.rs`:** copy `viz/svg.rs:109-117` verbatim as `pub(crate) fn fmt6(v: f64) -> String` (re-exported `pub` under `raster::fmt` for the test). Do NOT alter the algorithm — byte-exactness depends on it.
  - **`raster/mod.rs`:** `pub mod fmt;` + the seven types. `Rgb(pub u8, pub u8, pub u8)`. `Cell { pub ch: char, pub fg: Rgb, pub bg: Rgb }`. `CellGrid` with `filled`, `to_text` (map each row to its chars, join rows with `'\n'`), `to_ansi` (Task 4 fills the escape logic; a plain glyph fallback is fine now). `Node3 { id: String, pos: Vertex3, radius: f64, color: Rgb, style: NodeStyle, attrs: Value }` — instruments set `color` via `heat() -> Rgb`, type-matching `Face`/`Strut` (which are `Rgb`); the reused `viz::NodeStyle` (CSS `#rrggbb` `String` fill/stroke) rides along for **metadata only** and is NEVER the rasterizer's color source (its strings are type-incompatible with `Rgb` — Finding 8; the `#rrggbb`→`Rgb` parser is added in Task 3 only if hex metadata must round-trip). `Vertex3 { x: f64, y: f64, z: f64 }`. `Face { verts: [Vertex3;3], fill: Rgb, opacity: f64 }`. `Strut { a: Vertex3, b: Vertex3, color: Rgb }`. `SceneGraph3D` per the interface, with `empty()`. Derive `Debug, Clone, PartialEq, Serialize, Deserialize` on all. `rasterize` is a stub `todo!`-free signature returning an empty grid for now (Task 3 fills it).
  - **Lockfile (`--locked` discipline, Finding 9):** after adding `h3o` and `png`, run a plain (un-`--locked`) `cargo build -p hypergraph-rs --features raster-png` once so BOTH `h3o` and `png` resolve into `Cargo.lock` (`raster-png` pulls `cells3d` → `h3o` transitively, plus `png`), then commit the updated `Cargo.lock` **alongside** the manifest in Step 5 — every later `--locked` leg (Task 15 / `mise run rust:check`) fails on a stale lockfile.

- [ ] **Step 4: Run test to verify it passes** — same command. Expected: PASS — 4 tests, 0 failures.

- [ ] **Step 5: Commit** — `feat(hypergraph-rs): scaffold raster module, h3o dep, core 3D + cell types`.

---

### Task 2: Camera + projection (rotY∘rotX + perspective divide) + painter's depth sort

**Files:** Create `src/raster/camera.rs`; modify `src/raster/mod.rs`, `tests/test_raster.rs`.

**Interfaces:** `raster::Camera { ry: f64, rx: f64, dist: f64, fov: f64 }` (rotation about Y then X, camera distance, vertical FOV degrees — the deck's `cam.ry`/`cam.rx` model, `topology-deck.html`). `raster::project(v: &Vertex3, cam: &Camera, cols: u16, rows: u16) -> Projected` where `Projected { sx: f64, sy: f64, depth: f64 }` (screen x/y in **subcell** units, depth = camera-space z after the perspective divide, smaller = nearer). `raster::depth_sort(faces: &mut [Face], struts: &mut [Strut], cam: &Camera)` — stable painter's sort, farthest first, keyed on centroid camera-space depth; **stable** so equal-depth primitives keep scene order (determinism, no draw-order dependency). **Near-plane clip + NaN-safety (mine `tui-3d/src/clip.rs`, Finding 4):** `project` guards the perspective divide with a near-plane epsilon `const NEAR_EPS: f64 = 1e-6` — a primitive with ANY vertex at camera-space `z' ≤ NEAR_EPS` is **rejected whole** (not split), so `1/z'` never yields `inf`/`NaN`. Every depth comparison (`project`, `depth_sort` centroids) is **NaN-safe** — `a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal)`, NEVER `.unwrap()` on `partial_cmp` (Power-of-10 rule 4) — so a degenerate/clipped primitive can never panic the sort, and subcell writes are bounds-clamped downstream (Task 3).

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::{Camera, Vertex3, project, depth_sort};

fn cam() -> Camera { Camera { ry: 0.0, rx: 0.0, dist: 4.0, fov: 70.0 } }

#[test]
fn test_project_center_is_center() {
    let p = project(&Vertex3 { x: 0.0, y: 0.0, z: 0.0 }, &cam(), 80, 48);
    // Origin projects to the screen midpoint (subcell space: cols*2 wide, rows*4 tall for braille).
    assert!((p.sx - (80.0 * 2.0 - 1.0) / 2.0).abs() < 1e-9);
    assert!((p.sy - (48.0 * 4.0 - 1.0) / 2.0).abs() < 1e-9);
}

#[test]
fn test_project_nearer_z_has_smaller_depth() {
    let near = project(&Vertex3 { x: 0.0, y: 0.0, z: 1.0 }, &cam(), 80, 48);
    let far  = project(&Vertex3 { x: 0.0, y: 0.0, z: -1.0 }, &cam(), 80, 48);
    assert!(near.depth < far.depth);
}

#[test]
fn test_rotation_determinism_bit_identical() {
    let c = Camera { ry: 0.7, rx: 0.3, dist: 4.0, fov: 70.0 };
    let v = Vertex3 { x: 0.5, y: -0.2, z: 0.9 };
    assert_eq!(project(&v, &c, 80, 48).sx, project(&v, &c, 80, 48).sx);
}

#[test]
fn test_depth_sort_stable_on_ties() {
    // Two faces at identical depth keep insertion order (stable sort).
    // ... construct two coplanar faces, assert order preserved after depth_sort.
}
```

- [ ] **Step 2: Run** — Expected: FAIL (no `camera`).

- [ ] **Step 3: Implement `camera.rs`.** Mine the pipeline from `tui-3d-renderer/src/raster.rs::prepare_position` (the perspective divide + viewport remap) but with **pure f64** (not the repo's f32) and rotation composed as **rotate-Y then rotate-X** (the deck's model; `tui-3d`'s `rotY∘rotX` order, PART B step 1):
  1. Camera-space: `v' = RotX(rx) · RotY(ry) · v`, then translate `z' += dist` (move the world in front of the camera).
  2. Perspective divide with focal length `f = 1 / tan(fov·π/360)`: `ndc_x = f * v'.x / v'.z`, `ndc_y = f * v'.y / v'.z`, aspect-correct `ndc_x` by `rows*4 / (cols*2)`. **Near-plane clip FIRST (Finding 4):** if `v'.z ≤ NEAR_EPS` (`const NEAR_EPS: f64 = 1e-6`), the vertex is at/behind the camera — reject the whole primitive (filter it out / return a clipped sentinel) instead of dividing by a near-zero or negative `z'`. This is the `tui-3d/src/clip.rs` step Tasks 2/3 must NOT drop.
  3. Viewport remap to **subcell** space (braille 2×4 ⇒ width `cols*2`, height `rows*4`): `sx = (ndc_x + 1)/2 * (cols*2 - 1)`, `sy = (1 - ndc_y)/2 * (rows*4 - 1)` (y flips: NDC up = screen down).
  4. `depth = v'.z` (camera-space z; smaller = nearer under this convention — match `tui-3d`'s `z-buf` less-than test).
  - `depth_sort`: compute each primitive's centroid, transform to camera space, sort by `depth` **descending** (far first) with `sort_by` (stable) — NOT `sort_unstable` — using the **NaN-safe** comparator `a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal)`, never `.unwrap()` on `partial_cmp` (a degenerate/clipped centroid must never panic the sort; Finding 4). Bounded: one pass over a finite slice.

- [ ] **Step 4: Run** — Expected: PASS — 8 tests total.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): raster camera, perspective projection, painter's depth sort`.

---

### Task 3: Cell rasterizer — subcell depth buffer, integer Bresenham, scanline triangle fill, braille 2×4 glyph selection

**Files:** Create `src/raster/grid.rs`; modify `src/raster/mod.rs` (`rasterize` dispatcher), `tests/test_raster.rs`.

**Interfaces:** `rasterize(&SceneGraph3D, &Camera, cols, rows) -> CellGrid` (the real body). Internally: `SubBuffer { w: usize, h: usize, depth: Vec<f64>, color: Vec<Option<Rgb>> }` where `w = cols*2`, `h = rows*4` (braille subcell grid); `line_bresenham(&mut SubBuffer, a: Projected, b: Projected, color: Rgb)` (**integer** Bresenham — no FP accumulation, per CRITICAL CORRECTION 1); `fill_triangle(&mut SubBuffer, [Projected;3], color: Rgb)` (edge-function barycentric with an explicit **top-left fill rule** tie-break for watertight shared edges — the `tui-3d` repo omits this, a flagged gap); `compose_braille(&SubBuffer) -> CellGrid` (Task 4 finalizes color; this task emits fg = the lit color, bg = black). **Output is TEXT** — byte-exact golden.

Braille encoding (designed fresh — the reference does not ship it): 8 dots per cell in a 2-wide × 4-tall subcell block. Dot→bit map (Unicode U+2800 base): `(col0,row0)=0x01 (col1,row0)=0x08 (col0,row1)=0x02 (col1,row1)=0x10 (col0,row2)=0x04 (col1,row2)=0x20 (col0,row3)=0x40 (col1,row3)=0x80`. A cell's glyph = `char::from_u32(0x2800 + set_bits)`. An empty cell (0 bits) is `0x2800` (blank braille) — still a valid Tier-0 glyph.

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::{rasterize, SceneGraph3D, Camera, /* helpers */};

#[test]
fn test_single_subdot_maps_to_correct_braille() {
    // A degenerate face/point lighting only subcell (col0,row0) of cell (0,0) => U+2801 (⠁).
    // Build a SceneGraph3D with one primitive projecting to subcell (0,0); assert grid cell 0 == '⠁'.
}

#[test]
fn test_bresenham_line_is_integer_deterministic() {
    // A diagonal strut produces a byte-identical glyph row across two runs.
}

#[test]
fn test_triangle_fill_top_left_rule_no_double_shade() {
    // Two triangles sharing an edge: every shared-edge subcell is lit exactly once
    // (top-left rule), never gapped, never double-counted.
}

#[test]
fn test_depth_buffer_occludes_farther_primitive() {
    // Two overlapping faces at different depth: the nearer color wins every contested subcell.
}

#[test]
fn test_frame_is_pure_function_byte_exact() {
    let (s, c) = /* fixed scene + camera */;
    assert_eq!(rasterize(&s, &c, 40, 20).to_text(), rasterize(&s, &c, 40, 20).to_text());
}
```

- [ ] **Step 2: Run** — Expected: FAIL.

- [ ] **Step 3: Implement `grid.rs`.** Mine from `tui-3d/src/raster.rs`: the `z_buf` at full subcell resolution (`clear` → `f64::INFINITY`), the edge-function `edge(a,b,c) = (c.x-a.x)*(b.y-a.y) - (c.y-a.y)*(b.x-a.x)`, barycentric weights, and the depth test `if z >= depth[idx] { continue }`. **Reimplement, do NOT copy:**
  - **`line_bresenham`**: classic integer Bresenham over subcell coords (`dx,dy` integers, error accumulator) — deterministic, no float drift. Depth per pixel interpolated linearly and z-tested.
  - **`fill_triangle`**: bounding-box scan; inside test = all three edge functions have consistent sign; **top-left rule**: a pixel on a shared edge is filled iff that edge is a "top" or "left" edge (bias the `>=0` test per edge). Interpolate depth `z = Σ bc_k · p_k.depth`; z-test each subcell. Bounded by the (finite) bounding-box area.
  - **`compose_braille`**: for each terminal cell, read its 8 subcells; set the corresponding braille bit for each lit subcell; glyph = `0x2800 + bits`; `fg` = Task-4 color pick (for now: the color of the nearest lit subcell), `bg` = `Rgb(0,0,0)`. Bounded loops: `cols*rows` cells × 8 subcells.
  - **Bounds-clamped subcell writes (Finding 4):** every subcell index derived from a `Projected` coord is clamped into `0..w` × `0..h` (saturating, e.g. `(sx.round() as isize).clamp(0, w as isize - 1) as usize`) before indexing `depth`/`color` — a NaN or out-of-range projected coordinate (e.g. from an extreme random camera in Task 13) must never produce an out-of-bounds index panic; it is clamped or the subpixel is skipped, never `.unwrap()`-ed into a panic.
  - Wire `rasterize`: project all faces/struts (Task 2 — near-plane-clipped, so no `inf`/`NaN` coords reach the buffer), `depth_sort`, rasterize into `SubBuffer`, `compose_braille`.

- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): braille 2x4 cell rasterizer — depth buffer, integer Bresenham, scanline fill`.

---

### Task 4: Color under constraint — heat ramp, crimson/gold truecolor canon, degraded 256-palette quantization, one fg+bg per cell

**Files:** Create `src/raster/color.rs`; modify `src/raster/grid.rs` (`compose_braille` fg/bg pick), `src/raster/mod.rs` (`CellGrid::to_ansi`), `tests/test_raster.rs`.

**Interfaces:** `raster::heat(t: f64) -> Rgb` (transcribe verbatim from `topology-deck.html:279-284`: gold `[217,164,65]` → bone `[233,223,201]` → crimson `[196,58,46]`, linear mix, `t<.5 ? mix(gold,bone,t*2) : mix(bone,crimson,(t-.5)*2)`, floor each channel). `raster::to_ansi256(c: Rgb) -> u8` (deterministic 6×6×6 cube + 24-step grayscale, mined from `tui-3d/src/char.rs`). `raster::Palette { TrueColor, Ansi256 }`. Per-cell fg/bg selection under the braille "one fg + one bg" constraint.

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::{heat, to_ansi256, Rgb};

#[test]
fn test_heat_endpoints_match_deck_canon() {
    assert_eq!(heat(0.0), Rgb(217, 164, 65));   // gold
    assert_eq!(heat(0.5), Rgb(233, 223, 201));  // bone
    assert_eq!(heat(1.0), Rgb(196, 58, 46));    // crimson
}

#[test]
fn test_heat_clamps_out_of_range() {
    assert_eq!(heat(-1.0), heat(0.0));
    assert_eq!(heat(2.0), heat(1.0));
}

#[test]
fn test_ansi256_quantization_deterministic_and_pinned() {
    assert_eq!(to_ansi256(Rgb(196, 58, 46)), to_ansi256(Rgb(196, 58, 46)));
    // Pin the exact cube index for crimson (compute once, freeze as golden).
    assert_eq!(to_ansi256(Rgb(196, 58, 46)), /* PIN */ 167);
    // Grayscale ramp: mid-gray maps into 232..=255.
    assert!((232..=255).contains(&to_ansi256(Rgb(128, 128, 128))));
}

#[test]
fn test_cell_fg_bg_pick_deterministic_tiebreak() {
    // A cell whose subcells carry two distinct colors: fg = majority-lit color,
    // ties broken by nearest depth, then fixed subcell scan order. Assert stable.
}
```
*(Compute the crimson `to_ansi256` value once during Step 3 and freeze it into the `/* PIN */` slot before Step 4 — do not guess it.)*

- [ ] **Step 2: Run** — Expected: FAIL.

- [ ] **Step 3: Implement `color.rs`.**
  - `heat`: verbatim transcription of the deck's `heat`/`mix` (`mix(a,b,k)=a+(b-a)*k`, `|0` = `.floor() as u8`). This IS the crimson/gold truecolor canon.
  - `to_ansi256`: mine `tui-3d/src/char.rs`: grayscale (`r==g==b`) → 232..=255 ramp (`<8→16`, `>248→231`, else `((v-8)/247*24).round()+232`); else `16 + 36*round(r/255*5) + 6*round(g/255*5) + round(b/255*5)`. **Tie-break pinned:** use `f64::round` (round-half-away-from-zero, IEEE-754 reproducible per user CLAUDE.md §4) — never `as u8` truncation. This is integer-valued output so it gets a byte-exact golden (Step 1) despite the float path.
  - **fg/bg under constraint** (braille cells carry exactly one fg + one bg): fg = color of the **majority** of lit subcells; ties broken by (a) nearest depth, then (b) lowest subcell index (fixed scan order — never "whichever triangle rasterized first", the `tui-3d` half-block-shader nondeterminism gap). bg = the second-most-common lit color, else `Rgb(0,0,0)`. Under `Palette::Ansi256`, quantize fg+bg through `to_ansi256` *after* the pick (pick in truecolor, degrade last).
  - `CellGrid::to_ansi`: emit `\x1b[0m` reset + `\x1b[38;2;r;g;bm` fg + `\x1b[48;2;r;g;bm` bg (truecolor) or `\x1b[38;5;{n}m`/`48;5` (256) + glyph, per cell.

- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): color-under-constraint — heat canon, 256 palette, per-cell fg/bg pick`.

---

### Task 5: Transformation algebra — pure ~30-line fn (s=v·e, r, T, Φ) with byte-exact golden fixtures

**Files:** Create `src/raster/algebra.rs`; modify `src/raster/mod.rs`; create `tests/test_deck_fixture.rs`.

**Interfaces:** `raster::algebra::{Industry, transform, Economy}`. `Industry { name: String, lane: u8, c: f64, v: f64, e: f64 }`. `transform(inds: &[Industry]) -> Economy` where `Economy { r: f64, big_s: f64, cv: f64, t: Vec<f64>, phi: f64, rent_imperial: f64, rent_internal: f64 }`. Formulae (`topology-deck.html:181-193`): `s_i = v_i·e_i`; `S = Σs_i`; `CV = Σ(c_i+v_i)`; `r = S/CV`; `T_i = r·(c_i+v_i) − s_i`; `rent_imperial = −Σ_{lane=0}T_i`; `rent_internal = −Σ_{lane=1}T_i`; `Φ = Σ_{T_i>0}T_i`. This is the algebra the Sankey (Task 9) and Stage 0 (Task 10) consume; its **output numbers are the golden fixtures.** RNG-free ⇒ fully portable (no mulberry32 needed here).

The 13-industry table (embed as the Stage-0 fixture constructor `deck_industries()`; scout-verified c/v/e/lane):

| # | name | lane | c | v | e |
|---|---|---|---|---|---|
|0|export agriculture|0|14|10|3.2|
|1|textile assembly|0|10|9|3.0|
|2|extractive mining|0|26|7|2.6|
|3|migrant agriculture|1|9|8|2.4|
|4|meatpacking|1|12|8|2.2|
|5|logistics depots|1|18|9|1.9|
|6|carceral labor|1|4|5|4.0|
|7|defense aerospace|2|34|8|1.1|
|8|suburban construction|2|20|9|1.2|
|9|professional strata|2|16|11|0.9|
|10|pharma IP|3|44|6|1.0|
|11|platform rents|3|52|7|1.1|
|12|finance|3|60|5|0.8|

- [ ] **Step 1: Write the failing test** (golden numbers scout-verified via faithful node.js re-execution):
```rust
use hypergraph_rs::raster::algebra::{transform, deck_industries};

#[test]
fn test_raw_buildworld_r_before_any_tick() {
    let e = transform(&deck_industries());
    assert!((e.r - 0.47102137767220903).abs() < 1e-12); // raw buildWorld() r
    assert!((e.big_s - 198.3).abs() < 1e-9);            // S invariant
    assert!((e.cv - 421.0).abs() < 1e-9);               // ΣCV
}

#[test]
fn test_net_transfer_conservation_is_zero() {
    let e = transform(&deck_industries());
    let net: f64 = e.t.iter().sum();
    assert!(net.abs() < 1e-12, "ΣT must be 0 (zero-sum), got {net}");
}

#[test]
fn test_phi_equals_rent_sum_at_seed_1917_fixture() {
    // Deviation 6: empirical at these parameters, NOT a structural law.
    let e = transform(&deck_industries());
    assert!((e.phi - (e.rent_imperial + e.rent_internal)).abs() < 1e-9);
}
```
*(Note: `0.4608…` is the displayed r AFTER the deck's one startup `doTick()` — Task 10 covers the ticked value. Raw `transform` on the untouched table gives `0.47102…`.)*

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `algebra.rs`** — the ~30-line pure fn exactly per the formulae above; `deck_industries()` returns the 13-row table. Iteration over `inds` in slice order (deterministic).
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): transformation algebra (s=v·e, r, T, Φ) + 13-industry deck fixture`.

---

### Task 6: mulberry32 deck RNG port + buildWorld fixture + o(C) organization degree

**Files:** Create `src/raster/deck.rs`; modify `src/raster/mod.rs`, `tests/test_deck_fixture.rs`.

**Interfaces:** `raster::deck::{Mulberry32, build_world, org_degree, global_pi0, deck_hypergraph, DeckWorld}` (`deck_hypergraph(&DeckWorld) -> Hypergraph<Value,Value,Value>` is the shared `&Hypergraph` adapter reused by Task 7's spectral disc — Finding 6). `Mulberry32` is the byte-exact JS port (`topology-deck.html:120-121`): state `u32`, `next() -> f64` via `Math.imul`/`>>>`/`|0` semantics in Rust `wrapping_*` ops. `build_world(seed: u32) -> DeckWorld` ports `buildWorld` (`:129-179`): 28 agents in 3 blocs (`[{n:5,bloc:0,w:8.5},{n:9,bloc:1,w:5.6},{n:14,bloc:2,w:2.1}]`, `wealth = w + rnd()*1.4 - 0.7`, `subsistence = 3.0`), 4 communities (`cell α`/`cell β` = 6 periphery each, `union γ` = 6 core-labor, `capital δ` = 4 capital; capital δ = full chain, others 1 link; members via seeded Fisher-Yates), 4 extraction hyperedges (≤4 deduped periphery tails → 1 capital head, `mass = 1.4 + rnd()*1.2`). The returned `DeckWorld` also bundles the 13-industry table (`industries: Vec<Industry>`, initialized from `algebra::deck_industries()` — exactly the deck's `W.industries`, deck :158-177) so `do_tick` (Task 10) can mutate agents and industries **together** (Finding 1). `org_degree(community) -> f64 = 1.0` if `|C|<2` else `1 - (|Π₀(C)|-1)/(|C|-1)` — **reuse `algorithms::connected_components` on the induced `subhypergraph` (`core/globalviews.rs:13`)** for `|Π₀(C)|`. `global_pi0(&DeckWorld) -> usize` unions all community links + all extraction tails→head.

- [ ] **Step 1: Write the failing test** (golden vectors scout-verified):
```rust
use hypergraph_rs::raster::deck::{Mulberry32, build_world, org_degree, global_pi0};

#[test]
fn test_mulberry32_golden_vector_seed_1917() {
    let mut r = Mulberry32::new(1917);
    let got: Vec<f64> = (0..8).map(|_| r.next()).collect();
    let want = [0.9034266413655132, 0.5877873066347092, 0.40103743225336075,
                0.3932968541048467, 0.879000983433798, 0.6506219764705747,
                0.570820358581841, 0.25069746002554893];
    for (g, w) in got.iter().zip(want) { assert!((g - w).abs() < 1e-15, "{g} != {w}"); }
}

#[test]
fn test_mulberry32_spectral_seed_derivation() {
    // spectral() uses mulberry32(W.seed ^ 0x5EED) = mulberry32(1917 ^ 24301) = mulberry32(22928)
    assert_eq!(1917u32 ^ 0x5EED, 22928);
    let mut r = Mulberry32::new(22928);
    let want = [0.17359042004682124, 0.14775428897701204, 0.5368493683636189, 0.9822371983900666];
    for w in want { assert!((r.next() - w).abs() < 1e-15); }
}

#[test]
fn test_org_degrees_at_seed_1917() {
    let w = build_world(1917);
    let orgs: Vec<f64> = w.communities.iter().map(org_degree).collect();
    // cell α, cell β, union γ = 0.2 (6 members, 1 link, 5 components); capital δ = 1 (chain).
    assert!((orgs[0] - 0.2).abs() < 1e-12);
    assert!((orgs[1] - 0.2).abs() < 1e-12);
    assert!((orgs[2] - 0.2).abs() < 1e-12);
    assert!((orgs[3] - 1.0).abs() < 1e-12);
}

#[test]
fn test_global_pi0_is_13() {
    assert_eq!(global_pi0(&build_world(1917)), 13);
}
```

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `deck.rs`.** `Mulberry32`: `self.a = self.a.wrapping_add(0x6D2B79F5); let mut t = self.a; t = (t ^ (t >> 15)).wrapping_mul(t | 1); t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61)); ((t ^ (t >> 14)) as f64) / 4294967296.0` (the standard mulberry32 — verify against the Step-1 golden vector). `build_world` transcribes `:129-179` using `Mulberry32` for every `rnd()`. Build a shared `deck_hypergraph(&DeckWorld) -> Hypergraph<Value,Value,Value>` adapter (28 agents → nodes keyed by agent id; each community link → a 2-node edge) — the single bridge from `DeckWorld` to the shipped `&Hypergraph` APIs, since `subhypergraph`, `connected_components`, and Task 7's `normalized_laplacian` all take `&Hypergraph`, NOT `DeckWorld` (Finding 6). `org_degree` then builds the per-community induced subgraph via `subhypergraph` over that adapter and counts components via `connected_components`. Wire the 4 communities' link structure exactly (capital δ chain = |C|-1 links; others exactly 1). `global_pi0` hand-rolls path-halving union-find over all 28 agents (per `:196-204`), unioning community links + extraction tails→head.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): mulberry32 deck RNG port + buildWorld fixture + o(C) organization degree`.

---

### Task 7: Instrument I — the adjoint cylinder (`state → SceneGraph3D`)

**Files:** Create `src/raster/instruments/mod.rs`, `src/raster/instruments/cylinder.rs`; modify `src/raster/mod.rs`; create `tests/test_instruments.rs`. **Feature-gated `cells3d`** (spectral embedding = 3D layout, S9 lane gate). **`test_instruments.rs` MUST begin with a crate-level `#![cfg(feature = "cells3d")]` header (Finding 2)** — it imports the `cells3d`-gated `raster::instruments` module, so at DEFAULT features (cells3d OFF, per Task 15 / `mise run rust:check`) the whole file must compile to nothing rather than a missing-module error; this matches the shipped precedent `tests/test_communities.rs`, `test_convert.rs`, `test_dynamics.rs`, which each open with `#![cfg(feature = "…")]`.

**Interfaces:** `raster::instruments::Instrument { Cylinder, Terrain, Sankey }`; `raster::instruments::build(inst: Instrument, world: &DeckWorld, opts: &InstrumentOpts) -> SceneGraph3D`. Cylinder builder (`topology-deck.html:294-336`): bottom disc = every agent as a lone point (the atomized class-in-itself) at `(x·R·.85, -H/2, y·R·.85)` where `(x,y)` are **spectral positions** (reuse `communities::smallest_k_eigenvectors`, pinned seed — Deviation 2); each community's hull at height `y = -H/2 + o(C)·H`; ghost-edge struts from every member's bottom-disc point to its community-height point. Constants `R=1.05`, `H=1.55`. Hull via `layout::convex_hull` on the **camera-projected 2D points** (the deck computes hull post-projection; but PHASE-R-MAP's SceneGraph3D/rasterizer split wants camera-independent scene data — see OPEN DECISION D-R6; **default: world-space hull projected afterward**, so `SceneGraph3D` is camera-independent). Hull face color `heat(1 - o)` at opacity `0.12 + 0.3·o`.

**D-R1 dissolved here:** `InstrumentOpts { cylinder_height: CylinderHeight }` with `enum CylinderHeight { OrgDegree, HullFill }`, **default `OrgDegree`** (the deck's `o(C)`). `HullFill` (simpliciality / fill-fraction) is exposed but not default — no owner call needed, both ship.

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::deck::build_world;
use hypergraph_rs::raster::instruments::{build, Instrument, InstrumentOpts};

#[test]
fn test_cylinder_community_height_equals_org_degree() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    // capital δ (o=1) sits at top (+H/2); the three o=0.2 communities near the bottom.
    // Assert the community hull centroids' y = -H/2 + o*H within 1e-9.
}

#[test]
fn test_cylinder_every_agent_on_bottom_disc() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    // 28 agent points all at y = -H/2.
}

#[test]
fn test_cylinder_scene_is_camera_independent_and_deterministic() {
    let w = build_world(1917);
    assert_eq!(build(Instrument::Cylinder, &w, &InstrumentOpts::default()),
               build(Instrument::Cylinder, &w, &InstrumentOpts::default()));
}
```

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `cylinder.rs`.** First build the deck `Hypergraph<Value,Value,Value>` via the shared `deck_hypergraph(&DeckWorld)` adapter (Task 6: 28 agents → nodes, community links → 2-node edges), then reuse the eigensolver with the **correctly destructured** signatures — `normalized_laplacian(&hg)?` returns `(Array2<f64>, Vec<String>)` and takes `&Hypergraph` (NOT `DeckWorld`), so: `let (l, _ids) = normalized_laplacian(&hg)?; let ev = smallest_k_eigenvectors(&l, 2, Some(SPECTRAL_SEED));` — use the eigenvector columns for the disc `(x,y)` (pin `SPECTRAL_SEED` const, never `None` — scout mandate; Finding 6). Emit agent `Node3`s on the bottom disc, community hull `Face`s at their org-degree height (via `convex_hull` on projected members — world-space per D-R6 default), and per-member `Strut`s. Color via `heat`. Bounded loops: agents, communities, members.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): instrument I — adjoint cylinder (o(C) heights, spectral disc, ghost struts)`.

---

### Task 8: Instrument II — contradiction terrain (IDW heightfield + curvature cliffs)

**Files:** Create `src/raster/instruments/terrain.rs`; modify `instruments/mod.rs`, `tests/test_instruments.rs`. **Feature-gated `cells3d`.**

**Interfaces:** Terrain builder (`topology-deck.html:338-359`): per-agent tension `t = max(0, repression·(subsistence − wealth))` (only sub-subsistence agents contribute); IDW field `height(x,y) = (Σ_n t_n/(d²+ε)) / (Σ_n 1/(d²+ε)) / tmax`, `ε = 0.02` (scout: **materially smooths**, carry it exactly, not a knob), `tmax = max(0.001, max_n t_n)`; sampled on an `N=34 × M=24`-quad grid over world extent `S=1.35` (`x,z ∈ [-S,S]`), heightfield `y = -0.55 + t·0.95`; quad face color `heat(t)` at opacity `0.16 + t·0.55` (per-quad `t` = mean of two diagonal corners). Agents drawn on top at the SAME `height()` evaluated at their `(x,y)`, colored by bloc. **Curvature cliffs at bridge edges** (PHASE-R-MAP §3): where a community-link edge spans two spectral clusters (a "bridge"), raise the local curvature — for the deck fixture this is the IDW field itself; leave a documented seam `bridge_cliff(&mut heightfield, world)` that babylon's real per-edge Ollivier-Ricci gradients slot into later (no new formalism now).

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn test_terrain_grid_dimensions_and_extent() {
    // (N+1)*(M+1) = 35*25 = 875 grid vertices; extent [-1.35, 1.35].
}
#[test]
fn test_terrain_tension_only_below_subsistence() {
    // An agent with wealth >= subsistence contributes t = 0.
}
#[test]
fn test_terrain_idw_epsilon_is_exact_0_02() {
    // Field evaluated AT a node is finite and smoothed (ε=0.02), not a singularity.
}
#[test]
fn test_terrain_deterministic() { /* build == build */ }
```

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `terrain.rs`** per the constants above; emit the quad grid as pairs of `Face` triangles + agent `Node3`s. Bounded loops: `N·M` quads, agents. Constants `N=34, M=24, S=1.35, ε=0.02` as named `const`s with the scout's "not a render knob" note in a doc comment.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): instrument II — contradiction terrain (IDW heightfield, bridge cliffs)`.

---

### Task 9: Instrument III — OCC hyper-Sankey over the u64 H3 grid

**Files:** Create `src/raster/instruments/sankey.rs`, `src/raster/h3.rs`; modify `instruments/mod.rs`, `tests/test_instruments.rs`. **Feature-gated `cells3d`** (uses `h3o`).

**Interfaces:** Sankey builder (`topology-deck.html:361-435`) over the transformation algebra (Task 5). Each industry/cell snaps to a real `h3o::CellIndex` (via `raster/h3.rs`: pick a res-8 origin, place each industry at axial `(hq,hr)` via the **inverse** of `to_local_ij` — `CellIndex::try_from(LocalIJ::new(origin, hq, hr))` constructs a cell FROM offsets, whereas `to_local_ij` only goes the forward cell→ij direction — handling `LocalIjError::Pentagon` explicitly; **verify the exact `LocalIJ`/`try_from` API against docs.rs 0.10** (Finding 10)). Ground hexagon per entity at `hexXZ(q,r,s) = {x: s·√3·(q + r/2), z: s·1.5·r}`; stem from `y=-0.95` to altitude `occY(c,v) = -0.8 + 1.55·clamp((ln(c/v) + 0.3)/2.6, 0, 1)`; node radius `∝ √(c+v)`; node fill `heat(min(1, e/4))`; outline gold if `T>0` else crimson. **Ribbons** = transformation transfers: donors `{T<0}` sorted ascending, `negSum = Σ(−T_donor)`; for each absorber `j` and donor `i`, `flow = j.T·(−i.T)/negSum` (proportional bipartite allocation — a multi-tail→head extraction IS a directed hyperedge, PHASE-R-MAP §3). Sub-`0.4` flows skipped from **rendering** only; conservation tested on the full allocation (Deviation 5). Bezier midpoint raised `+0.32`. **FINE/COARSE zoom = coarse-graining functor:** FINE = 13 industries; COARSE = aggregate via `h3o CellIndex::parent()` (extensives `c,v,s,T,n` summed, position averaged, `e = s/v` recomputed) — the zoom functor `GROUP BY parent(h3_index)` (h3o scout: `parent` is a bit-op, `children` a contiguous range). **NO wall-clock flow-dot** (Deviation 3).

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::instruments::sankey::{build_sankey, Zoom};
use hypergraph_rs::raster::algebra::deck_industries;

#[test]
fn test_occ_altitude_finance_is_highest() {
    // finance q=12 => occY clamps to +0.75 (top); carceral labor q=0.8 => occY≈-0.7542.
}
#[test]
fn test_ribbon_flows_conserve_to_phi_unrendered() {
    // Σ over all donor→absorber flows (no 0.4 threshold) == Φ, both directions.
}
#[test]
fn test_net_t_zero_at_both_zooms() {
    let fine = build_sankey(&deck_industries(), Zoom::Fine);
    let coarse = build_sankey(&deck_industries(), Zoom::Coarse);
    assert!(fine.net_t().abs() < 1e-12);
    assert!(coarse.net_t().abs() < 1e-12); // partition-independent (Deviation 7)
}
#[test]
fn test_r_invariant_under_aggregation() {
    // r = ΣS/ΣCV is identical fine vs coarse (ratio of sums).
}
#[test]
fn test_h3_parent_is_u64_bit_op_never_string() {
    // CellIndex round-trips as u64; parent() reduces resolution; no String in the hot path.
}
```

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `h3.rs` + `sankey.rs`.** `h3.rs`: `industry_cell(origin: CellIndex, hq: i32, hr: i32) -> Result<CellIndex, LocalIjError>` (builds the cell via `CellIndex::try_from(LocalIJ::new(origin, hq, hr))` — the **inverse** of `to_local_ij`; Finding 10) and `aggregate_by_parent(cells: &[(CellIndex, Extensives)], coarse_res: Resolution) -> Vec<(CellIndex, Extensives)>` (GROUP BY `parent`). `sankey.rs`: `occY`, `hexXZ`, ribbon allocation, `Zoom` enum, `SankeyScene` with `net_t()`. Verify BOTH `CellIndex::parent` return type (Option vs Result) AND the `LocalIJ::new` / `CellIndex::try_from(LocalIJ)` inverse-index signatures + the `LocalIjError::Pentagon` arm against docs.rs 0.10 before binding (scout gap + Finding 10). Bounded loops: industries, donor×absorber pairs.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): instrument III — OCC hyper-Sankey over u64 H3 grid (h3o parent zoom, T ribbons)`.

---

### Task 10: Stage 0 — deck-world reproduction (falling-r sequence, net-T at both zooms, ticks)

**Files:** Modify `tests/test_deck_fixture.rs`; create `src/raster/deck.rs::do_tick` (economy tick).

**Interfaces:** `raster::deck::do_tick(world: &mut DeckWorld)` (`topology-deck.html:243-252`) — takes the **full DeckWorld** (agents + the 13-industry table, exactly as the deck's `W` bundles `W.agents` and `W.industries`), because `doTick` FIRST mutates agent wealth via the extraction take and only THEN accumulates `c` (Finding 1). Order verbatim (deck :245-251): (1) **wealth extraction** — for every extraction edge, `take = mass·0.12`; each tail loses `take/|tails|` under a `Math.max(0.2, …)` floor; the head gains `take·0.8`; every bloc-1 agent gains `take·0.2/9`; (2) **c-accumulation** — each industry's `c ×= 1.035` if `lane>=2` else `×= 1.015` (`v,e` untouched); (3) re-`transform(&world.industries)`. The wealth step is exactly why `pw` reads `1.8250060787286078` **POST-tick** (not the pre-tick 1.8855) and `psa` follows from it. This is Stage 0 — the deck-number acceptance gate. **Every assertion here is a scout-verified golden.**

- [ ] **Step 1: Write the failing test:**
```rust
use hypergraph_rs::raster::algebra::transform;
use hypergraph_rs::raster::deck::{build_world, do_tick};

#[test]
fn test_displayed_r_after_one_startup_tick() {
    let mut world = build_world(1917);
    do_tick(&mut world); // the deck runs doTick() once before first paint (:499)
    assert!((transform(&world.industries).r - 0.4608359187088229).abs() < 1e-12);
}

#[test]
fn test_falling_r_sequence_six_ticks() {
    let want = [0.4608359187088229, 0.45077630191919216, 0.44084464189389844,
                0.43104290735742473, 0.4213729206846403, 0.4118363577727333];
    let mut world = build_world(1917);
    for w in want {
        do_tick(&mut world);
        assert!((transform(&world.industries).r - w).abs() < 1e-12);
    }
}

#[test]
fn test_s_invariant_under_ticks() {
    let mut world = build_world(1917);
    for _ in 0..6 { do_tick(&mut world); assert!((transform(&world.industries).big_s - 198.3).abs() < 1e-9); }
}

#[test]
fn test_net_t_zero_every_tick_and_both_zooms() {
    // ΣT == 0 at tick1..6 (fine); and after aggregation (coarse) — Deviation 7.
}

#[test]
fn test_org_and_pi0_and_psa_golden() {
    // From Task 6 (all built on build_world(1917) then one do_tick):
    //   Π₀=13 (byte-exact), org=[0.2,0.2,0.2,1], meanOrg=0.4,
    //   pw=1.8250060787286078 (POST-tick, byte-exact — pure arithmetic, Finding 1),
    //   psr=0.22222222222222215 (pure division, byte-exact).
    // psa = sigmoid(pw-3.0) routes through exp() ⇒ TOLERANCE-bounded golden, NOT byte-exact:
    //   assert (psa - 0.23595349607058727).abs() < 1e-12
    //   (a faithful Python re-derivation gives 0.23595349607058722 — last-ULP drift;
    //    exp is the non-reproducible libm op, user CLAUDE.md §4 / Finding 3).
    // rupture NOT armed (psr<=psa).
}
```

- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement `do_tick(world: &mut DeckWorld)`** porting `doTick` (:245-251) verbatim IN ORDER (Finding 1): (1) **wealth extraction** over `world.extraction` — `take = mass·0.12`; each tail `wealth = max(0.2, wealth − take/|tails|)`; head `wealth += take·0.8`; every bloc-1 agent `wealth += take·0.2/9`; THEN (2) **c-accumulation** — `c ×= lane>=2 ? 1.035 : 1.015` (`v,e` untouched); THEN (3) `transform(&world.industries)`. Then add readouts computed on the **post-tick** state: `psa = sigmoid(pw − 3.0)` where `pw` = mean bloc-2 (periphery) wealth AFTER the tick (a tolerance-bounded golden — sigmoid routes through `exp`, Finding 3), `psr = min(1, (org0+org1)/2 / (repression/2.6·0.9))` — the deck's *calibrated* form, `:444`; note in a doc comment this diverges from the textbook `P(S|R)=Org/Repression`, see OPEN DECISION D-R7). Baseline `repression = 2.6`.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `test(hypergraph-rs): Stage 0 — deck-world number reproduction (falling-r, net-T, Π₀, org, psa)`.

---

### Task 11: Stage 1 — SceneGraph3D JSON golden fixtures (assert-the-source)

**Files:** Create `tests/fixtures/scene3d/{cylinder,terrain,sankey_fine,sankey_coarse}.json`; modify `tests/test_instruments.rs`.

**Interfaces:** For each instrument builder, serialize its `SceneGraph3D` (seed 1917, default opts) to JSON via the **derived `serde`/`ryu`** serializer (NOT `fmt6`-rounded floats — Finding 5: `fmt6` would round e.g. `0.4608359…→0.460836` and break Task 1's `from_str`→equal round-trip; `ryu` is already deterministic and platform-independent for a fixed `f64` bit pattern) and assert byte-equality against a committed golden file (the assert-the-source class, PHASE-R-MAP §4). Golden files are generated once (Step 3) then frozen.

- [ ] **Step 1: Write the failing test** — one `assert_eq!(serde_json::to_string_pretty(&scene)..., include_str!("fixtures/scene3d/<name>.json"))` per instrument. Expected FAIL: fixture files absent.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Generate + freeze goldens** — run a one-shot to write each scene's JSON to its fixture path, **eyeball for sanity** (heights, counts, colors match the deck), then commit the files. Serialize via the **derived `serde`/`ryu`** path (Task 1's `Serialize`), NOT `fmt6` (Finding 5); `fmt6` is reserved for hand-built banner/label **text** only. **Tolerance caveat (Finding 3):** the cylinder/terrain scenes are pure basic-arithmetic ⇒ their `ryu` JSON is byte-exact cross-platform; but the **Sankey** scene's `occY` altitude routes through `ln` (deck :362) — a libm transcendental that does NOT reproduce bit-for-bit across implementations (user CLAUDE.md §4). So the Sankey JSON golden is **platform-pinned** (byte-exact on the reference platform, frozen here), and any `ln`/`exp`-derived field carries an explicit tolerance policy rather than a universal byte-exact guarantee. The earlier "libm-free — only basic ops, IEEE-reproducible" justification is FALSE for the Sankey path and is retracted here.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `test(hypergraph-rs): Stage 1 — SceneGraph3D JSON golden fixtures (3 instruments)`.

---

### Task 12: Stage 2 — CellGrid text golden frames per (instrument, camera, cols×rows, palette)

**Files:** Create `tests/fixtures/frames/*.txt`; modify `tests/test_raster.rs`.

**Interfaces:** For a fixed matrix of `{instrument} × {camera angle} × {cols×rows} × {palette}`, rasterize and assert `CellGrid::to_text()` byte-equals a committed `.txt` golden. Recommended matrix: 3 instruments × 2 cameras (front, 3/4) × 1 size (80×40) × `TrueColor` glyph-floor text (the `to_text` form is palette-independent — glyphs only — so palette is asserted separately via a small `to_ansi()` golden). Keep the set minimal but covering (MVP: 6 frame goldens + 1 ansi golden). **Feature gating (Finding 2):** `test_raster.rs` is default-on (`raster`; cells3d OFF by default) and MUST stay compilable with cells3d off — so every frame golden built from a `cells3d`-gated instrument builder is wrapped in a per-test `#[cfg(feature = "cells3d")]` (the file stays ungated at the crate level, unlike `test_instruments.rs`, so Tasks 1-4's camera/grid/color tests still compile and run at default features).

- [ ] **Step 1: Write the failing test** — `assert_eq!(rasterize(&scene, &cam, 80, 40).to_text(), include_str!("fixtures/frames/<name>.txt"))` per matrix cell. Expected FAIL: fixtures absent.
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Generate + freeze** — write each frame, **visually inspect** the braille render is coherent (silhouettes read, labels present), then commit. These are the byte-exact presentation goldens.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `test(hypergraph-rs): Stage 2 — CellGrid text golden frames (instrument×camera×size)`.

---

### Task 13: Stage 3 — property tests (depth-sort stability, zoom-invariance, glyph-floor parity, mutation)

**Files:** Modify `tests/test_instruments.rs`, `tests/test_raster.rs`.

**Interfaces:** The five behavioral-contract properties (redundant verification, user CLAUDE.md "Tests as Behavioral Contracts §3):
1. **Depth-sort stability:** shuffling equal-depth primitives yields an identical frame (stable sort, no draw-order dependence).
2. **Zoom-invariance of extensives vs intensives:** `Σ` of every extensive (`c,v,s,T`) is identical fine vs coarse; intensive `r` is identical (ratio of sums); a *per-cell* intensive (e.g. `e`) is NOT required equal (it is recomputed post-aggregation) — assert the split explicitly.
3. **Glyph-floor information parity:** every labeled scalar the instrument computes (`r`, `Φ`, each rendered `o(C)`, `psa`/`psr`) appears as a literal substring in `CellGrid::to_text()` — no quantity is pixel-only (PHASE-R-MAP §4; the deck's `PHI=… r=…` banner is the model).
4. **Determinism sweep:** `rasterize(s,c,w,h) == rasterize(s,c,w,h)` across many random `(camera, size)` — bit-identical. Random-camera frames are required only to be **deterministic AND panic-free** (near-plane clip in Task 2, NaN-safe depth compare, bounds-clamped subcell writes in Task 3) — NOT visually meaningful: a camera behind/through the scene must yield a stable frame, never an `inf`/`NaN` coord or an out-of-bounds index panic (Finding 4).
5. **Mutation test (Deviation 6):** perturb one industry's `c/v/e` so a lane-2/3 industry becomes a donor; assert `Φ == rent_imperial + rent_internal` now **fails** — proving that equality is a seed-1917 fixture property, not a structural law, and that the test would catch a regression that hard-coded it.

- [ ] **Step 1: Write the failing tests** (one per property).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement** — mostly test-side; any small hooks (`SceneGraph3D` labeled-scalar accessor for property 3) added minimally.
- [ ] **Step 4: Run** — Expected: PASS.
- [ ] **Step 5: Commit** — `test(hypergraph-rs): Stage 3 — property contracts (depth stability, zoom invariance, glyph parity, mutation)`.

---

### Task 14: Rust-side PNG pixel-plate emission (ADR097 Tier-1)

**Files:**
- Create: `src/raster/png.rs`, `crates/hypergraph-rs/tests/test_png.rs`, `tests/fixtures/png/cylinder_128x96.png`
- Modify: `src/raster/mod.rs` (re-export `png` module under the `raster-png` gate)
- (No new manifest work: Task 1 already added the optional `png` dep, the workspace pin, and the `raster-png = ["cells3d","dep:png"]` feature.)

**Feature-gated `raster-png`** (`= ["cells3d", "dep:png"]`, owner-gated OUT of `default`; see Global Constraints + Task 1). ADR097 Tier-1 is a **pixel** plate: an actual RGB pixel raster of the SAME `SceneGraph3D` the instruments build — a **different tier** from the Tier-0 braille `CellGrid`. Tier-0 packs 2×4 subcells into one braille cell; Tier-1 emits **one `Rgb` per pixel** at arbitrary `(width_px, height_px)`. The kitty-TGP consumer path (ADR097) reads these PNG bytes. Because it renders the 3D scene, it rides on the `cells3d` 3D-lane gate; the pure-Rust `png` encoder is the only added dep. **DRY (project law, reuse ledger): the pixel path MUST reuse the shared geometry — it does NOT duplicate the rasterization math.**

**Interfaces:**
- `raster::png::PixelBuffer { w: u32, h: u32, px: Vec<Rgb> }` — RGB8, row-major, `px.len() == (w*h) as usize`. Derive `Debug + Clone + PartialEq`.
- `raster::png::render_pixels(scene: &SceneGraph3D, cam: &Camera, width_px: u32, height_px: u32) -> PixelBuffer` — the intermediate full-resolution pixel raster. It **shares** the geometry from Tasks 2/3: `project` (rotation + perspective divide + **near-plane clip** `NEAR_EPS`) and `depth_sort` (stable painter's) from **Task 2**, and the edge-function scanline **triangle fill** (explicit top-left rule), integer **Bresenham** line walk, **NaN-safe** depth test, and **bounds-clamped** subcell/pixel writes from **Task 3** — targeting a **pixel** depth buffer at `(width_px × height_px)` instead of the `cols*2 × rows*4` braille `SubBuffer`. Factor the rasterization core (edge function, top-left tie-break, depth test, Bresenham, clamp) so ONE implementation is parameterized by output resolution: the Tier-0 braille path passes `(cols*2, rows*4)` + `compose_braille`; the Tier-1 pixel path passes `(width_px, height_px)` and reads the buffer straight out as `Rgb` per pixel. The rotation+perspective-divide+near-plane-clip NDC core of `project` (Task 2 steps 1–2) is shared verbatim; only the final NDC→viewport remap resolution differs. Do NOT reimplement `fill_triangle`/`line_bresenham`/`project`/`depth_sort` — reuse ledger.
- `raster::png::render_png(scene: &SceneGraph3D, cam: &Camera, width_px: u32, height_px: u32) -> Vec<u8>` — encodes the `PixelBuffer` via the `png` crate with **PINNED encoder settings** (color type RGB8, bit depth 8, compression + filter fixed to ONE frozen choice, e.g. `Compression::Fast` + `FilterType::NoFilter`) for reproducibility. Verify the exact `png` 0.18 `Encoder`/`Compression`/`Filter` setter API against docs.rs before binding (Task 1 note).

**Files:** create `src/raster/png.rs`; modify `src/raster/mod.rs` (`#[cfg(feature = "raster-png")] pub mod png;`); create `tests/test_png.rs` with the crate-level header `#![cfg(feature = "raster-png")]` (Finding-2 precedent: `test_communities.rs`/`test_convert.rs`/`test_dynamics.rs` each open with `#![cfg(feature = "…")]`, so at default features — `raster-png` OFF — the file compiles to nothing rather than a missing-module error).

- [ ] **Step 1: Write the failing test** — `tests/test_png.rs`:
```rust
#![cfg(feature = "raster-png")]
use hypergraph_rs::raster::deck::build_world;
use hypergraph_rs::raster::instruments::{build, Instrument, InstrumentOpts};
use hypergraph_rs::raster::png::{render_pixels, render_png};
use hypergraph_rs::raster::{Camera, Rgb, SceneGraph3D};

fn fixture() -> (SceneGraph3D, Camera) {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    (s, Camera { ry: 0.7, rx: 0.3, dist: 4.0, fov: 70.0 })
}

#[test]
fn test_render_pixels_dims_and_len() {
    let (s, c) = fixture();
    let pb = render_pixels(&s, &c, 128, 96);
    assert_eq!((pb.w, pb.h), (128, 96));
    assert_eq!(pb.px.len(), 128 * 96);
}

#[test]
fn test_render_pixels_is_pure_function_byte_exact() {
    let (s, c) = fixture();
    assert_eq!(render_pixels(&s, &c, 128, 96), render_pixels(&s, &c, 128, 96));
}

#[test]
fn test_png_decodes_back_to_exact_pixelbuffer() {
    // PRIMARY golden — platform-INDEPENDENT for the pure-arithmetic geometry path:
    // decode the encoded PNG and assert the RGB pixels are byte-exact vs render_pixels.
    // (The decode dodges the cross-platform DEFLATE trap: pixels, not compressed bytes.)
    let (s, c) = fixture();
    let pb = render_pixels(&s, &c, 128, 96);
    let bytes = render_png(&s, &c, 128, 96);
    let mut decoded: Vec<Rgb> = Vec::new();
    // decode `bytes` via png::Decoder → RGB8 rows → Vec<Rgb> (verify 0.18 reader API on docs.rs)
    /* ...decode into `decoded`... */
    assert_eq!(decoded, pb.px);
}

#[test]
fn test_png_encoded_bytes_match_pinned_fixture() {
    // SECONDARY golden — PLATFORM-PINNED: encoded .png bytes are byte-exact on the
    // reference platform under the pinned `png` 0.18 crate + pinned encoder settings.
    // DEFLATE output is NOT guaranteed identical across arbitrary zlib impls (CLAUDE.md §4),
    // so this is a platform-pinned fixture, NOT a universal byte-exact guarantee.
    let (s, c) = fixture();
    assert_eq!(render_png(&s, &c, 128, 96), include_bytes!("fixtures/png/cylinder_128x96.png"));
}

#[test]
fn test_png_random_camera_deterministic_and_panic_free() {
    // Determinism + panic-free property: many random cameras → stable output, never
    // NaN/inf/oob. Reuses the near-plane clip (Task 2) + NaN-safe depth compare +
    // bounds-clamped pixel writes (Task 3): a camera behind/through the scene yields a
    // stable frame, never a panic. Render each camera twice; assert equal.
    let (s, _) = fixture();
    /* for a fixed list of adversarial (ry, rx, dist, fov): render_png twice, assert_eq */
    let _ = &s;
}
```

- [ ] **Step 2: Run** — `cd hypergraph-rs && cargo test -p hypergraph-rs --features raster-png --test test_png`. Expected: FAIL — module `raster::png` does not exist.

- [ ] **Step 3: Implement `png.rs`.**
  - `PixelBuffer` as above. `render_pixels`: reuse the shared geometry core (Task 2 `project` + near-plane clip + `depth_sort`; Task 3 `fill_triangle` top-left rule + `line_bresenham` + NaN-safe depth test + bounds-clamp), targeting a `(width_px × height_px)` `Rgb` + depth buffer (depth cleared to `f64::INFINITY`, farther primitives painted first, nearer wins). Refactor Task 3's rasterization primitives to take the target resolution as a parameter (or operate on a generic pixel target) so the braille and pixel tiers call ONE implementation — no copied triangle/line math (DRY). Empty/background pixels are a fixed clear color (e.g. `Rgb(0,0,0)`), keeping the buffer fully defined (every pixel carries an `Rgb`, mirroring the Tier-0 glyph floor).
  - `render_png`: build a `png::Encoder` (RGB8, 8-bit), set the FROZEN compression + filter, write `pb.px` flattened to `&[u8]`, return the encoded `Vec<u8>`. Pin ONE encoder-setting choice and never vary it — reproducibility depends on it.
  - **Determinism + goldens.** PRIMARY = decode-and-compare-pixels (`test_png_decodes_back_to_exact_pixelbuffer`): the decoded RGB `PixelBuffer` is byte-exact and **platform-independent for the pure-arithmetic geometry path** (cylinder/terrain are basic IEEE ops → reproduce cross-platform, CLAUDE.md §4). SECONDARY = the encoded `.png` bytes frozen as a **platform-pinned** fixture (byte-exact only on the reference platform under pinned `png` 0.18, since DEFLATE bytes are not guaranteed identical across zlib impls). **Tolerance caveat (Finding 3, mirrors Task 11):** an instrument whose geometry routes through a libm transcendental — the **Sankey** `occY` uses `ln` (deck :362) — keeps its existing tolerance note: its pixel golden inherits the platform-pinned policy (not a universal byte-exact guarantee), whereas the cylinder golden (basic arithmetic) is byte-exact cross-platform. Generate the fixture once, eyeball the decoded image for sanity (silhouette reads), then freeze.

- [ ] **Step 4: Run** — same command. Expected: PASS.
- [ ] **Step 5: Commit** — `feat(hypergraph-rs): Rust-side PNG pixel-plate emission (ADR097 Tier-1, shared geometry)`.

---

### Task 15: Clippy, fmt, feature-matrix build, full suite

**Files:** any files with lints.

- [ ] **Step 1: Clippy (raster + cells3d + raster-png):** `cargo clippy -p hypergraph-rs --features cells3d -- -D warnings` AND `cargo clippy -p hypergraph-rs --features raster-png -- -D warnings`. Fix all. Expected lints & preferred fixes:
  - `clippy::too_many_arguments` on projection/raster/png fns — prefer grouping into `Camera`/`Projected` structs over `#[allow]`.
  - `clippy::excessive_precision` on the embedded golden constants — `#[allow]` locally with a "deck golden, exact" comment (they are byte-parity fixtures).
  - `clippy::float_cmp` in determinism tests — false positive for the bit-equality contract; `#[allow]` in the test module with a comment.
- [ ] **Step 2: Feature matrix** — confirm each compiles:
  ```bash
  cd hypergraph-rs
  cargo build -p hypergraph-rs --no-default-features --features raster
  cargo build -p hypergraph-rs --no-default-features --features cells3d
  cargo build -p hypergraph-rs --no-default-features --features raster-png
  cargo build -p hypergraph-rs                       # default (raster on, cells3d + raster-png off)
  cargo build -p hypergraph-rs --no-default-features
  ```
  `--features cells3d` must pull `raster` + `communities` transitively; `--features raster-png` must pull `cells3d` (→ `raster` + `communities` + `h3o`) + `png` transitively; default must NOT enable `cells3d` OR `raster-png` (the S9 gate), and `--no-default-features` must exclude both.
- [ ] **Step 3: Full gate** — `mise run rust:check` (full workspace — fmt --check + clippy --workspace --all-targets --locked -D warnings + test --workspace --locked + doc -D warnings). This is the canonical composite gate (Phase-5 Task-12 style); prefer it over spelling out the cargo commands.
- [ ] **Step 4: Commit** — `chore(hypergraph-rs): clippy + fmt + feature-matrix cleanup (raster/cells3d/raster-png)`.

---

## Phase R Completion Summary

After all 15 tasks the `hypergraph-rs` core crate has:

- **`raster/` module** (feature `raster` ⇒ `viz`, default-on): `Rgb`, `Cell`, `CellGrid` (row-major, `to_text`/`to_ansi`), `SceneGraph3D` (+ `Node3`/`Vertex3`/`Face`/`Strut`), `Camera` + `project` (rotY∘rotX + perspective divide) + `depth_sort` (stable painter's), the braille 2×4 cell rasterizer (subcell depth buffer, integer Bresenham, top-left-rule scanline fill), color-under-constraint (`heat` crimson/gold canon, 256-palette quantization, per-cell fg/bg pick), the transformation algebra (`transform`/`Economy`), the `mulberry32` deck RNG + `build_world` fixture + `o(C)` org degree + `do_tick`.
- **`raster/instruments/` + `raster/h3.rs`** (feature `cells3d` ⇒ `raster` + `communities` + `dep:h3o`, owner-gated OUT of default per S9): three builders — adjoint cylinder (o(C) heights, `CylinderHeight` enum default `OrgDegree`), contradiction terrain (IDW heightfield), OCC hyper-Sankey over real `h3o::CellIndex` (u64 parent-aggregation zoom functor, T ribbons).
- **`raster/png.rs`** (feature `raster-png` ⇒ `cells3d` + `dep:png`, owner-gated OUT of default per S9): the ADR097 Tier-1 pixel plate — `render_pixels` (RGB8 `PixelBuffer` at arbitrary `width_px × height_px`, sharing Task 2/3 project/depth_sort/near-plane-clip/edge-function-fill — no duplicated math) and `render_png` (pinned-encoder PNG bytes for the kitty-TGP consumer).
- **New tests**, all deterministic/golden: Stage 0 deck-number reproduction (falling-r, S invariant, net-T at both zooms, Π₀=13, org=[0.2,0.2,0.2,1], pw/psa), Stage 1 SceneGraph3D JSON goldens (assert-the-source), Stage 2 CellGrid text frame goldens, Stage 3 property contracts (depth stability, zoom invariance, glyph-floor parity, mutation), and Task 14 PNG-plate goldens (PRIMARY = decoded-pixel byte-exact / cross-platform; SECONDARY = platform-pinned `.png` bytes; determinism + panic-free random-camera property). Clippy + fmt clean; five-way feature matrix verified (`raster`, `cells3d`, `raster-png`, default, `--no-default-features`).

**Conformance note:** spectral disc positions are structural-parity, not deck-byte-exact (Deviation 2); Rust's own spectral output is self-golden at Stage 1. `Φ == rent_imperial + rent_internal` is a seed-1917 fixture property, not a structural law (Deviation 6, mutation-tested). The coarse-zoom partition is h3o-derived; only the partition-independent invariants (`net-T ≡ 0`, `r` = ratio of sums) are deck-golden (Deviation 7).

**Next: Phase R.2** — worldtube stratigraphy (time-as-z, the deferred 4th instrument, D-R2) once babylon commits a per-tick projection. (The Tier-1 PNG-plate question, D-R3, is now RESOLVED and lands in this phase as Task 14 — no longer deferred.)

---

## OPEN OWNER DECISIONS

Each is resolved to an executable default so this plan runs under defaults; the ones needing a genuine owner call are flagged.

- **D-R1 — cylinder height metric (`o(C)` vs hull-fill/simpliciality).** **DISSOLVED — no owner call needed.** Task 7 exposes both via `enum CylinderHeight { OrgDegree, HullFill }`, default `OrgDegree` (the deck's choice). Both ship; the owner may flip the default later at zero cost.
- **D-R2 — worldtube stratigraphy (4th instrument, time-as-z).** **RECOMMEND: DEFER to Phase R.2.** The deck v3 ships only THREE instruments; a worldtube needs a *committed per-tick projection* (which babylon has not fixed). Building it now would invent a time-parameterization the engine does not yet emit — a new-formalism risk the closed-surface constraint forbids. Deferred; not in the 15 tasks.
- **D-R3 — Rust-side PNG plate emission vs leaving Tier-1 pixel plates to babylon textual-image.** **RESOLVED (owner, 2026-07-22): INCLUDE Rust PNG — implemented in Task 14.** The owner ruled that Tier-1 pixel emission collapses into the crate now rather than deferring to babylon's textual-image path: the glyph floor already satisfies ADR097 Tier-0 natively, and Task 14 adds `render_pixels`/`render_png` (feature `raster-png`, owner-gated OUT of default) that regenerate the Tier-1 pixel plate from the SAME `SceneGraph3D` by reusing the Task 2/3 geometry (information parity by construction, no duplicated rasterization math). The prior recommendation (glyphs now, PNG deferred) is superseded by this ruling. *(Historical note, per doc immutability: the earlier reasoning weighed "no PNG consumer yet"; the owner accepted that scope to unblock the ADR097 kitty-TGP consumer.)*
- **D-R4 — effort envelope.** Phase-5-sized core (~2.3k-line plan, 15 tasks) + golden fixtures + 3 instrument builders + PNG plate + h3o + png + mulberry32. **Estimate ≈ 3.2 Mtok, ≈ 3 execution windows.** The instrument builders (Tasks 7–9) and the fixture-generation tasks (11–12, and Task 14's PNG plate — all require visual inspection before freezing) are the heaviest; Tasks 1–6 are mechanical Sonnet work, Tasks 10/13 are assertion-dense but shallow.
- **D-R5 — coarse-zoom H3 partition: lane-uniform coarsening vs the deck's asymmetric 6-cell behavior.** **RECOMMEND: do NOT require lane-uniform coarsening.** Real `h3o::parent()` will not reproduce the deck's synthetic `(hq>>1,hr>>1)` 6-cell split, and it need not: the load-bearing invariant (`net-T ≡ 0`, `r` = ratio of sums) is *partition-independent* — it survives any grouping, including real H3. Assert the invariants as deck-golden; treat the per-cell `T` decomposition as an h3o-derived (re-derived) golden. Flag for owner awareness only; the plan runs under this default (Deviation 7).
- **D-R6 — community hull: camera-dependent (deck) vs camera-independent scene data.** **RECOMMEND: camera-independent (world-space hull, projected afterward).** The deck computes the hull on camera-projected 2D points (`:330`), making the silhouette a function of camera orientation; but PHASE-R-MAP's SceneGraph3D/rasterizer split wants the scene to be camera-independent assert-the-source data with projection only at the rasterizer stage. Default to world-space hulls (visually near-identical, architecturally clean). Owner may override to match the deck exactly if pixel-parity with the reference is later required.
- **D-R7 — `psr` calibrated form (deck `/2.6·0.9`) vs the textbook `P(S|R)=Org/Repression`.** **RECOMMEND: reproduce the deck's calibrated form for the fixture** so `psr=0.2222` is a true deck golden (Task 10), with a doc comment noting it diverges from the parent project's canonical formula (game/CLAUDE.md §7). When babylon supplies real repression state, the textbook form is the production path; the calibrated constants are deck-specific mock tuning that does not travel.

---

## Constraints footer

- **No publishing** (no `cargo publish`, no crate release).
- **cargo single-flight** — one cargo invocation at a time (the executor runs commands serially; never parallelize cargo).
- **Subagents never Fable** — Sonnet for mechanical task execution, Opus for the phase-end review (the workflow already sets `model: 'sonnet'` per task, `model: 'opus'` for Review).
- **Commit trailer:** every commit ends with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Deck reference** lives at `plans/reference/topology-deck.html` — the source of truth for `heat()`, `spectral()`, `drawSankey`, and every constant; transcribe from it, do not re-derive from memory.
