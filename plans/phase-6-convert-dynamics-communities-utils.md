# hypergraph-rs Phase 6 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port XGI 0.10.2's `convert/`, `dynamics/`, `communities/`, and `utils/`
slices into the `hypergraph-rs` core crate, feature-gated per spec §10.1, on top of
the Phase 0+1 `Hypergraph<N,E,M>` core. Every ported function is verified line-by-line
against the installed XGI source (`.venv/lib/python3.13/site-packages/xgi/…`), cited
`file:line`, and driven by TDD (red → green → commit).

**Depends on:** Phase 0+1 (core `Hypergraph`) — built and merged. This plan assumes the
exact core API from `docs/superpowers/plans/2026-07-18-hypergraph-rs-phase-0-1.md`:
`new`, `add_node`, `has_node`, `add_edge`, `has_edge`, `remove_edge`, `remove_node`,
`add_node_to_edge`, `remove_node_from_edge`, `memberships`, `members`, `node_ids`,
`edge_ids`, `node_attrs`/`_mut`, `edge_attrs`/`_mut`, `set_node_attributes`,
`set_edge_attributes` (concrete `Value` impl), `graph_attr`, `set_graph_attr`,
`num_nodes`, `num_edges`, `add_nodes_from`, `add_edges_from`, `copy`, `clear`,
`clear_edges`, `PartialEq`, `Debug`. The default generic types are
`N = E = M = serde_json::Value`.

---

## ⚠️ Ground-truth corrections (read before starting)

These correct claims in the design spec / task brief that the installed XGI source
**disproves**. The source is authoritative (the preflight proved docstrings and the
spec's module summaries lie).

1. **`dynamics/` is Kuramoto synchronization, NOT epidemic (SIR/SIS) models.**
   The spec (§4.3, §10.3 Phase 6) and common summaries say "epidemic models". The
   *actual* `xgi/dynamics/` directory contains a single file `synchronization.py`
   (`dynamics/__init__.py:1-9`), exporting `simulate_kuramoto`,
   `compute_kuramoto_order_parameter`, `simulate_simplicial_kuramoto`,
   `compute_simplicial_order_parameter`. **There is no epidemics code to port.** This
   plan ports the Kuramoto pieces whose dependencies exist by Phase 6
   (`simulate_kuramoto`, `compute_kuramoto_order_parameter`); the *simplicial* Kuramoto
   pair needs `boundary_matrix` (Phase-3 linalg/Hodge) and `SimplicialComplex`
   (Phase 2) and is deferred (see open questions).

2. **The executed Phase 0+1 workspace pinned `rand = "0.8"`, `rand_pcg = "0.3"`**
   (Phase-0-1 plan lines 222-223) — stale. The 2026-07-18 dependency probe proved
   rustworkx-core 0.18 resolves `rand 0.10.2` / `rand_pcg 0.10.2` / `rand_core 0.10.1`,
   and that rand 0.10's range sampling moved to the `RngExt` trait (`rand::Rng` alone is
   insufficient — every `.random_range(...)` call site needs `use rand::RngExt;`).
   **Phase 2 is the first code to use randomness and owns this correction**: Phase 2's
   earliest randomness-needing task edits `[workspace.dependencies]` to the 0.10.2/0.10.1
   lines and wires `rand.workspace`/`rand_pcg.workspace`/`rand_core.workspace` into the
   core crate as optional deps. **Phase 6 inherits this — it does not re-edit
   `[workspace.dependencies]` or redeclare those three `[dependencies]` lines**; Task 1
   only adds new `[features]` entries that reference the already-optional deps.

3. **The spec's `ndarray = "0.16"` (§10.1) is wrong for the blessed line.** The probe
   confirmed rustworkx-core 0.18 forces `ndarray 0.17.2`; any workspace ndarray must pin
   `"0.17"` to unify. **Phase 3 owns this correction** (it adds `ndarray = "0.17"` and
   `sprs = "0.11"` to `[workspace.dependencies]` and wires `ndarray.workspace` into the
   core crate as an optional dep for its own linalg/stats feature). **Phase 6 inherits
   this — it does not re-add `ndarray` to `[workspace.dependencies]` or redeclare it in
   the core crate's `[dependencies]`**; Task 1 only adds a `communities` feature entry
   referencing the already-optional `dep:ndarray`.

4. **`communities::spectral_clustering` needs the normalized hypergraph Laplacian**
   (`linalg/laplacian_matrix.py:187-254` — a Phase-3 module) and `scipy`'s `eigsh`.
   The probe found no LAPACK-free ndarray eigensolver. Per the probe's recommendation
   (b), this plan **builds the normalized Laplacian inline** (dense, self-contained,
   citing the XGI formula) and **hand-rolls power-iteration + Hotelling deflation** for
   the k smallest eigenvectors, with a documented tolerance. No LAPACK/`ndarray-linalg`
   dependency is introduced.

---

## Deliberate deviations from XGI (Phase 6)

Each is verified against the installed 0.10.2 source. The Phase-7 Python binding shims
where noted; determinism improvements are permanent.

- **Concrete `Value` types throughout.** All of `convert`/`dynamics`/`communities`
  operate on the default `Hypergraph` (`= Hypergraph<serde_json::Value, …>`), matching
  XGI's dynamic-attribute surface. Generic re-parameterization is deferred to Babylon's
  typed wrapper (spec §12).
- **`from_bipartite_graph` reads a `petgraph::stable_graph::StableDiGraph<BipartiteNode, ()>`,
  not a NetworkX graph.** XGI's version (`convert/bipartite_graph.py:9-97`) reads a
  `nx.Graph` with a `bipartite=0/1` node attribute. Our `BipartiteNode` carries the same
  `bipartite: u8` marker; the semantics (0 = node, 1 = hyperedge, bipartiteness
  validated) are faithful. This refines the spec §4.1 signature (`StableDiGraph<N,E>`),
  which was under-specified (no way to tell node from hyperedge without a marker).
- **`convert` iteration order is insertion order** (edges in `edge_ids()` order, members
  in the substrate's insertion order). XGI's underlying `set`s are hash-ordered
  (`PYTHONHASHSEED`-dependent). Conformance compares memberships as **sets**, never
  ordered lists (Phase-0-1 determinism rule).
- **`spectral_clustering` / `_kmeans` deviations (exact parity is impossible — §9.4
  RNG divergence; assertions are structural):**
  - **k-means centroid update is per-dimension mean.** XGI's
    `centroids = [np.mean(vectors[cid]) …]` (`communities/spectral.py:124`) omits
    `axis=0`, collapsing each centroid to a *scalar* (a latent bug). We compute the
    correct per-dimension centroid.
  - **`k == 1` returns cluster label `0`.** XGI returns `1` (`spectral.py:100`),
    contradicting its own "clusters begin at 0" docstring (a latent bug). We return `0`.
  - **Empty-cluster reseeding.** When a cluster loses all points mid-iteration we reseed
    its centroid to the point farthest from any centroid (XGI leaves this NaN/undefined).
    This makes 2-way separation of well-separated data deterministic and robust.
  - **Return type is `IndexMap<String, usize>`** (insertion-ordered by node), not a
    Python dict.
- **`simulate_kuramoto` uses seeded `rand_pcg::Pcg64Mcg`, not `numpy.random`.** When
  `omega`/`theta` are supplied explicitly the simulation is exactly deterministic; when
  defaulted they are drawn from the seeded PCG (normal via Box–Muller for `omega`,
  uniform `[0,2π)` for `theta`). Sequence-level parity with numpy is not claimed
  (spec §9.4 option (a): structural properties, not exact trajectories).
- **`utils` ports only pure, non-Python-machinery helpers.** Ported: `dual_dict`,
  `powerset`, `subfaces`, `get_network_type`, `convert_labels_to_integers`. Excluded and
  why: `IDDict` (already the `IndexMap` bimaps), `update_uid_counter` (already baked into
  core `add_edge`), `request_json_from_url*` (network machinery — `network` feature,
  later phase), `hist`/`crest_r` (pandas/matplotlib), `geometric`/`find_triangles`/
  `binomial_sequence` (generator helpers — Phase 4), `tensor.py`
  (`pairwise_incidence`/`ttsv1`/`ttsv2` — linalg/centrality, Phase 3), `trie.py::Trie`
  (simpliciality-algorithm machinery, Phase 3).
- **Degenerate-input leniency (undeclared in XGI, intentional here):** three ported
  functions handle edge cases more gracefully than the Python source, which crashes on
  them instead of returning a value:
  - `subfaces` with `order == Some(0)` and empty `edges`: XGI's `if order and order >
    max_order` (`utilities.py:353`) is falsy for `order == 0` and skips the guard,
    so an empty `edges` list crashes in `max([])`. Our `subfaces` computes
    `max_order = -1` for empty input and returns `Ok(vec![])` instead.
  - `from_bipartite_edgelist` on an empty edge list: XGI indexes `edges[0]`
    (`bipartite_edges.py:28`) unconditionally and raises `IndexError`. Ours returns an
    empty hypergraph.
  - `compute_kuramoto_order_parameter` on an empty timestep row: `np.mean` of an empty
    slice yields `NaN` (with a numpy warning). Ours returns `0.0` for that timestep.

---

## Global constraints (carried from Phase 0+1)

- **Rust edition 2021**, MSRV 1.85, license BSD-3-Clause.
- **Determinism (III.7 parity):** insertion-ordered iteration everywhere; seeded
  `rand_pcg` for all randomness; **no `HashMap` in any public-facing order**. Internal
  dedup may use `HashSet` (never iterated for output).
- **No `unsafe`.** **Every loop has a fixed upper bound** (Power-of-10 rule 2): all loops
  iterate over sized collections or an explicit numeric bound (`timesteps`, `max_iter`).
- **`assert_eq!`/`assert_ne!` require `Debug` on operands** — `Hypergraph` has `Debug`
  (Phase-0-1 Task 18), so `assert_eq!(h1, h2)` is fine; for `serde_json::Value` and
  primitive/`Vec` operands it is also fine.
- **`serde_json::Value`-specific methods (`as_object_mut`, `as_array`, …) live in
  concrete `impl`/`fn`s over `Value`, never generic ones.** All Phase-6 modules are
  concrete-`Value`, so this is automatic.
- **Integration tests are direct children of `crates/hypergraph-rs/tests/`.** Feature-
  gated test files begin with `#![cfg(feature = "…")]` so a bare `cargo test` (features
  off) compiles them to empty rather than failing on absent modules.
- **Commit after each task.** Conventional commits, ending with
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

## File structure added by Phase 6

```
crates/hypergraph-rs/
├── Cargo.toml                         # + [features] (convert/dynamics/communities, referencing Phase-2/3's already-optional rand/rand_pcg/rand_core/ndarray deps); + ndarray dev-dep if not already present
├── src/
│   ├── lib.rs                         # + pub mod utils; + gated convert/dynamics/communities
│   ├── core/                          # (unchanged; + graph_attrs() getter in Task 4)
│   ├── utils/
│   │   └── mod.rs                     # dual_dict, powerset, subfaces, get_network_type, convert_labels_to_integers, UtilError
│   ├── convert/
│   │   └── mod.rs                     # hyperedge list/dict, bipartite edgelist, bipartite graph, to_graph, hypergraph_dict, cut_to_order, BipartiteNode, ConvertError
│   ├── dynamics/
│   │   └── mod.rs                     # simulate_kuramoto, compute_kuramoto_order_parameter
│   └── communities/
│       └── mod.rs                     # normalized_laplacian, smallest_k_eigenvectors, kmeans, spectral_clustering, SpectralError
└── tests/
    ├── test_utils.rs                  # (always compiled)
    ├── test_convert.rs                # #![cfg(feature = "convert")]
    ├── test_dynamics.rs               # #![cfg(feature = "dynamics")]
    └── test_communities.rs            # #![cfg(feature = "communities")]
```

Workspace `Cargo.toml` at `hypergraph-rs/Cargo.toml` (the workspace root) is **not**
touched by Phase 6 — the `rand`/`rand_pcg`/`rand_core` pins (corrected by Phase 2) and
`ndarray`/`sprs` (added by Phase 3) are already correct there and already wired into the
core crate's `[dependencies]` as optional deps.

---

## Task 1: Add convert/dynamics/communities feature flags (workspace deps already correct)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

This is a configuration task (no red/green). It wires the new `convert`/`dynamics`/
`communities` modules and adds their feature flags on top of the dependency pins Phase 2
(`rand`/`rand_pcg`/`rand_core`) and Phase 3 (`ndarray`/`sprs`) already corrected — Phase 6
introduces no new external crate and does not touch `hypergraph-rs/Cargo.toml` (the
workspace root).

- [ ] **Step 1: Confirm the workspace dependency pins are already correct (no edit)**

Read `hypergraph-rs/Cargo.toml` (workspace root) and confirm `[workspace.dependencies]`
already carries `rand = "0.10.2"`, `rand_pcg = "0.10.2"`, `rand_core = "0.10.1"` (Phase 2)
and `ndarray = "0.17"` (Phase 3). **Do not edit this file.** If any of those four lines is
missing or has a different value, STOP — Phase 2 and/or Phase 3 have not been executed yet
(or executed differently than assumed); resolve that before continuing Phase 6.

- [ ] **Step 2: Add convert/dynamics/communities features to the core crate `Cargo.toml`**

`hypergraph-rs/crates/hypergraph-rs/Cargo.toml`'s `[dependencies]` already carries `rand`,
`rand_pcg`, `rand_core` (Phase 2, `optional = true`) and `ndarray` (Phase 3,
`optional = true`) — **do not redeclare those lines** (a duplicate dependency key is a
Cargo.toml parse error). Read the file's existing `[features]` table and append the three
new entries below it, alongside whatever `default = [...]` and other feature entries
Phase 2/3 already left in place:

```toml
[features]
# ... existing entries from Phase 2 (e.g. "generators") and Phase 3 (e.g. "linalg"/"stats") ...
convert = []
dynamics = ["dep:rand", "dep:rand_pcg", "dep:rand_core"]
communities = ["dep:rand", "dep:rand_pcg", "dep:rand_core", "dep:ndarray"]
```

If `[dev-dependencies]` does not already have `ndarray = { workspace = true }` (Phase 3
may have added it for its own linalg tests), add it — `test_communities.rs` builds
`Array2` inputs directly and needs `ndarray` as a test-only dependency.

Note: spec §10.1's stated default feature set is
`["algorithms", "generators", "stats", "readwrite", "layout", "viz"]` plus `all`/`network`
— those feature modules land across Phases 2-5/8 (each phase adds its own entry to the
existing `[features]` table as its module lands). Phase 6 does not touch `default = [...]`
(whatever Phase 2/3 left it as); the eventual restored default set is **not** a
WASM-minimal design choice (the spec default in fact includes `viz`/`layout`, which is the
opposite of WASM-minimal) — it is simply deferred until the gated code exists. Gated test
files carry `#![cfg(feature = "…")]` so a bare `cargo test` still succeeds.

- [ ] **Step 3: Wire the modules in `src/lib.rs`**

Append to `hypergraph-rs/crates/hypergraph-rs/src/lib.rs` (after the existing
`pub use` lines):

```rust
pub mod utils;

#[cfg(feature = "convert")]
pub mod convert;

#[cfg(feature = "dynamics")]
pub mod dynamics;

#[cfg(feature = "communities")]
pub mod communities;
```

Create an empty stub so the crate compiles before Task 2 fills it in — write to
`hypergraph-rs/crates/hypergraph-rs/src/utils/mod.rs`:

```rust
//! Pure utility functions ported from `xgi/utils/` (non-Python-machinery only).
```

- [ ] **Step 4: Verify the workspace builds with every feature**

Run: `cd hypergraph-rs && cargo build -p hypergraph-rs --features "convert dynamics communities"`
Expected: builds successfully (empty `utils` module, no gated code yet). If Cargo reports
two `ndarray` versions, the Phase-3 workspace pin is not actually `"0.17"` — stop and fix
`hypergraph-rs/Cargo.toml` (workspace root), don't work around it locally in Phase 6.

Run: `cd hypergraph-rs && cargo build`
Expected: builds (all features off).

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): add convert/dynamics/communities feature flags + utils module stub

Wires the Phase-2/3 already-optional rand/rand_pcg/rand_core/ndarray deps into new
convert/dynamics/communities feature flags; no dependency pins changed in this phase.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: utils — `dual_dict` + `powerset`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/utils/mod.rs`

**Interfaces:**
- `pub fn dual_dict(edge_dict: &IndexMap<String, Vec<String>>) -> IndexMap<String, Vec<String>>`
  — reverse an id→members map to member→ids, first-seen order (XGI `utils/utilities.py:65-94`,
  a `defaultdict(set)`; we preserve node first-seen insertion order instead of hash order).
- `pub fn powerset(items: &[String], include_empty: bool, include_singletons: bool, include_full: bool, max_size: Option<usize>) -> Vec<Vec<String>>`
  — all subsets, lexicographic-by-index order (XGI `utils/utilities.py:97-151`).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`:

```rust
use hypergraph_rs::utils::{dual_dict, powerset};
use indexmap::IndexMap;

fn edge_dict(pairs: &[(&str, &[&str])]) -> IndexMap<String, Vec<String>> {
    let mut m = IndexMap::new();
    for (k, vs) in pairs {
        m.insert(k.to_string(), vs.iter().map(|s| s.to_string()).collect());
    }
    m
}

#[test]
fn test_dual_dict_reverses() {
    // XGI: dual_dict({0:[1,2,3], 1:[0,2]}) == {1:{0}, 2:{0,1}, 3:{0}, 0:{1}}
    let d = edge_dict(&[("0", &["1", "2", "3"]), ("1", &["0", "2"])]);
    let dual = dual_dict(&d);
    assert_eq!(dual.get("1").unwrap(), &vec!["0".to_string()]);
    assert_eq!(dual.get("2").unwrap(), &vec!["0".to_string(), "1".to_string()]);
    assert_eq!(dual.get("3").unwrap(), &vec!["0".to_string()]);
    assert_eq!(dual.get("0").unwrap(), &vec!["1".to_string()]);
    // First-seen node order: 1, 2, 3, 0
    let keys: Vec<&String> = dual.keys().collect();
    assert_eq!(keys, vec!["1", "2", "3", "0"]);
}

#[test]
fn test_dual_dict_empty() {
    let dual = dual_dict(&IndexMap::new());
    assert!(dual.is_empty());
}

#[test]
fn test_powerset_default_len14() {
    // XGI doctest: powerset([1,2,3,4]) -> singletons..triples (no full set) = 14 subsets.
    let items: Vec<String> = ["1", "2", "3", "4"].iter().map(|s| s.to_string()).collect();
    let ps = powerset(&items, false, true, false, None);
    assert_eq!(ps.len(), 14);
    assert_eq!(ps[0], vec!["1".to_string()]);
    assert_eq!(ps.last().unwrap(), &vec!["2".to_string(), "3".to_string(), "4".to_string()]);
}

#[test]
fn test_powerset_include_empty_and_full() {
    let items: Vec<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
    // include_empty + include_full: {} , {a}, {b}, {a,b} = 4
    let ps = powerset(&items, true, true, true, None);
    assert_eq!(ps.len(), 4);
    assert!(ps[0].is_empty());
    assert_eq!(ps.last().unwrap(), &vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_powerset_max_size() {
    let items: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    // singletons + pairs only (max_size 2): 3 + 3 = 6
    let ps = powerset(&items, false, true, false, Some(2));
    assert_eq!(ps.len(), 6);
    assert!(ps.iter().all(|s| s.len() <= 2 && !s.is_empty()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: FAIL — `dual_dict` / `powerset` do not exist (compile error).

- [ ] **Step 3: Implement `dual_dict`, `powerset`, and the shared `combinations` helper**

Write to `hypergraph-rs/crates/hypergraph-rs/src/utils/mod.rs` (replacing the stub):

```rust
//! Pure utility functions ported from `xgi/utils/` (non-Python-machinery only).
//!
//! Ported: `dual_dict`, `powerset`, `subfaces`, `get_network_type`,
//! `convert_labels_to_integers`. See the Phase-6 plan header for the exclusion list.

use indexmap::IndexMap;
use thiserror::Error;

/// Errors raised by utility functions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum UtilError {
    /// `subfaces` was asked for an order larger than the max order of the input edges.
    #[error("order {order} exceeds the maximum edge order {max_order}")]
    OrderTooLarge { order: i64, max_order: i64 },
}

/// Reverse an id→members map into a member→ids map.
///
/// XGI parity: `xgi.dual_dict` (`utils/utilities.py:65-94`). XGI returns
/// `defaultdict(set)` (hash-ordered); we return an insertion-ordered `IndexMap`
/// keyed by node first-seen order (a III.7 determinism improvement). Per-node edge
/// lists preserve the order in which edges were encountered.
pub fn dual_dict(edge_dict: &IndexMap<String, Vec<String>>) -> IndexMap<String, Vec<String>> {
    let mut node_dict: IndexMap<String, Vec<String>> = IndexMap::new();
    for (edge_id, members) in edge_dict {
        for node in members {
            node_dict
                .entry(node.clone())
                .or_default()
                .push(edge_id.clone());
        }
    }
    node_dict
}

/// All `r`-length index-combinations of `items`, in lexicographic-by-index order.
///
/// Matches Python `itertools.combinations` ordering. Bounded: at most `C(n, r)`
/// combinations, each built by a fixed-length inner loop.
fn combinations(items: &[String], r: usize) -> Vec<Vec<String>> {
    let n = items.len();
    let mut out: Vec<Vec<String>> = Vec::new();
    if r == 0 {
        out.push(Vec::new());
        return out;
    }
    if r > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..r).collect();
    loop {
        out.push(idx.iter().map(|&i| items[i].clone()).collect());
        // Advance the index vector (odometer over strictly-increasing indices).
        let mut i = r; // position to bump, scanned from the right
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - r {
                idx[i] += 1;
                for j in (i + 1)..r {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out; // all positions maxed out
            }
        }
    }
}

/// All subsets of `items`, with XGI's option flags.
///
/// XGI parity: `xgi.powerset` (`utils/utilities.py:97-151`).
/// `include_empty` overrides `include_singletons` when true; `max_size` caps the
/// largest subset returned. Order is by ascending size, then lexicographic-by-index.
///
/// **Flag order differs from XGI.** Python's positional signature is
/// `powerset(iterable, include_empty, include_full, include_singletons, max_size)`; this
/// Rust signature is `(items, include_empty, include_singletons, include_full, max_size)`
/// — `include_full` and `include_singletons` are swapped. A future PyO3 binding that
/// mirrors Python's positional call order must remap these two arguments, not pass them
/// straight through.
pub fn powerset(
    items: &[String],
    include_empty: bool,
    include_singletons: bool,
    include_full: bool,
    max_size: Option<usize>,
) -> Vec<Vec<String>> {
    let n = items.len();
    let mut start = if include_singletons { 1 } else { 2 };
    if include_empty {
        start = 0;
    }
    let max = match max_size {
        Some(m) => m.min(n),
        None => {
            if include_full {
                n
            } else {
                n.saturating_sub(1)
            }
        }
    };
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut r = start;
    while r <= max {
        out.extend(combinations(items, r));
        r += 1;
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: PASS — 5 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port utils dual_dict + powerset

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: utils — `subfaces` + `get_network_type`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/utils/mod.rs`

**Interfaces:**
- `pub fn subfaces(edges: &[Vec<String>], order: Option<i64>) -> Result<Vec<Vec<String>>, UtilError>`
  — subfaces of a list of edges (XGI `utils/utilities.py:310-374`). `None` = all subfaces
  down to nodes; `Some(-1)` = one order below; `Some(d>=0)` = order-`d` subfaces.
- `pub fn get_network_type(_h: &Hypergraph) -> String` — the lowercased type name
  (XGI `utils/utilities.py:560-561`). For the undirected `Hypergraph`, `"hypergraph"`.

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`:

```rust
use hypergraph_rs::utils::{get_network_type, subfaces};
use hypergraph_rs::Hypergraph;

fn edges(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect()
}

#[test]
fn test_subfaces_none_all_down_to_nodes() {
    // {1,2,3} -> singletons + pairs (no full triple): C(3,1)+C(3,2) = 3+3 = 6
    let es = edges(&[&["1", "2", "3"]]);
    let faces = subfaces(&es, None).unwrap();
    assert_eq!(faces.len(), 6);
    assert!(faces.contains(&vec!["1".to_string()]));
    assert!(faces.contains(&vec!["1".to_string(), "2".to_string()]));
}

#[test]
fn test_subfaces_order_minus1() {
    // {1,2,3} one order below (size 2): the 3 pairs.
    let es = edges(&[&["1", "2", "3"]]);
    let faces = subfaces(&es, Some(-1)).unwrap();
    assert_eq!(faces.len(), 3);
    assert!(faces.iter().all(|f| f.len() == 2));
}

#[test]
fn test_subfaces_order_d() {
    // order=2 -> subfaces of size 3. {1,2,3,4} has C(4,3)=4 triples.
    let es = edges(&[&["1", "2", "3", "4"]]);
    let faces = subfaces(&es, Some(2)).unwrap();
    assert_eq!(faces.len(), 4);
    assert!(faces.iter().all(|f| f.len() == 3));
}

#[test]
fn test_subfaces_order_too_large_errors() {
    let es = edges(&[&["1", "2"]]); // max order = 1
    let result = subfaces(&es, Some(5));
    assert!(result.is_err());
}

#[test]
fn test_get_network_type() {
    let h: Hypergraph = Hypergraph::new();
    assert_eq!(get_network_type(&h), "hypergraph");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: FAIL — `subfaces` / `get_network_type` do not exist.

- [ ] **Step 3: Implement `subfaces` and `get_network_type`**

Append inside `src/utils/mod.rs`. Add `use crate::Hypergraph;` to the top imports, then:

```rust
/// Subfaces of a list of hyperedges.
///
/// XGI parity: `xgi.subfaces` (`utils/utilities.py:310-374`). Faces are NOT deduplicated
/// (a face may appear once per parent edge), matching XGI. Singleton and empty edges
/// contribute nothing.
pub fn subfaces(edges: &[Vec<String>], order: Option<i64>) -> Result<Vec<Vec<String>>, UtilError> {
    let max_order: i64 = edges.iter().map(|e| e.len() as i64 - 1).max().unwrap_or(-1);
    if let Some(o) = order {
        if o > max_order {
            return Err(UtilError::OrderTooLarge { order: o, max_order });
        }
    }
    let mut faces: Vec<Vec<String>> = Vec::new();
    for edge in edges {
        let size = edge.len();
        if size <= 1 {
            continue;
        }
        let to_add = match order {
            None => powerset(edge, false, true, false, None),
            Some(-1) => combinations(edge, size - 1),
            Some(o) if o >= 0 => combinations(edge, (o + 1) as usize),
            Some(_) => Vec::new(), // order < -1 is undefined in XGI; treat as empty
        };
        faces.extend(to_add);
    }
    Ok(faces)
}

/// The lowercased network type name.
///
/// XGI parity: `xgi.get_network_type` (`utils/utilities.py:560-561`). For the undirected
/// `Hypergraph` this is the constant `"hypergraph"` (matching XGI's
/// `type(H).__name__.lower()` for `Hypergraph`).
pub fn get_network_type(_h: &Hypergraph) -> String {
    "hypergraph".to_string()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: PASS — 10 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port utils subfaces + get_network_type

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: utils — `convert_labels_to_integers` (+ core `graph_attrs()` getter)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/utils/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`

**Interfaces:**
- Core: `pub fn graph_attrs(&self) -> &serde_json::Map<String, serde_json::Value>` — read
  all graph-level attrs (needed here and by `to_hypergraph_dict` in Task 9).
- `pub fn convert_labels_to_integers(h: &Hypergraph, label_attribute: &str) -> Hypergraph`
  — relabel nodes/edges to sequential integer strings (`"0"`, `"1"`, …), stashing the old
  id in `label_attribute` (XGI `utils/utilities.py:377-452`).

**XGI reference:** `convert_labels_to_integers` builds `node_dict`/`edge_dict` mapping old
ids to `range(n)` in iteration order (`utilities.py:406-407`), copies attrs, clears
in-place keeping net attrs, and re-adds with integer ids + the old label under
`label_attribute` (`utilities.py:409-449`). We build a **new** graph instead of
clear-in-place (our core `clear()` also wipes graph attrs; a functional rebuild preserves
them cleanly and is equivalent for the returned value).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_utils.rs`:

```rust
use hypergraph_rs::utils::convert_labels_to_integers;

#[test]
fn test_convert_labels_basic() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into()], Some("first".into()), serde_json::Value::Null)
        .unwrap();
    h.add_edge(vec!["y".into(), "z".into()], Some("second".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    // Nodes x,y,z -> "0","1","2" ; edges first,second -> "0","1"
    assert_eq!(relabeled.node_ids(), vec!["0", "1", "2"]);
    assert_eq!(relabeled.edge_ids(), vec!["0", "1"]);
    assert_eq!(relabeled.num_nodes(), 3);
    assert_eq!(relabeled.num_edges(), 2);
}

#[test]
fn test_convert_labels_stores_old_label() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into()], Some("first".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    assert_eq!(relabeled.node_attrs("0").unwrap()["label"], "x");
    assert_eq!(relabeled.edge_attrs("0").unwrap()["label"], "first");
}

#[test]
fn test_convert_labels_preserves_members_as_sets() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into(), "z".into()], Some("e".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    let mut members = relabeled.members("0").unwrap();
    members.sort();
    assert_eq!(members, vec!["0".to_string(), "1".to_string(), "2".to_string()]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: FAIL — `convert_labels_to_integers` does not exist.

- [ ] **Step 3: Implement the core getter and the utility**

Add inside the inherent `impl<N, E, M> Hypergraph<N, E, M>` block in
`src/core/hypergraph.rs` (e.g., after `set_graph_attr`):

```rust
    /// Read all graph-level attributes. XGI parity: `H._net_attr`.
    pub fn graph_attrs(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.graph_attrs
    }
```

Append to `src/utils/mod.rs`. Add `use serde_json::{json, Value};` to the top imports,
then:

```rust
/// Relabel node and edge ids to sequential integer strings, stashing the old ids.
///
/// XGI parity: `xgi.convert_labels_to_integers` (`utils/utilities.py:377-452`). Nodes and
/// edges are mapped to `"0".."n-1"` / `"0".."m-1"` in insertion order; the old id is
/// written to `label_attribute` on each. Graph-level attrs are carried over. Returns a
/// new `Hypergraph` (we rebuild rather than clear-in-place; see the Phase-6 plan header).
pub fn convert_labels_to_integers(h: &Hypergraph, label_attribute: &str) -> Hypergraph {
    let node_index: IndexMap<String, usize> = h
        .node_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let edge_index: IndexMap<String, usize> = h
        .edge_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    let mut out = Hypergraph::new();

    // Carry over graph-level attributes.
    for (k, v) in h.graph_attrs() {
        out.set_graph_attr(k, v.clone());
    }

    // Nodes: new integer id + old id under label_attribute.
    for (old_id, &i) in &node_index {
        let mut obj = h
            .node_attrs(old_id)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.insert(label_attribute.to_string(), json!(old_id));
        out.add_node(&i.to_string(), Value::Object(obj));
    }

    // Edges: remap members, new integer id + old id under label_attribute.
    for (old_edge, &j) in &edge_index {
        let members: Vec<String> = h
            .members(old_edge)
            .unwrap_or_default()
            .iter()
            .map(|m| node_index[m].to_string())
            .collect();
        let mut obj = h
            .edge_attrs(old_edge)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        obj.insert(label_attribute.to_string(), json!(old_edge));
        out.add_edge(members, Some(j.to_string()), Value::Object(obj))
            .expect("relabeled edge has non-empty members and a fresh unique id");
    }

    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_utils`
Expected: PASS — 13 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port utils convert_labels_to_integers + core graph_attrs getter

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: convert — hyperedge list/dict round-trips

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub fn from_hyperedge_list(edges: Vec<Vec<String>>) -> Hypergraph` (XGI `convert/hyperedges.py:64-90`)
- `pub fn to_hyperedge_list(h: &Hypergraph) -> Vec<Vec<String>>` (XGI `convert/hyperedges.py:93-112`)
- `pub fn from_hyperedge_dict(d: Vec<(String, Vec<String>)>) -> Hypergraph` (XGI `convert/hyperedges.py:14-38`)
- `pub fn to_hyperedge_dict(h: &Hypergraph) -> IndexMap<String, Vec<String>>` (XGI `convert/hyperedges.py:41-61`)

**XGI reference:** `from_hyperedge_list` calls `H.add_edges_from(d)` (auto ids);
`from_hyperedge_dict` calls `H.add_edges_from((members, uid) …)`; the `to_*` forms read
`H.edges.members()`. Members are compared as sets (hash-order in XGI).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
#![cfg(feature = "convert")]

use hypergraph_rs::convert::{
    from_hyperedge_dict, from_hyperedge_list, to_hyperedge_dict, to_hyperedge_list,
};
use hypergraph_rs::Hypergraph;

fn rows(r: &[&[&str]]) -> Vec<Vec<String>> {
    r.iter().map(|e| e.iter().map(|s| s.to_string()).collect()).collect()
}

#[test]
fn test_from_hyperedge_list_auto_ids() {
    let h = from_hyperedge_list(rows(&[&["a", "b"], &["b", "c", "d"]]));
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 4);
    assert_eq!(h.edge_ids(), vec!["0", "1"]);
}

#[test]
fn test_to_hyperedge_list() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let list = to_hyperedge_list(&h);
    assert_eq!(list.len(), 1);
    let mut m = list[0].clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_from_hyperedge_dict_keeps_ids() {
    let d = vec![
        ("first".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("second".to_string(), vec!["c".to_string()]),
    ];
    let h = from_hyperedge_dict(d);
    assert!(h.has_edge("first"));
    assert!(h.has_edge("second"));
    assert_eq!(h.num_nodes(), 3);
}

#[test]
fn test_to_hyperedge_dict() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let d = to_hyperedge_dict(&h);
    assert_eq!(d.keys().collect::<Vec<_>>(), vec!["e1", "e2"]);
    assert_eq!(d.get("e2").unwrap(), &vec!["c".to_string()]);
}

#[test]
fn test_hyperedge_dict_roundtrip_members_as_sets() {
    let d = vec![
        ("e1".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("e2".to_string(), vec!["b".to_string(), "c".to_string()]),
    ];
    let h = from_hyperedge_dict(d);
    let back = to_hyperedge_dict(&h);
    let mut m = back.get("e1").unwrap().clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — `convert` module has no such functions (compile error).

- [ ] **Step 3: Implement the convert module skeleton + hyperedge functions**

Write to `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`:

```rust
//! Conversions to/from hyperedge lists/dicts, bipartite edgelists, bipartite petgraph
//! graphs, the 1-skeleton graph, the standardized hypergraph dict, and order-cutting.
//!
//! Ported from `xgi/convert/`. Operates on the default `Hypergraph`
//! (`serde_json::Value` attrs). The `pandas` slice is NOT ported (spec §4.4); HIF,
//! encapsulation-DAG, line-graph, simplex, and incidence conversions are deferred
//! (see the Phase-6 plan open questions).

use indexmap::IndexMap;
use serde_json::Value;
use thiserror::Error;

use crate::Hypergraph;

/// Errors raised by conversion functions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ConvertError {
    /// The input structure was malformed for the requested conversion.
    #[error("malformed input: {0}")]
    Malformed(String),
    /// A supposedly-bipartite graph had an edge joining two same-kind nodes.
    #[error("graph is not bipartite: edge joins two nodes of the same kind")]
    NotBipartite,
    /// A bipartite-graph node lacked a valid `bipartite` marker (0 or 1).
    #[error("node {0} has an invalid bipartite marker (expected 0 or 1)")]
    InvalidMarker(String),
    /// `cut_to_order` was given an order above the network's maximum edge order.
    #[error("order {order} exceeds the maximum edge order {max_order}")]
    OrderTooLarge { order: i64, max_order: i64 },
}

/// Build a hypergraph from a list of hyperedges (auto-generated ids).
///
/// XGI parity: `xgi.from_hyperedge_list` (`convert/hyperedges.py:64-90`).
pub fn from_hyperedge_list(edges: Vec<Vec<String>>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for members in edges {
        // Non-empty edges only; empty inputs are skipped (our core rejects empty edges).
        if !members.is_empty() {
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    h
}

/// Extract the hyperedge list (members per edge, edge insertion order).
///
/// XGI parity: `xgi.to_hyperedge_list` (`convert/hyperedges.py:93-112`).
pub fn to_hyperedge_list(h: &Hypergraph) -> Vec<Vec<String>> {
    h.edge_ids()
        .into_iter()
        .map(|e| h.members(&e).unwrap_or_default())
        .collect()
}

/// Build a hypergraph from an id→members dict (preserving ids).
///
/// XGI parity: `xgi.from_hyperedge_dict` (`convert/hyperedges.py:14-38`).
pub fn from_hyperedge_dict(d: Vec<(String, Vec<String>)>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for (uid, members) in d {
        if !members.is_empty() {
            let _ = h.add_edge(members, Some(uid), Value::Null);
        }
    }
    h
}

/// Extract the id→members dict (edge insertion order).
///
/// XGI parity: `xgi.to_hyperedge_dict` (`convert/hyperedges.py:41-61`).
pub fn to_hyperedge_dict(h: &Hypergraph) -> IndexMap<String, Vec<String>> {
    h.edge_ids()
        .into_iter()
        .map(|e| {
            let members = h.members(&e).unwrap_or_default();
            (e, members)
        })
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 5 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert hyperedge list/dict round-trips

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: convert — bipartite edgelist round-trip

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub fn from_bipartite_edgelist(edges: Vec<(String, String)>) -> Hypergraph` — each
  `(node, edge)` pair adds the node to the edge (XGI `convert/bipartite_edges.py:9-42`,
  undirected branch).
- `pub fn to_bipartite_edgelist(h: &Hypergraph) -> Vec<(String, String)>` — one
  `(node, edge)` pair per membership (XGI `convert/bipartite_edges.py:45-73`).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
use hypergraph_rs::convert::{from_bipartite_edgelist, to_bipartite_edgelist};

#[test]
fn test_from_bipartite_edgelist() {
    let el = vec![
        ("a".to_string(), "e1".to_string()),
        ("b".to_string(), "e1".to_string()),
        ("c".to_string(), "e2".to_string()),
    ];
    let h = from_bipartite_edgelist(el);
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 3);
    let mut m = h.members("e1").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_to_bipartite_edgelist() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let el = to_bipartite_edgelist(&h);
    assert_eq!(el.len(), 2);
    assert!(el.contains(&("a".to_string(), "e1".to_string())));
    assert!(el.contains(&("b".to_string(), "e1".to_string())));
}

#[test]
fn test_bipartite_edgelist_roundtrip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let back = from_bipartite_edgelist(to_bipartite_edgelist(&h));
    assert_eq!(back.num_nodes(), 3);
    assert_eq!(back.num_edges(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — the two functions don't exist.

- [ ] **Step 3: Implement**

Append to `src/convert/mod.rs`:

```rust
/// Build a hypergraph from a bipartite edgelist of `(node, edge)` pairs.
///
/// XGI parity: `xgi.from_bipartite_edgelist` (`convert/bipartite_edges.py:9-42`,
/// undirected branch — each pair calls `add_node_to_edge(edge, node)`).
pub fn from_bipartite_edgelist(edges: Vec<(String, String)>) -> Hypergraph {
    let mut h = Hypergraph::new();
    for (node, edge) in edges {
        // add_node_to_edge auto-creates both; infallible on the default Value types.
        let _ = h.add_node_to_edge(&edge, &node);
    }
    h
}

/// Extract a bipartite edgelist of `(node, edge)` pairs (edge order, then member order).
///
/// XGI parity: `xgi.to_bipartite_edgelist` (`convert/bipartite_edges.py:45-73`).
pub fn to_bipartite_edgelist(h: &Hypergraph) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for e in h.edge_ids() {
        for n in h.members(&e).unwrap_or_default() {
            out.push((n, e.clone()));
        }
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 8 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert bipartite edgelist round-trip

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 7: convert — bipartite petgraph graph round-trip

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub struct BipartiteNode { pub id: String, pub bipartite: u8 }` (0 = node, 1 = hyperedge).
- `pub fn to_bipartite_graph(h: &Hypergraph) -> StableDiGraph<BipartiteNode, ()>` — build a
  petgraph bipartite graph (XGI `convert/bipartite_graph.py:110-185`, node/edge layers).
- `pub fn from_bipartite_graph(g: &StableDiGraph<BipartiteNode, ()>) -> Result<Hypergraph, ConvertError>`
  — reconstruct (XGI `convert/bipartite_graph.py:9-97`, bipartiteness validated).

**Deviation:** operates on a `petgraph` graph with a `bipartite` marker, not NetworkX
(see the Phase-6 deviations note).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
use hypergraph_rs::convert::{from_bipartite_graph, to_bipartite_graph, BipartiteNode};
use rustworkx_core::petgraph::stable_graph::StableDiGraph;

#[test]
fn test_to_bipartite_graph_markers() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let g = to_bipartite_graph(&h);
    // 2 node-layer + 1 edge-layer = 3 vertices; each membership = 1 arc.
    assert_eq!(g.node_count(), 3);
    let nodes = g.node_weights().filter(|w| w.bipartite == 0).count();
    let edges = g.node_weights().filter(|w| w.bipartite == 1).count();
    assert_eq!(nodes, 2);
    assert_eq!(edges, 1);
}

#[test]
fn test_from_bipartite_graph_reconstructs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let g = to_bipartite_graph(&h);
    let back = from_bipartite_graph(&g).unwrap();
    assert_eq!(back.num_nodes(), 3);
    assert_eq!(back.num_edges(), 2);
    let mut m = back.members("e1").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_from_bipartite_graph_invalid_marker_errors() {
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    g.add_node(BipartiteNode { id: "bad".into(), bipartite: 2 });
    assert!(from_bipartite_graph(&g).is_err());
}

#[test]
fn test_from_bipartite_graph_non_bipartite_errors() {
    // Two node-layer vertices joined directly -> not bipartite.
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    let a = g.add_node(BipartiteNode { id: "a".into(), bipartite: 0 });
    let b = g.add_node(BipartiteNode { id: "b".into(), bipartite: 0 });
    g.add_edge(a, b, ());
    assert!(from_bipartite_graph(&g).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — `BipartiteNode` / the two functions don't exist.

- [ ] **Step 3: Implement**

Append to `src/convert/mod.rs`. Add to the top imports:

```rust
use rustworkx_core::petgraph::stable_graph::StableDiGraph;
use rustworkx_core::petgraph::visit::{EdgeRef, IntoEdgeReferences};
```

(Both traits are required: `IntoEdgeReferences` supplies the `.edge_references()` method
itself — in petgraph 0.8.3 it is not an inherent `StableGraph` method — and `EdgeRef`
supplies `.source()`/`.target()`/`.weight()` on each yielded item.)

Then:

```rust
/// A vertex of a bipartite representation: an id plus a layer marker.
///
/// `bipartite == 0` -> a hypergraph node; `bipartite == 1` -> a hyperedge. Mirrors XGI's
/// `bipartite=0/1` NetworkX node attribute (`convert/bipartite_graph.py`).
#[derive(Clone, Debug, PartialEq)]
pub struct BipartiteNode {
    /// The original node or edge id.
    pub id: String,
    /// Layer marker: 0 = node, 1 = hyperedge.
    pub bipartite: u8,
}

/// Build a bipartite petgraph graph: node-layer + edge-layer vertices, one arc per
/// membership (node -> hyperedge).
///
/// XGI parity: `xgi.to_bipartite_graph` (`convert/bipartite_graph.py:110-185`). Vertices
/// are added in node-insertion order first, then edge-insertion order.
pub fn to_bipartite_graph(h: &Hypergraph) -> StableDiGraph<BipartiteNode, ()> {
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    let mut node_idx = IndexMap::new();
    for n in h.node_ids() {
        let idx = g.add_node(BipartiteNode { id: n.clone(), bipartite: 0 });
        node_idx.insert(n, idx);
    }
    for e in h.edge_ids() {
        let e_idx = g.add_node(BipartiteNode { id: e.clone(), bipartite: 1 });
        for m in h.members(&e).unwrap_or_default() {
            g.add_edge(node_idx[&m], e_idx, ());
        }
    }
    g
}

/// Reconstruct a hypergraph from a bipartite petgraph graph.
///
/// XGI parity: `xgi.from_bipartite_graph` (`convert/bipartite_graph.py:9-97`). Validates
/// that every arc joins a node-layer (0) and an edge-layer (1) vertex.
pub fn from_bipartite_graph(
    g: &StableDiGraph<BipartiteNode, ()>,
) -> Result<Hypergraph, ConvertError> {
    // Validate markers.
    for w in g.node_weights() {
        if w.bipartite > 1 {
            return Err(ConvertError::InvalidMarker(w.id.clone()));
        }
    }
    let mut h = Hypergraph::new();
    // Add all node-layer vertices first (so isolated nodes survive).
    for w in g.node_weights() {
        if w.bipartite == 0 {
            h.add_node(&w.id, Value::Null);
        }
    }
    // Each arc: one endpoint node-layer, one edge-layer.
    for edge in g.edge_references() {
        let src = g.node_weight(edge.source()).expect("valid source");
        let dst = g.node_weight(edge.target()).expect("valid target");
        let (node, hyperedge) = match (src.bipartite, dst.bipartite) {
            (0, 1) => (&src.id, &dst.id),
            (1, 0) => (&dst.id, &src.id),
            _ => return Err(ConvertError::NotBipartite),
        };
        let _ = h.add_node_to_edge(hyperedge, node);
    }
    Ok(h)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 12 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert bipartite petgraph graph round-trip

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 8: convert — `to_graph` (1-skeleton projection)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub fn to_graph(h: &Hypergraph) -> UnGraph<String, ()>` — the unweighted 1-skeleton
  (two nodes adjacent iff they co-occur in some hyperedge). XGI `convert/graph.py:8-28`
  builds it from the adjacency matrix; we build it directly from memberships (equivalent,
  and independent of the Phase-3 linalg module).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
use hypergraph_rs::convert::to_graph;

#[test]
fn test_to_graph_node_count() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let g = to_graph(&h);
    assert_eq!(g.node_count(), 3);
    // A triangle-hyperedge projects to a 3-clique: 3 undirected edges.
    assert_eq!(g.edge_count(), 3);
}

#[test]
fn test_to_graph_disjoint_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let g = to_graph(&h);
    assert_eq!(g.node_count(), 4);
    assert_eq!(g.edge_count(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — `to_graph` doesn't exist.

- [ ] **Step 3: Implement**

Append to `src/convert/mod.rs`. Add to imports:

```rust
use rustworkx_core::petgraph::graph::UnGraph;
use std::collections::HashSet;
```

Then:

```rust
/// The unweighted 1-skeleton (graph projection) of the hypergraph.
///
/// XGI parity: `xgi.to_graph` (`convert/graph.py:8-28`). Two nodes are linked iff they
/// share at least one hyperedge. Node weights are the node ids (node insertion order);
/// duplicate pairs are added once. Built directly from memberships (no linalg dependency).
pub fn to_graph(h: &Hypergraph) -> UnGraph<String, ()> {
    let mut g: UnGraph<String, ()> = UnGraph::new_undirected();
    let mut node_idx = IndexMap::new();
    for n in h.node_ids() {
        let idx = g.add_node(n.clone());
        node_idx.insert(n, idx);
    }
    // Dedup pairs by their (min, max) positional indices — deterministic, no HashMap order.
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a = node_idx[&members[i]];
                let b = node_idx[&members[j]];
                let key = if a.index() <= b.index() {
                    (a.index(), b.index())
                } else {
                    (b.index(), a.index())
                };
                if seen.insert(key) {
                    g.add_edge(a, b, ());
                }
            }
        }
    }
    g
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 14 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert to_graph (1-skeleton projection)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 9: convert — standardized hypergraph dict round-trip

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub fn to_hypergraph_dict(h: &Hypergraph) -> Value` — the xgi-data standard dict
  (XGI `convert/hypergraph_dict.py:12-72`): keys `type`, `hypergraph-data`, `node-data`,
  `edge-data`, `edge-dict` (members **sorted**).
- `pub fn from_hypergraph_dict(data: &Value) -> Result<Hypergraph, ConvertError>`
  (XGI `convert/hypergraph_dict.py:75-165`).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
use hypergraph_rs::convert::{from_hypergraph_dict, to_hypergraph_dict};

#[test]
fn test_to_hypergraph_dict_shape() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["b".into(), "a".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.set_graph_attr("name", serde_json::json!("demo"));
    let d = to_hypergraph_dict(&h);
    assert_eq!(d["type"], "hypergraph");
    assert_eq!(d["hypergraph-data"]["name"], "demo");
    // Members are sorted in edge-dict.
    assert_eq!(d["edge-dict"]["e1"], serde_json::json!(["a", "b"]));
    assert!(d["node-data"].get("a").is_some());
    assert!(d["edge-data"].get("e1").is_some());
}

#[test]
fn test_from_hypergraph_dict() {
    let data = serde_json::json!({
        "type": "hypergraph",
        "hypergraph-data": {"name": "demo"},
        "node-data": {"a": {}, "b": {}},
        "edge-data": {"e1": {"w": 3}},
        "edge-dict": {"e1": ["a", "b"]}
    });
    let h = from_hypergraph_dict(&data).unwrap();
    assert_eq!(h.num_nodes(), 2);
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.graph_attr("name"), Some(&serde_json::json!("demo")));
    assert_eq!(h.edge_attrs("e1").unwrap()["w"], 3);
}

#[test]
fn test_hypergraph_dict_roundtrip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::json!({"w": 1})).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let back = from_hypergraph_dict(&to_hypergraph_dict(&h)).unwrap();
    assert_eq!(back.num_nodes(), 3);
    assert_eq!(back.num_edges(), 2);
    let mut m = back.members("e2").unwrap();
    m.sort();
    assert_eq!(m, vec!["b".to_string(), "c".to_string()]);
}

#[test]
fn test_from_hypergraph_dict_missing_key_errors() {
    let data = serde_json::json!({"type": "hypergraph"}); // no edge-dict/node-data
    assert!(from_hypergraph_dict(&data).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — the two functions don't exist.

- [ ] **Step 3: Implement**

Append to `src/convert/mod.rs`. Add `use serde_json::Map;` to the imports, then:

```rust
/// Coerce a node/edge attr value to the shape XGI serializes: a JSON object. XGI's
/// `H.nodes[idx]`/`H.edges[idx]` are always dicts (never `None`), so a default
/// `Value::Null` attr (e.g. from auto-created members) must emit `{}`, not `null`, to
/// match the xgi-data contract.
fn attrs_as_object(v: Option<&Value>) -> Value {
    match v.and_then(|v| v.as_object()) {
        Some(obj) => Value::Object(obj.clone()),
        None => Value::Object(Map::new()),
    }
}

/// Serialize to the standardized xgi-data hypergraph dict.
///
/// XGI parity: `xgi.to_hypergraph_dict` (`convert/hypergraph_dict.py:12-72`). `edge-dict`
/// members are sorted (stringwise). `type` is the constant `"hypergraph"`. `node-data`/
/// `edge-data` entries are always JSON objects (a `Value::Null` attr serializes as `{}`,
/// matching XGI's `H.nodes[idx]` dict-always contract — see `attrs_as_object`).
pub fn to_hypergraph_dict(h: &Hypergraph) -> Value {
    let mut root = Map::new();
    root.insert("type".into(), Value::String("hypergraph".into()));
    root.insert("hypergraph-data".into(), Value::Object(h.graph_attrs().clone()));

    let mut node_data = Map::new();
    for n in h.node_ids() {
        node_data.insert(n.clone(), attrs_as_object(h.node_attrs(&n)));
    }
    root.insert("node-data".into(), Value::Object(node_data));

    let mut edge_data = Map::new();
    let mut edge_dict = Map::new();
    for e in h.edge_ids() {
        edge_data.insert(e.clone(), attrs_as_object(h.edge_attrs(&e)));
        let mut members = h.members(&e).unwrap_or_default();
        members.sort();
        edge_dict.insert(
            e.clone(),
            Value::Array(members.into_iter().map(Value::String).collect()),
        );
    }
    root.insert("edge-data".into(), Value::Object(edge_data));
    root.insert("edge-dict".into(), Value::Object(edge_dict));
    Value::Object(root)
}

/// Deserialize from the standardized xgi-data hypergraph dict.
///
/// XGI parity: `xgi.from_hypergraph_dict` (`convert/hypergraph_dict.py:75-165`). Requires
/// `hypergraph-data`, `node-data`, `edge-dict`; `edge-data` attrs are applied if present.
pub fn from_hypergraph_dict(data: &Value) -> Result<Hypergraph, ConvertError> {
    let obj = data
        .as_object()
        .ok_or_else(|| ConvertError::Malformed("top-level value is not an object".into()))?;

    let mut h = Hypergraph::new();

    let hdata = obj
        .get("hypergraph-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing hypergraph-data".into()))?;
    for (k, v) in hdata {
        h.set_graph_attr(k, v.clone());
    }

    let node_data = obj
        .get("node-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing node-data".into()))?;
    for (nid, attrs) in node_data {
        h.add_node(nid, attrs.clone());
    }

    let edge_dict = obj
        .get("edge-dict")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ConvertError::Malformed("missing edge-dict".into()))?;
    for (eid, members_val) in edge_dict {
        let members: Vec<String> = members_val
            .as_array()
            .ok_or_else(|| ConvertError::Malformed(format!("edge {eid} members not an array")))?
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect();
        if !members.is_empty() {
            h.add_edge(members, Some(eid.clone()), Value::Null)
                .map_err(|e| ConvertError::Malformed(format!("edge {eid}: {e}")))?;
        }
    }

    // Optional edge attributes.
    if let Some(edge_data) = obj.get("edge-data").and_then(|v| v.as_object()) {
        let pairs: Vec<(String, Map<String, Value>)> = edge_data
            .iter()
            .filter(|(eid, _)| h.has_edge(eid))
            .filter_map(|(eid, v)| v.as_object().map(|o| (eid.clone(), o.clone())))
            .collect();
        h.set_edge_attributes(pairs);
    }

    Ok(h)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 18 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert standardized hypergraph dict round-trip

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 10: convert — `cut_to_order`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/convert/mod.rs`

**Interfaces:**
- `pub fn cut_to_order(h: &Hypergraph, order: i64) -> Result<Hypergraph, ConvertError>`
  — a copy keeping only edges of order ≤ `order` (edge order = `len(members) - 1`). XGI
  `convert/higher_order_network.py:219-244`; errors if `order` exceeds the max edge order.

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_convert.rs`:

```rust
use hypergraph_rs::convert::cut_to_order;

#[test]
fn test_cut_to_order_removes_large_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("small".into()), serde_json::Value::Null).unwrap(); // order 1
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("big".into()), serde_json::Value::Null).unwrap(); // order 2
    let cut = cut_to_order(&h, 1).unwrap();
    assert!(cut.has_edge("small"));
    assert!(!cut.has_edge("big"));
    // Nodes are preserved even when their large edge is cut.
    assert!(cut.has_node("c"));
}

#[test]
fn test_cut_to_order_keeps_all_when_order_is_max() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let cut = cut_to_order(&h, 2).unwrap();
    assert_eq!(cut.num_edges(), 2);
}

#[test]
fn test_cut_to_order_too_large_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap(); // max order 1
    assert!(cut_to_order(&h, 3).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: FAIL — `cut_to_order` doesn't exist.

- [ ] **Step 3: Implement**

Append to `src/convert/mod.rs`:

```rust
/// A copy of the hypergraph keeping only edges of order ≤ `order`.
///
/// XGI parity: `xgi.cut_to_order` (`convert/higher_order_network.py:219-244`). Edge order
/// is `len(members) - 1`. Errors if `order` exceeds the network's maximum edge order.
/// Nodes are preserved even when their edges are cut.
pub fn cut_to_order(h: &Hypergraph, order: i64) -> Result<Hypergraph, ConvertError> {
    let max_order: i64 = h
        .edge_ids()
        .into_iter()
        .map(|e| h.members(&e).map(|m| m.len() as i64 - 1).unwrap_or(0))
        .max()
        .unwrap_or(-1);
    if order > max_order {
        return Err(ConvertError::OrderTooLarge { order, max_order });
    }
    let mut cut = h.copy();
    let to_remove: Vec<String> = h
        .edge_ids()
        .into_iter()
        .filter(|e| h.members(e).map(|m| m.len() as i64 - 1).unwrap_or(0) > order)
        .collect();
    for e in to_remove {
        let _ = cut.remove_edge(&e);
    }
    Ok(cut)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features convert --test test_convert`
Expected: PASS — 21 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port convert cut_to_order

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 11: dynamics — `compute_kuramoto_order_parameter`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_dynamics.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/dynamics/mod.rs`

**Interfaces:**
- `pub fn compute_kuramoto_order_parameter(theta_time: &[Vec<f64>]) -> Vec<f64>` — the
  Kuramoto order parameter per timestep: `r_t = |mean_n exp(i·theta_time[t][n])|`. XGI
  `dynamics/synchronization.py:121-141` (pure; no RNG).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_dynamics.rs`:

```rust
#![cfg(feature = "dynamics")]

use hypergraph_rs::dynamics::compute_kuramoto_order_parameter;

#[test]
fn test_order_param_fully_synchronized_is_one() {
    // All phases equal -> perfect synchrony -> r = 1.
    let theta_time = vec![vec![0.5, 0.5, 0.5], vec![1.2, 1.2, 1.2]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert_eq!(r.len(), 2);
    assert!((r[0] - 1.0).abs() < 1e-12);
    assert!((r[1] - 1.0).abs() < 1e-12);
}

#[test]
fn test_order_param_antiphase_is_zero() {
    use std::f64::consts::PI;
    // Two nodes exactly antiphase -> r = 0.
    let theta_time = vec![vec![0.0, PI]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert!(r[0].abs() < 1e-12);
}

#[test]
fn test_order_param_length_matches_timesteps() {
    let theta_time = vec![vec![0.0, 0.1], vec![0.2, 0.3], vec![0.4, 0.5]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|&x| (0.0..=1.0 + 1e-12).contains(&x)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features dynamics --test test_dynamics`
Expected: FAIL — the `dynamics` module has no such function.

- [ ] **Step 3: Implement**

Write to `hypergraph-rs/crates/hypergraph-rs/src/dynamics/mod.rs`:

```rust
//! Kuramoto synchronization dynamics ported from `xgi/dynamics/synchronization.py`.
//!
//! NOTE: XGI's `dynamics/` slice is Kuramoto synchronization, NOT epidemic (SIR/SIS)
//! models — the spec's "epidemic" summary is wrong (see the Phase-6 plan header). This
//! module ports the pieces whose dependencies exist by Phase 6: `simulate_kuramoto`
//! (pairwise + triangle coupling) and `compute_kuramoto_order_parameter`. The simplicial
//! Kuramoto pair needs the Phase-3 Hodge boundary matrix + `SimplicialComplex` and is
//! deferred.
//!
//! Complex arithmetic is done manually with `(re, im)` f64 pairs (no `num-complex`
//! dependency). Randomness (default `omega`/`theta`) uses seeded `rand_pcg::Pcg64Mcg`;
//! sequence-level parity with numpy is NOT claimed (spec §9.4).

/// The Kuramoto order parameter timeseries.
///
/// XGI parity: `xgi.compute_kuramoto_order_parameter`
/// (`dynamics/synchronization.py:121-141`). For each timestep `t`,
/// `r_t = |(1/N) Σ_n exp(i·theta[t][n])|` — a synchrony measure in `[0, 1]`.
pub fn compute_kuramoto_order_parameter(theta_time: &[Vec<f64>]) -> Vec<f64> {
    theta_time
        .iter()
        .map(|row| {
            let n = row.len();
            if n == 0 {
                return 0.0;
            }
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for &theta in row {
                re += theta.cos();
                im += theta.sin();
            }
            re /= n as f64;
            im /= n as f64;
            (re * re + im * im).sqrt()
        })
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features dynamics --test test_dynamics`
Expected: PASS — 3 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port dynamics compute_kuramoto_order_parameter

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 12: dynamics — `simulate_kuramoto`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_dynamics.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/dynamics/mod.rs`

**Interfaces:**
- `pub fn simulate_kuramoto(h: &Hypergraph, k2: f64, k3: f64, omega: Option<Vec<f64>>, theta: Option<Vec<f64>>, timesteps: usize, dt: f64, seed: Option<u64>) -> (Vec<Vec<f64>>, Vec<f64>)`
  — Euler integration of the pairwise+triangle Kuramoto model. Returns
  `(theta_time [timesteps × N], times [timesteps])`. XGI
  `dynamics/synchronization.py:18-118`.

**XGI reference:** relabels to integers, extracts size-2 edges as `links` and size-3 as
`triangles` (`synchronization.py:72-76`), defaults `omega` from `N(0,1)` and `theta` from
`U[0,2π)` (`:83-87`), then integrates the phase ODE via Euler
(`:89-116`) with the complex accumulators `r1` (links) and `r2` (triangles). We derive
integer node indices from `node_ids()` insertion order directly (no in-place relabel).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_dynamics.rs`:

```rust
use hypergraph_rs::dynamics::simulate_kuramoto;
use hypergraph_rs::Hypergraph;

fn two_edge_hg() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("link".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("tri".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_simulate_uniform_phase_stays_uniform() {
    // Identical phases + zero natural frequency => zero coupling drive => phases frozen.
    let h = two_edge_hg();
    let omega = vec![0.0, 0.0, 0.0];
    let theta = vec![0.5, 0.5, 0.5];
    let (theta_time, _times) =
        simulate_kuramoto(&h, 1.0, 1.0, Some(omega), Some(theta), 5, 0.01, None);
    for row in &theta_time {
        for &x in row {
            assert!((x - 0.5).abs() < 1e-9);
        }
    }
}

#[test]
fn test_simulate_shape() {
    let h = two_edge_hg();
    let (theta_time, times) = simulate_kuramoto(
        &h, 2.0, 3.0, Some(vec![0.1, 0.2, 0.3]), Some(vec![0.0, 1.0, 2.0]), 7, 0.002, None,
    );
    assert_eq!(theta_time.len(), 7);
    assert!(theta_time.iter().all(|r| r.len() == 3));
    assert_eq!(times.len(), 7);
    assert!((times[1] - 0.002).abs() < 1e-12);
}

#[test]
fn test_simulate_deterministic_with_seed() {
    // Defaulted omega/theta drawn from the seeded PCG => identical runs for the same seed.
    let h = two_edge_hg();
    let (a, _) = simulate_kuramoto(&h, 1.0, 1.0, None, None, 4, 0.01, Some(42));
    let (b, _) = simulate_kuramoto(&h, 1.0, 1.0, None, None, 4, 0.01, Some(42));
    assert_eq!(a, b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features dynamics --test test_dynamics`
Expected: FAIL — `simulate_kuramoto` doesn't exist.

- [ ] **Step 3: Implement**

Append to `src/dynamics/mod.rs`. Add to the top of the file:

```rust
use indexmap::IndexMap;
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;

use crate::Hypergraph;
```

Then:

```rust
/// Accumulate `exp(i·phi)` into a `(re, im)` complex accumulator.
#[inline]
fn add_exp(acc: &mut (f64, f64), phi: f64) {
    acc.0 += phi.cos();
    acc.1 += phi.sin();
}

/// Imaginary part of `r · exp(-i·theta)` for `r = (re, im)`:
/// `Im((re + i·im)(cos θ − i·sin θ)) = im·cos θ − re·sin θ`.
#[inline]
fn imag_times_conj(r: (f64, f64), theta: f64) -> f64 {
    r.1 * theta.cos() - r.0 * theta.sin()
}

/// Simulate the Kuramoto model on a hypergraph (pairwise + triangle coupling).
///
/// XGI parity: `xgi.simulate_kuramoto` (`dynamics/synchronization.py:18-118`). Integrates
/// the phase ODE via the explicit Euler method over `timesteps` steps of size `dt`.
/// Returns `(theta_time, times)` where `theta_time[t][n]` is node `n`'s phase at step `t`.
///
/// Node indices follow `h.node_ids()` insertion order. `links` are size-2 edges and
/// `triangles` are size-3 edges (other sizes do not contribute — matching XGI's
/// `filterby("size", 2 | 3)`). Defaulted `omega` ~ N(0,1) (Box–Muller) and
/// `theta` ~ U[0,2π) are drawn from a seeded `Pcg64Mcg`; supply both for exact
/// determinism independent of the RNG.
#[allow(clippy::too_many_arguments)]
pub fn simulate_kuramoto(
    h: &Hypergraph,
    k2: f64,
    k3: f64,
    omega: Option<Vec<f64>>,
    theta: Option<Vec<f64>>,
    timesteps: usize,
    dt: f64,
    seed: Option<u64>,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = h.num_nodes();
    let node_index: IndexMap<String, usize> = h
        .node_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    // Extract links (size 2) and triangles (size 3) as index tuples.
    let mut links: Vec<(usize, usize)> = Vec::new();
    let mut triangles: Vec<(usize, usize, usize)> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        match members.len() {
            2 => links.push((node_index[&members[0]], node_index[&members[1]])),
            3 => triangles.push((
                node_index[&members[0]],
                node_index[&members[1]],
                node_index[&members[2]],
            )),
            _ => {}
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed.unwrap_or(0));

    let omega: Vec<f64> = omega.unwrap_or_else(|| {
        // N(0, 1) via Box–Muller from the seeded PCG.
        (0..n)
            .map(|_| {
                let u1 = 1.0 - rng.random_range(0.0..1.0); // in (0, 1]
                let u2 = rng.random_range(0.0..1.0);
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
            })
            .collect()
    });
    let mut theta: Vec<f64> = theta.unwrap_or_else(|| {
        (0..n)
            .map(|_| rng.random_range(0.0..1.0) * 2.0 * std::f64::consts::PI)
            .collect()
    });

    let mut theta_time: Vec<Vec<f64>> = vec![vec![0.0; n]; timesteps];
    let times: Vec<f64> = (0..timesteps).map(|t| t as f64 * dt).collect();

    for t in 0..timesteps {
        theta_time[t] = theta.clone();

        let mut r1 = vec![(0.0_f64, 0.0_f64); n];
        let mut r2 = vec![(0.0_f64, 0.0_f64); n];

        for &(i, j) in &links {
            add_exp(&mut r1[i], theta[j]);
            add_exp(&mut r1[j], theta[i]);
        }
        for &(i, j, k) in &triangles {
            add_exp(&mut r2[i], 2.0 * theta[j] - theta[k]);
            add_exp(&mut r2[i], 2.0 * theta[k] - theta[j]);
            add_exp(&mut r2[j], 2.0 * theta[i] - theta[k]);
            add_exp(&mut r2[j], 2.0 * theta[k] - theta[i]);
            add_exp(&mut r2[k], 2.0 * theta[i] - theta[j]);
            add_exp(&mut r2[k], 2.0 * theta[j] - theta[i]);
        }

        let mut next = vec![0.0_f64; n];
        for idx in 0..n {
            let d_theta = omega[idx]
                + k2 * imag_times_conj(r1[idx], theta[idx])
                + k3 * imag_times_conj(r2[idx], theta[idx]);
            next[idx] = theta[idx] + d_theta * dt;
        }
        theta = next;
    }

    (theta_time, times)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features dynamics --test test_dynamics`
Expected: PASS — 6 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): port dynamics simulate_kuramoto (pairwise + triangle)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 13: communities — normalized hypergraph Laplacian (internal builder)

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/communities/mod.rs`

**Interfaces:**
- `pub fn normalized_laplacian(h: &Hypergraph) -> Result<(Array2<f64>, Vec<String>), SpectralError>`
  — the dense normalized hypergraph Laplacian + the row→node-id map. XGI
  `linalg/laplacian_matrix.py:187-254` (a Phase-3 module, built inline here so
  `communities` is self-contained). Errors on any isolated node.

**XGI formula (`laplacian_matrix.py:252`):**
`L = I − Dv^{-1/2} · H · W · De^{-1} · Hᵀ · Dv^{-1/2}` where `H` is the incidence matrix
(nodes × edges), `Dv` = node degrees, `De` = edge sizes, `W` = edge weights (all 1,
unweighted). Isolated nodes (`Dv[i] == 0`) raise (`laplacian_matrix.py:222-225`).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`:

```rust
#![cfg(feature = "communities")]

use hypergraph_rs::communities::normalized_laplacian;
use hypergraph_rs::Hypergraph;

fn two_triangles() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("t1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["d".into(), "e".into(), "f".into()], Some("t2".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_laplacian_dims_and_rowdict() {
    let h = two_triangles();
    let (l, rowdict) = normalized_laplacian(&h).unwrap();
    assert_eq!(l.nrows(), 6);
    assert_eq!(l.ncols(), 6);
    assert_eq!(rowdict.len(), 6);
    assert_eq!(rowdict[0], "a");
}

#[test]
fn test_laplacian_diagonal_is_identity_minus_selfterm() {
    // For a single simplex-triangle, each node has degree 1; the diagonal of L is
    // 1 - (1/edge_size) = 1 - 1/3 = 2/3.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("t".into()), serde_json::Value::Null).unwrap();
    let (l, _) = normalized_laplacian(&h).unwrap();
    for i in 0..3 {
        assert!((l[[i, i]] - (2.0 / 3.0)).abs() < 1e-12);
    }
}

#[test]
fn test_laplacian_isolated_node_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e".into()), serde_json::Value::Null).unwrap();
    h.add_node("lonely", serde_json::Value::Null); // degree 0
    assert!(normalized_laplacian(&h).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: FAIL — the `communities` module has no such function.

- [ ] **Step 3: Implement**

Write to `hypergraph-rs/crates/hypergraph-rs/src/communities/mod.rs`:

```rust
//! Spectral community detection ported from `xgi/communities/spectral.py`.
//!
//! `spectral_clustering` builds the normalized hypergraph Laplacian (formula from the
//! Phase-3 `linalg/laplacian_matrix.py`, inlined here for self-containment), extracts the
//! `k` smallest-eigenvalue eigenvectors via hand-rolled power iteration + Hotelling
//! deflation (the dependency probe found no LAPACK-free ndarray eigensolver), and runs
//! Forgy k-means. Exact XGI parity is impossible (RNG divergence, spec §9.4); assertions
//! are structural, and results are deterministic under a fixed `seed`.

use ndarray::{Array2, Array1};
use thiserror::Error;

use crate::Hypergraph;

/// Errors raised by spectral clustering.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SpectralError {
    /// More clusters were requested than there are nodes.
    #[error("cannot request {k} clusters from a hypergraph with {n} nodes")]
    TooManyClusters { k: usize, n: usize },
    /// The hypergraph has an isolated node (the normalized Laplacian divides by degree).
    #[error("every node must belong to an edge (isolated node breaks the normalized Laplacian)")]
    IsolatedNode,
}

/// The dense normalized hypergraph Laplacian and its row→node-id map.
///
/// XGI parity: `normalized_hypergraph_laplacian` (`linalg/laplacian_matrix.py:187-254`),
/// unweighted. `L = I − Dv^{-1/2} H De^{-1} Hᵀ Dv^{-1/2}`, with `Dv` = node degrees,
/// `De` = edge sizes. Errors on any isolated node.
pub fn normalized_laplacian(
    h: &Hypergraph,
) -> Result<(Array2<f64>, Vec<String>), SpectralError> {
    let node_ids = h.node_ids();
    let edge_ids = h.edge_ids();
    let n = node_ids.len();
    let m = edge_ids.len();

    // Node index map.
    let mut node_pos = std::collections::HashMap::new();
    for (i, id) in node_ids.iter().enumerate() {
        node_pos.insert(id.clone(), i);
    }

    // Incidence (n x m), edge sizes De (m), node degrees Dv (n).
    let mut incidence = Array2::<f64>::zeros((n, m));
    let mut de = vec![0.0_f64; m];
    let mut dv = vec![0.0_f64; n];
    for (j, e) in edge_ids.iter().enumerate() {
        let members = h.members(e).unwrap_or_default();
        for member in &members {
            let i = node_pos[member];
            incidence[[i, j]] = 1.0;
            de[j] += 1.0;
            dv[i] += 1.0;
        }
    }

    // Isolated-node guard.
    for &d in &dv {
        if d == 0.0 {
            return Err(SpectralError::IsolatedNode);
        }
    }

    // M[i][k] = Σ_j (H[i][j] H[k][j] / De[j]) · Dv[i]^{-1/2} Dv[k]^{-1/2}
    // L = I − M.
    let dv_invsqrt: Vec<f64> = dv.iter().map(|&d| d.powf(-0.5)).collect();
    let mut l = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        for k in 0..n {
            let mut acc = 0.0_f64;
            for j in 0..m {
                let hij = incidence[[i, j]];
                let hkj = incidence[[k, j]];
                if hij != 0.0 && hkj != 0.0 {
                    acc += hij * hkj / de[j];
                }
            }
            let m_ik = acc * dv_invsqrt[i] * dv_invsqrt[k];
            l[[i, k]] = if i == k { 1.0 - m_ik } else { -m_ik };
        }
    }

    // Silence the unused-import lint until the eigensolver (Task 14) uses Array1.
    let _ = Array1::<f64>::zeros(0);

    Ok((l, node_ids))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: PASS — 3 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): communities normalized hypergraph Laplacian builder

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 14: communities — symmetric eigensolver (power iteration + deflation)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/communities/mod.rs`

**Interfaces:**
- `pub fn smallest_k_eigenvectors(l: &Array2<f64>, k: usize, seed: Option<u64>) -> Array2<f64>`
  — the `k` eigenvectors of the symmetric matrix `l` with the smallest eigenvalues, as
  columns of an `n × k` array. Replaces `scipy.sparse.linalg.eigsh(..., which="SA")`
  (XGI `communities/spectral.py:61`).

**Method:** shift to `B = cI − L` where `c` is a Gershgorin upper bound on `L`'s spectral
radius (`c = max_i Σ_j |L[i,j]|`), so `B`'s largest-eigenvalue eigenvectors are `L`'s
smallest. Power-iterate with Gram–Schmidt deflation against previously-found eigenvectors.
Documented tolerance: convergence when `‖v_new − v_old‖ < 1e-9` or after 1000 iterations.
Sign of each eigenvector is arbitrary (does not affect downstream k-means).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`:

```rust
use hypergraph_rs::communities::smallest_k_eigenvectors;

#[test]
fn test_eigen_returns_k_columns() {
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let x = smallest_k_eigenvectors(&l, 2, Some(1));
    assert_eq!(x.nrows(), 6);
    assert_eq!(x.ncols(), 2);
}

#[test]
fn test_eigen_nullspace_separates_two_components() {
    // Two disconnected triangles => the 2-dim smallest-eigenvalue subspace separates
    // components: rows a,b,c share coordinates distinct from d,e,f.
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let x = smallest_k_eigenvectors(&l, 2, Some(1));
    // Distance within a component is ~0; across components is not.
    let dist = |i: usize, j: usize| -> f64 {
        ((x[[i, 0]] - x[[j, 0]]).powi(2) + (x[[i, 1]] - x[[j, 1]]).powi(2)).sqrt()
    };
    // Rows 0,1,2 = a,b,c ; rows 3,4,5 = d,e,f (node insertion order).
    assert!(dist(0, 1) < 1e-6, "a,b within component should coincide");
    assert!(dist(0, 2) < 1e-6, "a,c within component should coincide");
    assert!(dist(3, 4) < 1e-6, "d,e within component should coincide");
    assert!(dist(0, 3) > 1e-3, "components should be separated");
}

#[test]
fn test_eigen_deterministic_with_seed() {
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let a = smallest_k_eigenvectors(&l, 2, Some(7));
    let b = smallest_k_eigenvectors(&l, 2, Some(7));
    assert_eq!(a, b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: FAIL — `smallest_k_eigenvectors` doesn't exist.

- [ ] **Step 3: Implement**

In `src/communities/mod.rs`: add the RNG imports to the top of the file:

```rust
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;
```

Remove the temporary `let _ = Array1::<f64>::zeros(0);` line from `normalized_laplacian`
(the eigensolver below now uses `Array1`). Then append:

```rust
/// Dot product of two column vectors.
#[inline]
fn dot(a: &Array1<f64>, b: &Array1<f64>) -> f64 {
    let mut s = 0.0;
    for i in 0..a.len() {
        s += a[i] * b[i];
    }
    s
}

/// Matrix–vector product `m · v` for a dense symmetric matrix.
fn matvec(m: &Array2<f64>, v: &Array1<f64>) -> Array1<f64> {
    let n = m.nrows();
    let mut out = Array1::<f64>::zeros(n);
    for i in 0..n {
        let mut s = 0.0;
        for j in 0..n {
            s += m[[i, j]] * v[j];
        }
        out[i] = s;
    }
    out
}

/// The `k` smallest-eigenvalue eigenvectors of the symmetric matrix `l`, as columns.
///
/// Replaces `scipy` `eigsh(which="SA")` (`communities/spectral.py:61`). Uses power
/// iteration on `B = cI − L` (largest of `B` = smallest of `L`) with Gram–Schmidt
/// deflation. Convergence tolerance `1e-9`; at most 1000 iterations per eigenvector.
/// Eigenvector signs are arbitrary. Deterministic under `seed`.
pub fn smallest_k_eigenvectors(l: &Array2<f64>, k: usize, seed: Option<u64>) -> Array2<f64> {
    let n = l.nrows();
    let k = k.min(n);

    // Gershgorin bound on the spectral radius: c = max_i Σ_j |L[i,j]|.
    let mut c = 0.0_f64;
    for i in 0..n {
        let mut row_abs = 0.0;
        for j in 0..n {
            row_abs += l[[i, j]].abs();
        }
        if row_abs > c {
            c = row_abs;
        }
    }
    // B = cI − L.
    let mut b = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        for j in 0..n {
            b[[i, j]] = if i == j { c - l[[i, j]] } else { -l[[i, j]] };
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed.unwrap_or(0));
    let mut eigvecs: Vec<Array1<f64>> = Vec::with_capacity(k);

    const TOL: f64 = 1e-9;
    const MAX_ITER: usize = 1000;

    for _ in 0..k {
        // Random init.
        let mut v = Array1::<f64>::zeros(n);
        for i in 0..n {
            v[i] = rng.random_range(0.0..1.0) - 0.5;
        }
        // Deflate + normalize the initial vector.
        deflate(&mut v, &eigvecs);
        normalize(&mut v);

        for _ in 0..MAX_ITER {
            let mut w = matvec(&b, &v);
            deflate(&mut w, &eigvecs);
            normalize(&mut w);
            // Fix sign for a stable convergence check (largest-|component| positive).
            align_sign(&mut w);
            let mut prev = v.clone();
            align_sign(&mut prev);
            let mut diff = 0.0;
            for i in 0..n {
                diff += (w[i] - prev[i]).powi(2);
            }
            v = w;
            if diff.sqrt() < TOL {
                break;
            }
        }
        eigvecs.push(v);
    }

    // Pack as columns of an n x k array.
    let mut x = Array2::<f64>::zeros((n, k));
    for (col, vec) in eigvecs.iter().enumerate() {
        for i in 0..n {
            x[[i, col]] = vec[i];
        }
    }
    x
}

/// Gram–Schmidt: remove the components of `v` along each already-found eigenvector.
fn deflate(v: &mut Array1<f64>, basis: &[Array1<f64>]) {
    for u in basis {
        let proj = dot(v, u);
        for i in 0..v.len() {
            v[i] -= proj * u[i];
        }
    }
}

/// Normalize `v` to unit L2 norm (no-op on a zero vector).
fn normalize(v: &mut Array1<f64>) {
    let norm = dot(v, v).sqrt();
    if norm > 1e-300 {
        for i in 0..v.len() {
            v[i] /= norm;
        }
    }
}

/// Flip the sign of `v` so its largest-magnitude component is non-negative
/// (removes the eigenvector sign ambiguity for deterministic comparison).
fn align_sign(v: &mut Array1<f64>) {
    let mut max_abs = 0.0;
    let mut sign = 1.0;
    for i in 0..v.len() {
        if v[i].abs() > max_abs {
            max_abs = v[i].abs();
            sign = if v[i] < 0.0 { -1.0 } else { 1.0 };
        }
    }
    if sign < 0.0 {
        for i in 0..v.len() {
            v[i] = -v[i];
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: PASS — 6 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): communities symmetric eigensolver (power iteration + deflation)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 15: communities — `kmeans` (Forgy init, seeded)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/communities/mod.rs`

**Interfaces:**
- `pub fn kmeans(x: &Array2<f64>, k: usize, max_iter: usize, seed: Option<u64>) -> Vec<usize>`
  — cluster the `n` rows of `x` into `k` groups; returns a cluster index per row. XGI
  `communities/spectral.py:75-136`, with the deviations noted in the header (correct
  per-dimension centroid mean, `k==1 → 0`, empty-cluster reseeding).

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`:

```rust
use hypergraph_rs::communities::kmeans;
use ndarray::Array2;

#[test]
fn test_kmeans_separates_two_clusters() {
    // Two well-separated 1-D point clusters near 0 and near 10.
    let mut x = Array2::<f64>::zeros((6, 1));
    for (i, &v) in [0.0, 0.1, 0.2, 10.0, 10.1, 10.2].iter().enumerate() {
        x[[i, 0]] = v;
    }
    let labels = kmeans(&x, 2, 100, Some(42));
    assert_eq!(labels.len(), 6);
    // The first three share a label, the last three share the other label.
    assert_eq!(labels[0], labels[1]);
    assert_eq!(labels[1], labels[2]);
    assert_eq!(labels[3], labels[4]);
    assert_eq!(labels[4], labels[5]);
    assert_ne!(labels[0], labels[3]);
}

#[test]
fn test_kmeans_k1_all_zero() {
    let x = Array2::<f64>::zeros((4, 2));
    let labels = kmeans(&x, 1, 100, Some(1));
    assert_eq!(labels, vec![0, 0, 0, 0]);
}

#[test]
fn test_kmeans_deterministic_with_seed() {
    let mut x = Array2::<f64>::zeros((6, 1));
    for (i, &v) in [0.0, 0.1, 0.2, 10.0, 10.1, 10.2].iter().enumerate() {
        x[[i, 0]] = v;
    }
    let a = kmeans(&x, 2, 100, Some(5));
    let b = kmeans(&x, 2, 100, Some(5));
    assert_eq!(a, b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: FAIL — `kmeans` doesn't exist.

- [ ] **Step 3: Implement**

Append to `src/communities/mod.rs`:

```rust
/// Squared Euclidean distance between row `i` of `x` and a centroid vector.
fn sq_dist(x: &Array2<f64>, i: usize, centroid: &[f64]) -> f64 {
    let mut s = 0.0;
    for d in 0..centroid.len() {
        let diff = x[[i, d]] - centroid[d];
        s += diff * diff;
    }
    s
}

/// Forgy-initialized k-means over the rows of `x`.
///
/// XGI parity: `_kmeans` (`communities/spectral.py:75-136`), with three deliberate
/// deviations (see the Phase-6 plan header): correct per-dimension centroid mean,
/// `k == 1 → 0`, and empty-cluster reseeding to the farthest point. Deterministic under
/// `seed`. At most `max_iter` update rounds (bounded loop).
///
/// **Requires `k <= x.nrows()`.** `spectral_clustering` guards this at the call site
/// (`SpectralError::TooManyClusters`); calling `kmeans` directly with `k` greater than the
/// row count is a misuse this function does not itself validate (the Forgy-init fallback
/// loop is bounded by row count and simply returns fewer than `k` distinct centroids).
pub fn kmeans(x: &Array2<f64>, k: usize, max_iter: usize, seed: Option<u64>) -> Vec<usize> {
    let n = x.nrows();
    let dim = x.ncols();
    if k <= 1 || n == 0 {
        return vec![0; n];
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed.unwrap_or(0));

    // Forgy init: pick k distinct rows as centroids.
    let mut chosen: Vec<usize> = Vec::with_capacity(k);
    let mut attempts = 0usize;
    while chosen.len() < k && attempts < n * 100 {
        let cand = rng.random_range(0..n);
        if !chosen.contains(&cand) {
            chosen.push(cand);
        }
        attempts += 1;
    }
    // Fallback: if distinct picks failed (degenerate), fill sequentially. Bounded by `n`
    // (Power-of-10 rule 2): `kmeans` requires `k <= n`, so this loop provably terminates
    // once `idx` reaches `n` — the `idx < n` guard is the static bound, not a safety net.
    let mut idx = 0usize;
    while chosen.len() < k && idx < n {
        if !chosen.contains(&idx) {
            chosen.push(idx);
        }
        idx += 1;
    }
    let mut centroids: Vec<Vec<f64>> = chosen
        .iter()
        .map(|&r| (0..dim).map(|d| x[[r, d]]).collect())
        .collect();

    let mut labels = vec![0usize; n];

    for _ in 0..max_iter {
        // Assignment step.
        let mut new_labels = vec![0usize; n];
        for i in 0..n {
            let mut best = 0usize;
            let mut best_d = f64::INFINITY;
            for (c, centroid) in centroids.iter().enumerate() {
                let d = sq_dist(x, i, centroid);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
            new_labels[i] = best;
        }

        // Update step: per-dimension mean; reseed empty clusters to the farthest point.
        let mut next = vec![vec![0.0_f64; dim]; k];
        let mut counts = vec![0usize; k];
        for i in 0..n {
            let c = new_labels[i];
            counts[c] += 1;
            for d in 0..dim {
                next[c][d] += x[[i, d]];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..dim {
                    next[c][d] /= counts[c] as f64;
                }
            } else {
                // Reseed to the point farthest from its currently-assigned centroid.
                let mut far_i = 0usize;
                let mut far_d = -1.0;
                for i in 0..n {
                    let d = sq_dist(x, i, &centroids[new_labels[i]]);
                    if d > far_d {
                        far_d = d;
                        far_i = i;
                    }
                }
                for d in 0..dim {
                    next[c][d] = x[[far_i, d]];
                }
            }
        }
        centroids = next;

        let changed = new_labels != labels;
        labels = new_labels;
        if !changed {
            break;
        }
    }

    labels
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: PASS — 9 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): communities Forgy k-means (seeded, per-dim centroids)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 16: communities — `spectral_clustering`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/communities/mod.rs`

**Interfaces:**
- `pub fn spectral_clustering(h: &Hypergraph, k: usize, max_iter: usize, seed: Option<u64>) -> Result<IndexMap<String, usize>, SpectralError>`
  — the full pipeline (XGI `communities/spectral.py:14-72`): guard `k > n`, build the
  normalized Laplacian, extract the `k` smallest eigenvectors, k-means, map back to node
  ids. Returns an insertion-ordered node-id→cluster map.

- [ ] **Step 1: Write the failing test**

Append to `hypergraph-rs/crates/hypergraph-rs/tests/test_communities.rs`:

```rust
use hypergraph_rs::communities::spectral_clustering;

#[test]
fn test_spectral_two_triangles_separated() {
    let h = two_triangles();
    let clusters = spectral_clustering(&h, 2, 1000, Some(42)).unwrap();
    assert_eq!(clusters.len(), 6);
    // Each triangle is internally coherent; the two triangles differ.
    let ca = clusters["a"];
    assert_eq!(clusters["b"], ca);
    assert_eq!(clusters["c"], ca);
    let cd = clusters["d"];
    assert_eq!(clusters["e"], cd);
    assert_eq!(clusters["f"], cd);
    assert_ne!(ca, cd);
}

#[test]
fn test_spectral_too_many_clusters_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e".into()), serde_json::Value::Null).unwrap();
    assert!(spectral_clustering(&h, 5, 100, Some(1)).is_err());
}

#[test]
fn test_spectral_k1_all_zero() {
    let h = two_triangles();
    let clusters = spectral_clustering(&h, 1, 100, Some(1)).unwrap();
    assert!(clusters.values().all(|&c| c == 0));
}

#[test]
fn test_spectral_deterministic_with_seed() {
    let h = two_triangles();
    let a = spectral_clustering(&h, 2, 1000, Some(3)).unwrap();
    let b = spectral_clustering(&h, 2, 1000, Some(3)).unwrap();
    assert_eq!(a, b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: FAIL — `spectral_clustering` doesn't exist.

- [ ] **Step 3: Implement**

In `src/communities/mod.rs`, add `use indexmap::IndexMap;` to the top imports, then
append:

```rust
/// Spectral clustering of a hypergraph into `k` partitions.
///
/// XGI parity: `xgi.spectral_clustering` (`communities/spectral.py:14-72`). Builds the
/// normalized Laplacian, embeds nodes via the `k` smallest-eigenvalue eigenvectors, and
/// k-means-clusters them. Returns an insertion-ordered node-id→cluster map (clusters
/// begin at 0). Deterministic under `seed`; structural (not exact-XGI) parity (§9.4).
pub fn spectral_clustering(
    h: &Hypergraph,
    k: usize,
    max_iter: usize,
    seed: Option<u64>,
) -> Result<IndexMap<String, usize>, SpectralError> {
    let n = h.num_nodes();
    if k > n {
        return Err(SpectralError::TooManyClusters { k, n });
    }
    let (l, rowdict) = normalized_laplacian(h)?;
    let x = smallest_k_eigenvectors(&l, k, seed);
    let labels = kmeans(&x, k, max_iter, seed);
    let mut out = IndexMap::new();
    for (i, node_id) in rowdict.into_iter().enumerate() {
        out.insert(node_id, labels[i]);
    }
    Ok(out)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features communities --test test_communities`
Expected: PASS — 13 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): communities spectral_clustering pipeline

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 17: clippy + fmt + full-feature suite verification

**Files:**
- Modify: any files with clippy warnings

- [ ] **Step 1: Clippy with all Phase-6 features**

Run: `cd hypergraph-rs && cargo clippy -p hypergraph-rs --features "convert dynamics communities" -- -D warnings`
Expected: may warn; fix all. Likely candidates:
- `clippy::needless_range_loop` in the Laplacian/eigen/k-means loops — these index
  multiple arrays by the same `i`/`j`/`d`, so the index form is clearer; add a scoped
  `#[allow(clippy::needless_range_loop)]` with a one-line justification where it fires.
- `clippy::type_complexity` on `Result<(Array2<f64>, Vec<String>), SpectralError>` — add a
  scoped `#[allow]` if it fires (the tuple is self-explanatory).
- unused imports — remove any left over (e.g., the removed temporary `Array1` stub line).

- [ ] **Step 2: Clippy the whole workspace (features off) — the gated modules compile away**

Run: `cd hypergraph-rs && cargo clippy --workspace -- -D warnings`
Expected: PASS (utils always compiled; convert/dynamics/communities absent).

- [ ] **Step 3: Full test suite, all Phase-6 features on**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --features "convert dynamics communities"`
Expected: PASS — the Phase-0-1 core suite (64) + `test_utils` (13) + `test_convert` (21) +
`test_dynamics` (6) + `test_communities` (13); 64 + 13 + 21 + 6 + 13 = 117 total. Confirm 0
failures across all test targets.

- [ ] **Step 4: Bare workspace test (features off) still green**

Run: `cd hypergraph-rs && cargo test --workspace`
Expected: PASS — gated test files compile to empty (`#![cfg(feature = "…")]`); `test_utils`
runs (13); core suite runs (64). No compile error from the absent modules.

- [ ] **Step 5: rustfmt**

Run: `cd hypergraph-rs && cargo fmt --all`

- [ ] **Step 6: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs): Phase-6 clippy + fmt cleanup

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 6 Completion Summary

After all 17 tasks, the `hypergraph-rs` core crate additionally provides
(feature-gated per spec §10.1):

- **`utils`** (always on): `dual_dict`, `powerset`, `subfaces`, `get_network_type`,
  `convert_labels_to_integers`, `UtilError`. Core gained a `graph_attrs()` getter.
- **`convert`** (`--features convert`): `from_hyperedge_list`/`to_hyperedge_list`,
  `from_hyperedge_dict`/`to_hyperedge_dict`, `from_bipartite_edgelist`/
  `to_bipartite_edgelist`, `to_bipartite_graph`/`from_bipartite_graph` (petgraph +
  `BipartiteNode`), `to_graph` (1-skeleton), `to_hypergraph_dict`/`from_hypergraph_dict`,
  `cut_to_order`, `ConvertError`.
- **`dynamics`** (`--features dynamics`): `simulate_kuramoto`,
  `compute_kuramoto_order_parameter` (**Kuramoto synchronization — NOT epidemics**).
- **`communities`** (`--features communities`): `normalized_laplacian`,
  `smallest_k_eigenvectors` (power iteration + deflation, no LAPACK),
  `kmeans`, `spectral_clustering`, `SpectralError`.
- **Dependency line corrected** to the probe-blessed `rand 0.10.2` / `rand_pcg 0.10.2` /
  `rand_core 0.10.1` / `ndarray 0.17`; feature flags `convert`/`dynamics`/`communities`.
- **53 new tests** (13 utils + 21 convert + 6 dynamics + 13 communities), all passing, on
  top of the Phase-0-1 core suite (64) — 117 total. Clippy + rustfmt clean.

**Deferred to later phases** (dependencies land in Phases 2/3, or explicitly out of scope):
`simulate_simplicial_kuramoto` + `compute_simplicial_order_parameter` (Hodge boundary
matrix + `SimplicialComplex`); `convert`'s `hif_dict`, `encapsulation_dag`, `line_graph`,
`simplex`, `incidence`, `to_hypergraph`/`to_dihypergraph`/`to_simplicial_complex`
dispatchers (need Phase-2 Di/SC or Phase-3 linalg), and the `pandas` slice (never ported,
spec §4.4); `utils` `tensor.py`/`trie.py`/network/generator helpers.

**Conformance gate (Phase 7):** the ported XGI test files
(`tests/communities/test_spectral.py`, `tests/dynamics/*`, `tests/convert/*`,
`tests/utils/test_utilities.py`) run via PyO3 with `import hypergraph_rs as xgi`. Generator
and clustering tests use the §9.4 split (structural properties, not exact numpy
trajectories); this plan's Rust tests already assert those structural invariants.
