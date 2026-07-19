# hypergraph-rs Phase 8 Implementation Plan — WASM bindings (`hypergraph-rs-wasm`)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the `hypergraph-rs-wasm` stub into the browser-facing surface of
the library (spec §7): a `#[wasm_bindgen]` `Hypergraph` class wrapping the core
`Hypergraph<Value, Value, Value>`, exposing `new()`, `addNode`, `addEdge`,
`memberships`, `members`, `numNodes`/`numEdges`, `render` → `SceneGraph`
(as a JS object), and `fromJson`/`toJson`. Build it to WebAssembly with
`wasm-pack`, run `#[wasm_bindgen_test]` shape/behavior tests (headless browser if
one is available, else Node), type-check the auto-generated TypeScript, and
verify the spec §7.5 size budget (**<500 KB gzipped core**, **<1.5 MB gzipped
all-features**) with real `gzip | wc -c` measurements. **The budget is read as
binary units — 500 KiB (512000 bytes) core / 1.5 MiB (1572864 bytes)
all-features** — so the byte thresholds in Task 7 are deliberate and stated (the
spec §7.5 "KB"/"MB" is interpreted as KiB/MiB; the convention is declared, not
silent — see Task 7).

**This phase runs strictly AFTER Phases 0–7.** It relies on the *real* landed
core API (Phases 0–1 code + Phases 2–6 modules + Phase 5 `viz`/`layout` +
Phase 4 `readwrite::json`), NOT the idealized snippets in the design spec. Where
the spec's §7 code disagrees with the landed core, **the landed core wins** and
the divergence is called out in a `SPEC CORRECTION` note.

**Tech Stack:** Rust 1.91.1 / cargo 1.91.1 (workspace MSRV 1.85, edition 2021),
`wasm32-unknown-unknown` target (already installed), `wasm-bindgen` 0.2 +
`wasm-bindgen-test` 0.3, `serde-wasm-bindgen` 0.6, `serde` + `serde_json`,
`getrandom` 0.4 (`wasm_js` feature — see the risk note below), `wasm-pack`
(installed by Task 1), Node v25.9.0 / npm 11.12.1, `tsc` via `npx`. No `unsafe`,
no unbounded loops. BSD-3-Clause.

**HARD CONSTRAINT — NO PUBLISHING.** This plan contains **no** `wasm-pack
publish`, `npm publish`, `cargo publish`, `gh release`, or any upload step.
Publishing is an owner decision. We **build, pack-dry-run, and smoke-test only**.
`npm pack --dry-run` (which lists tarball contents without uploading) is the
furthest packaging step permitted.

---

## CRITICAL RISK (read before Task 1) — does `rustworkx-core` compile to wasm?

The core `Hypergraph` **is** a `rustworkx_core::petgraph::stable_graph::StableDiGraph`
(landed Phase 0–1 code: `crates/hypergraph-rs/src/core/hypergraph.rs:7,30`).
`rustworkx-core` is therefore an **unconditional** link dependency of every wasm
build — even the minimal `readwrite,viz` slice. Its own dependency block
(verified in `hypergraph-rs/Cargo.lock`) is:

```
rustworkx-core 0.18.0 → { fixedbitset, foldhash, hashbrown, indexmap, ndarray,
                          num-traits, petgraph, priority-queue, rand,
                          rand_distr, rand_pcg, rayon, rayon-cond }
```

Two of those are historically hostile to `wasm32-unknown-unknown`:

1. **`rayon` 1.13 / `rayon-core` 1.12** — needs `std::thread`. Recent rayon
   compiles on `wasm32-unknown-unknown` and degrades to serial execution, but
   this is version-sensitive and must be *proven*, not assumed.
2. **`getrandom` 0.4.3** (pulled via `rand 0.10.2 → rand_core 0.10.1`) — on
   `wasm32-unknown-unknown` it is a **hard compile error by default**; a backend
   must be selected. In getrandom 0.4 the backend is selected by enabling the
   **`wasm_js` cargo feature** (the old `--cfg getrandom_backend="wasm_js"`
   RUSTFLAGS route is the documented *legacy* fallback). Verified via the
   getrandom docs (github.com/rust-random/getrandom `_autodocs/configuration.md`,
   fetched 2026-07-19): *"getrandom = { version = \"0.4\", features =
   [\"wasm_js\"] }"* is current; RUSTFLAGS is legacy.

   > **SPEC CORRECTION.** Spec §7.1 lists `getrandom = { version = "0.2",
   > features = ["js"] }` and `js-sys = "0.2"`. Both are wrong against the
   > landed lock: getrandom resolves to **0.4.3** (feature is **`wasm_js`**, not
   > `js`), and `js-sys` is a `0.3.x` crate (and is not needed at all for this
   > surface). We use `getrandom 0.4` `wasm_js` and omit `js-sys`.

**Nothing in this binding's public surface calls an RNG** — layouts are seeded
`Pcg64Mcg` (Phase 5: `Pcg64Mcg::seed_from_u64`, no `OsRng`), generators are
seeded `Pcg64Mcg` (Phase 4), and `add_node`/`add_edge`/`render`/`fromJson`/
`toJson` never touch entropy. The `getrandom` `wasm_js` feature is required
**purely so the transitive `rand → getrandom` dependency compiles for the wasm
target** (cargo feature unification propagates the feature to the transitive
crate).

**THIRD failure mode — a *native* one that surfaces before wasm (review P8-D1).**
The `readwrite,viz` core slice enables neither `generators` nor
`dynamics`/`communities`, but the landed core
`crates/hypergraph-rs/src/core/hypergraph.rs` imports `rand`, `rand_core`,
`rand_pcg` at **module scope** (lines 4–6) — unconditionally — to serve
`random_edge_shuffle` (line 653) and `double_edge_swap` (line 592). After Phase 4
makes `rand_pcg`/`rand_core` optional (pulled by `generators`) and Phase 6 makes
`rand` optional (pulled by `dynamics`/`communities`), a `readwrite,viz`-only
slice that omits all of them fails to compile with `E0432: unresolved import
rand_pcg / rand_core / rand` — **unless those imports and the two RNG methods are
feature-gated**. This has nothing to do with wasm; it bites a plain native build
of the slice. Task 2 **Step 1** is a native prerequisite gate that catches this
cheaply, with a concrete contingency (a default-on `random` core feature) so
native behavior is unchanged. Only once the native slice is green does a wasm
failure unambiguously mean a wasm-target problem.

**Task 2 is a pure compile-gate that settles this empirically. If the wasm
build fails on `rayon`/`rayon-core` (missing `std::thread` / atomics), STOP and
escalate** — the remedy (a `[patch]` to a wasm-compatible rayon config,
`wasm-bindgen-rayon`, or persuading `rustworkx-core` to feature-gate rayon) is a
cross-cutting design decision outside a clean Phase 8, not a hack to improvise.

---

## Architecture & landed-API contract (what Tasks 3–5 bind to)

Verified against the landed core + the Phase 4/5 plans (the exact signatures
this phase depends on):

| Core item | Real signature (landed) | Source |
|---|---|---|
| `Hypergraph::new` | `pub fn new() -> Self` | `core/hypergraph.rs:76` |
| `add_node` | `pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool` | `core/hypergraph.rs:109` |
| `add_edge` | `pub fn add_edge(&mut self, members: Vec<String>, idx: Option<String>, attrs: E) -> Result<String, EdgeError>` (`where N: Default, M: Default + Clone`) | `core/hypergraph.rs:129` |
| `memberships` | `pub fn memberships(&self, node_id: &str) -> Option<Vec<String>>` | `core/hypergraph.rs:202` |
| `members` | `pub fn members(&self, edge_id: &str) -> Option<Vec<String>>` | `core/hypergraph.rs:220` |
| `num_nodes` / `num_edges` | `pub fn num_nodes(&self) -> usize` / `num_edges` | `core/hypergraph.rs:96,101` |
| `render` | `pub fn render(&self, strategy: LayoutStrategy) -> SceneGraph` on `impl<M> Hypergraph<Value, Value, M>` | Phase 5 plan Task 8 (`viz/mod.rs`) |
| `LayoutStrategy` | `enum { Spring{iterations,seed}, Bipartite{align}, HyperedgeAware{iterations,halo_padding}, Radial{rings} }`, `#[derive(…, Serialize, Deserialize)]` | Phase 5 plan §layout |
| `SceneGraph` | `struct { nodes, hyperedges, bounding_box, metadata }`, `#[derive(…, Serialize, Deserialize)]` | Phase 5 plan §viz |
| `to_hypergraph_dict` | `pub fn to_hypergraph_dict(h: &Hypergraph) -> serde_json::Value` | Phase 4 plan Task 13 (`readwrite/json.rs`) |
| `from_hypergraph_dict` | `pub fn from_hypergraph_dict(data: &serde_json::Value) -> Result<Hypergraph, ReadWriteError>` | Phase 4 plan Task 13 |

> **SPEC CORRECTION (the big one).** Spec §7.2 writes `toJson` as
> `serde_json::to_string(&self.inner)` and `fromJson` as
> `serde_json::from_str(json)`. **`Hypergraph` does not derive `Serialize`/
> `Deserialize`** (confirmed: no derive on the struct at `core/hypergraph.rs:26`,
> and no `impl Serialize for Hypergraph` anywhere across the Phase 0–6 plans —
> JSON I/O is done through the dict functions instead). Those two lines
> **would not compile.** We route `toJson`/`fromJson` through
> `readwrite::json::to_hypergraph_dict` / `from_hypergraph_dict` (the
> xgi-data dict schema, XGI `convert/hypergraph_dict.py`).

> **SPEC CORRECTION.** Spec §7.2's `addEdge` returns `String` and calls
> `self.inner.add_edge(...)` as if infallible. The landed `add_edge` returns
> `Result<String, EdgeError>`. We surface it as `Result<String, JsValue>`
> (throws a JS exception on `EmptyMembers` / `AlreadyExists`).

### Declared deviations from XGI semantics on the WASM surface (deliberate)

The WASM surface is **not** a conformance target (only the Phase 7 Python
binding is). We therefore do **not** replicate XGI's Python-only shims
(warn-and-noop on duplicate `idx`, silently-create-empty-edge on empty members,
`IDNotFound` on missing lookups — those belong to the Phase 7 binding). The
JS-idiomatic surface instead:

- `addEdge` **throws** on invalid input (`EmptyMembers` / duplicate `idx`),
  because JS callers expect exceptions, not silent no-ops.
- `memberships`/`members` return **`[]`** for an unknown id (via
  `unwrap_or_default`), not an exception — matching the spec §7.2 snippet
  (`.unwrap_or_default()`).
- `render` with an `undefined`/undeserializable strategy falls back to
  `HyperedgeAware { iterations: 100, halo_padding: 0.5 }` (spec §7.2 default).

These choices are declared here so a later reader does not mistake them for
bugs. If the owner later wants the WASM surface to mirror XGI's warn/noop
semantics, that is a separate follow-up.

### Feature slices for the two size builds

The wasm crate depends on the core with **`default-features = false, features =
["readwrite", "viz"]`** (viz implies layout — Phase 5). That is the **core
slice** measured against the 500 KB budget. A wasm-crate `full` feature forwards
the remaining core features for the **all-features** measurement:

```
full = algorithms + generators + stats + dynamics + communities + convert
```

`network` (reqwest) is **never** enabled — it cannot compile to wasm and is off
in the core anyway.

> **Honesty caveat (recorded, not hidden).** The public surface defined in this
> phase (8 methods) only *calls into* `readwrite` + `viz`. Under LTO + `wasm-opt
> -Oz`, code from the `full` features that the surface never references is
> largely dead-code-eliminated, so the all-features `.wasm` may be only modestly
> larger than core. The all-features build still **compiles every heavy module
> in** and is a genuine **ceiling** measurement against the 1.5 MB budget; it
> also future-proofs the budget for when the surface grows to expose algorithms.
> This is called out in Task 7 and in the completion summary.

---

## File map

```
hypergraph-rs/
├── Cargo.toml                                   # MODIFY (Task 7): + [profile.release] lto = true
├── .gitignore                                   # MODIFY (Task 1): ignore pkg/ + ts-smoke/node_modules/
└── crates/hypergraph-rs-wasm/
    ├── Cargo.toml                               # REWRITE (Task 2): deps, [features], dev-deps, wasm-opt metadata
    ├── src/lib.rs                               # REWRITE (Tasks 3–5): the #[wasm_bindgen] Hypergraph surface
    ├── tests/
    │   └── wasm.rs                              # CREATE (Tasks 3–5): #[wasm_bindgen_test] shape tests
    ├── ts-smoke/
    │   ├── tsconfig.json                        # CREATE (Task 6): strict tsc config
    │   └── smoke.ts                             # CREATE (Task 6): exercises the generated .d.ts
    └── PHASE8_HANDOFF.md                         # CREATE (Task 8): completion summary
```

`wasm-pack build --target web` writes generated artifacts to
`crates/hypergraph-rs-wasm/pkg/` (`hypergraph_rs_wasm.js`,
`hypergraph_rs_wasm_bg.wasm`, `hypergraph_rs_wasm.d.ts`, `package.json`). **`pkg/`
is a build product — gitignored in Task 1, never committed.**

---

## Task 1: Install `wasm-pack`, verify the wasm toolchain, and probe the test runtime

**Files:**
- Modify: `hypergraph-rs/.gitignore` (add `pkg/` + `ts-smoke/node_modules/`)

**Goal:** get `wasm-pack` on PATH (prefer the prebuilt binary over a slow
`cargo install`), confirm the wasm target, and decide **once** whether Tasks 3–5
tests run in a headless browser or in Node.

- [ ] **Step 1: Confirm the wasm target is present**

Run:
```bash
rustup target list --installed | grep -x 'wasm32-unknown-unknown'
```
Expected: prints `wasm32-unknown-unknown` (the env fact says it is installed). If
absent: `rustup target add wasm32-unknown-unknown`.

- [ ] **Step 2: Install `wasm-pack` (prebuilt binary, not `cargo install`)**

Prefer the npm-distributed prebuilt binary (fast, no source build):
```bash
npm install -g wasm-pack
wasm-pack --version
```
If the npm route is unavailable, use the official installer script:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
wasm-pack --version
```
Expected: `wasm-pack --version` prints a version (e.g. `wasm-pack 0.13.x`).
**Do NOT** use `cargo install wasm-pack` (compiles from source, slow) unless
both prebuilt routes fail.

- [ ] **Step 3: Probe for a headless-browser test runtime, record the decision**

`wasm-pack test --headless` needs a browser **and** its webdriver on PATH. Probe:
```bash
for b in geckodriver chromedriver; do command -v "$b" >/dev/null 2>&1 && echo "found driver: $b"; done
for b in firefox google-chrome google-chrome-stable chromium chromium-browser; do
  command -v "$b" >/dev/null 2>&1 && echo "found browser: $b"
done
# Decision rule:
#   - a webdriver (geckodriver OR chromedriver) AND a matching browser present
#     => BROWSER MODE => `wasm-pack test --headless --firefox` (or --chrome)
#   - otherwise        => NODE MODE => `wasm-pack test --node`
```
**Record the chosen mode in the phase handoff** (Task 8). Node v25.9.0 satisfies
the getrandom `wasm_js` backend requirement (Node v19+), so **Node mode is a
first-class fallback**, not a degraded one. Every later "run the wasm tests" step
below writes both commands; use the recorded one.

- [ ] **Step 4: Ignore build products**

Append to `hypergraph-rs/.gitignore` (create the file if absent):
```gitignore
# wasm-pack build output (Phase 8)
crates/hypergraph-rs-wasm/pkg/
crates/hypergraph-rs-wasm/ts-smoke/node_modules/
```

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs-wasm): ignore wasm-pack build products; toolchain probe

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
(A global `wasm-pack` install leaves nothing to commit for the install itself —
that is expected; the commit carries only the `.gitignore` change.)

---

## Task 2: Configure the wasm crate + prove `rustworkx-core` compiles to wasm (the gate)

**Files:**
- Rewrite: `hypergraph-rs/crates/hypergraph-rs-wasm/Cargo.toml`
- **Contingency only** (Step 1 fails — see below): `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`
  (add a default-on `random` feature) and `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`
  (`#[cfg(feature = "random")]`-gate the RNG-dependent methods + their module-scope `use rand*` imports)

**Goal:** wire the real dependencies (core `readwrite,viz` slice + `wasm-bindgen`
+ `serde-wasm-bindgen` + `getrandom/wasm_js`), declare the `full` feature and the
`wasm-opt -Oz` size pass, and **compile the crate to `wasm32-unknown-unknown`**.
This is the single most important gate in the phase (see CRITICAL RISK above) —
it settles the rayon/getrandom wasm question before any binding code is written.
**A native prerequisite gate (Step 1) runs first**, because the `readwrite,viz`
core slice can fail to compile *natively* — before wasm even enters the picture —
if the core's module-scope `use rand*` imports are not feature-gated (see the
CRITICAL RISK "third failure mode" note below).

- [ ] **Step 1: NATIVE prerequisite gate — the `readwrite,viz` core slice must compile natively FIRST**

Before touching wasm at all, prove the exact feature slice the wasm crate will
depend on compiles as a *native* build:
```bash
cd hypergraph-rs
cargo build -p hypergraph-rs --no-default-features --features readwrite,viz
```
Expected: **succeeds.** This is the cheapest possible check of the slice and it
isolates a failure mode that has nothing to do with wasm: the landed core
`crates/hypergraph-rs/src/core/hypergraph.rs` imports `rand`, `rand_core`,
`rand_pcg` at **module scope** (lines 4–6: `use rand::RngExt;`,
`use rand_core::SeedableRng;`, `use rand_pcg::Pcg64Mcg;`) — unconditionally, to
serve `random_edge_shuffle` (line 653) and `double_edge_swap` (line 592). Phase 4
makes `rand_pcg`/`rand_core` `optional = true` (pulled only by `generators`) and
Phase 6 re-adds `rand` as optional behind `dynamics`/`communities`. A
`readwrite,viz`-only slice enables **none** of those features, so unless the
RNG-dependent methods and their module-scope `use rand*` lines are feature-gated,
this native build fails with `E0432: unresolved import rand_pcg / rand_core /
rand`.

**If this native build FAILS on unresolved `rand` / `rand_pcg` / `rand_core`
(contingency — additive, declared deviation):** feature-gate the RNG surface so
the slice compiles without it, leaving native default behavior identical.
1. In `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`, add a **default-on**
   feature that owns the RNG crates:
   ```toml
   [features]
   default = ["random"]      # keep existing default members; ADD "random"
   random = ["dep:rand", "dep:rand_core", "dep:rand_pcg"]
   ```
   (Match the exact `dep:`/optional spelling Phases 4/6 landed; if `rand*` are
   already optional and pulled by other features, `random` just adds a
   default-on path that also enables them.)
2. In `crates/hypergraph-rs/src/core/hypergraph.rs`, gate the three module-scope
   imports and both RNG methods:
   ```rust
   #[cfg(feature = "random")]
   use rand::RngExt;
   #[cfg(feature = "random")]
   use rand_core::SeedableRng;
   #[cfg(feature = "random")]
   use rand_pcg::Pcg64Mcg;
   ```
   and put `#[cfg(feature = "random")]` on `pub fn double_edge_swap(...)` and
   `pub fn random_edge_shuffle(...)` (and any `use super::error::...SwapError`
   only they need). Do **not** touch their bodies.
3. Re-run the native gate. Because `random` is a **default** feature, every
   existing native consumer (Python extension, CLI, tests) still gets these
   methods unchanged; only the wasm slice's `default-features = false, features =
   ["readwrite", "viz"]` (which omits `random`) drops them. This is a genuine
   deviation from the landed core, so record it in the completion summary
   (Task 8) declared-deviations list.

**Only after this native gate is green** does any wasm build make sense — a wasm
failure is then unambiguously a wasm-target issue (getrandom/rayon), not a
feature-slice issue.

- [ ] **Step 2: Note the current (stub) wasm build baseline**

Run:
```bash
cd hypergraph-rs
cargo build -p hypergraph-rs-wasm --target wasm32-unknown-unknown --release
```
Expected: **succeeds trivially** today — the stub only depends on `wasm-bindgen`
and pulls no rayon/getrandom. This baseline confirms the wasm *target* works; the
real risk test comes after Step 3 adds the core dependency. Note the result.

- [ ] **Step 3: Rewrite the wasm crate manifest**

Write `hypergraph-rs/crates/hypergraph-rs-wasm/Cargo.toml`:
```toml
[package]
name = "hypergraph-rs-wasm"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "WASM bindings for hypergraph-rs"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# Core slice measured against the 500 KB budget: readwrite (fromJson/toJson) +
# viz (render; viz implies layout). default-features = false keeps algorithms,
# generators, stats, dynamics, communities, convert OUT of the core build.
hypergraph-rs = { path = "../hypergraph-rs", default-features = false, features = ["readwrite", "viz"] }
wasm-bindgen = "0.2"
serde = { workspace = true }
serde_json = { workspace = true }
serde-wasm-bindgen = "0.6"

# getrandom's wasm backend. Required PURELY so the transitive
# rand 0.10.2 -> rand_core 0.10.1 -> getrandom 0.4.3 dependency compiles for
# wasm32-unknown-unknown (cargo feature unification propagates `wasm_js` to the
# transitive crate). Target-gated so native (python/cli) builds never see it.
# NB: getrandom is 0.4 with feature `wasm_js` (NOT 0.2 / `js` as spec §7.1 says).
[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dependencies]
getrandom = { version = "0.4", features = ["wasm_js"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[features]
default = []
# All-features CEILING build for the 1.5 MB budget (Task 7). Forwards every
# heavy core module EXCEPT `network` (reqwest cannot target wasm). The public
# surface does not call these, but they compile in and set the size ceiling.
full = [
    "hypergraph-rs/algorithms",
    "hypergraph-rs/generators",
    "hypergraph-rs/stats",
    "hypergraph-rs/dynamics",
    "hypergraph-rs/communities",
    "hypergraph-rs/convert",
]

# wasm-pack honors this metadata for the NON-root wasm crate (unlike a cargo
# [profile.release], which cargo ignores outside the workspace root). -Oz is
# binaryen's aggressive size pass — the primary lever for the size budget.
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz"]
```

> **SPEC CORRECTION (build profile placement).** Spec §7.1 puts
> `[profile.release] opt-level = "s", lto = true` **in the wasm crate**. Cargo
> **ignores `[profile.*]` in a non-root workspace member** (it emits
> `warning: profiles for the non-root package … will be ignored`). We do **not**
> force `opt-level = "s"` workspace-wide, because that would regress the
> perf-sensitive Python extension (the whole point of the port). Instead we drive
> wasm size with `wasm-opt = ["-Oz"]` (above, wasm-crate-scoped, honored) plus a
> workspace-root `lto = true` in Task 7 (LTO helps *both* size and native perf).

- [ ] **Step 4: Compile the real crate to wasm — THE GATE**

Run:
```bash
cd hypergraph-rs
cargo build -p hypergraph-rs-wasm --target wasm32-unknown-unknown --release
```
Expected: **succeeds.** This proves that `rustworkx-core 0.18` (and its
`rayon`/`rayon-core`/`ndarray`/`rand_distr` deps) and `getrandom 0.4.3`
**do compile to `wasm32-unknown-unknown`** in the `readwrite,viz` slice.

**If it FAILS:**
- On an **`unresolved import rand` / `rand_pcg` / `rand_core` (E0432)** error →
  this is the *native* feature-slice failure mode, surfacing here only if Step 1
  was skipped. Go back to **Step 1** and apply its contingency (feature-gate the
  RNG-dependent core methods + their module-scope `use rand*` imports behind a
  default-on `random` feature), then re-run Step 1's native gate before returning
  here. This is a core-crate/feature issue, **not** a wasm-target issue.
- On a `getrandom` "target not supported / no backend" error → confirm the
  `[target.'cfg(...)'.dependencies] getrandom … features = ["wasm_js"]` block is
  present and spelled exactly as above. Legacy fallback (only if the feature
  alone is insufficient for the resolved getrandom patch): create
  `crates/hypergraph-rs-wasm/.cargo/config.toml` with
  ```toml
  [target.wasm32-unknown-unknown]
  rustflags = ['--cfg', 'getrandom_backend="wasm_js"']
  ```
  (target-scoped; native builds unaffected). Re-run.
- On a `rayon` / `rayon-core` error (`std::thread` / atomics not available) →
  **STOP and escalate.** Do not hack a `[patch]` in ad hoc; record the exact
  error in the handoff and raise it as a design decision (candidate remedies:
  pin/patch a wasm-capable rayon, adopt `wasm-bindgen-rayon`, or ask
  `rustworkx-core` upstream to feature-gate rayon). This blocks the phase.

- [ ] **Step 5: Also confirm the all-features slice compiles to wasm**

Run:
```bash
cd hypergraph-rs
cargo build -p hypergraph-rs-wasm --target wasm32-unknown-unknown --release --features full
```
Expected: **succeeds** (heavier, but every generator/dynamics/communities module
is seeded-`Pcg64Mcg`-only per Phases 4–6, so still no entropy backend needed
beyond `wasm_js`). Same escalation rule as Step 4 on failure. Note that the
`full` slice **does** enable `generators` (hence the core `random`/RNG path), so
if Step 1's contingency was applied, the RNG imports are live again here — a
correct cross-check that the feature-gate is additive, not lossy.

- [ ] **Step 6: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "build(hypergraph-rs-wasm): wire core deps + getrandom wasm_js + wasm-opt; wasm compile gate green

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: The `#[wasm_bindgen] Hypergraph` structure surface (constructor, nodes, edges, memberships, members)

**Files:**
- Rewrite: `hypergraph-rs/crates/hypergraph-rs-wasm/src/lib.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`

**Interfaces produced:** JS class `Hypergraph` with `new()`, `addNode(id, attrs)
-> bool`, `addEdge(members, idx?, attrs) -> string` (throws on invalid),
`memberships(nodeId) -> string[]`, `members(edgeId) -> string[]`,
`numNodes() -> number`, `numEdges() -> number`.

- [ ] **Step 1: Write the failing test**

Write `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`:
```rust
//! Shape/behavior tests for the WASM binding. A bare `#[wasm_bindgen_test]`
//! (no `run_in_browser` configure macro) runs under BOTH `wasm-pack test --node`
//! and `wasm-pack test --headless --firefox`, so the Task 1 runtime decision
//! selects the mode without touching this file.

use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

use hypergraph_rs_wasm::Hypergraph;

#[wasm_bindgen_test]
fn new_hypergraph_is_empty() {
    let h = Hypergraph::new();
    assert_eq!(h.num_nodes(), 0);
    assert_eq!(h.num_edges(), 0);
}

#[wasm_bindgen_test]
fn add_node_returns_true_then_false() {
    let mut h = Hypergraph::new();
    assert!(h.add_node("a", JsValue::NULL));
    assert!(!h.add_node("a", JsValue::NULL)); // already exists
    assert_eq!(h.num_nodes(), 1);
}

#[wasm_bindgen_test]
fn add_edge_creates_members_and_returns_id() {
    let mut h = Hypergraph::new();
    let id = h
        .add_edge(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Some("e1".to_string()),
            JsValue::NULL,
        )
        .expect("add_edge should succeed");
    assert_eq!(id, "e1");
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.num_nodes(), 3); // members auto-created

    // members() is deterministic petgraph-adjacency order — compare as a set.
    let mut m = h.members("e1");
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    let ms = h.memberships("a");
    assert_eq!(ms, vec!["e1".to_string()]);
}

#[wasm_bindgen_test]
fn add_edge_empty_members_throws() {
    let mut h = Hypergraph::new();
    let res = h.add_edge(vec![], None, JsValue::NULL);
    assert!(res.is_err(), "empty members must throw (EmptyMembers)");
}

#[wasm_bindgen_test]
fn add_edge_duplicate_idx_throws() {
    let mut h = Hypergraph::new();
    h.add_edge(vec!["a".to_string()], Some("e1".to_string()), JsValue::NULL)
        .unwrap();
    let res = h.add_edge(vec!["b".to_string()], Some("e1".to_string()), JsValue::NULL);
    assert!(res.is_err(), "duplicate idx must throw (AlreadyExists)");
}

#[wasm_bindgen_test]
fn missing_id_lookups_return_empty() {
    let h = Hypergraph::new();
    assert!(h.memberships("nope").is_empty());
    assert!(h.members("nope").is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Browser mode (if Task 1 chose it):
```bash
cd hypergraph-rs
wasm-pack test --headless --firefox crates/hypergraph-rs-wasm
```
Node mode (otherwise):
```bash
cd hypergraph-rs
wasm-pack test --node crates/hypergraph-rs-wasm
```
Expected: **FAIL to compile** — `hypergraph_rs_wasm::Hypergraph` has no
`num_nodes`/`add_node`/etc. yet (the stub only has `_init`).

- [ ] **Step 3: Implement the structure surface in `lib.rs`**

Write `hypergraph-rs/crates/hypergraph-rs-wasm/src/lib.rs` (this fully replaces
the stub, including its old `_init` function):
```rust
//! WASM bindings for hypergraph-rs (spec §7).
//!
//! A thin adapter over the core `Hypergraph<Value, Value, Value>`. Attribute
//! values cross the JS boundary as `JsValue` (converted via
//! `serde-wasm-bindgen`); structural ids cross as `String`/`Vec<String>`.
//!
//! This surface is NOT an XGI conformance target (that is the Python binding,
//! Phase 7). Deliberate JS-idiomatic deviations are documented on each method.

use serde_json::Value;
use wasm_bindgen::prelude::*;

use hypergraph_rs::Hypergraph as RustHypergraph;

/// A hypergraph, exposed to JavaScript/TypeScript.
#[wasm_bindgen]
pub struct Hypergraph {
    inner: RustHypergraph<Value, Value, Value>,
}

impl Default for Hypergraph {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Hypergraph {
    /// Create an empty hypergraph.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustHypergraph::new(),
        }
    }

    /// Number of agent nodes.
    #[wasm_bindgen(js_name = numNodes)]
    pub fn num_nodes(&self) -> usize {
        self.inner.num_nodes()
    }

    /// Number of hyperedges.
    #[wasm_bindgen(js_name = numEdges)]
    pub fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    /// Add a node with JS attributes. Returns `true` if a new node was created,
    /// `false` if it already existed. A non-object/`undefined` `attrs`
    /// deserializes to `null`.
    #[wasm_bindgen(js_name = addNode)]
    pub fn add_node(&mut self, id: &str, attrs: JsValue) -> bool {
        let attrs: Value = serde_wasm_bindgen::from_value(attrs).unwrap_or(Value::Null);
        self.inner.add_node(id, attrs)
    }

    /// Add a hyperedge over `members`, with an optional explicit `idx` and JS
    /// `attrs`. Returns the edge id.
    ///
    /// Deviation from XGI (declared): **throws** a JS exception on empty
    /// `members` (`EmptyMembers`) or duplicate `idx` (`AlreadyExists`), rather
    /// than XGI's silent-empty-edge / warn-and-noop (those are Phase-7
    /// Python-conformance shims, not part of this JS surface).
    #[wasm_bindgen(js_name = addEdge)]
    pub fn add_edge(
        &mut self,
        members: Vec<String>,
        idx: Option<String>,
        attrs: JsValue,
    ) -> Result<String, JsValue> {
        let attrs: Value = serde_wasm_bindgen::from_value(attrs).unwrap_or(Value::Null);
        self.inner
            .add_edge(members, idx, attrs)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Edge ids that `node_id` belongs to. Returns `[]` for an unknown node
    /// (declared JS-idiomatic deviation; XGI raises `IDNotFound`).
    #[wasm_bindgen(js_name = memberships)]
    pub fn memberships(&self, node_id: &str) -> Vec<String> {
        self.inner.memberships(node_id).unwrap_or_default()
    }

    /// Member node ids of `edge_id`. Returns `[]` for an unknown edge (declared
    /// deviation, as above).
    #[wasm_bindgen(js_name = members)]
    pub fn members(&self, edge_id: &str) -> Vec<String> {
        self.inner.members(edge_id).unwrap_or_default()
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Browser mode:
```bash
cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm
```
Node mode:
```bash
cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm
```
Expected: **PASS** — 6 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-wasm): Hypergraph structure surface (nodes, edges, memberships, members)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: `toJson` / `fromJson` (via `readwrite::json`, the xgi-data dict schema)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs-wasm/src/lib.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`

**Interfaces produced:** `toJson() -> string`, static `fromJson(json: string)
-> Hypergraph` (throws on parse/schema error).

- [ ] **Step 1: Add the failing tests**

Append to `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`:
```rust
#[wasm_bindgen_test]
fn to_json_then_from_json_round_trips() {
    let mut h = Hypergraph::new();
    h.add_edge(
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        Some("e0".to_string()),
        JsValue::NULL,
    )
    .unwrap();
    h.add_edge(
        vec!["2".to_string(), "3".to_string()],
        Some("e1".to_string()),
        JsValue::NULL,
    )
    .unwrap();

    let json = h.to_json();
    assert!(json.contains("\"type\""), "dict schema has a `type` key");
    assert!(json.contains("edge-dict"), "dict schema has `edge-dict`");

    let h2 = Hypergraph::from_json(&json).expect("fromJson should parse");
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    let mut m = h2.members("e0");
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string(), "3".to_string()]);
}

#[wasm_bindgen_test]
fn from_json_invalid_throws() {
    let res = Hypergraph::from_json("{ this is not json ");
    assert!(res.is_err(), "malformed JSON must throw");
}
```

- [ ] **Step 2: Run to verify it fails**

Browser: `cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm`
Node: `cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm`
Expected: **FAIL to compile** — `to_json`/`from_json` do not exist yet.

- [ ] **Step 3: Implement `toJson`/`fromJson`**

Add this import at the top of `lib.rs` (merge with the existing `use` lines):
```rust
use hypergraph_rs::readwrite::json::{from_hypergraph_dict, to_hypergraph_dict};
```

Add to the `#[wasm_bindgen] impl Hypergraph { … }` block (after `members`):
```rust
    /// Serialize to the xgi-data dict JSON schema (XGI `to_hypergraph_dict`:
    /// `{type, hypergraph-data, node-data, edge-data, edge-dict}`).
    ///
    /// NOTE: the core `Hypergraph` does NOT derive `Serialize`, so this routes
    /// through `readwrite::json::to_hypergraph_dict` rather than serializing the
    /// struct directly (spec §7.2's `serde_json::to_string(&self.inner)` would
    /// not compile).
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        let dict = to_hypergraph_dict(&self.inner);
        serde_json::to_string(&dict).unwrap_or_else(|_| "{}".to_string())
    }

    /// Parse a hypergraph from the xgi-data dict JSON schema. Throws a JS
    /// exception on a JSON parse error or a schema/readwrite error.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<Hypergraph, JsValue> {
        let value: Value = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))?;
        let inner = from_hypergraph_dict(&value)
            .map_err(|e| JsValue::from_str(&format!("hypergraph decode error: {e}")))?;
        Ok(Self { inner })
    }
```

- [ ] **Step 4: Run to verify it passes**

Browser: `cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm`
Node: `cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm`
Expected: **PASS** — 8 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-wasm): toJson/fromJson via readwrite::json dict schema

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: `render` → `SceneGraph` as a JS object

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs-wasm/src/lib.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`

**Interfaces produced:** `render(strategy?: JsValue) -> JsValue` — returns the
`SceneGraph` as a plain JS object (via `serde-wasm-bindgen`). A missing/undefined
or undeserializable `strategy` falls back to `HyperedgeAware { iterations: 100,
halo_padding: 0.5 }`.

**Strategy JSON shape (externally-tagged serde enum, for TS/JS callers):**
`{ "Spring": { "iterations": 50, "seed": 42 } }`,
`{ "Bipartite": { "align": "Vertical" } }`,
`{ "HyperedgeAware": { "iterations": 100, "halo_padding": 0.5 } }`,
`{ "Radial": { "rings": 3 } }`. (`seed` is a plain JS number — values < 2^53
deserialize to `u64` cleanly.)

> **SPEC NOTE.** Spec §7.3's React sketch writes `LayoutStrategy.HyperedgeAware`
> as if the wasm module exports a `LayoutStrategy` enum object. `wasm-bindgen`
> does **not** export the Rust enum as a JS value, so callers pass the
> serde-shaped object above (or `undefined` for the default). This is the honest
> wasm surface; the ergonomic named-const wrapper, if wanted, belongs to the
> optional `@hypergraph-rs/react` TS package (Phase 10), not here.

- [ ] **Step 1: Add the failing tests**

Append to `hypergraph-rs/crates/hypergraph-rs-wasm/tests/wasm.rs`:
```rust
use hypergraph_rs::viz::SceneGraph;

fn sample() -> Hypergraph {
    let mut h = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        JsValue::NULL,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e2".to_string()),
        JsValue::NULL,
    )
    .unwrap();
    h
}

#[wasm_bindgen_test]
fn render_default_strategy_returns_scene_object() {
    let h = sample();
    let js = h.render(JsValue::UNDEFINED); // undefined => HyperedgeAware default
    assert!(!js.is_null() && !js.is_undefined());
    let scene: SceneGraph = serde_wasm_bindgen::from_value(js).expect("scene deserializes");
    assert_eq!(scene.nodes.len(), 3);
    assert_eq!(scene.hyperedges.len(), 2);
    let (min_x, min_y, max_x, max_y) = scene.bounding_box;
    assert!(max_x >= min_x && max_y >= min_y);
}

#[wasm_bindgen_test]
fn render_explicit_spring_strategy() {
    let h = sample();
    let strat = serde_wasm_bindgen::to_value(
        &serde_json::json!({ "Spring": { "iterations": 50, "seed": 1 } }),
    )
    .unwrap();
    let js = h.render(strat);
    let scene: SceneGraph = serde_wasm_bindgen::from_value(js).unwrap();
    assert_eq!(scene.nodes.len(), 3);
    // node glyphs preserve insertion order (a, b, c)
    let ids: Vec<String> = scene.nodes.iter().map(|n| n.id.clone()).collect();
    assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}
```

- [ ] **Step 2: Run to verify it fails**

Browser: `cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm`
Node: `cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm`
Expected: **FAIL to compile** — `render` does not exist yet.

- [ ] **Step 3: Implement `render`**

Add this import to the top of `lib.rs` (merge with existing `use` lines):
```rust
use hypergraph_rs::layout::LayoutStrategy;
```

Add to the `#[wasm_bindgen] impl Hypergraph { … }` block (after `from_json`):
```rust
    /// Render the hypergraph to a `SceneGraph`, returned as a plain JS object
    /// (the rendering contract, spec §5.3 / §7.2).
    ///
    /// `strategy` is the serde-shaped `LayoutStrategy` object (see the module
    /// docs); `undefined`/`null` or an undeserializable value falls back to
    /// `HyperedgeAware { iterations: 100, halo_padding: 0.5 }`.
    #[wasm_bindgen(js_name = render)]
    pub fn render(&self, strategy: JsValue) -> JsValue {
        let default = LayoutStrategy::HyperedgeAware {
            iterations: 100,
            halo_padding: 0.5,
        };
        let strategy: LayoutStrategy = if strategy.is_undefined() || strategy.is_null() {
            default
        } else {
            serde_wasm_bindgen::from_value(strategy).unwrap_or(default)
        };
        let scene = self.inner.render(strategy);
        serde_wasm_bindgen::to_value(&scene).unwrap_or(JsValue::NULL)
    }
```

- [ ] **Step 4: Run to verify it passes**

Browser: `cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm`
Node: `cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm`
Expected: **PASS** — 10 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs-wasm): render -> SceneGraph as a JS object

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: Build for the web + type-check the generated TypeScript

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-wasm/ts-smoke/tsconfig.json`
- Create: `hypergraph-rs/crates/hypergraph-rs-wasm/ts-smoke/smoke.ts`

**Goal:** produce the `--target web` package (`.js` + `.wasm` + `.d.ts`) and
prove the auto-generated `hypergraph_rs_wasm.d.ts` type-checks under strict
`tsc` against realistic call sites — the TypeScript-types-check deliverable.

- [ ] **Step 1: Build the web package (core slice)**

Run:
```bash
cd hypergraph-rs
wasm-pack build crates/hypergraph-rs-wasm --target web --release
ls -la crates/hypergraph-rs-wasm/pkg/
```
Expected: `pkg/` contains `hypergraph_rs_wasm.js`, `hypergraph_rs_wasm_bg.wasm`,
`hypergraph_rs_wasm.d.ts`, and `package.json`. Inspect the `.d.ts` — it must
declare `class Hypergraph` with `numNodes`, `numEdges`, `addNode`, `addEdge`,
`memberships`, `members`, `toJson`, `static fromJson`, `render`.

- [ ] **Step 2: Write the TypeScript smoke file (the failing check)**

Write `hypergraph-rs/crates/hypergraph-rs-wasm/ts-smoke/smoke.ts`:
```typescript
// Type-level smoke test: imports the wasm-pack generated types and exercises
// every bound method so `tsc --noEmit --strict` fails if the .d.ts surface
// regresses. No runtime execution — this is a compile-only contract.
import init, { Hypergraph } from "../pkg/hypergraph_rs_wasm.js";

export async function smoke(): Promise<void> {
  await init();

  const h = new Hypergraph();

  const created: boolean = h.addNode("a", { color: "red" });
  void created;

  const edgeId: string = h.addEdge(["a", "b", "c"], "e1", { weight: 2 });
  void edgeId;

  // idx is optional (Option<String> => string | undefined)
  const auto: string = h.addEdge(["b", "c"], undefined, null);
  void auto;

  const nNodes: number = h.numNodes();
  const nEdges: number = h.numEdges();
  void nNodes;
  void nEdges;

  const ms: string[] = h.memberships("a");
  const mem: string[] = h.members("e1");
  void ms;
  void mem;

  // render returns the SceneGraph as `any` (JsValue); default strategy.
  const scene = h.render(undefined);
  void scene;

  const spring = h.render({ Spring: { iterations: 50, seed: 1 } });
  void spring;

  const json: string = h.toJson();
  const h2: Hypergraph = Hypergraph.fromJson(json);
  void h2;
}
```

Write `hypergraph-rs/crates/hypergraph-rs-wasm/ts-smoke/tsconfig.json`:
```json
{
  "compilerOptions": {
    "strict": true,
    "noEmit": true,
    "target": "es2020",
    "module": "es2020",
    "moduleResolution": "bundler",
    "lib": ["es2020", "dom"],
    "skipLibCheck": false,
    "types": []
  },
  "include": ["smoke.ts"]
}
```

- [ ] **Step 3: Run the type check**

Run (uses the project-agnostic `tsc` via `npx`; node v25 / npm 11 are present):
```bash
cd hypergraph-rs/crates/hypergraph-rs-wasm/ts-smoke
npx --yes typescript@5 tsc --project tsconfig.json
```
Expected: **exit 0, no output** (strict type-check passes). If `tsc` reports that
`addEdge`'s second arg is required, or `fromJson` is not `static`, or a method
name is camel-wrong — the `js_name` attributes in `lib.rs` disagree with the
smoke file; reconcile (the generated `.d.ts` is the source of truth for the JS
names, so fix `smoke.ts` to match it, or fix a wrong `js_name` in `lib.rs`).

> **Note on `render`'s type.** `render` returns `JsValue`, which wasm-bindgen
> types as `any` in the `.d.ts`. That is expected for this phase — a precise
> `SceneGraph` TS interface is the job of the optional `@hypergraph-rs/react`
> package (Phase 10), which owns the hand-written `SceneGraph` type.

- [ ] **Step 4: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "test(hypergraph-rs-wasm): --target web build + strict tsc type-check of generated .d.ts

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```
(`pkg/` and `ts-smoke/node_modules/` are gitignored from Task 1, so only the
`ts-smoke/` sources are committed.)

---

## Task 7: Size-budget verification (spec §7.5 — <500 KB core, <1.5 MB all-features)

**Files:**
- Modify: `hypergraph-rs/Cargo.toml` (workspace root — add `[profile.release] lto = true`)

**Goal:** measure the **gzipped** `.wasm` for both feature slices and assert the
spec §7.5 budget with real commands.

> **Unit convention (declared, per orchestrator ruling).** Spec §7.5 writes the
> budget as "500 KB" / "1.5 MB". This plan reads those as **binary** units —
> **500 KiB = 512000 bytes** (core) and **1.5 MiB = 1572864 bytes**
> (all-features) — and the `test` thresholds in Step 3 use exactly those byte
> counts. This is ~2.4% (core) / ~4.9% (full) more headroom than a literal
> decimal reading (500000 / 1500000); the KiB/MiB interpretation is chosen
> deliberately and stated here so the convention is not silently conflated.

- [ ] **Step 1: Add workspace-root LTO (size + native perf, no `opt-level` regression)**

Append to `hypergraph-rs/Cargo.toml` (workspace root — the only place cargo
honors `[profile.*]`):
```toml
[profile.release]
lto = true
```
> **Rationale / SPEC CORRECTION.** Spec §7.1's `opt-level = "s"` is deliberately
> **not** adopted globally: it would slow the perf-sensitive Python extension.
> LTO helps both size and native perf, so it is safe workspace-wide; wasm-only
> size is driven by `wasm-opt = ["-Oz"]` (Task 2). If, and only if, Step 3's core
> measurement exceeds 500 KB, the escalation is a wasm-only size lever (see the
> open note at the end of this task) — not to regress `opt-level` for everyone.

- [ ] **Step 2: Rebuild both slices with the profile in effect**

Run:
```bash
cd hypergraph-rs
# Core slice (readwrite + viz only)
wasm-pack build crates/hypergraph-rs-wasm --target web --release
cp crates/hypergraph-rs-wasm/pkg/hypergraph_rs_wasm_bg.wasm /tmp/hgrs_core.wasm

# All-features CEILING slice
wasm-pack build crates/hypergraph-rs-wasm --target web --release -- --features full
cp crates/hypergraph-rs-wasm/pkg/hypergraph_rs_wasm_bg.wasm /tmp/hgrs_full.wasm
```
(The `--` forwards `--features full` to `cargo build`. wasm-pack's `wasm-opt -Oz`
pass from Task 2 runs on both.)

- [ ] **Step 3: Measure gzipped sizes and assert the budget**

Run:
```bash
CORE=$(gzip -9 -c /tmp/hgrs_core.wasm | wc -c)
FULL=$(gzip -9 -c /tmp/hgrs_full.wasm | wc -c)
echo "core gzipped:  ${CORE} bytes  (budget 512000 = 500 KiB)"
echo "full gzipped:  ${FULL} bytes  (budget 1572864 = 1.5 MiB)"
test "${CORE}" -lt 512000  && echo "CORE OK"  || echo "CORE OVER BUDGET"
test "${FULL}" -lt 1572864 && echo "FULL OK"  || echo "FULL OVER BUDGET"
```
Expected: `CORE OK` and `FULL OK`. Record both byte counts in the handoff.

**If CORE is OVER BUDGET** (unlikely for a serde + petgraph slice after
`wasm-opt -Oz`): remedies in order — (a) verify `wasm-opt -Oz` actually ran
(wasm-pack prints `[INFO]: Optimizing wasm binaries with wasm-opt`), (b) confirm
no `debug = true` leaked into the release profile, (c) as a last resort a
wasm-only `opt-level = "s"` — which requires isolating the wasm crate into its
own cargo workspace (cargo cannot scope `opt-level` per-crate), an **owner
decision to escalate**, not to improvise. Record the over-budget number and stop.

- [ ] **Step 4: Commit**

```bash
cd hypergraph-rs
git add Cargo.toml
git commit -m "build(hypergraph-rs): workspace release LTO; wasm size budget verified (<500KB core, <1.5MB full)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

> **Open note (carry to handoff).** With the 8-method surface, `full`-feature
> code the surface never calls is largely dead-code-eliminated, so
> `/tmp/hgrs_full.wasm` may be close to `/tmp/hgrs_core.wasm`. Both still satisfy
> their budgets; the `full` number is a genuine *ceiling* and future-proofs the
> budget for when the surface grows to expose algorithms. Report both raw
> numbers so the owner sees the real gap, not a claim.

---

## Task 8: Package smoke (dry-run, NO publish) + phase completion summary

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs-wasm/PHASE8_HANDOFF.md`

**Goal:** confirm the generated npm package is well-formed **without publishing
anything**, and write the completion summary.

- [ ] **Step 1: Inspect the generated package manifest (no publish)**

Run:
```bash
cd hypergraph-rs/crates/hypergraph-rs-wasm/pkg
cat package.json
npm pack --dry-run
```
`npm pack --dry-run` **lists** the tarball contents (`.js`, `.wasm`, `.d.ts`,
`package.json`) **without** creating or uploading anything. Confirm the `files`
list includes the `.wasm`, the `.js`, and the `.d.ts`, and that `main`/`module`/
`types` point at real files. **Do NOT run `wasm-pack publish`, `npm publish`, or
`npm pack` without `--dry-run`.**

- [ ] **Step 2: Full green re-run of the wasm test suite**

Browser: `cd hypergraph-rs && wasm-pack test --headless --firefox crates/hypergraph-rs-wasm`
Node: `cd hypergraph-rs && wasm-pack test --node crates/hypergraph-rs-wasm`
Expected: **PASS — 10 tests, 0 failures.**

- [ ] **Step 3: Write the completion summary**

Write `hypergraph-rs/crates/hypergraph-rs-wasm/PHASE8_HANDOFF.md` capturing:
- **Test runtime chosen** (browser+driver name, or Node) and why.
- **Native prerequisite gate** (Task 2 Step 1): whether `cargo build -p
  hypergraph-rs --no-default-features --features readwrite,viz` compiled as-is,
  or whether the `random`-feature contingency (default-on core feature gating
  `random_edge_shuffle`/`double_edge_swap` + their module-scope `use rand*`
  imports) had to be applied — and, if applied, an explicit note that it is
  additive (native default behavior unchanged; only the wasm slice drops the RNG
  methods).
- **Compile-gate result** (Task 2): confirmation that `rustworkx-core` + rayon +
  getrandom compiled to `wasm32-unknown-unknown`, or the escalation record if not.
- **Size numbers**: core gzipped bytes, full gzipped bytes, both vs budget, and
  the dead-code-elimination caveat.
- **Surface delivered**: `new`, `addNode`, `addEdge`, `memberships`, `members`,
  `numNodes`, `numEdges`, `toJson`, `fromJson`, `render`.
- **Declared deviations** (from XGI + from spec §7 + from landed core): recap the
  corrections (getrandom 0.4/`wasm_js`; no `js-sys`; `Hypergraph` not `Serialize`
  → dict-based JSON; `add_edge` fallible → throws; `render` strategy is a serde
  object not an exported enum; LTO not `opt-level="s"`; size budget read as
  KiB/MiB; **and — if Task 2 Step 1's contingency fired — the additive default-on
  core `random` feature that gates the RNG methods**).
- **Not done here (by design)**: `@hypergraph-rs/react` TS package + precise
  `SceneGraph` TS type (Phase 10); publishing (owner decision).

- [ ] **Step 4: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "docs(hypergraph-rs-wasm): Phase 8 completion summary + package dry-run smoke

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Definition of done (Phase 8)

- **Native prerequisite gate green:** `cargo build -p hypergraph-rs
  --no-default-features --features readwrite,viz` compiles (with the default-on
  `random` feature-gate contingency applied iff the module-scope `use rand*`
  imports otherwise break the slice — recorded in the handoff if so).
- `cargo build -p hypergraph-rs-wasm --target wasm32-unknown-unknown --release`
  and `… --features full` both **compile** (the rayon/getrandom gate is green).
- `wasm-pack test` (browser **or** node, per Task 1) — **10 tests pass**.
- `wasm-pack build --target web --release` produces `.js` + `.wasm` + `.d.ts`;
  strict `tsc` type-checks the generated `.d.ts` against `ts-smoke/smoke.ts`.
- Size budget **measured with `gzip -9 | wc -c`**: core `< 512000` bytes,
  full `< 1572864` bytes — both recorded in the handoff.
- No `publish` step anywhere; `npm pack --dry-run` is the furthest packaging
  action taken.
- Surface: `new`, `addNode`, `addEdge`, `memberships`, `members`, `numNodes`,
  `numEdges`, `render`, `toJson`, `fromJson`.
- Every commit carries the `Co-Authored-By: Claude Fable 5` trailer.
