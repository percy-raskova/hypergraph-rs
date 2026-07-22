# PROPOSAL — Re-scope Phases 5/8/9/10 around Babylon's raster lane

**Status: PROPOSAL for owner review — the phase plans in this directory stay canonical
until the owner approves this.** Drafted 2026-07-22 after the owner directive
("contemplate rasterization … maybe factor that in over wasm/react — rasterization of
the topology and hypergraphs") and a read-only scout of the babylon tree. Every claim
below cites the babylon-side source it derives from.

## What changed since the plans were written (2026-07-19)

1. **The React web cockpit is superseded.** Owner ruling R1: "The React web cockpit is
   superseded as the primary client" (`ai/_inbox/tui/20260719archiveinterfacedesign.md:21`).
   The TUI (Textual 8.2, ADR099) is the client. NORTH_STAR's roadmap lists no React
   anywhere, including post-1.0. → **Phase 10 as written targets a dead client.**
2. **A ratified render seam exists** (ADR097): Tier 0 glyph canon (design target — fully
   informative in character cells), Tier 1 pixel plates (kitty TGP only; sixel rejected
   fleet-wide; information parity — pixel never carries unique info), Tier 2 SVG export
   (deterministic, snapshot-testable).
3. **The graph-render lane ruling** (`ai/_inbox/graph-render-lane-ruling.md`):
   - Assert the SOURCE, display the RENDER — golden artifacts are text-form inputs
     (orderings, DOT, layout JSON), never raster bytes.
   - Renderers consume **orderings, never force layouts** (design canon S9).
   - hypergraph-rs is named "the eventual high-performance renderer", plugging in via
     "its planned CLI/PyO3 surfaces" as the **ordering/projection provider**.
4. **A cell-grid rasterizer is the anticipated direction**
   (`ai/_inbox/post-v1.0.0/rasterizer.md:5`): SVG/DOT → half-blocks/quadrants/sextants/
   braille, possibly living *inside hypergraph-rs* — "graph layout and rasterization in
   one deterministic Rust pass"; the output IS text, hence byte-exact golden-testable.
   (Inbox note, not a ratified ADR — directional signal.)
5. **Formalism surface is closed** (NORTH_STAR): babylon consumes orderings/DOT/JSON
   artifacts; new layout mathematics stays inside hypergraph-rs as library capability.

## Proposal, phase by phase

| Phase | As written | Proposed |
|---|---|---|
| 5 layout+viz | Layouts + SceneGraph JSON + SVG emitter | **KEEP, with additions.** SceneGraph JSON is precisely the "assert the source" artifact; SVG is Tier 2. ADD first-class *ordering providers* mirroring babylon's existing projection shapes: PAOH ordering `(nodes_in_order, edges_sorted)`, Levi/bipartite ego-tree walk (our internal representation IS the Levi graph), incidence dot-grid ordering. These are re-expressions of existing structure — no new formalism. |
| 8 WASM | wasm-bindgen surface + size budgets | **DEFER indefinitely.** Its consumer (web client) is superseded. Nothing else in babylon's architecture wants a browser build. Revisit only if a web target returns post-1.0. |
| NEW: raster | — | **Phase R — cell-grid rasterizer** (replaces Phase 8's slot): `SceneGraph → CellGrid` in pure Rust — half-block/quadrant/sextant/braille glyph selection, deterministic, no wall-clock/randomness, output is text (diffable, golden-testable byte-exactly). Serves ADR097 Tier 0 natively; Tier 1 pixel plates regenerate from the same SceneGraph on the babylon side (information parity by construction, the `map_room.py` pattern). |
| 9 CLI | inspect/convert/validate/matrix/run/generate/render/selftest | **KEEP — promoted.** The CLI is a named plug-in surface. Re-scope `render` output formats to: `--format json` (SceneGraph), `--format svg` (Tier 2), `--format cells` (Phase R glyph raster), `--format ordering` (PAOH/Levi/incidence orderings). Drop nothing else. |
| 10 React | `@hypergraph-rs/react` SVG components | **CANCEL as written.** Salvage its one durable idea — the wire contract — into Phase R's `SceneGraph`/`CellGrid`/ordering JSON schemas, versioned and fixture-tested the same way the plan intended for React props. |

## Phase R extension: 3D cell rasterization (owner suggestion, 2026-07-22)

Reference: https://github.com/ludvigsandberg/tui-3d-renderer — Rust, MIT, a CPU
software rasterizer drawing 3D meshes into terminal cells (ANSI + Unicode block
shading). It is an application, not a library crate, so we would mine the technique
(projection, depth-buffered triangle fill, block-character shading) and implement it
natively in Phase R rather than depend on it. Proposed shape:

- **`SceneGraph3D`**: 3D node positions via **spectral embedding** — eigenvectors 2–4
  of the hypergraph Laplacian we already ship (Phase 6's power-iteration eigensolver
  reused). Deterministic with NO seed and no new formalism: existing mathematics,
  re-projected. Hyperedges as translucent hulls; **simplices rendered as actual
  shaded 3D faces** (our `SimplicialComplex` faces are literally triangles/tetrahedra —
  the depth buffer is the honest renderer for them).
- **Rasterizer**: pure function `(SceneGraph3D, camera, cols, rows) → CellGrid` —
  no wall-clock, no randomness, byte-exact frames, golden-testable. Static plates at
  fixed camera angles first; an optional pre-rendered "turntable" frame ring later.
- **Babylon consumption is owner-gated**: the graph-render lane ruling has topology
  surfaces consuming orderings, never layouts (S9) — a 3D spatial embedding needs an
  explicit lane extension ruling. Until then this is a hypergraph-rs library/CLI
  capability (`render --format cells3d`), with the 3D SceneGraph JSON as the
  assert-the-source golden artifact and cell frames as presentation tier.

## What does NOT change

- The current critical path (reconciliation → Phase 4 → 6 → 7 → swap) is untouched;
  everything above is post-Phase-7 work.
- Phase 7's binding surface gains three trivially-cheap exports later (orderings), but
  its plan needs no edit now.
- No publishing, cargo single-flight, subagent model rules — all unchanged.

## Open questions for the owner

1. Approve the table above as the new shape for Phases 5/8/9/10? (Phase plans would be
   amended only when each phase is next up, per the review-then-execute rhythm.)
2. Phase R scope: glyph rasterization only (cells), or also PNG plate emission on the
   Rust side? (Babylon's ruling implies PNG can stay Python-side/textual-image; Rust-side
   PNG would collapse the tiers further per `rasterizer.md` but is more scope.)
3. Should the ordering providers (PAOH/Levi) land as part of Phase 5, or earlier as a
   small standalone task so babylon's TUI topology views can consume them via CLI sooner?
