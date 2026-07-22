# hypergraph-rs Phase 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `generators/`, `stats/`, and `readwrite/` modules of `hypergraph-rs` — porting XGI 0.10.2's `xgi/generators/*.py`, `xgi/stats/*.py`, and `xgi/readwrite/*.py` faithfully, with seeded deterministic RNG (`rand_pcg::Pcg64Mcg`), a pure-data stat dispatch surface, and serde-based read/write for JSON, edgelist, bipartite edgelist, incidence-matrix, HIF, and BIGG.

**Depends on:** Phase 0+1 (core `Hypergraph<N,E,M>` — landed, API is ground truth) and Phase 2/3 (`DiHypergraph`, `SimplicialComplex`, `linalg::incidence_matrix`). The directed-stat and simplicial-generator tasks name the Phase-2 types they consume; their exact method signatures are called out under **Open questions** and MUST be reconciled against the landed Phase-2 code before implementation (do not invent a Phase-2 API — grep the crate first).

**Tech Stack:** Rust 1.85+ (edition 2021), `rustworkx-core` 0.18, `indexmap` 2, `serde` + `serde_json` 1, `thiserror` 2, plus **new consumers for Phase 4:** `rand_pcg` 0.10.2 + `rand_core` 0.10.1 (seeded `Pcg64Mcg`) — the workspace pins themselves are corrected by Phase 2 (see Task 1, Step 1); Phase 4 only adds them as optional deps of this crate. BSD-3-Clause.

## Global Constraints

- **Determinism (III.7 parity):** every generator that samples randomness takes an explicit `seed: Option<u64>` and builds its RNG via `rand_pcg::Pcg64Mcg` seeded from that. No global/thread RNG, no `getrandom`, no `HashMap` iteration in any public-facing order. All id-keyed maps are `IndexMap` (insertion-ordered) so generator output ordering is reproducible bit-for-bit given a seed.
- **Conformance philosophy (spec §9.4):** XGI seeds `numpy.random`; we seed `Pcg64Mcg`. Same seed does NOT reproduce numpy's sequence, so generator tests assert **structural properties** (node count, edge count, uniformity, order distribution, determinism-under-fixed-seed), never exact membership vs numpy. Do not attempt to match numpy byte-for-byte.
- **Bounded loops (Power-of-10 rule 2):** every generator `while` loop advances a strictly-increasing index bounded by a precomputed `max_index` (combinatorial count). The `geometric()` sampler is clamped to return `>= 1` (see Task 2) so these loops cannot stall — XGI's own `ceil(log r / log(1-p))` already returns `>= 1` for any `p` in `(0,1)` (it never returns 0 there), so the clamp is a defensive restatement of that invariant (and a guard for the `p<=0`/`p==1` sentinel paths), not a divergence from XGI.
- **No `unsafe`.** MyPy-strict-equivalent: explicit return types, no `unwrap()` on fallible IO in `readwrite` (return `ReadWriteError`); `unwrap()` is allowed only in tests and on infallible internal invariants with a comment.
- **TDD discipline:** every task is red → green → commit. Integration tests live in **direct children** of `crates/hypergraph-rs/tests/` (one file per module family: `test_generators.rs`, `test_stats.rs`, `test_readwrite.rs`). Run a single file with `--test <stem>` (e.g. `--test test_generators`).
- **Feature-gated:** the three new modules are behind the `generators`, `stats`, `readwrite` feature flags (all in `default`). `rand_pcg`/`rand_core` are optional deps enabled by `generators`. Network fetching (`load_bigg_data` HTTP, `load_xgi_data`) is behind `network` (OFF by default) — offline BIGG *parsing* is always available.
- **Commit convention:** Conventional commits, `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer. `cd hypergraph-rs && git add -A && git commit`.
- **Generic defaults:** all generators return `Hypergraph<serde_json::Value, serde_json::Value, serde_json::Value>` — attrs default to `serde_json::Value::Null` (which is `Value::default()`), matching XGI's attribute-free generator output. Node ids are the decimal string of the integer index (`"0"`, `"1"`, …).

## Deliberate deviations from XGI (Phase-4-specific; in addition to the Phase 0+1 deviations)

1. **RNG family.** XGI uses `numpy.random.default_rng`; we use `rand_pcg::Pcg64Mcg`. Determinism is per-call via `seed`, structural-property parity only (spec §9.4). *(Not a bug — an unavoidable cross-language RNG difference, blessed by the spec.)*
2. **`geometric()` clamped to `>= 1`** (Task 2). XGI's `ceil(log(r)/log(1-p))` already returns `>= 1` for any `p` in `(0,1)` — `r` is drawn from `[0,1)` so `log(r) < 0` and `log(1-p) < 0` for `p` in `(0,1)`, making the ratio strictly positive and its ceiling `>= 1`; XGI never returns 0 for a valid `p` in `(0,1)`. Our Rust clamp to `max(1)` is therefore a defensive restatement of that invariant (it also guards the `p<=0`/`p==1` sentinel paths), not a fix for a real XGI 0-return case; it keeps the `index += geometric(...)` loops in the random/uniform generators strictly advancing (Power-of-10 rule 2). Verified against `xgi/utils/utilities.py:564`.
3. **Empty / duplicate edges.** Our core `add_edge` returns `Err(EmptyMembers)` / `Err(AlreadyExists)` (Phase 0+1 deviation). Generators construct only non-empty edges with auto-ids, so they never hit these; where a generator could produce a duplicate member-set, it is added with `idx=None` (a distinct auto-id → a legitimate parallel edge, matching XGI's set-of-edges-with-distinct-ids semantics).
4. **Incidence-matrix file format.** XGI writes via `numpy.savetxt` (default `%.18e`). Our `write_incidence_matrix` emits integer `0`/`1` cells joined by the delimiter; `read_incidence_matrix` parses any numeric cell as truthy-if-nonzero. Our writer+reader round-trip with each other (the conformance requirement); we do NOT promise byte-identical files to numpy's `%.18e`. Documented on both functions.
5. **HSBM / HPPM uniform generators deferred.** `uniform_HSBM` / `uniform_HPPM` require an m-dimensional probability *tensor* input; without an `ndarray` tensor type in the Phase-4 surface they are out of scope here (see Open questions). `uniform_erdos_renyi_hypergraph` and `uniform_hypergraph_configuration_model` — the two Babylon-relevant uniform generators — ARE ported.
6. **`flag_complex` family deferred.** `flag_complex`, `flag_complex_d2`, `random_flag_complex*` consume a *NetworkX graph* and use `nx.enumerate_all_cliques`; those belong to the `convert` slice (Phase 6). Phase 4 ports `random_simplicial_complex` (the pure-hypergraph simplicial generator).
7. **Degree-filtered `edge_order`/`edge_size` clamp at 0, not XGI's possible `-1`.** XGI's `order(degree)` computes `sum(node_degree == degree for n in members) - 1`, which is `-1` when zero members match `degree`. Our `Stat<usize>` cannot represent a negative order, so the Task 11 degree-filtered `edge_order` (and, structurally, `edge_size`) `saturating_sub(1)`, clamping the result at `0` instead of XGI's `-1`. Pinned by `test_edge_order_degree_filter_zero_matches_clamps_at_zero` (Task 11).

**Phase 4 review fixes (P4-REV, post-implementation adversarial review):**

- **Finding 1 (blocker, fixed).** `fast_random_hypergraph`/`uniform_erdos_renyi_hypergraph` panicked (`attempt to multiply with overflow` in `rng::comb`) whenever a requested edge size exceeded `n`: `comb(n, size).saturating_sub(1)` produced `max_index = 0` instead of XGI's signed `-1` (`comb(..., exact=True) - 1`), so the geometric-skip loop ran once on an out-of-range index and drove `index_to_edge_comb` into a huge combinatorial argument. Fixed by skipping the order/size entirely when the raw combination count is `0` (mirroring XGI's `while index <= -1` never running). Verified against the oracle: `fast_random_hypergraph(3, [0.9], order=[10], seed=1)` and `uniform_erdos_renyi_hypergraph(3, 5, 0.9, seed=1)` both yield 3 nodes, 0 edges in XGI 0.10.2.
- **Finding 2 (major, fixed).** `rng::comb`'s `u64` multiplicative accumulator (`acc * (n - i)` before dividing by `(i + 1)`) could overflow mid-loop even when the final coefficient fits in `u64` (e.g. `comb(100, 16)`, `comb(100, 17)`). Switched the accumulator to `u128` with `checked_mul`, saturating to `u64::MAX` on overflow (documented inline: at that magnitude the geometric-skip callers would never exhaustively enumerate anyway, matching the spirit of XGI's exact-bigint `scipy.special.comb`). `index_to_edge_comb`'s internal `comb` calls inherit the fix transitively (its own `i128` index arithmetic was audited and found safe given Finding 1's index-bound guard). Oracle: `comb(100,15) == 253338471349988640`, `comb(100,16) == 1345860629046814650`, `comb(100,17) == 6650134872937201800` (all fit `u64`); `comb(100,18) == 30664510802988208300` exceeds `u64::MAX` and now saturates instead of panicking.
- **Finding 3 (minor, fixed).** `bigg_dict_to_dihypergraph` passed `Value::Null` as every reaction edge's attrs; XGI's `_bigg_to_dihypergraph` sets `name=r["name"]` on every `DH.add_edge(...)` call (forward, reverse, and both reversible edges, `bigg_data.py:129,142,156,159`). Fixed by building a `{"name": ...}` attrs object once per reaction and passing it to all four `add_edge` sites. Required adding `DiHypergraph::edge_attrs` (mirroring the existing `Hypergraph::edge_attrs`) since no edge-attr reader existed to pin the fix with a test.
- **Finding 4 (advisory, partially fixed after oracle verification).** (a) **Not applied** — the finding assumed `to_hypergraph_dict` always emits a default `"name": ""`; empirically probing the oracle (`xgi.convert.to_hypergraph_dict` and `write_json` on a fresh `Hypergraph()`) shows `hypergraph-data` is `{}` when unset. The `to_hypergraph_dict` docstring comment ("name always gets written (default is an empty string)") is stale relative to the actual 0.10.2 code (`data["hypergraph-data"] = {}; data["hypergraph-data"].update(H._net_attr)`, and `_net_attr` starts as `{}`). Pinned by `test_to_hypergraph_dict_has_no_default_name_when_unset` documenting the verified (non-)behavior. (b) **Fixed** — member lists were always sorted lexicographically (`"10" < "2"`); XGI sorts by the members' native Python type (numeric for int ids, lexicographic for str ids — oracle-verified both ways, including a mixed-id edge sorting lexicographically). Since our node ids are always `String`, `to_hypergraph_dict` now sorts numerically when every member of an edge parses as an integer, else lexicographically.
- **Finding 5 (advisory, documentation only).** No code changes. `generators/simple.rs::sunflower` now has an inline note: XGI's `sunflower(l, c, m)` with `m == c` infinite-loops (`while c <= c` never advances `start_label`, `simple.py:114`); our bounded `for p in 0..l` instead terminates after `l` core-only petals — a deliberate Power-of-10-driven improvement. `generators/classic.rs::complete_hypergraph` now has an inline note: XGI's `complete_hypergraph(n, None, max_order=0, include_singletons=False)` raises `AssertionError` (`end >= start` fails, `classic.py:264`, oracle-verified); our Rust range `start..=(max_order+1)` is simply empty in that case, so we return an empty hypergraph instead of erroring.

---

## File Structure (added/modified this phase)

```
hypergraph-rs/
├── Cargo.toml                                    # MODIFY: workspace deps (rand_pcg/rand_core)
├── crates/hypergraph-rs/
│   ├── Cargo.toml                                # MODIFY: [features] + optional deps
│   ├── src/
│   │   ├── lib.rs                                # MODIFY: feature-gated pub mod + re-exports
│   │   ├── generators/
│   │   │   ├── mod.rs                            # re-exports
│   │   │   ├── rng.rs                            # seeded Pcg64Mcg + samplers + combinatorics
│   │   │   ├── classic.rs                        # trivial, complete, complement, empty
│   │   │   ├── lattice.rs                        # ring_lattice
│   │   │   ├── simple.rs                         # star_clique, sunflower
│   │   │   ├── random.rs                         # fast_random, random, chung_lu, watts_strogatz
│   │   │   ├── uniform.rs                        # erdos_renyi, configuration_model
│   │   │   └── simplicial.rs                     # random_simplicial_complex
│   │   ├── stats/
│   │   │   ├── mod.rs                            # Stat<T> dispatch surface + re-exports
│   │   │   ├── nodestats.rs                      # degree, attrs
│   │   │   ├── edgestats.rs                      # order, size, attrs
│   │   │   ├── dinodestats.rs                    # degree, in_degree, out_degree
│   │   │   └── diedgestats.rs                    # order, size, head/tail order/size
│   │   └── readwrite/
│   │       ├── mod.rs                            # ReadWriteError + re-exports
│   │       ├── json.rs                           # to/from_hypergraph_dict, write/read_json
│   │       ├── edgelist.rs                       # write/read/parse_edgelist
│   │       ├── bipartite.rs                      # write/read/parse_bipartite_edgelist
│   │       ├── incidence.rs                      # write/read_incidence_matrix
│   │       ├── hif.rs                            # to/from_hif_dict, write/read_hif
│   │       └── bigg.rs                           # bigg_dict_to_dihypergraph (offline)
│   └── tests/
│       ├── test_generators.rs                    # NEW
│       ├── test_stats.rs                         # NEW
│       └── test_readwrite.rs                     # NEW
```

---

## Task 1: Feature flags, workspace deps, and module scaffolding

**Files:**
- Modify: `hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/generators/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/stats/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/mod.rs`

- [ ] **Step 1: Verify the workspace dependency pins (Phase 2 owns that correction, not Phase 4)**

Phase 2 runs before Phase 4 in the execution order and is the SOLE owner of the workspace-manifest correction: its earliest task needing randomness edits `[workspace.dependencies]` in `hypergraph-rs/Cargo.toml` from the Phase 0+1 plan's speculative `rand = "0.8"` / `rand_pcg = "0.3"` (declared, never consumed) to the probe-blessed `rand = "0.10.2"`, `rand_pcg = "0.10.2"`, `rand_core = "0.10.1"` (matching rustworkx-core 0.18's resolution), then wires `rand.workspace`/`rand_pcg.workspace`/`rand_core.workspace` into the core crate. **Phase 4 must NOT re-edit those workspace lines** — verify they're already correct instead:

```bash
rg '^rand(_pcg|_core)? = ' hypergraph-rs/Cargo.toml
```

Expected: `rand = "0.10.2"`, `rand_pcg = "0.10.2"`, `rand_core = "0.10.1"` all present under `[workspace.dependencies]`. If they are missing or still read `0.8`/`0.3`, STOP — Phase 2 has not landed its correction yet, and Phase 4 depends on it (do not duplicate the fix here).

- [ ] **Step 2: Add feature flags + optional deps to this crate**

In `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`, add a `[features]` section and make the RNG deps optional. If Phase 2 already added unconditional `rand_pcg`/`rand_core` lines to this crate's `[dependencies]` (grep first: `rg 'rand_(pcg|core)' crates/hypergraph-rs/Cargo.toml`), add `optional = true` to those existing lines instead of re-declaring them; otherwise add the lines below as written. The full `[dependencies]` + `[features]` become:

```toml
[dependencies]
rustworkx-core.workspace = true
indexmap.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
rand_pcg = { workspace = true, optional = true }
rand_core = { workspace = true, optional = true }

[features]
default = ["generators", "stats", "readwrite"]
generators = ["dep:rand_pcg", "dep:rand_core"]
stats = []
readwrite = []
network = []   # HTTP dataset fetch (load_bigg_data / load_xgi_data) — OFF by default; offline parsing is always available
```

Note: `serde`/`serde_json` are unconditional (readwrite and the core's `serde_json::Value` attrs need them). Only the RNG deps are optional.

- [ ] **Step 3: Wire the new modules into `lib.rs`**

Append to `hypergraph-rs/crates/hypergraph-rs/src/lib.rs` (after the existing `pub use core::...` lines):

```rust
#[cfg(feature = "generators")]
pub mod generators;
#[cfg(feature = "stats")]
pub mod stats;
#[cfg(feature = "readwrite")]
pub mod readwrite;
```

- [ ] **Step 4: Create empty module roots**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/mod.rs`:

```rust
//! Hypergraph generators (XGI `xgi/generators/*.py` port).
//!
//! Every randomized generator takes an explicit `seed: Option<u64>` and builds
//! a `rand_pcg::Pcg64Mcg` from it. Determinism is per-call (III.7 parity); the
//! conformance gate asserts structural properties, not numpy-sequence parity.

pub mod rng;
pub mod classic;
pub mod lattice;
pub mod simple;
pub mod random;
pub mod uniform;
pub mod simplicial;

pub use classic::{complement, complete_hypergraph, empty_hypergraph, trivial_hypergraph};
pub use lattice::ring_lattice;
pub use simple::{star_clique, sunflower};
pub use random::{
    chung_lu_hypergraph, fast_random_hypergraph, random_hypergraph, watts_strogatz_hypergraph,
};
pub use uniform::{uniform_erdos_renyi_hypergraph, uniform_hypergraph_configuration_model};
pub use simplicial::random_simplicial_complex;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/stats/mod.rs`:

```rust
//! Node/edge statistics (XGI `xgi/stats/*.py` port).
//!
//! XGI wraps stats in lazy proxy objects (`IDStat`) with `.asdict()`/`.aslist()`/
//! `.filterby()`. We compute eagerly into an insertion-ordered [`Stat`] value that
//! offers the same terminal operations as pure data.

pub mod nodestats;
pub mod edgestats;
pub mod dinodestats;
pub mod diedgestats;

use indexmap::IndexMap;

/// An id → quantity mapping, insertion-ordered (mirrors XGI's `IDStat` result dict).
///
/// `T` is the stat's value type (`usize` for degree/order/size, `serde_json::Value`
/// for attribute stats). Terminal ops mirror XGI's `as*` / `filterby` surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Stat<T> {
    inner: IndexMap<String, T>,
}

impl<T> Stat<T> {
    /// Construct from an insertion-ordered map (internal to the stat functions).
    pub fn new(inner: IndexMap<String, T>) -> Self {
        Self { inner }
    }

    /// XGI `.asdict()` — borrow the id → value map.
    pub fn asdict(&self) -> &IndexMap<String, T> {
        &self.inner
    }

    /// XGI `.aslist()` — values in id insertion order.
    pub fn aslist(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.inner.values().cloned().collect()
    }

    /// Number of ids in the mapping (XGI `len(stat)`).
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Value for one id (XGI `stat[idx]`), if present.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.inner.get(id)
    }
}

impl<T: PartialEq> Stat<T> {
    /// XGI `.filterby('stat', value)` — ids whose value equals `value`, in order.
    pub fn filterby(&self, value: &T) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(_, v)| *v == value)
            .map(|(k, _)| k.clone())
            .collect()
    }
}

impl<T: PartialOrd> Stat<T> {
    /// Ids whose value is strictly greater than `value` (XGI `filterby(..., mode="gt")`).
    pub fn filterby_gt(&self, value: &T) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(_, v)| *v > value)
            .map(|(k, _)| k.clone())
            .collect()
    }
}
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/mod.rs`:

```rust
//! Read/write hypergraphs in various formats (XGI `xgi/readwrite/*.py` port).

pub mod json;
pub mod edgelist;
pub mod bipartite;
pub mod incidence;
pub mod hif;
pub mod bigg;

use thiserror::Error;

/// Error raised by a read/write operation.
#[derive(Debug, Error)]
pub enum ReadWriteError {
    /// The underlying file could not be read or written.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The bytes were not valid JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The data was structurally invalid for the target format.
    #[error("format error: {0}")]
    Format(String),
}

// TASK 13: uncomment when json.rs lands
// pub use json::{from_hypergraph_dict, read_json, to_hypergraph_dict, write_json};
// TASK 14: uncomment when edgelist.rs lands
// pub use edgelist::{parse_edgelist, read_edgelist, write_edgelist};
// TASK 15: uncomment when bipartite.rs lands
// pub use bipartite::{parse_bipartite_edgelist, read_bipartite_edgelist, write_bipartite_edgelist};
// TASK 16: uncomment when incidence.rs lands
// pub use incidence::{read_incidence_matrix, write_incidence_matrix};
// TASK 17: uncomment when hif.rs lands
// pub use hif::{from_hif_dict, read_hif, to_hif_dict, write_hif};
```

Note: `bigg` re-exports are added in Task 18 (they depend on `DiHypergraph`); leaving `pub mod bigg;` here with an empty file now would fail to compile, so create `bigg.rs` as a one-line placeholder in this step:

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/bigg.rs`:

```rust
//! BIGG metabolic-network parsing (XGI `readwrite/bigg_data.py` port). Filled in Task 18.
```

And create placeholder files for every module named above so the workspace compiles after this task (each is filled by a later task). Create these one-line placeholders now:

- `generators/rng.rs`, `generators/classic.rs`, `generators/lattice.rs`, `generators/simple.rs`, `generators/random.rs`, `generators/uniform.rs`, `generators/simplicial.rs`
- `stats/nodestats.rs`, `stats/edgestats.rs`, `stats/dinodestats.rs`, `stats/diedgestats.rs`
- `readwrite/json.rs`, `readwrite/edgelist.rs`, `readwrite/bipartite.rs`, `readwrite/incidence.rs`, `readwrite/hif.rs`

Because `mod.rs` re-exports concrete symbols from these files, a bare placeholder will not compile. Instead, for Task 1 **only** declare the submodules and defer the `pub use` re-export lines: comment out every `pub use` block in the three `mod.rs` files with a `// TASK N: uncomment when <file> lands` marker, and uncomment each as its task completes. Each placeholder file gets just its doc-comment header. This keeps `cargo build` green at every step.

- [ ] **Step 5: Verify the workspace builds with the new (empty) module tree**

Run: `cd hypergraph-rs && cargo build`
Expected: builds successfully (empty modules, re-exports commented out). Warnings OK.

- [ ] **Step 6: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): scaffold generators/stats/readwrite modules + feature flags

Adds [features] (generators/stats/readwrite/network), consumes the
Phase-2-pinned rand_pcg 0.10.2 + rand_core 0.10.1 as optional deps behind
the generators feature, and the empty module tree with the Stat<T> dispatch
surface + ReadWriteError.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: RNG foundation — seeded Pcg64Mcg, samplers, combinatorics

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/rng.rs`

**Interfaces:** `seeded_rng(seed: Option<u64>) -> Pcg64Mcg`, `uniform_f64(&mut Pcg64Mcg) -> f64` (in `[0,1)`), `random_below(&mut Pcg64Mcg, n: u64) -> u64` (in `[0,n)`), `choice_without_replacement(&mut Pcg64Mcg, pool_len, k) -> Vec<usize>`, `geometric(p: f64, &mut Pcg64Mcg) -> u64` (clamped `>= 1`), `comb(n: u64, k: u64) -> u64`, `index_to_edge_comb(index, n, m) -> Vec<usize>`, `index_to_edge_prod(index, n, m) -> Vec<usize>`.

**XGI source:** `geometric` at `xgi/utils/utilities.py:564` (`ceil(log(r)/log(1-p))`, `p==1 → 1`, `p==0 → inf`); `_index_to_edge_comb` at `xgi/generators/uniform.py:457`; `_index_to_edge_prod` at `uniform.py:405`.

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::rng::{
    choice_without_replacement, comb, geometric, index_to_edge_comb, index_to_edge_prod,
    random_below, seeded_rng, uniform_f64,
};

#[test]
fn test_seeded_rng_is_deterministic() {
    let mut a = seeded_rng(Some(42));
    let mut b = seeded_rng(Some(42));
    let seq_a: Vec<f64> = (0..10).map(|_| uniform_f64(&mut a)).collect();
    let seq_b: Vec<f64> = (0..10).map(|_| uniform_f64(&mut b)).collect();
    assert_eq!(seq_a, seq_b);
}

#[test]
fn test_different_seeds_diverge() {
    let mut a = seeded_rng(Some(1));
    let mut b = seeded_rng(Some(2));
    let va: Vec<f64> = (0..10).map(|_| uniform_f64(&mut a)).collect();
    let vb: Vec<f64> = (0..10).map(|_| uniform_f64(&mut b)).collect();
    assert!(va != vb);
}

#[test]
fn test_uniform_f64_in_unit_interval() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        let x = uniform_f64(&mut r);
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn test_random_below_in_range() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        let x = random_below(&mut r, 5);
        assert!(x < 5);
    }
}

#[test]
fn test_choice_without_replacement_distinct() {
    let mut r = seeded_rng(Some(7));
    let picks = choice_without_replacement(&mut r, 10, 4);
    assert_eq!(picks.len(), 4);
    let mut sorted = picks.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 4, "picks must be distinct");
    assert!(picks.iter().all(|&i| i < 10));
}

#[test]
fn test_geometric_is_at_least_one() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        assert!(geometric(0.3, &mut r) >= 1);
    }
    // p == 1 always yields exactly 1
    assert_eq!(geometric(1.0, &mut r), 1);
}

#[test]
fn test_comb_values() {
    assert_eq!(comb(4, 3), 4);
    assert_eq!(comb(5, 2), 10);
    assert_eq!(comb(10, 0), 1);
    assert_eq!(comb(3, 5), 0);
    assert_eq!(comb(52, 5), 2_598_960);
}

#[test]
fn test_index_to_edge_comb_is_bijective_and_sorted() {
    // For n=4, m=3 there are comb(4,3)=4 combinations, indices 0..4.
    let all: Vec<Vec<usize>> = (0..4).map(|i| index_to_edge_comb(i, 4, 3)).collect();
    // each is strictly increasing (a valid combination)
    for e in &all {
        assert_eq!(e.len(), 3);
        assert!(e.windows(2).all(|w| w[0] < w[1]));
        assert!(e.iter().all(|&x| x < 4));
    }
    // all distinct
    let mut set = all.clone();
    set.sort();
    set.dedup();
    assert_eq!(set.len(), 4);
}

#[test]
fn test_index_to_edge_prod_shape() {
    // n=4, m=3: index 3 -> (0,0,3) per the XGI docstring example.
    assert_eq!(index_to_edge_prod(3, 4, 3), vec![0, 0, 3]);
    assert_eq!(index_to_edge_prod(0, 4, 3), vec![0, 0, 0]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL — `rng` symbols don't exist (compile error).

- [ ] **Step 3: Implement `rng.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/rng.rs`:

```rust
//! Seeded RNG + sampling + combinatorics helpers for the generators.
//!
//! We drive `rand_pcg::Pcg64Mcg` through the version-stable `rand_core` traits
//! (`SeedableRng::seed_from_u64` + `RngCore::next_u64`) and derive all sampling
//! primitives ourselves. This sidesteps rand 0.10's `Rng`/`RngExt` trait split
//! (the probe showed `random_range` moved to `RngExt`) and guarantees a fixed,
//! documented f64 construction for cross-build reproducibility.

use rand_core::{RngCore, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Build a `Pcg64Mcg` from an optional seed. `None` uses a fixed default seed so
/// that "unseeded" callers are STILL deterministic within a process/build
/// (III.7: non-determinism is a bug — we never reach for entropy).
pub fn seeded_rng(seed: Option<u64>) -> Pcg64Mcg {
    Pcg64Mcg::seed_from_u64(seed.unwrap_or(0))
}

/// A uniform f64 in `[0, 1)` from 53 bits of the generator (IEEE-754 double
/// mantissa). Fixed construction: `(next_u64() >> 11) / 2^53`.
pub fn uniform_f64(rng: &mut Pcg64Mcg) -> f64 {
    let bits = rng.next_u64() >> 11;
    (bits as f64) * (1.0f64 / ((1u64 << 53) as f64))
}

/// A uniform integer in `[0, n)`. Panics via debug-assert if `n == 0`.
/// Uses modulo reduction (deterministic; slight low-bit bias is acceptable —
/// generator conformance is structural, not sequence-exact).
pub fn random_below(rng: &mut Pcg64Mcg, n: u64) -> u64 {
    debug_assert!(n > 0, "random_below requires n > 0");
    rng.next_u64() % n
}

/// Sample `k` distinct indices from `0..pool_len` (partial Fisher–Yates).
/// Returns them in draw order. Requires `k <= pool_len`.
pub fn choice_without_replacement(rng: &mut Pcg64Mcg, pool_len: usize, k: usize) -> Vec<usize> {
    debug_assert!(k <= pool_len, "cannot draw {k} distinct from {pool_len}");
    let mut idx: Vec<usize> = (0..pool_len).collect();
    let mut out = Vec::with_capacity(k);
    // Bounded loop: exactly k iterations.
    for i in 0..k {
        let remaining = pool_len - i;
        let j = i + (random_below(rng, remaining as u64) as usize);
        idx.swap(i, j);
        out.push(idx[i]);
    }
    out
}

/// The geometric sampler (number of trials to first success), clamped to `>= 1`.
///
/// XGI: `ceil(log(r) / log(1 - p))`, `p == 1 -> 1`, `p == 0 -> inf`
/// (`xgi/utils/utilities.py:564`). We clamp to `max(1)` so the `index += geometric`
/// loops in the random/uniform generators strictly advance (Power-of-10 rule 2).
pub fn geometric(p: f64, rng: &mut Pcg64Mcg) -> u64 {
    if p >= 1.0 {
        return 1;
    }
    if p <= 0.0 {
        return u64::MAX; // "infinity" sentinel; callers guard p > 0
    }
    let mut r = uniform_f64(rng);
    if r <= 0.0 {
        r = f64::MIN_POSITIVE; // avoid log(0)
    }
    let g = (r.ln() / (1.0 - p).ln()).ceil();
    let g = if g.is_finite() && g >= 1.0 { g as u64 } else { 1 };
    g.max(1)
}

/// Exact binomial coefficient `C(n, k)` (0 if `k > n`). Multiplicative form,
/// integer-exact because the running product of `k` consecutive integers is
/// divisible by `k!` at each step.
pub fn comb(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut acc: u64 = 1;
    // Bounded loop: exactly k iterations.
    for i in 0..k {
        acc = acc / 1 * (n - i) / (i + 1);
    }
    acc
}

/// Map a linear index to the `m`-combination of `0..n` at that index (sorted,
/// strictly increasing). Port of `_index_to_edge_comb` (`uniform.py:457`).
pub fn index_to_edge_comb(index: u64, n: u64, m: u64) -> Vec<usize> {
    let mut c: Vec<usize> = Vec::with_capacity(m as usize);
    let mut r = index as i128 + 1; // 1-indexed internally
    let mut j: i128 = -1;
    // Bounded loop: exactly m iterations.
    for s in 1..=m {
        let mut cs = j + 1;
        // Bounded: cs climbs within 0..n, at most n steps.
        loop {
            let take = comb((n as i128 - 1 - cs) as u64, (m - s) as u64) as i128;
            if r - take > 0 {
                r -= take;
                cs += 1;
            } else {
                break;
            }
        }
        c.push(cs as usize);
        j = cs;
    }
    c
}

/// Map a linear index to an `m`-tuple over `0..n` with repetition (product order,
/// last coordinate fastest). Port of `_index_to_edge_prod` (`uniform.py:405`).
pub fn index_to_edge_prod(index: u64, n: u64, m: u64) -> Vec<usize> {
    let mut out = Vec::with_capacity(m as usize);
    // Bounded loop: exactly m iterations.
    for r in (0..m).rev() {
        let place = n.pow(r as u32);
        out.push(((index / place) % n) as usize);
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 9 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): generators RNG foundation (seeded Pcg64Mcg, samplers, combinatorics)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: Classic generators — empty, trivial, complete, complement

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/classic.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/mod.rs` (uncomment `classic` re-exports)

**Interfaces:** `empty_hypergraph() -> Hypergraph`, `trivial_hypergraph(n: usize) -> Hypergraph`, `complete_hypergraph(n: usize, order: Option<usize>, max_order: Option<usize>, include_singletons: bool) -> Result<Hypergraph, GeneratorError>`, `complement(h: &Hypergraph) -> Hypergraph`.

**XGI source:** `classic.py` — `empty_hypergraph` (:43), `trivial_hypergraph` (:160, `n` nodes 0 edges), `complete_hypergraph` (:205, exactly one of `order`/`max_order`; `order` → all `(order+1)`-subsets; `max_order` → all subsets of size `start..=max_order+1`, `start=1` if singletons else `2`), `complement` (:274, all possible edges up to `max_edge_order(H)+1` minus existing).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::{
    complement, complete_hypergraph, empty_hypergraph, trivial_hypergraph,
};
use hypergraph_rs::Hypergraph;

#[test]
fn test_empty_hypergraph() {
    let h = empty_hypergraph();
    assert_eq!(h.num_nodes(), 0);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_trivial_hypergraph() {
    let h = trivial_hypergraph(5);
    assert_eq!(h.num_nodes(), 5);
    assert_eq!(h.num_edges(), 0);
    assert!(h.has_node("0"));
    assert!(h.has_node("4"));
}

#[test]
fn test_complete_hypergraph_single_order() {
    // order=2 -> all 3-subsets of 4 nodes = comb(4,3) = 4 edges
    let h = complete_hypergraph(4, Some(2), None, false).unwrap();
    assert_eq!(h.num_nodes(), 4);
    assert_eq!(h.num_edges(), 4);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_complete_hypergraph_max_order_no_singletons() {
    // max_order=2, no singletons -> subsets of size 2 and 3 of 4 nodes
    // = comb(4,2)+comb(4,3) = 6 + 4 = 10 edges
    let h = complete_hypergraph(4, None, Some(2), false).unwrap();
    assert_eq!(h.num_edges(), 10);
}

#[test]
fn test_complete_hypergraph_max_order_with_singletons() {
    // sizes 1,2,3 of 4 nodes = 4 + 6 + 4 = 14
    let h = complete_hypergraph(4, None, Some(2), true).unwrap();
    assert_eq!(h.num_edges(), 14);
}

#[test]
fn test_complete_hypergraph_rejects_both_none() {
    assert!(complete_hypergraph(4, None, None, false).is_err());
}

#[test]
fn test_complete_hypergraph_rejects_both_some() {
    assert!(complete_hypergraph(4, Some(2), Some(3), false).is_err());
}

#[test]
fn test_complement_disjoint_edge_sets() {
    // H on 4 nodes with one 3-edge {0,1,2}; complement (max size 3) contains
    // every subset of size 1..=3 except {0,1,2}.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_nodes_from((0..4).map(|i| (i.to_string(), serde_json::Value::Null)));
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let hc = complement(&h);
    assert_eq!(hc.num_nodes(), 4);
    // no edge in hc has the same member-set as {0,1,2}
    let has_012 = hc.edge_ids().iter().any(|e| {
        let mut m = hc.members(e).unwrap();
        m.sort();
        m == vec!["0".to_string(), "1".to_string(), "2".to_string()]
    });
    assert!(!has_012, "complement must exclude the original edge");
    assert!(hc.num_edges() > 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL — `classic` functions don't exist.

- [ ] **Step 3: Implement `classic.rs` and add a shared `GeneratorError`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/classic.rs`:

```rust
//! Classic hypergraph generators (XGI `generators/classic.py`).

use serde_json::Value;
use thiserror::Error;

use crate::Hypergraph;

/// Error raised by a generator given invalid parameters.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum GeneratorError {
    /// A parameter was outside its valid range.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}

/// The empty hypergraph (0 nodes, 0 edges). XGI `empty_hypergraph()`.
pub fn empty_hypergraph() -> Hypergraph {
    Hypergraph::new()
}

/// A hypergraph with `n` nodes ("0".."n-1") and no edges. XGI `trivial_hypergraph(n)`.
pub fn trivial_hypergraph(n: usize) -> Hypergraph {
    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    h
}

/// Enumerate every strictly-increasing `k`-combination of `0..n` (lexicographic).
/// Bounded: yields exactly `comb(n, k)` combinations.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    if k == 0 || k > n {
        return result;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        result.push(idx.iter().map(|&i| i).collect());
        // advance to next combination
        let mut i = k;
        // Bounded: at most k steps.
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return result;
            }
        }
        if k == 0 {
            return result;
        }
    }
}

/// A complete hypergraph on `n` nodes. Exactly one of `order` / `max_order` must
/// be `Some`. XGI `complete_hypergraph`.
pub fn complete_hypergraph(
    n: usize,
    order: Option<usize>,
    max_order: Option<usize>,
    include_singletons: bool,
) -> Result<Hypergraph, GeneratorError> {
    match (order, max_order) {
        (None, None) => {
            return Err(GeneratorError::InvalidParameter(
                "at least one of order and max_order must be specified".into(),
            ))
        }
        (Some(_), Some(_)) => {
            return Err(GeneratorError::InvalidParameter(
                "order and max_order cannot both be specified".into(),
            ))
        }
        _ => {}
    }

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    let mut edges: Vec<Vec<usize>> = Vec::new();
    if let Some(order) = order {
        edges.extend(combinations(n, order + 1));
    } else if let Some(max_order) = max_order {
        let start = if include_singletons { 1 } else { 2 };
        // Bounded: sizes start..=max_order+1.
        for r in start..=(max_order + 1) {
            edges.extend(combinations(n, r));
        }
    }

    for e in edges {
        let members: Vec<String> = e.iter().map(|i| i.to_string()).collect();
        // Non-empty by construction; auto-id.
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}

/// The complement of `h`: same nodes, every possible edge (size 1..=maxsize)
/// EXCEPT those present in `h`. XGI `complement(H)`.
pub fn complement(h: &Hypergraph) -> Hypergraph {
    let nodes = h.node_ids();
    let n = nodes.len();
    let mut hc = Hypergraph::new();
    if n == 0 {
        return hc;
    }
    hc.add_nodes_from(nodes.iter().map(|nid| (nid.clone(), Value::Null)));

    // node id -> dense index 0..n
    let index: std::collections::HashMap<&str, usize> =
        nodes.iter().enumerate().map(|(i, nid)| (nid.as_str(), i)).collect();

    // existing edges as sorted index-tuples
    let mut existing: std::collections::HashSet<Vec<usize>> = std::collections::HashSet::new();
    let mut max_size = 0usize;
    for e in h.edge_ids() {
        let mut idxs: Vec<usize> = h.members(&e).unwrap().iter().map(|m| index[m.as_str()]).collect();
        idxs.sort_unstable();
        max_size = max_size.max(idxs.len());
        existing.insert(idxs);
    }
    if max_size == 0 {
        max_size = 1;
    }

    // every possible edge of size 1..=max_size not in `existing`
    // Bounded: sizes 1..=max_size, each a finite combination set.
    for size in 1..=max_size {
        for combo in combinations(n, size) {
            if !existing.contains(&combo) {
                let members: Vec<String> = combo.iter().map(|&i| nodes[i].clone()).collect();
                let _ = hc.add_edge(members, None, Value::Null);
            }
        }
    }
    hc
}
```

In `generators/mod.rs`, uncomment the `classic` re-export line **and** add `pub use classic::GeneratorError;` to the re-exports.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 17 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): classic generators (empty, trivial, complete, complement)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: Lattice generator — ring_lattice

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/lattice.rs`
- Modify: `generators/mod.rs` (uncomment `lattice` re-export)

**Interfaces:** `ring_lattice(n: usize, d: usize, k: usize, l: usize) -> Result<Hypergraph, GeneratorError>`.

**XGI source:** `lattice.py:15` — d-uniform on n nodes; edges `[node] + [(start + l + i) mod n for i in 0..d-1]` for `node in 0..n` and `start in node+1 ..= node + k/2`; `k < 0` → error (unrepresentable in `usize`, so we accept `k: usize` and only warn-equivalent on `k < 2` / odd `k` by no special handling); then `add_nodes_from(0..n)`.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::ring_lattice;

#[test]
fn test_ring_lattice_node_count_and_uniformity() {
    // n=10, d=3, k=4, l=1
    let h = ring_lattice(10, 3, 4, 1).unwrap();
    assert_eq!(h.num_nodes(), 10);
    // each of n nodes contributes k/2 edges -> 10 * 2 = 20 edges
    assert_eq!(h.num_edges(), 20);
    // d-uniform: every edge has <= d members (dedup can shrink)
    for e in h.edge_ids() {
        let sz = h.members(&e).unwrap().len();
        assert!(sz >= 1 && sz <= 3);
    }
}

#[test]
fn test_ring_lattice_is_deterministic() {
    // ring_lattice has no randomness; two builds are structurally identical
    let a = ring_lattice(8, 2, 4, 0).unwrap();
    let b = ring_lattice(8, 2, 4, 0).unwrap();
    assert_eq!(a.num_nodes(), b.num_nodes());
    assert_eq!(a.num_edges(), b.num_edges());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL — `ring_lattice` doesn't exist.

- [ ] **Step 3: Implement `lattice.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/lattice.rs`:

```rust
//! Lattice hypergraph generators (XGI `generators/lattice.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use crate::Hypergraph;

/// A d-uniform ring-lattice hypergraph on `n` nodes: each node is in `k` edges,
/// consecutive edges overlap by `d - l`. XGI `ring_lattice(n, d, k, l)`.
///
/// `k` is `usize` here (negative k is unrepresentable); XGI's `k < 2` /
/// odd-`k` warnings are elided — the structural output is unchanged.
pub fn ring_lattice(n: usize, d: usize, k: usize, l: usize) -> Result<Hypergraph, GeneratorError> {
    if n == 0 {
        return Err(GeneratorError::InvalidParameter("n must be > 0".into()));
    }
    if d == 0 {
        return Err(GeneratorError::InvalidParameter("d must be >= 1".into()));
    }
    let mut h = Hypergraph::new();
    // Bounded: n * (k/2) edges, each with d members.
    for node in 0..n {
        for start in (node + 1)..=(node + k / 2) {
            let mut members: Vec<String> = Vec::with_capacity(d);
            members.push(node.to_string());
            for i in 0..(d - 1) {
                members.push(((start + l + i) % n).to_string());
            }
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    Ok(h)
}
```

Uncomment the `lattice` re-export in `generators/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 19 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): ring_lattice generator

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: Simple generators — star_clique, sunflower

**Files:**
- Modify: `tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/simple.rs`
- Modify: `generators/mod.rs` (uncomment `simple` re-export)

**Interfaces:** `star_clique(n_star: usize, n_clique: usize, d_max: usize) -> Result<Hypergraph, GeneratorError>`, `sunflower(l: usize, c: usize, m: usize) -> Result<Hypergraph, GeneratorError>`.

**XGI source:** `simple.py:18` (`star_clique`: validate `n_star>0`, `n_clique>0`, `0 <= d_max <= n_clique-1`; star center is node 0; star legs `(0, i)` for `i in 1..n_star`; connect via `(0, n_star)`; clique hyperedges = all `(d+1)`-subsets of clique nodes for `d in 1..=d_max`) and `simple.py:81` (`sunflower`: `m >= c`; core `0..c`; petals of size `m-c`, `l` of them).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::{star_clique, sunflower};

#[test]
fn test_star_clique_node_count() {
    // n_star=6, n_clique=7 -> 13 nodes
    let h = star_clique(6, 7, 2).unwrap();
    assert_eq!(h.num_nodes(), 13);
    assert!(h.num_edges() > 0);
}

#[test]
fn test_star_clique_rejects_bad_dmax() {
    // d_max must be <= n_clique - 1
    assert!(star_clique(6, 7, 7).is_err());
    assert!(star_clique(0, 7, 2).is_err());
}

#[test]
fn test_sunflower_is_m_uniform() {
    // l=3 petals, c=2 core, m=4 edge size
    let h = sunflower(3, 2, 4).unwrap();
    assert_eq!(h.num_edges(), 3);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 4);
    }
}

#[test]
fn test_sunflower_rejects_m_less_than_c() {
    assert!(sunflower(3, 5, 4).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL.

- [ ] **Step 3: Implement `simple.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/simple.rs`:

```rust
//! Simple hypergraph generators (XGI `generators/simple.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use crate::Hypergraph;

/// Enumerate strictly-increasing `k`-combinations of the slice `pool`.
fn combinations_of(pool: &[usize], k: usize) -> Vec<Vec<usize>> {
    let n = pool.len();
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.iter().map(|&i| pool[i]).collect());
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out;
            }
        }
    }
}

/// A star network joined to a clique promoted to hyperedges up to `d_max`.
/// XGI `star_clique(n_star, n_clique, d_max)`.
pub fn star_clique(
    n_star: usize,
    n_clique: usize,
    d_max: usize,
) -> Result<Hypergraph, GeneratorError> {
    if n_star == 0 {
        return Err(GeneratorError::InvalidParameter("n_star must be > 0".into()));
    }
    if n_clique == 0 {
        return Err(GeneratorError::InvalidParameter("n_clique must be > 0".into()));
    }
    if d_max > n_clique - 1 {
        return Err(GeneratorError::InvalidParameter("d_max must be <= n_clique - 1".into()));
    }

    let nodes_star: Vec<usize> = (0..n_star).collect();
    let nodes_clique: Vec<usize> = (n_star..(n_star + n_clique)).collect();

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..(n_star + n_clique)).map(|i| (i.to_string(), Value::Null)));

    // star legs (center = nodes_star[0])
    for i in 1..n_star {
        let _ = h.add_edge(
            vec![nodes_star[0].to_string(), nodes_star[i].to_string()],
            None,
            Value::Null,
        );
    }
    // connect star to clique
    let _ = h.add_edge(
        vec![nodes_star[0].to_string(), nodes_clique[0].to_string()],
        None,
        Value::Null,
    );
    // clique hyperedges up to order d_max (sizes 2..=d_max+1)
    for d in 1..=d_max {
        for combo in combinations_of(&nodes_clique, d + 1) {
            let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
            let _ = h.add_edge(members, None, Value::Null);
        }
    }
    Ok(h)
}

/// An m-uniform sunflower hypergraph: `l` petals sharing a `c`-node core.
/// XGI `sunflower(l, c, m)`.
pub fn sunflower(l: usize, c: usize, m: usize) -> Result<Hypergraph, GeneratorError> {
    if m < c {
        return Err(GeneratorError::InvalidParameter("m cannot be smaller than c".into()));
    }
    let core: Vec<String> = (0..c).map(|i| i.to_string()).collect();
    let mut h = Hypergraph::new();
    let petal = m - c;
    // Bounded: exactly l petals.
    for p in 0..l {
        let mut members = core.clone();
        let start = c + p * petal;
        for i in 0..petal {
            members.push((start + i).to_string());
        }
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}
```

Uncomment the `simple` re-export in `generators/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 23 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): star_clique + sunflower generators

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: Random generators — fast_random_hypergraph, random_hypergraph

**Files:**
- Modify: `tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/random.rs`
- Modify: `generators/mod.rs` (uncomment `random` re-exports — the full set lands over Tasks 6–7; uncomment all four names now and stub `chung_lu`/`watts_strogatz` as `todo!()`-free minimal shims? No — instead uncomment only `fast_random_hypergraph` and `random_hypergraph` now, add the other two in Task 7).

**Interfaces:** `fast_random_hypergraph(n: usize, ps: &[f64], order: Option<&[usize]>, seed: Option<u64>) -> Result<Hypergraph, GeneratorError>`, `random_hypergraph(n, ps, order, seed) -> Result<Hypergraph, GeneratorError>`.

**XGI source:** `random.py:25` (`fast_random_hypergraph`: for each `(d, p)` pair, if `p==1` add all `(d+1)`-combinations, elif `p>0` skip-sample indices via `geometric` bounded by `comb(n,d+1)-1`, mapping each via `_index_to_edge_comb`) and `random.py:102` (`random_hypergraph`: Bernoulli over every `(d+1)`-combination). `_check_input_args` (`random.py:166`): if `order is None`, `order = [i+1 for i in range(len(ps))]`; all `ps in [0,1]`.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::{fast_random_hypergraph, random_hypergraph};

#[test]
fn test_fast_random_hypergraph_node_count() {
    let h = fast_random_hypergraph(50, &[0.1, 0.01], None, Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 50);
}

#[test]
fn test_fast_random_hypergraph_is_deterministic() {
    let a = fast_random_hypergraph(30, &[0.2, 0.05], None, Some(99)).unwrap();
    let b = fast_random_hypergraph(30, &[0.2, 0.05], None, Some(99)).unwrap();
    assert_eq!(a.num_edges(), b.num_edges());
    assert_eq!(a.edge_ids().len(), b.edge_ids().len());
    // exact structural equality (same seed -> same graph)
    assert!(a == b);
}

#[test]
fn test_fast_random_hypergraph_p_one_is_complete_at_order() {
    // order=[1], ps=[1.0] -> every 2-subset of n nodes = comb(n,2)
    let h = fast_random_hypergraph(5, &[1.0], Some(&[1]), Some(3)).unwrap();
    assert_eq!(h.num_edges(), 10); // comb(5,2)
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 2);
    }
}

#[test]
fn test_fast_random_edge_orders_match_requested() {
    // Only 3-node edges requested (order=[2]); every edge has 3 members.
    let h = fast_random_hypergraph(20, &[0.5], Some(&[2]), Some(5)).unwrap();
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_random_hypergraph_node_count_and_determinism() {
    let a = random_hypergraph(15, &[0.3, 0.05], None, Some(7)).unwrap();
    let b = random_hypergraph(15, &[0.3, 0.05], None, Some(7)).unwrap();
    assert_eq!(a.num_nodes(), 15);
    assert!(a == b);
}

#[test]
fn test_random_hypergraph_rejects_bad_probability() {
    assert!(random_hypergraph(10, &[1.5], Some(&[1]), Some(1)).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL.

- [ ] **Step 3: Implement the two functions in `random.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/random.rs`:

```rust
//! Random (non-uniform) hypergraph generators (XGI `generators/random.py`).

use serde_json::Value;

use super::classic::GeneratorError;
use super::rng::{comb, geometric, index_to_edge_comb, seeded_rng, uniform_f64};
use crate::Hypergraph;

/// Resolve `(ps, order)` per XGI `_check_input_args`: default `order = 1..=len(ps)`;
/// validate every `p in [0, 1]` and lengths match.
fn check_input_args(ps: &[f64], order: Option<&[usize]>) -> Result<Vec<usize>, GeneratorError> {
    let order: Vec<usize> = match order {
        None => (0..ps.len()).map(|i| i + 1).collect(),
        Some(o) => {
            if o.len() != ps.len() {
                return Err(GeneratorError::InvalidParameter(
                    "the length of ps must match the length of order".into(),
                ));
            }
            o.to_vec()
        }
    };
    if ps.iter().any(|&p| !(0.0..=1.0).contains(&p)) {
        return Err(GeneratorError::InvalidParameter(
            "all elements of ps must be between 0 and 1".into(),
        ));
    }
    Ok(order)
}

/// Enumerate strictly-increasing `k`-combinations of `0..n`.
fn all_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out;
            }
        }
    }
}

/// Fast random hypergraph via geometric skip-sampling. XGI `fast_random_hypergraph`.
pub fn fast_random_hypergraph(
    n: usize,
    ps: &[f64],
    order: Option<&[usize]>,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let order = check_input_args(ps, order)?;
    let mut rng = seeded_rng(seed);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    for (d, &p) in order.iter().zip(ps.iter()) {
        let d = *d;
        if p == 1.0 {
            for combo in all_combinations(n, d + 1) {
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
            }
        } else if p > 0.0 {
            let max_index = comb(n as u64, (d + 1) as u64).saturating_sub(1);
            // -1 for zero-indexing (geometric >= 1 here)
            let mut index = geometric(p, &mut rng) - 1;
            // Bounded: index strictly increases by >= 1 each step, capped by max_index.
            while index <= max_index {
                let combo = index_to_edge_comb(index, n as u64, (d + 1) as u64);
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
                index += geometric(p, &mut rng);
            }
        }
    }
    Ok(h)
}

/// Random hypergraph via Bernoulli over every combination. XGI `random_hypergraph`.
pub fn random_hypergraph(
    n: usize,
    ps: &[f64],
    order: Option<&[usize]>,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let order = check_input_args(ps, order)?;
    let mut rng = seeded_rng(seed);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));

    for (d, &p) in order.iter().zip(ps.iter()) {
        for combo in all_combinations(n, *d + 1) {
            if uniform_f64(&mut rng) <= p {
                let members: Vec<String> = combo.iter().map(|i| i.to_string()).collect();
                let _ = h.add_edge(members, None, Value::Null);
            }
        }
    }
    Ok(h)
}
```

In `generators/mod.rs`, change the `random` re-export to only these two for now:
```rust
pub use random::{fast_random_hypergraph, random_hypergraph};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 29 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): fast_random_hypergraph + random_hypergraph

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 7: Random generators — chung_lu_hypergraph, watts_strogatz_hypergraph

**Files:**
- Modify: `tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/random.rs`
- Modify: `generators/mod.rs` (extend `random` re-export to all four)

**Interfaces:** `chung_lu_hypergraph(k1: &IndexMap<String, f64>, k2: &IndexMap<String, f64>, seed: Option<u64>) -> Hypergraph`, `watts_strogatz_hypergraph(n, d, k, l, p, seed) -> Result<Hypergraph, GeneratorError>`.

**XGI source:** `random.py:201` (`chung_lu_hypergraph`: sort `k1`/`k2` by value desc; for each node `u` skip through edge-slots with `geometric`, connecting `u` to edge `v` when `r < q/p`) and `random.py:420` (`watts_strogatz_hypergraph`: `ring_lattice`, then rewire each edge with prob `p` by replacing all-but-min member with random node choices).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::{chung_lu_hypergraph, watts_strogatz_hypergraph};
use indexmap::IndexMap;

#[test]
fn test_chung_lu_node_count_and_determinism() {
    let mut k1: IndexMap<String, f64> = IndexMap::new();
    let mut k2: IndexMap<String, f64> = IndexMap::new();
    for i in 0..20 {
        k1.insert(i.to_string(), (1 + (i % 5)) as f64);
        k2.insert(i.to_string(), (1 + (i % 5)) as f64);
    }
    let a = chung_lu_hypergraph(&k1, &k2, Some(4));
    let b = chung_lu_hypergraph(&k1, &k2, Some(4));
    assert_eq!(a.num_nodes(), 20);
    assert!(a == b);
}

#[test]
fn test_watts_strogatz_preserves_node_count() {
    let h = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    assert_eq!(h.num_nodes(), 12);
    assert!(h.num_edges() > 0);
}

#[test]
fn test_watts_strogatz_is_deterministic() {
    let a = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    let b = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    assert!(a == b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL.

- [ ] **Step 3: Implement both functions (append to `random.rs`)**

Add these imports at the top of `random.rs` (merge with existing `use` lines):

```rust
use indexmap::IndexMap;
use super::lattice::ring_lattice;
use super::rng::random_below;
```

Append to `random.rs`:

```rust
/// Chung–Lu hypergraph from node-degree and edge-size sequences.
/// XGI `chung_lu_hypergraph(k1, k2)`.
///
/// `k1`: node id -> degree; `k2`: edge id -> size. Determinism: nodes/edges are
/// sorted by value descending with a stable tiebreak on the (insertion-ordered)
/// key, so the traversal is reproducible.
pub fn chung_lu_hypergraph(
    k1: &IndexMap<String, f64>,
    k2: &IndexMap<String, f64>,
    seed: Option<u64>,
) -> Hypergraph {
    let mut rng = seeded_rng(seed);

    // sort by value desc; stable tiebreak preserves IndexMap insertion order
    let mut node_labels: Vec<&String> = k1.keys().collect();
    node_labels.sort_by(|a, b| k2_cmp(k1[*b], k1[*a]));
    let mut edge_labels: Vec<&String> = k2.keys().collect();
    edge_labels.sort_by(|a, b| k2_cmp(k2[*b], k2[*a]));

    let m = k2.len();
    let s: f64 = k1.values().sum();

    let mut h = Hypergraph::new();
    for &nid in &node_labels {
        h.add_node(nid, Value::Null);
    }

    for &u in &node_labels {
        let mut j = 0usize;
        if m == 0 {
            continue;
        }
        let mut v = edge_labels[0];
        let mut p = ((k1[u] * k2[v]) / s).min(1.0);
        // Bounded: j strictly increases toward m.
        while j < m {
            if p != 1.0 {
                if p <= 0.0 {
                    // geometric(p<=0) returns the u64::MAX "infinity" sentinel;
                    // XGI relies on Python float `j += inf` to end the loop for
                    // this node. Break instead of adding the sentinel, which
                    // would overflow `j` (a real panic risk when j > 0).
                    break;
                }
                j += geometric(p, &mut rng) as usize;
            }
            if j < m {
                v = edge_labels[j];
                let q = ((k1[u] * k2[v]) / s).min(1.0);
                let r = uniform_f64(&mut rng);
                if p > 0.0 && r < q / p {
                    let _ = h.add_node_to_edge(v, u);
                }
                p = q;
                j += 1;
            }
        }
    }
    h
}

/// Total order on f64 for descending sort (NaN sinks last). Not exported.
fn k2_cmp(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

/// Watts–Strogatz hypergraph: a ring lattice with each edge rewired with prob `p`.
/// XGI `watts_strogatz_hypergraph(n, d, k, l, p)`.
pub fn watts_strogatz_hypergraph(
    n: usize,
    d: usize,
    k: usize,
    l: usize,
    p: f64,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let mut rng = seeded_rng(seed);
    let mut h = ring_lattice(n, d, k, l)?;

    let mut to_remove: Vec<String> = Vec::new();
    let mut to_add: Vec<Vec<String>> = Vec::new();

    for e in h.edge_ids() {
        if uniform_f64(&mut rng) < p {
            let members = h.members(&e).unwrap();
            // Keep the numeric-min member (node ids are decimal integers here),
            // matching XGI's `min(H.edges.members(e))` over integer node ids.
            // A lexicographic `str` min would diverge for n >= 11 (e.g. "10" < "2").
            let keep = members
                .iter()
                .min_by_key(|s| s.parse::<u64>().unwrap_or(u64::MAX))
                .cloned()
                .unwrap();
            let mut new_edge: Vec<String> = Vec::with_capacity(d);
            for _ in 0..(d - 1) {
                let idx = random_below(&mut rng, n as u64) as usize;
                new_edge.push(idx.to_string());
            }
            new_edge.push(keep);
            to_remove.push(e);
            to_add.push(new_edge);
        }
    }
    for e in to_remove {
        let _ = h.remove_edge(&e);
    }
    for members in to_add {
        let _ = h.add_edge(members, None, Value::Null);
    }
    Ok(h)
}
```

Extend the `random` re-export in `generators/mod.rs` to all four names (as written in the Task 1 scaffold).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 32 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): chung_lu + watts_strogatz generators

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 8: Uniform generators — erdos_renyi, configuration_model

**Files:**
- Modify: `tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/uniform.rs`
- Modify: `generators/mod.rs` (uncomment `uniform` re-exports)

**Interfaces:** `uniform_erdos_renyi_hypergraph(n: usize, m: usize, p: f64, p_type: PType, multiedges: bool, seed: Option<u64>) -> Result<Hypergraph, GeneratorError>`, `uniform_hypergraph_configuration_model(k: &IndexMap<String, usize>, m: usize, seed: Option<u64>) -> Hypergraph`. Add `pub enum PType { Prob, Degree }`.

**XGI source:** `uniform.py:294` (`uniform_erdos_renyi_hypergraph`: wiring prob `q` from `p_type`; `q==1 && !multiedges` → `complete_hypergraph(n, order=m-1)`; `q==0` → nodes only; else geometric skip-sample over `comb(n,m)-1` (no multiedges, `_index_to_edge_comb`) or `n^m-1` (multiedges, `_index_to_edge_prod`), discarding tuples whose dedup size `!= m`) and `uniform.py:22` (`uniform_hypergraph_configuration_model`: build stub list, draw `m` distinct stubs per edge, keep if `len(edge)==m`).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::{
    uniform_erdos_renyi_hypergraph, uniform_hypergraph_configuration_model,
};
use hypergraph_rs::generators::uniform::PType;

#[test]
fn test_uniform_er_is_m_uniform() {
    let h = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(2)).unwrap();
    assert_eq!(h.num_nodes(), 30);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_uniform_er_p_zero_is_empty_edges() {
    let h = uniform_erdos_renyi_hypergraph(10, 2, 0.0, PType::Prob, false, Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 10);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_uniform_er_p_one_is_complete() {
    // q==1, no multiedges -> complete_hypergraph(n, order=m-1) = comb(n, m) edges
    let h = uniform_erdos_renyi_hypergraph(5, 2, 1.0, PType::Prob, false, Some(1)).unwrap();
    assert_eq!(h.num_edges(), 10); // comb(5,2)
}

#[test]
fn test_uniform_er_is_deterministic() {
    let a = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(9)).unwrap();
    let b = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(9)).unwrap();
    assert!(a == b);
}

#[test]
fn test_uniform_er_rejects_bad_probability() {
    assert!(uniform_erdos_renyi_hypergraph(10, 2, 2.0, PType::Prob, false, Some(1)).is_err());
}

#[test]
fn test_configuration_model_is_m_uniform() {
    let mut k: IndexMap<String, usize> = IndexMap::new();
    for i in 0..30 {
        k.insert(i.to_string(), 3);
    }
    let h = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    assert_eq!(h.num_nodes(), 30);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_configuration_model_is_deterministic() {
    let mut k: IndexMap<String, usize> = IndexMap::new();
    for i in 0..30 {
        k.insert(i.to_string(), 3);
    }
    let a = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    let b = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    assert!(a == b);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL.

- [ ] **Step 3: Implement `uniform.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/uniform.rs`:

```rust
//! Uniform random hypergraph generators (XGI `generators/uniform.py`).
//!
//! `uniform_HSBM` / `uniform_HPPM` require an m-dimensional probability tensor and
//! are deferred (see the plan's Open questions).

use std::collections::HashSet;

use indexmap::IndexMap;
use serde_json::Value;

use super::classic::{complete_hypergraph, GeneratorError};
use super::rng::{
    choice_without_replacement, comb, geometric, index_to_edge_comb, index_to_edge_prod, seeded_rng,
};
use crate::Hypergraph;

/// How `p` is interpreted in `uniform_erdos_renyi_hypergraph`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PType {
    /// `p` is the per-edge probability.
    Prob,
    /// `p` is the target mean degree.
    Degree,
}

/// m-uniform Erdős–Rényi hypergraph. XGI `uniform_erdos_renyi_hypergraph`.
pub fn uniform_erdos_renyi_hypergraph(
    n: usize,
    m: usize,
    p: f64,
    p_type: PType,
    multiedges: bool,
    seed: Option<u64>,
) -> Result<Hypergraph, GeneratorError> {
    let q = match p_type {
        PType::Degree => {
            if multiedges {
                p / (m as f64 * (n as f64).powi((m - 1) as i32))
            } else {
                p * n as f64 / (m as f64 * comb(n as u64, m as u64) as f64)
            }
        }
        PType::Prob => p,
    };
    if !(0.0..=1.0).contains(&q) {
        return Err(GeneratorError::InvalidParameter("probability not in [0, 1]".into()));
    }
    if q == 1.0 && !multiedges {
        return complete_hypergraph(n, Some(m - 1), None, false);
    }

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    if q == 0.0 {
        return Ok(h);
    }

    let mut rng = seeded_rng(seed);
    let max_index: u64 = if multiedges {
        (n as u64).pow(m as u32).saturating_sub(1)
    } else {
        comb(n as u64, m as u64).saturating_sub(1)
    };

    let mut index = geometric(q, &mut rng) - 1;
    // Bounded: index strictly increases by >= 1 each step, capped by max_index.
    while index <= max_index {
        let tuple = if multiedges {
            index_to_edge_prod(index, n as u64, m as u64)
        } else {
            index_to_edge_comb(index, n as u64, m as u64)
        };
        // dedup to a set; keep only if still size m (discard self-loop artifacts)
        let set: HashSet<usize> = tuple.iter().copied().collect();
        if set.len() == m {
            let members: Vec<String> = tuple.iter().map(|i| i.to_string()).collect();
            let _ = h.add_edge(members, None, Value::Null);
        }
        index += geometric(q, &mut rng);
    }
    Ok(h)
}

/// m-uniform configuration model from a node-degree sequence.
/// XGI `uniform_hypergraph_configuration_model(k, m)`.
///
/// XGI increases random node degrees when `sum(k) % m != 0`; we instead drop the
/// remainder stubs (documented deviation — still deterministic, avoids mutating
/// the caller's degree map). Loopy/duplicate-collapsed edges are discarded.
pub fn uniform_hypergraph_configuration_model(
    k: &IndexMap<String, usize>,
    m: usize,
    seed: Option<u64>,
) -> Hypergraph {
    let mut rng = seeded_rng(seed);

    // stub list: each node id repeated by its degree, in insertion order
    let mut stubs: Vec<String> = Vec::new();
    for (idx, &deg) in k {
        for _ in 0..deg {
            stubs.push(idx.clone());
        }
    }
    // drop remainder stubs so len is divisible by m (deviation from XGI)
    let rem = stubs.len() % m;
    for _ in 0..rem {
        stubs.pop();
    }

    let mut h = Hypergraph::new();
    for key in k.keys() {
        h.add_node(key, Value::Null);
    }

    // Bounded: each iteration removes exactly m stubs; stubs.len()/m iterations.
    while stubs.len() >= m {
        let picks = choice_without_replacement(&mut rng, stubs.len(), m);
        // Dedup in draw order (NOT via HashSet iteration, whose order is
        // process-randomized) so `members()` is reproducible bit-for-bit given
        // a seed, per the Global Constraint.
        let mut seen: HashSet<String> = HashSet::new();
        let mut members: Vec<String> = Vec::with_capacity(m);
        for &pi in &picks {
            let s = stubs[pi].clone();
            if seen.insert(s.clone()) {
                members.push(s);
            }
        }
        if members.len() == m {
            let _ = h.add_edge(members, None, Value::Null);
        }
        // remove chosen stubs, largest index first
        let mut sorted = picks.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for pi in sorted {
            stubs.remove(pi);
        }
    }
    h
}
```

Uncomment the `uniform` re-export in `generators/mod.rs`. Because `PType` lives in `uniform`, also add `pub use uniform::PType;` (or the test's `use hypergraph_rs::generators::uniform::PType;` path resolves directly since `uniform` is a public module — keep the module public, which it already is).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 39 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): uniform ER + configuration-model generators

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 9: Simplicial generator — random_simplicial_complex

**Files:**
- Modify: `tests/test_generators.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/generators/simplicial.rs`
- Modify: `generators/mod.rs` (uncomment `simplicial` re-export)

**DEPENDS ON Phase 2 `SimplicialComplex`.** Before implementing, grep the landed crate for the real API: `rg "impl.*SimplicialComplex" crates/hypergraph-rs/src`. This plan assumes `SimplicialComplex::new() -> Self`, `add_nodes_from(impl IntoIterator<Item=(String, Value)>)`, and `add_simplices_from(impl IntoIterator<Item = Vec<String>>)` (the closure of each simplex over its subfaces is `SimplicialComplex`'s responsibility, per XGI `core/simplicialcomplex.py`). If the real signatures differ, adapt the calls — the sampling logic is unchanged. **If Phase 2 is not yet landed, SKIP this task and record it in the phase handoff.**

**XGI source:** `simplicial_complexes.py:24` — for each order `d = i+1`, draw a Bernoulli mask over all `(d+1)`-combinations of `0..N`; add kept combinations as simplices (subfaces auto-added by `SimplicialComplex`).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_generators.rs`:

```rust
use hypergraph_rs::generators::random_simplicial_complex;

#[test]
fn test_random_simplicial_complex_node_count() {
    let s = random_simplicial_complex(20, &[0.1, 0.01], Some(3)).unwrap();
    assert_eq!(s.num_nodes(), 20);
}

#[test]
fn test_random_simplicial_complex_is_deterministic() {
    let a = random_simplicial_complex(15, &[0.2, 0.05], Some(8)).unwrap();
    let b = random_simplicial_complex(15, &[0.2, 0.05], Some(8)).unwrap();
    assert_eq!(a.num_nodes(), b.num_nodes());
    assert_eq!(a.num_edges(), b.num_edges());
}

#[test]
fn test_random_simplicial_complex_rejects_bad_probability() {
    assert!(random_simplicial_complex(10, &[1.5], Some(1)).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: FAIL.

- [ ] **Step 3: Implement `simplicial.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/generators/simplicial.rs` (adjust the `SimplicialComplex` import path / method names to the real Phase-2 API if they differ):

```rust
//! Simplicial-complex generators (XGI `generators/simplicial_complexes.py`).
//!
//! Only `random_simplicial_complex` is ported here; the `flag_complex` family
//! consumes NetworkX graphs and belongs to the `convert` slice (Phase 6).

use serde_json::Value;

use super::classic::GeneratorError;
use super::rng::{seeded_rng, uniform_f64};
use crate::SimplicialComplex; // adjust to the real Phase-2 path if this re-export differs — `crate::core::...` is unlikely to be the public path

/// Enumerate strictly-increasing `k`-combinations of `0..n`.
fn all_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                idx[i] += 1;
                for j in (i + 1)..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return out;
            }
        }
    }
}

/// Random simplicial complex: connect any `d+1` nodes with a simplex with prob
/// `ps[d-1]`; all subfaces are added. XGI `random_simplicial_complex(N, ps)`.
pub fn random_simplicial_complex(
    n: usize,
    ps: &[f64],
    seed: Option<u64>,
) -> Result<SimplicialComplex, GeneratorError> {
    if ps.iter().any(|&p| !(0.0..=1.0).contains(&p)) {
        return Err(GeneratorError::InvalidParameter(
            "all elements of ps must be between 0 and 1".into(),
        ));
    }
    let mut rng = seeded_rng(seed);

    let mut simplices: Vec<Vec<String>> = Vec::new();
    for (i, &p) in ps.iter().enumerate() {
        let d = i + 1; // ps[0] is prob of edges (d=1)
        for combo in all_combinations(n, d + 1) {
            if uniform_f64(&mut rng) <= p {
                simplices.push(combo.iter().map(|x| x.to_string()).collect());
            }
        }
    }

    let mut s = SimplicialComplex::new();
    s.add_nodes_from((0..n).map(|i| (i.to_string(), Value::Null)));
    s.add_simplices_from(simplices);
    Ok(s)
}
```

Uncomment the `simplicial` re-export in `generators/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_generators`
Expected: PASS — 42 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): random_simplicial_complex generator

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 10: Node statistics — degree, attrs

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_stats.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/stats/nodestats.rs`
- Modify: `stats/mod.rs` (add `pub use nodestats::*;` re-export block)

**Interfaces:** `node_degree(h: &Hypergraph, order: Option<usize>) -> Stat<usize>` (unweighted only — no vestigial `weight` param, since it would be silently ignored; weighted degree is the separate `node_degree_weighted(h: &Hypergraph, weight: &str, order: Option<usize>) -> Stat<f64>`). `node_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<serde_json::Value>`.

**XGI source:** `nodestats.py:120` (`degree`: `len(memberships)`, or count only edges of `size == order+1`, or sum an edge-attr weight) and `nodestats.py:40` (`attrs`: one attr with `missing` imputation, or the whole attr dict).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_stats.rs`:

```rust
use hypergraph_rs::stats::nodestats::{node_attrs, node_degree};
use hypergraph_rs::Hypergraph;

fn triangle_plus() -> Hypergraph {
    // XGI docstring fixture: [[1,2,3],[2,3,4,5],[3,4,5]]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into(), "4".into(), "5".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "5".into()],
        Some("2".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_node_degree_matches_xgi() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // XGI: {1:1, 2:2, 3:3, 4:2, 5:2}
    assert_eq!(*deg.get("1").unwrap(), 1);
    assert_eq!(*deg.get("2").unwrap(), 2);
    assert_eq!(*deg.get("3").unwrap(), 3);
    assert_eq!(*deg.get("4").unwrap(), 2);
    assert_eq!(*deg.get("5").unwrap(), 2);
}

#[test]
fn test_node_degree_by_order() {
    let h = triangle_plus();
    // order=2 -> only size-3 edges (edges 0 and 2)
    let deg = node_degree(&h, Some(2));
    assert_eq!(*deg.get("3").unwrap(), 2); // 3 is in both size-3 edges
    assert_eq!(*deg.get("2").unwrap(), 1); // 2 only in edge 0 (size 3)
}

#[test]
fn test_node_degree_aslist_order() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // node insertion order is 1,2,3,4,5
    assert_eq!(deg.aslist(), vec![1, 2, 3, 2, 2]);
}

#[test]
fn test_node_degree_filterby() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    let mut two = deg.filterby(&2);
    two.sort();
    assert_eq!(two, vec!["2".to_string(), "4".to_string(), "5".to_string()]);
}

#[test]
fn test_node_attrs_single() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"color": "red"}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let colors = node_attrs(&h, Some("color"));
    assert_eq!(colors.get("a").unwrap(), &serde_json::json!("red"));
    assert_eq!(colors.get("b").unwrap(), &serde_json::json!("blue"));
}

#[test]
fn test_node_attrs_missing_imputes_null() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"age": 20}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let ages = node_attrs(&h, Some("age"));
    assert_eq!(ages.get("a").unwrap(), &serde_json::json!(20));
    assert_eq!(ages.get("b").unwrap(), &serde_json::Value::Null);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: FAIL.

- [ ] **Step 3: Implement `nodestats.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/stats/nodestats.rs`:

```rust
//! Node statistics (XGI `stats/nodestats.py`).

use indexmap::IndexMap;
use serde_json::Value;

use super::Stat;
use crate::Hypergraph;

/// Node degree: number of edges a node belongs to, optionally restricted to
/// edges of a given `order` (size `order + 1`). XGI `nodestats.degree` (unweighted).
pub fn node_degree(h: &Hypergraph, order: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in h.node_ids() {
        let memberships = h.memberships(&nid).unwrap_or_default();
        let count = match order {
            None => memberships.len(),
            Some(o) => memberships
                .iter()
                .filter(|e| h.members(e).map(|m| m.len()) == Some(o + 1))
                .count(),
        };
        out.insert(nid, count);
    }
    Stat::new(out)
}

/// Weighted node degree: sum of an edge-attr weight over the node's edges
/// (default weight 1 when the attr is absent/non-numeric). XGI `degree(weight=...)`.
pub fn node_degree_weighted(h: &Hypergraph, weight: &str, order: Option<usize>) -> Stat<f64> {
    let mut out: IndexMap<String, f64> = IndexMap::new();
    for nid in h.node_ids() {
        let memberships = h.memberships(&nid).unwrap_or_default();
        let mut sum = 0.0f64;
        for e in &memberships {
            if let Some(o) = order {
                if h.members(e).map(|m| m.len()) != Some(o + 1) {
                    continue;
                }
            }
            let w = h
                .edge_attrs(e)
                .and_then(|a| a.get(weight))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            sum += w;
        }
        out.insert(nid, sum);
    }
    Stat::new(out)
}

/// Node attributes: one attr (with `Null` imputed when missing) or the whole
/// attr object per node. XGI `nodestats.attrs`.
pub fn node_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<Value> {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for nid in h.node_ids() {
        let all = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
        let v = match attr {
            None => all,
            Some(a) => all.get(a).cloned().unwrap_or(Value::Null),
        };
        out.insert(nid, v);
    }
    Stat::new(out)
}
```

Add to `stats/mod.rs` re-exports (below the `Stat` definition): `pub use nodestats::{node_attrs, node_degree, node_degree_weighted};`

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: PASS — 6 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): node stats (degree, weighted degree, attrs)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 11: Edge statistics — order, size, attrs

**Files:**
- Modify: `tests/test_stats.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/stats/edgestats.rs`
- Modify: `stats/mod.rs` (add `pub use edgestats::*;`)

**Interfaces:** `edge_order(h: &Hypergraph, degree: Option<usize>) -> Stat<usize>`, `edge_size(h: &Hypergraph, degree: Option<usize>) -> Stat<usize>`, `edge_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<Value>`.

**XGI source:** `edgestats.py:108` (`order`: `len(members) - 1`, or `sum(node_degree == degree) - 1`), `edgestats.py:148` (`size`: `len(members)` or `sum(node_degree == degree)`), `edgestats.py:33` (`attrs`).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_stats.rs`:

```rust
use hypergraph_rs::stats::edgestats::{edge_attrs, edge_order, edge_size};

#[test]
fn test_edge_order_and_size_match_xgi() {
    let h = triangle_plus();
    let ord = edge_order(&h, None);
    // XGI: order {0:2, 1:3, 2:2}
    assert_eq!(*ord.get("0").unwrap(), 2);
    assert_eq!(*ord.get("1").unwrap(), 3);
    assert_eq!(*ord.get("2").unwrap(), 2);
    let sz = edge_size(&h, None);
    // size {0:3, 1:4, 2:3}
    assert_eq!(*sz.get("0").unwrap(), 3);
    assert_eq!(*sz.get("1").unwrap(), 4);
    assert_eq!(*sz.get("2").unwrap(), 3);
}

#[test]
fn test_edge_size_aslist_order() {
    let h = triangle_plus();
    assert_eq!(edge_size(&h, None).aslist(), vec![3, 4, 3]);
}

#[test]
fn test_edge_order_degree_filter_zero_matches_clamps_at_zero() {
    let h = triangle_plus();
    // No node in edge "0" ({1,2,3}) has degree 99. XGI's order(degree) would
    // report -1 (sum(0) - 1); our Stat<usize> cannot represent a negative
    // order, so this clamps at 0 (documented deviation #7).
    let ord = edge_order(&h, Some(99));
    assert_eq!(*ord.get("0").unwrap(), 0);
}

#[test]
fn test_edge_attrs_single_with_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("one".into()), serde_json::json!({"color": "red"})).unwrap();
    h.add_edge(vec!["b".into()], Some("two".into()), serde_json::json!({"age": 30})).unwrap();
    let colors = edge_attrs(&h, Some("color"));
    assert_eq!(colors.get("one").unwrap(), &serde_json::json!("red"));
    assert_eq!(colors.get("two").unwrap(), &serde_json::Value::Null);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: FAIL.

- [ ] **Step 3: Implement `edgestats.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/stats/edgestats.rs`:

```rust
//! Edge statistics (XGI `stats/edgestats.py`).

use indexmap::IndexMap;
use serde_json::Value;

use super::Stat;
use crate::Hypergraph;

/// Count members whose node-degree equals `degree` (helper for degree-filtered order/size).
fn members_with_degree(h: &Hypergraph, edge_id: &str, degree: usize) -> usize {
    h.members(edge_id)
        .unwrap_or_default()
        .iter()
        .filter(|n| h.memberships(n).map(|m| m.len()) == Some(degree))
        .count()
}

/// Edge order: `size - 1`, optionally counting only members of a given node-degree.
/// XGI `edgestats.order`.
pub fn edge_order(h: &Hypergraph, degree: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in h.edge_ids() {
        let val = match degree {
            None => h.members(&eid).map(|m| m.len()).unwrap_or(0).saturating_sub(1),
            Some(d) => members_with_degree(h, &eid, d).saturating_sub(1),
        };
        out.insert(eid, val);
    }
    Stat::new(out)
}

/// Edge size: number of members. XGI `edgestats.size`.
pub fn edge_size(h: &Hypergraph, degree: Option<usize>) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in h.edge_ids() {
        let val = match degree {
            None => h.members(&eid).map(|m| m.len()).unwrap_or(0),
            Some(d) => members_with_degree(h, &eid, d),
        };
        out.insert(eid, val);
    }
    Stat::new(out)
}

/// Edge attributes: one attr (Null-imputed) or the whole object. XGI `edgestats.attrs`.
pub fn edge_attrs(h: &Hypergraph, attr: Option<&str>) -> Stat<Value> {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for eid in h.edge_ids() {
        let all = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
        let v = match attr {
            None => all,
            Some(a) => all.get(a).cloned().unwrap_or(Value::Null),
        };
        out.insert(eid, v);
    }
    Stat::new(out)
}
```

Add `pub use edgestats::{edge_attrs, edge_order, edge_size};` to `stats/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: PASS — 10 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): edge stats (order, size, attrs)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 12: Directed statistics — dinodestats + diedgestats

**Files:**
- Modify: `tests/test_stats.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/stats/dinodestats.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/stats/diedgestats.rs`
- Modify: `stats/mod.rs` (add the two re-export blocks)

**DEPENDS ON Phase 2 `DiHypergraph`.** Grep the landed crate first: `rg "impl.*DiHypergraph" crates/hypergraph-rs/src`. This plan assumes the following DiHypergraph query surface (mirrors `xgi/core/dihypergraph.py`): `node_ids()->Vec<String>`, `edge_ids()->Vec<String>`, `in_edges(node)->Vec<String>` (edges whose HEAD contains the node — the node is a receiver, matching XGI `dinodestats.in_degree`), `out_edges(node)->Vec<String>` (edges whose TAIL contains the node — the node is a sender, matching XGI `dinodestats.out_degree`), `tail(edge)->Vec<String>`, `head(edge)->Vec<String>`, `dimembers(edge)->(Vec<String>, Vec<String>)`. **If the real API names differ, adapt the calls; the arithmetic is unchanged. If DiHypergraph is not landed, SKIP and record in the handoff.**

**XGI source:** `dinodestats.py:109` (`degree` = `len(in_edges) + len(out_edges)` counted as membership in either head or tail; `in_degree`, `out_degree`) and `diedgestats.py:107` (`order` = `len(head ∪ tail) - 1`, `size` = `len(head ∪ tail)`, `head_size`, `tail_size`, `head_order`, `tail_order`).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_stats.rs`:

```rust
use hypergraph_rs::stats::dinodestats::{di_in_degree, di_out_degree};
use hypergraph_rs::stats::diedgestats::{di_edge_size, di_head_size, di_tail_size};
use hypergraph_rs::DiHypergraph;

fn di_fixture() -> DiHypergraph {
    // XGI: DiHypergraph([[{1,2},{5,6}], [{4},{1,3}]])
    // edge 0: tail {1,2} head {5,6}; edge 1: tail {4} head {1,3}
    let mut d = DiHypergraph::new();
    d.add_edge(
        vec!["1".into(), "2".into()],
        vec!["5".into(), "6".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    d.add_edge(
        vec!["4".into()],
        vec!["1".into(), "3".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    d
}

#[test]
fn test_di_edge_sizes() {
    let d = di_fixture();
    // union sizes: edge 0 = {1,2,5,6} = 4; edge 1 = {4,1,3} = 3
    let sz = di_edge_size(&d);
    assert_eq!(*sz.get("0").unwrap(), 4);
    assert_eq!(*sz.get("1").unwrap(), 3);
    let hs = di_head_size(&d);
    assert_eq!(*hs.get("0").unwrap(), 2); // head {5,6}
    let ts = di_tail_size(&d);
    assert_eq!(*ts.get("1").unwrap(), 1); // tail {4}
}

#[test]
fn test_di_node_in_out_degree() {
    let d = di_fixture();
    // XGI convention: in_degree counts HEAD-membership (receiver), out_degree
    // counts TAIL-membership (sender).
    // node 1: tail of edge 0 (a sender), head of edge 1 (a receiver)
    //   -> in_degree(head-membership)=1, out_degree(tail-membership)=1
    let ind = di_in_degree(&d);
    let outd = di_out_degree(&d);
    assert_eq!(*ind.get("1").unwrap(), 1);
    assert_eq!(*outd.get("1").unwrap(), 1);
    // node 4: ONLY in the tail of edge 1 (a sender, never a receiver) — this is
    // the asymmetric case that distinguishes the two conventions.
    assert_eq!(*ind.get("4").unwrap(), 0);
    assert_eq!(*outd.get("4").unwrap(), 1);
    // node 6: ONLY in the head of edge 0 (a receiver, never a sender).
    assert_eq!(*ind.get("6").unwrap(), 1);
    assert_eq!(*outd.get("6").unwrap(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: FAIL.

- [ ] **Step 3: Implement `dinodestats.rs` and `diedgestats.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/stats/dinodestats.rs` (adjust query names to the real DiHypergraph API):

```rust
//! Directed node statistics (XGI `stats/dinodestats.py`).

use indexmap::IndexMap;

use super::Stat;
use crate::DiHypergraph;

/// In-degree: number of edges whose HEAD contains the node (the node is a
/// receiver). XGI `dinodestats.in_degree`.
pub fn di_in_degree(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        out.insert(nid.clone(), d.in_edges(&nid).len());
    }
    Stat::new(out)
}

/// Out-degree: number of edges whose TAIL contains the node (the node is a
/// sender). XGI `dinodestats.out_degree`.
pub fn di_out_degree(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        out.insert(nid.clone(), d.out_edges(&nid).len());
    }
    Stat::new(out)
}

/// Degree: membership in either head or tail. XGI `dinodestats.degree`.
pub fn di_degree(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for nid in d.node_ids() {
        out.insert(nid.clone(), d.in_edges(&nid).len() + d.out_edges(&nid).len());
    }
    Stat::new(out)
}
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/stats/diedgestats.rs`:

```rust
//! Directed edge statistics (XGI `stats/diedgestats.py`).

use std::collections::HashSet;

use indexmap::IndexMap;

use super::Stat;
use crate::DiHypergraph;

fn union_size(d: &DiHypergraph, edge_id: &str) -> usize {
    let mut set: HashSet<String> = HashSet::new();
    for n in d.tail(edge_id) {
        set.insert(n);
    }
    for n in d.head(edge_id) {
        set.insert(n);
    }
    set.len()
}

/// Edge size: `|head ∪ tail|`. XGI `diedgestats.size`.
pub fn di_edge_size(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        out.insert(eid.clone(), union_size(d, &eid));
    }
    Stat::new(out)
}

/// Edge order: `|head ∪ tail| - 1`. XGI `diedgestats.order`.
pub fn di_edge_order(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        out.insert(eid.clone(), union_size(d, &eid).saturating_sub(1));
    }
    Stat::new(out)
}

/// Head size: `|head|`. XGI `diedgestats.head_size`.
pub fn di_head_size(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        out.insert(eid.clone(), d.head(&eid).len());
    }
    Stat::new(out)
}

/// Tail size: `|tail|`. XGI `diedgestats.tail_size`.
pub fn di_tail_size(d: &DiHypergraph) -> Stat<usize> {
    let mut out: IndexMap<String, usize> = IndexMap::new();
    for eid in d.edge_ids() {
        out.insert(eid.clone(), d.tail(&eid).len());
    }
    Stat::new(out)
}
```

Add to `stats/mod.rs`: `pub use dinodestats::{di_degree, di_in_degree, di_out_degree};` and `pub use diedgestats::{di_edge_order, di_edge_size, di_head_size, di_tail_size};`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_stats`
Expected: PASS — 12 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): directed node/edge stats (di in/out degree, head/tail size)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 13: readwrite JSON — to/from_hypergraph_dict, write/read_json

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/json.rs`

**Interfaces:** `to_hypergraph_dict(h: &Hypergraph) -> serde_json::Value`, `from_hypergraph_dict(data: &serde_json::Value) -> Result<Hypergraph, ReadWriteError>`, `write_json(h: &Hypergraph, path: &Path) -> Result<(), ReadWriteError>`, `read_json(path: &Path) -> Result<Hypergraph, ReadWriteError>`.

**XGI source / schema:** `convert/hypergraph_dict.py` — the xgi-data JSON is `{"type": <netname>, "hypergraph-data": {...}, "node-data": {id: attrs}, "edge-data": {id: attrs}, "edge-dict": {id: [sorted-member-strings]}}`. `from_hypergraph_dict` adds nodes from `node-data`, edges from `edge-dict` (passing each edge's `edge-data` attrs straight into `add_edge`, not a separate merge pass — see Step 3), then applies `edge-data` attrs. IDs are strings. `"type"` is XGI's lowercased class name (`get_network_type`), i.e. `"hypergraph"`. `"hypergraph-data"` mirrors the FULL graph-attr dict (`H._net_attr`), not just `"name"`.

**Core API note:** `to_hypergraph_dict` needs to serialize every graph attribute, not just `"name"`. Grep the landed core for a full-map accessor (`rg "fn graph_attrs" crates/hypergraph-rs/src/core`); if the only accessor is the single-key `graph_attr`/`set_graph_attr` pair, add `pub fn graph_attrs(&self) -> &serde_json::Map<String, Value>` to the core (mirrors those two, landed Phase 0+1 code) before implementing this task.

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::json::{from_hypergraph_dict, read_json, to_hypergraph_dict, write_json};
use hypergraph_rs::Hypergraph;

fn sample() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        serde_json::json!({"weight": 2}),
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into()],
        Some("e1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.set_graph_attr("name", serde_json::json!("sample"));
    h.set_graph_attr("note", serde_json::json!("second graph attr"));
    h
}

#[test]
fn test_to_hypergraph_dict_schema() {
    let h = sample();
    let d = to_hypergraph_dict(&h);
    assert_eq!(d["type"], serde_json::json!("hypergraph"));
    assert_eq!(d["hypergraph-data"]["name"], serde_json::json!("sample"));
    // every graph attr is serialized, not just "name" (XGI updates the whole
    // hypergraph-data dict from H._net_attr).
    assert_eq!(
        d["hypergraph-data"]["note"],
        serde_json::json!("second graph attr")
    );
    assert!(d["node-data"].get("1").is_some());
    assert!(d["edge-data"].get("e0").is_some());
    // edge-dict members are sorted strings
    assert_eq!(
        d["edge-dict"]["e0"],
        serde_json::json!(["1", "2", "3"])
    );
}

#[test]
fn test_json_round_trip_preserves_structure() {
    let h = sample();
    let d = to_hypergraph_dict(&h);
    let h2 = from_hypergraph_dict(&d).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    // members as sets
    let mut m = h2.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    assert_eq!(h2.edge_attrs("e0").unwrap()["weight"], 2);
    assert_eq!(h2.graph_attr("name"), Some(&serde_json::json!("sample")));
    assert_eq!(
        h2.graph_attr("note"),
        Some(&serde_json::json!("second graph attr"))
    );
}

#[test]
fn test_write_read_json_file_round_trip() {
    let h = sample();
    let dir = std::env::temp_dir();
    let path = dir.join("hypergraph_rs_test_json.json");
    write_json(&h, &path).unwrap();
    let h2 = read_json(&path).unwrap();
    assert_eq!(h2.num_nodes(), h.num_nodes());
    assert_eq!(h2.num_edges(), h.num_edges());
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `json.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/json.rs`:

```rust
//! Read/write the xgi-data standardized JSON format (XGI `readwrite/json.py`
//! + `convert/hypergraph_dict.py`).

use std::path::Path;

use serde_json::{json, Map, Value};

use super::ReadWriteError;
use crate::Hypergraph;

/// Serialize a hypergraph to the xgi-data dict schema. XGI `to_hypergraph_dict`.
pub fn to_hypergraph_dict(h: &Hypergraph) -> Value {
    let mut node_data = Map::new();
    for nid in h.node_ids() {
        node_data.insert(nid.clone(), h.node_attrs(&nid).cloned().unwrap_or(Value::Null));
    }
    let mut edge_data = Map::new();
    let mut edge_dict = Map::new();
    for eid in h.edge_ids() {
        edge_data.insert(eid.clone(), h.edge_attrs(&eid).cloned().unwrap_or(Value::Null));
        let mut members = h.members(&eid).unwrap_or_default();
        members.sort();
        edge_dict.insert(eid.clone(), json!(members));
    }
    // H.graph dict — copy the FULL graph-attr map (XGI:
    // `data["hypergraph-data"].update(H._net_attr)`), not just "name".
    let hgraph_data = h.graph_attrs().clone();

    json!({
        "type": "hypergraph",
        "hypergraph-data": Value::Object(hgraph_data),
        "node-data": Value::Object(node_data),
        "edge-data": Value::Object(edge_data),
        "edge-dict": Value::Object(edge_dict),
    })
}

/// Rebuild a hypergraph from the xgi-data dict schema. XGI `from_hypergraph_dict`.
pub fn from_hypergraph_dict(data: &Value) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();

    if let Some(hd) = data.get("hypergraph-data").and_then(|v| v.as_object()) {
        for (k, v) in hd {
            h.set_graph_attr(k, v.clone());
        }
    }

    let node_data = data
        .get("node-data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ReadWriteError::Format("missing node-data".into()))?;
    for (nid, attrs) in node_data {
        h.add_node(nid, attrs.clone());
    }

    let edge_dict = data
        .get("edge-dict")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ReadWriteError::Format("missing edge-dict".into()))?;
    let edge_data = data.get("edge-data").and_then(|v| v.as_object());
    for (eid, members) in edge_dict {
        let arr = members
            .as_array()
            .ok_or_else(|| ReadWriteError::Format(format!("edge {eid} members not an array")))?;
        let member_ids: Vec<String> = arr
            .iter()
            .map(|m| match m {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        // Pass the edge's attrs straight into add_edge rather than seeding
        // Value::Null and patching via set_edge_attributes afterward: that
        // merge path calls `.as_object_mut()` on the existing attrs, which is
        // `None` for `Value::Null` and silently no-ops (P4-01 fix).
        let attrs = edge_data
            .and_then(|ed| ed.get(eid))
            .cloned()
            .unwrap_or(Value::Null);
        h.add_edge(member_ids, Some(eid.clone()), attrs)
            .map_err(|e| ReadWriteError::Format(format!("edge {eid}: {e}")))?;
    }

    Ok(h)
}

/// Write a hypergraph to a pretty-printed JSON file. XGI `write_json`.
pub fn write_json(h: &Hypergraph, path: &Path) -> Result<(), ReadWriteError> {
    let data = to_hypergraph_dict(h);
    let s = serde_json::to_string_pretty(&data)?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Read a hypergraph from a JSON file. XGI `read_json`.
pub fn read_json(path: &Path) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&s)?;
    from_hypergraph_dict(&data)
}
```

Uncomment the `json` re-export line (`// TASK 13: uncomment...`) in `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 3 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite JSON (xgi-data schema round-trip)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 14: readwrite edgelist — write/read/parse

**Files:**
- Modify: `tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/edgelist.rs`

**Interfaces:** `write_edgelist(h: &Hypergraph, path: &Path, delimiter: char) -> Result<(), ReadWriteError>`, `read_edgelist(path: &Path, delimiter: Option<char>, comments: &str) -> Result<Hypergraph, ReadWriteError>`, `parse_edgelist(lines: impl IntoIterator<Item = String>, delimiter: Option<char>, comments: &str) -> Result<Hypergraph, ReadWriteError>`.

**XGI source:** `edgelist.py` — one edge per line, members joined by the delimiter; `parse_edgelist` strips `comments`, splits on the delimiter (whitespace if `None`), and `add_edge`s each line.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::edgelist::{parse_edgelist, read_edgelist, write_edgelist};

#[test]
fn test_parse_edgelist_whitespace() {
    let lines = vec!["1 2 3".to_string(), "2 3 4 5".to_string(), "# a comment".to_string()];
    let h = parse_edgelist(lines, None, "#").unwrap();
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 5);
}

#[test]
fn test_parse_edgelist_custom_delimiter() {
    let lines = vec!["a,b,c".to_string(), "b,c".to_string()];
    let h = parse_edgelist(lines, Some(','), "#").unwrap();
    assert_eq!(h.num_edges(), 2);
    let mut m = h.members("0").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_write_read_edgelist_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], None, serde_json::Value::Null).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], None, serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_edgelist.csv");
    write_edgelist(&h, &path, ',').unwrap();
    let h2 = read_edgelist(&path, Some(','), "#").unwrap();
    assert_eq!(h2.num_edges(), 2);
    assert_eq!(h2.num_nodes(), 4);
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `edgelist.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/edgelist.rs`:

```rust
//! Read/write hyperedge lists (XGI `readwrite/edgelist.py`).

use std::path::Path;

use serde_json::Value;

use super::ReadWriteError;
use crate::Hypergraph;

/// Split a line on `delimiter` (whitespace when `None`), dropping empties.
fn split_line(line: &str, delimiter: Option<char>) -> Vec<String> {
    match delimiter {
        Some(d) => line.split(d).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        None => line.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

/// Strip the comment suffix (everything at/after the first `comments` token).
fn strip_comment(line: &str, comments: &str) -> String {
    match line.find(comments) {
        Some(p) => line[..p].to_string(),
        None => line.to_string(),
    }
}

/// Parse an iterable of lines into a hypergraph (one edge per line). XGI `parse_edgelist`.
pub fn parse_edgelist(
    lines: impl IntoIterator<Item = String>,
    delimiter: Option<char>,
    comments: &str,
) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();
    for raw in lines {
        let line = strip_comment(&raw, comments);
        let members = split_line(&line, delimiter);
        if members.is_empty() {
            continue;
        }
        h.add_edge(members, None, Value::Null)
            .map_err(|e| ReadWriteError::Format(format!("bad edge line: {e}")))?;
    }
    Ok(h)
}

/// Read a hypergraph from an edgelist file. XGI `read_edgelist`.
pub fn read_edgelist(
    path: &Path,
    delimiter: Option<char>,
    comments: &str,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    parse_edgelist(s.lines().map(|l| l.to_string()), delimiter, comments)
}

/// Write a hypergraph as an edgelist file (members joined by `delimiter`). XGI `write_edgelist`.
pub fn write_edgelist(h: &Hypergraph, path: &Path, delimiter: char) -> Result<(), ReadWriteError> {
    let mut out = String::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        out.push_str(&members.join(&delimiter.to_string()));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}
```

Uncomment the `edgelist` re-export line (`// TASK 14: uncomment...`) in `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 6 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite edgelist (write/read/parse)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 15: readwrite bipartite edgelist — write/read/parse

**Files:**
- Modify: `tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/bipartite.rs`

**Interfaces:** `write_bipartite_edgelist(h, path, delimiter) -> Result<(), ReadWriteError>`, `read_bipartite_edgelist(path, delimiter, comments, dual) -> Result<Hypergraph, ReadWriteError>`, `parse_bipartite_edgelist(lines, delimiter, comments, dual) -> Result<Hypergraph, ReadWriteError>`.

**XGI source:** `bipartite.py` — each line is `node<delim>edge` (or `edge<delim>node` when `dual`); write emits one `node edge` line per membership; parse requires `>= 2` entries per line and calls `add_node_to_edge(edge, node)`.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::bipartite::{
    parse_bipartite_edgelist, read_bipartite_edgelist, write_bipartite_edgelist,
};

#[test]
fn test_parse_bipartite_basic() {
    // node edge pairs: 1->e0, 2->e0, 2->e1
    let lines = vec!["1 e0".to_string(), "2 e0".to_string(), "2 e1".to_string()];
    let h = parse_bipartite_edgelist(lines, None, "#", false).unwrap();
    assert_eq!(h.num_nodes(), 2);
    assert_eq!(h.num_edges(), 2);
    let mut m = h.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn test_parse_bipartite_dual_swaps_columns() {
    // dual: first column is edge, second is node
    let lines = vec!["e0 1".to_string(), "e0 2".to_string()];
    let h = parse_bipartite_edgelist(lines, None, "#", true).unwrap();
    assert_eq!(h.num_edges(), 1);
    let mut m = h.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn test_parse_bipartite_rejects_short_line() {
    let lines = vec!["only-one-token".to_string()];
    assert!(parse_bipartite_edgelist(lines, None, "#", false).is_err());
}

#[test]
fn test_write_read_bipartite_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_bipartite.csv");
    write_bipartite_edgelist(&h, &path, ' ').unwrap();
    let h2 = read_bipartite_edgelist(&path, None, "#", false).unwrap();
    assert_eq!(h2.num_edges(), 1);
    assert_eq!(h2.num_nodes(), 2);
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `bipartite.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/bipartite.rs`:

```rust
//! Read/write bipartite (node, edge) edgelists (XGI `readwrite/bipartite.py`).

use std::path::Path;

use super::ReadWriteError;
use crate::Hypergraph;

fn split_line(line: &str, delimiter: Option<char>) -> Vec<String> {
    match delimiter {
        Some(d) => line.split(d).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        None => line.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

fn strip_comment(line: &str, comments: &str) -> String {
    match line.find(comments) {
        Some(p) => line[..p].to_string(),
        None => line.to_string(),
    }
}

/// Parse a bipartite edgelist. Each line has `>= 2` tokens; the node column is 0
/// and the edge column is 1 (swapped when `dual`). XGI `parse_bipartite_edgelist`.
pub fn parse_bipartite_edgelist(
    lines: impl IntoIterator<Item = String>,
    delimiter: Option<char>,
    comments: &str,
    dual: bool,
) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();
    let (node_col, edge_col) = if dual { (1, 0) } else { (0, 1) };
    for raw in lines {
        let line = strip_comment(&raw, comments);
        let toks = split_line(&line, delimiter);
        if toks.is_empty() {
            continue;
        }
        if toks.len() < 2 {
            return Err(ReadWriteError::Format(
                "each line must contain at least two entries".into(),
            ));
        }
        let node = &toks[node_col];
        let edge = &toks[edge_col];
        h.add_node_to_edge(edge, node)
            .map_err(|e| ReadWriteError::Format(format!("bad incidence: {e}")))?;
    }
    Ok(h)
}

/// Read a bipartite edgelist file. XGI `read_bipartite_edgelist`.
pub fn read_bipartite_edgelist(
    path: &Path,
    delimiter: Option<char>,
    comments: &str,
    dual: bool,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    parse_bipartite_edgelist(s.lines().map(|l| l.to_string()), delimiter, comments, dual)
}

/// Write a bipartite edgelist (`node<delim>edge` per membership). XGI `write_bipartite_edgelist`.
pub fn write_bipartite_edgelist(
    h: &Hypergraph,
    path: &Path,
    delimiter: char,
) -> Result<(), ReadWriteError> {
    let mut out = String::new();
    for eid in h.edge_ids() {
        for node in h.members(&eid).unwrap_or_default() {
            out.push_str(&node);
            out.push(delimiter);
            out.push_str(&eid);
            out.push('\n');
        }
    }
    std::fs::write(path, out)?;
    Ok(())
}
```

Uncomment the `bipartite` re-export line (`// TASK 15: uncomment...`) in `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 10 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite bipartite edgelist (write/read/parse + dual)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 16: readwrite incidence matrix — write/read

**Files:**
- Modify: `tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/incidence.rs`

**Interfaces:** `write_incidence_matrix(h: &Hypergraph, path: &Path, delimiter: char) -> Result<(), ReadWriteError>`, `read_incidence_matrix(path: &Path, delimiter: Option<char>) -> Result<Hypergraph, ReadWriteError>`.

**XGI source:** `incidence.py` — writes the dense incidence matrix (rows=nodes, cols=edges), reads it back via `from_incidence_matrix`. **Deviation (documented above):** we hand-roll the dense 0/1 matrix (no `ndarray`/`sprs`), emitting integer cells; the reader treats any non-zero cell as membership. Node/edge ids are regenerated as `"0".."n-1"` on read (a bare matrix has no id labels — matches XGI `from_incidence_matrix`, which keys by position).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::incidence::{read_incidence_matrix, write_incidence_matrix};

#[test]
fn test_write_read_incidence_round_trip() {
    // 3 nodes, 2 edges: edge0={0,1}, edge1={1,2}
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_incidence.csv");
    write_incidence_matrix(&h, &path, ' ').unwrap();
    let h2 = read_incidence_matrix(&path, None).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    // positional edge "0" has members {0,1}
    let mut m = h2.members("0").unwrap();
    m.sort();
    assert_eq!(m, vec!["0".to_string(), "1".to_string()]);
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `incidence.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/incidence.rs`:

```rust
//! Read/write dense incidence matrices (XGI `readwrite/incidence.py`).
//!
//! Deviation from XGI: we emit integer 0/1 cells (not numpy `%.18e`) and read any
//! non-zero cell as membership. Our writer + reader round-trip together; node/edge
//! ids are positional ("0".."k-1") on read, matching `from_incidence_matrix`.

use std::path::Path;

use serde_json::Value;

use super::ReadWriteError;
use crate::Hypergraph;

/// Write the dense incidence matrix: rows = nodes (node-id order), cols = edges
/// (edge-id order), cell = 1 if the node is a member. XGI `write_incidence_matrix`.
pub fn write_incidence_matrix(
    h: &Hypergraph,
    path: &Path,
    delimiter: char,
) -> Result<(), ReadWriteError> {
    let nodes = h.node_ids();
    let edges = h.edge_ids();
    // membership lookup: for each edge, the set of member node ids
    let edge_members: Vec<std::collections::HashSet<String>> = edges
        .iter()
        .map(|e| h.members(e).unwrap_or_default().into_iter().collect())
        .collect();

    let mut out = String::new();
    for node in &nodes {
        let row: Vec<&str> = edge_members
            .iter()
            .map(|set| if set.contains(node) { "1" } else { "0" })
            .collect();
        out.push_str(&row.join(&delimiter.to_string()));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// Read a dense incidence matrix into a hypergraph with positional ids.
/// XGI `read_incidence_matrix` (via `from_incidence_matrix`).
pub fn read_incidence_matrix(
    path: &Path,
    delimiter: Option<char>,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let rows: Vec<Vec<f64>> = s
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let cells: Vec<&str> = match delimiter {
                Some(d) => line.split(d).map(|c| c.trim()).filter(|c| !c.is_empty()).collect(),
                None => line.split_whitespace().collect(),
            };
            cells
                .iter()
                .map(|c| c.parse::<f64>().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let n_nodes = rows.len();
    let n_edges = rows.first().map(|r| r.len()).unwrap_or(0);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n_nodes).map(|i| (i.to_string(), Value::Null)));

    // Column-major: for each edge (col), collect member nodes (rows with nonzero).
    for e in 0..n_edges {
        let members: Vec<String> = (0..n_nodes)
            .filter(|&r| rows[r].get(e).copied().unwrap_or(0.0) != 0.0)
            .map(|r| r.to_string())
            .collect();
        if members.is_empty() {
            continue; // our core rejects empty edges; skip all-zero columns
        }
        h.add_edge(members, Some(e.to_string()), Value::Null)
            .map_err(|err| ReadWriteError::Format(format!("column {e}: {err}")))?;
    }
    Ok(h)
}
```

Uncomment the `incidence` re-export line (`// TASK 16: uncomment...`) in `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 11 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite incidence matrix (dense 0/1 round-trip)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 17: readwrite HIF — to/from_hif_dict, write/read_hif (undirected)

**Files:**
- Modify: `tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/hif.rs`

**Interfaces:** `to_hif_dict(h: &Hypergraph) -> serde_json::Value`, `from_hif_dict(data: &serde_json::Value) -> Result<Hypergraph, ReadWriteError>`, `write_hif(h, path) -> Result<(), ReadWriteError>`, `read_hif(path) -> Result<Hypergraph, ReadWriteError>`.

**XGI source / schema:** `convert/hif_dict.py` — `{"metadata": {...}, "network-type": "undirected", "nodes": [{"node": id, "attrs": {...}}], "edges": [{"edge": id, "attrs": {...}}], "incidences": [{"edge": e, "node": n}]}`. Only nodes with attrs (or isolates) and edges with attrs (or empties) appear in `nodes`/`edges`; ALL memberships appear in `incidences`. `from_hif_dict` builds via `add_node_to_edge` then applies attrs. **Directed HIF (`network-type: directed`) needs DiHypergraph — out of scope here; `from_hif_dict` returns `Format` error for non-undirected/asc types.** (`asc` is accepted and loaded as a plain Hypergraph — the SimplicialComplex promotion is deferred to when SimplicialComplex round-trip lands.)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::hif::{from_hif_dict, read_hif, to_hif_dict, write_hif};

#[test]
fn test_to_hif_dict_schema() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), serde_json::json!({"w": 3})).unwrap();
    h.add_node("iso", serde_json::json!({"color": "red"}));
    h.set_graph_attr("name", serde_json::json!("hif-sample"));
    h.set_graph_attr("note", serde_json::json!("second graph attr"));
    let d = to_hif_dict(&h);
    assert_eq!(d["network-type"], serde_json::json!("undirected"));
    // every graph attr is serialized into metadata, not just "name"
    assert_eq!(d["metadata"]["name"], serde_json::json!("hif-sample"));
    assert_eq!(d["metadata"]["note"], serde_json::json!("second graph attr"));
    // two incidences for e0
    let inc = d["incidences"].as_array().unwrap();
    assert_eq!(inc.len(), 2);
    // isolate node with attrs appears in nodes
    let nodes = d["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["node"] == serde_json::json!("iso")));
    // edge with attrs appears in edges
    let edges = d["edges"].as_array().unwrap();
    assert!(edges.iter().any(|e| e["edge"] == serde_json::json!("e0")));
}

#[test]
fn test_hif_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), serde_json::json!({"w": 3})).unwrap();
    h.add_edge(vec!["2".into(), "3".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let d = to_hif_dict(&h);
    let h2 = from_hif_dict(&d).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    assert_eq!(h2.edge_attrs("e0").unwrap()["w"], 3);
}

#[test]
fn test_write_read_hif_file() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_hif.json");
    write_hif(&h, &path).unwrap();
    let h2 = read_hif(&path).unwrap();
    assert_eq!(h2.num_nodes(), 2);
    assert_eq!(h2.num_edges(), 1);
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `hif.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/hif.rs`:

```rust
//! Read/write the HIF standard (XGI `readwrite/hif.py` + `convert/hif_dict.py`).
//! Undirected only; directed HIF requires DiHypergraph (deferred).

use std::path::Path;

use serde_json::{json, Map, Value};

use super::ReadWriteError;
use crate::Hypergraph;

/// True if a serde object is present and non-empty.
fn nonempty_obj(v: &Value) -> bool {
    v.as_object().map(|o| !o.is_empty()).unwrap_or(false)
}

/// Serialize a hypergraph to the HIF dict. XGI `to_hif_dict` (undirected branch).
pub fn to_hif_dict(h: &Hypergraph) -> Value {
    // metadata from graph attrs — the FULL map (XGI: `data["metadata"].update(H._net_attr)`),
    // not just "name" (see the Task 13 core-API note re: `graph_attrs()`).
    let metadata = h.graph_attrs().clone();

    // nodes: isolates OR nodes with attrs
    let mut nodes = Vec::new();
    for nid in h.node_ids() {
        let attrs = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
        let is_isolate = h.memberships(&nid).map(|m| m.is_empty()).unwrap_or(true);
        if is_isolate || nonempty_obj(&attrs) {
            let mut rec = Map::new();
            rec.insert("node".to_string(), json!(nid));
            if nonempty_obj(&attrs) {
                rec.insert("attrs".to_string(), attrs);
            }
            nodes.push(Value::Object(rec));
        }
    }

    // edges: empty edges OR edges with attrs
    let mut edges = Vec::new();
    let mut incidences = Vec::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        let attrs = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
        if members.is_empty() || nonempty_obj(&attrs) {
            let mut rec = Map::new();
            rec.insert("edge".to_string(), json!(eid));
            if nonempty_obj(&attrs) {
                rec.insert("attrs".to_string(), attrs);
            }
            edges.push(Value::Object(rec));
        }
        for node in members {
            incidences.push(json!({"edge": eid, "node": node}));
        }
    }

    json!({
        "metadata": Value::Object(metadata),
        "network-type": "undirected",
        "nodes": nodes,
        "edges": edges,
        "incidences": incidences,
    })
}

/// Rebuild a hypergraph from a HIF dict. XGI `from_hif_dict` (undirected/asc branch).
pub fn from_hif_dict(data: &Value) -> Result<Hypergraph, ReadWriteError> {
    let network_type = data
        .get("network-type")
        .and_then(|v| v.as_str())
        .unwrap_or("undirected");
    if network_type == "directed" {
        return Err(ReadWriteError::Format(
            "directed HIF requires DiHypergraph (not supported in Phase 4)".into(),
        ));
    }

    let mut h = Hypergraph::new();

    if let Some(md) = data.get("metadata").and_then(|v| v.as_object()) {
        for (k, v) in md {
            h.set_graph_attr(k, v.clone());
        }
    }

    let incidences = data
        .get("incidences")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("missing incidences".into()))?;
    for rec in incidences {
        let node = rec.get("node").and_then(as_id).ok_or_else(|| {
            ReadWriteError::Format("incidence missing node".into())
        })?;
        let edge = rec.get("edge").and_then(as_id).ok_or_else(|| {
            ReadWriteError::Format("incidence missing edge".into())
        })?;
        h.add_node_to_edge(&edge, &node)
            .map_err(|e| ReadWriteError::Format(format!("incidence: {e}")))?;
    }

    if let Some(nodes) = data.get("nodes").and_then(|v| v.as_array()) {
        for rec in nodes {
            if let Some(nid) = rec.get("node").and_then(as_id) {
                let attrs = rec.get("attrs").cloned();
                if !h.has_node(&nid) {
                    h.add_node(&nid, attrs.unwrap_or(Value::Null));
                } else if let Some(a) = attrs {
                    // Assign the attrs object wholesale rather than merging via
                    // set_node_attributes: that merge calls `.as_object_mut()`
                    // on the existing (Value::Null, from add_node_to_edge's
                    // default) attrs, which returns None and silently no-ops
                    // (P4-01 fix).
                    if let Some(slot) = h.node_attrs_mut(&nid) {
                        *slot = a;
                    }
                }
            }
        }
    }

    if let Some(edges) = data.get("edges").and_then(|v| v.as_array()) {
        for rec in edges {
            if let Some(eid) = rec.get("edge").and_then(as_id) {
                let attrs = rec.get("attrs").cloned();
                if h.has_edge(&eid) {
                    if let Some(a) = attrs {
                        // Same direct-assignment fix as above (P4-01).
                        if let Some(slot) = h.edge_attrs_mut(&eid) {
                            *slot = a;
                        }
                    }
                }
                // empty edges (in `edges` but no incidences) can't be represented
                // by our core (add_edge rejects empty members) — skipped, documented.
            }
        }
    }

    Ok(h)
}

/// Coerce a JSON id (string or number) to a String key.
fn as_id(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Write a hypergraph to a HIF JSON file. XGI `write_hif`.
pub fn write_hif(h: &Hypergraph, path: &Path) -> Result<(), ReadWriteError> {
    let data = to_hif_dict(h);
    std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

/// Read a hypergraph from a HIF JSON file. XGI `read_hif`.
pub fn read_hif(path: &Path) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&s)?;
    from_hif_dict(&data)
}
```

Note: `node_attrs_mut(&self, id: &str) -> Option<&mut Value>` / `edge_attrs_mut(&self, id: &str) -> Option<&mut Value>` (Phase 0+1, landed) return a mutable reference to the raw attrs slot — assigning through it (`*slot = a`) replaces the value directly, unlike `set_node_attributes`/`set_edge_attributes`, whose object-merge is a no-op when the existing attrs are `Value::Null` (P4-01). If the real accessor names differ, grep first (`rg "fn (node|edge)_attrs_mut" crates/hypergraph-rs/src/core`) and adapt.

Uncomment the `hif` re-export line (`// TASK 17: uncomment...`) in `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 14 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite HIF (undirected to/from_hif_dict + file I/O)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 18: readwrite BIGG — offline dict → DiHypergraph parse (network-gated fetch)

**Files:**
- Modify: `tests/test_readwrite.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/readwrite/bigg.rs`
- Modify: `readwrite/mod.rs` (add `pub use bigg::bigg_dict_to_dihypergraph;`)

**DEPENDS ON Phase 2 `DiHypergraph`.** Grep first: `rg "impl.*DiHypergraph" crates/hypergraph-rs/src`. Assumes `DiHypergraph::new()`, `add_node(&str, Value) -> bool`, `add_edge(tail: Vec<String>, head: Vec<String>, idx: Option<String>, attrs: Value) -> Result<String, _>`, `set_graph_attr(&str, Value)`. **If DiHypergraph is not landed, SKIP and record in the handoff.** Network fetch (`load_bigg_data`) is behind the `network` feature and OUT OF SCOPE (leave a `#[cfg(feature = "network")]` stub returning a `Format` "network feature not enabled" error) — only the offline model-dict → DiHypergraph parse is implemented and tested here.

**XGI source:** `bigg_data.py:75` `_bigg_to_dihypergraph` — metabolites → nodes; each reaction becomes a directed edge with reactants as TAIL and products as HEAD, direction decided by `lower_bound`/`upper_bound`: forward (`l>=0 && u>0`), reverse (`l<0 && u<=0`), reversible (`l<0 && u>0`, emits both `id` and `id_reverse`).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_readwrite.rs`:

```rust
use hypergraph_rs::readwrite::bigg::bigg_dict_to_dihypergraph;

#[test]
fn test_bigg_forward_reaction() {
    // minimal BIGG model dict: metabolites A,B; one forward reaction A -> B
    let model = serde_json::json!({
        "id": "toy",
        "metabolites": [{"id": "A", "name": "alpha"}, {"id": "B", "name": "beta"}],
        "reactions": [{
            "id": "r1", "name": "reaction one",
            "lower_bound": 0.0, "upper_bound": 1000.0,
            "metabolites": {"A": -1.0, "B": 1.0}
        }]
    });
    let dh = bigg_dict_to_dihypergraph(&model, None).unwrap();
    assert_eq!(dh.num_nodes(), 2);
    assert_eq!(dh.num_edges(), 1);
    // A is reactant (tail), B is product (head)
    assert_eq!(dh.tail("r1"), vec!["A".to_string()]);
    assert_eq!(dh.head("r1"), vec!["B".to_string()]);
}

#[test]
fn test_bigg_reversible_reaction_emits_two_edges() {
    let model = serde_json::json!({
        "id": "toy",
        "metabolites": [{"id": "A", "name": "alpha"}, {"id": "B", "name": "beta"}],
        "reactions": [{
            "id": "r1", "name": "rev",
            "lower_bound": -1000.0, "upper_bound": 1000.0,
            "metabolites": {"A": -1.0, "B": 1.0}
        }]
    });
    let dh = bigg_dict_to_dihypergraph(&model, None).unwrap();
    assert_eq!(dh.num_edges(), 2);
    assert!(dh.has_edge("r1"));
    assert!(dh.has_edge("r1_reverse"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: FAIL.

- [ ] **Step 3: Implement `bigg.rs`**

Write to `hypergraph-rs/crates/hypergraph-rs/src/readwrite/bigg.rs` (adjust DiHypergraph method names to the real Phase-2 API):

```rust
//! BIGG metabolic-network parsing (XGI `readwrite/bigg_data.py`).
//!
//! Offline dict → DiHypergraph. The HTTP fetch (`load_bigg_data`) is behind the
//! `network` feature and unimplemented (see the plan).

use serde_json::Value;

use super::ReadWriteError;
use crate::DiHypergraph;

/// Convert a BIGG model dict (optionally with the index dict for organism metadata)
/// into a DiHypergraph. XGI `_bigg_to_dihypergraph`.
pub fn bigg_dict_to_dihypergraph(
    model: &Value,
    index: Option<&Value>,
) -> Result<DiHypergraph, ReadWriteError> {
    let mut dh = DiHypergraph::new();

    let id = model
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing id".into()))?;
    dh.set_graph_attr("name", Value::String(id.to_string()));

    if let Some(idx) = index {
        if let Some(results) = idx.get("results").and_then(|v| v.as_array()) {
            if let Some(info) = results.iter().find(|it| it.get("bigg_id").and_then(|v| v.as_str()) == Some(id)) {
                if let Some(org) = info.get("organism") {
                    dh.set_graph_attr("organism", org.clone());
                }
            }
        }
    }

    // metabolites -> nodes
    let metabolites = model
        .get("metabolites")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing metabolites".into()))?;
    for m in metabolites {
        if let Some(mid) = m.get("id").and_then(|v| v.as_str()) {
            let name = m.get("name").cloned().unwrap_or(Value::Null);
            dh.add_node(mid, serde_json::json!({ "name": name }));
        }
    }

    // reactions -> directed edges
    let reactions = model
        .get("reactions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing reactions".into()))?;
    for r in reactions {
        let rid = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let l = r.get("lower_bound").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let u = r.get("upper_bound").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let coeffs = match r.get("metabolites").and_then(|v| v.as_object()) {
            Some(o) => o,
            None => continue,
        };

        let split = |positive_is_product: bool| -> (Vec<String>, Vec<String>) {
            let mut reactants = Vec::new();
            let mut products = Vec::new();
            for (met, val) in coeffs {
                let v = val.as_f64().unwrap_or(0.0);
                let is_product = if positive_is_product { v > 0.0 } else { v >= 0.0 };
                if is_product {
                    products.push(met.clone());
                } else {
                    reactants.push(met.clone());
                }
            }
            (reactants, products)
        };

        // forward: l>=0 && u>0
        if l >= 0.0 && u > 0.0 {
            let (reactants, products) = split(true);
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(reactants, products, Some(rid.to_string()), Value::Null);
            }
        }
        // reverse: l<0 && u<=0  (positive coeff -> reactant)
        if l < 0.0 && u <= 0.0 {
            let (products, reactants) = {
                // positive_is_product=false flips roles per XGI's reverse branch
                let mut reactants = Vec::new();
                let mut products = Vec::new();
                for (met, val) in coeffs {
                    let v = val.as_f64().unwrap_or(0.0);
                    if v >= 0.0 {
                        reactants.push(met.clone());
                    } else {
                        products.push(met.clone());
                    }
                }
                (products, reactants)
            };
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(reactants, products, Some(rid.to_string()), Value::Null);
            }
        }
        // reversible: l<0 && u>0 -> forward + <id>_reverse
        // XGI's reversible branch classifies only val>0 -> product, val<0 ->
        // reactant, DROPPING any metabolite with coefficient exactly 0 — unlike
        // the forward/reverse branches (and `split`), it does not lump v==0 into
        // reactants. Mirror that exactly here instead of reusing split(true).
        if l < 0.0 && u > 0.0 {
            let mut reactants = Vec::new();
            let mut products = Vec::new();
            for (met, val) in coeffs {
                let v = val.as_f64().unwrap_or(0.0);
                if v > 0.0 {
                    products.push(met.clone());
                } else if v < 0.0 {
                    reactants.push(met.clone());
                }
                // v == 0.0: dropped, matching XGI's reversible branch.
            }
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(
                    reactants.clone(),
                    products.clone(),
                    Some(rid.to_string()),
                    Value::Null,
                );
                let _ = dh.add_edge(
                    products,
                    reactants,
                    Some(format!("{rid}_reverse")),
                    Value::Null,
                );
            }
        }
    }
    Ok(dh)
}

/// Fetch a BIGG dataset over HTTP. Behind the `network` feature (OFF by default).
#[cfg(feature = "network")]
pub fn load_bigg_data(_dataset: &str) -> Result<DiHypergraph, ReadWriteError> {
    Err(ReadWriteError::Format(
        "load_bigg_data HTTP fetch not implemented in Phase 4".into(),
    ))
}
```

Add `pub use bigg::bigg_dict_to_dihypergraph;` to `readwrite/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_readwrite`
Expected: PASS — 16 tests.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): readwrite BIGG offline dict → DiHypergraph parse

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 19: Feature-matrix build check, clippy, fmt, full suite

**Files:** any files with clippy warnings.

- [ ] **Step 1: Build each feature slice to prove the gates are clean**

Run all four; each must build:
```bash
cd hypergraph-rs
cargo build -p hypergraph-rs --no-default-features
cargo build -p hypergraph-rs --no-default-features --features generators
cargo build -p hypergraph-rs --no-default-features --features stats
cargo build -p hypergraph-rs --no-default-features --features readwrite
cargo build -p hypergraph-rs --all-features
```
Expected: all succeed. `--no-default-features` (no generators/stats/readwrite) compiles the bare core. If a module leaks a symbol across a feature boundary (e.g. `readwrite` using a `stats` type), gate or move it until each slice is independent.

- [ ] **Step 2: Clippy on the workspace**

Run: `cd hypergraph-rs && cargo clippy --workspace --all-features -- -D warnings`
Expected: fix all warnings. Likely candidates: `clippy::needless_range_loop` in the combination enumerators (allow with a one-line justification — the index arithmetic is intentional), `clippy::type_complexity` on the split closures, `clippy::too_many_arguments` on `uniform_erdos_renyi_hypergraph` / `watts_strogatz_hypergraph` (allow with a comment — the arity mirrors XGI's signature).

- [ ] **Step 3: rustfmt**

Run: `cd hypergraph-rs && cargo fmt --all`

- [ ] **Step 4: Full test suite**

Run: `cd hypergraph-rs && cargo test --workspace --all-features`
Expected: PASS — Phase 0+1's 64 core tests + Phase 4's `test_generators` (42), `test_stats` (12), `test_readwrite` (16) = **134 new-and-existing tests green** (plus whatever Phase 2/3 added). If the Phase-2-dependent tasks (9, 12, 18) were skipped because `DiHypergraph`/`SimplicialComplex` weren't landed, subtract their tests and note the skip in the handoff.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs): Phase 4 clippy + fmt + feature-matrix cleanup

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 4 Completion Summary

After all 19 tasks, `hypergraph-rs` gains three feature-gated modules:

- **`generators`** — `empty_hypergraph`, `trivial_hypergraph`, `complete_hypergraph`, `complement`, `ring_lattice`, `star_clique`, `sunflower`, `fast_random_hypergraph`, `random_hypergraph`, `chung_lu_hypergraph`, `watts_strogatz_hypergraph`, `uniform_erdos_renyi_hypergraph`, `uniform_hypergraph_configuration_model`, `random_simplicial_complex` — all seeded via `Pcg64Mcg`, all deterministic, all loops statically bounded.
- **`stats`** — a `Stat<T>` dispatch surface (`asdict`/`aslist`/`filterby`/`filterby_gt`) plus `node_degree`(+weighted), `node_attrs`, `edge_order`, `edge_size`, `edge_attrs`, and the directed `di_*` degree/size stats.
- **`readwrite`** — JSON (xgi-data schema), edgelist, bipartite edgelist, dense incidence matrix, HIF (undirected), and offline BIGG → DiHypergraph, all via `serde_json` with a `ReadWriteError` type; network fetch gated behind `network` (off).

**New crate-level deps:** `rand_pcg.workspace = true`, `rand_core.workspace = true`, both `optional = true` under the `generators` feature. The workspace-level pins (`rand_pcg = "0.10.2"`, `rand_core = "0.10.1"`, replacing the unused speculative `rand 0.8`/`rand_pcg 0.3`) are corrected once, by Phase 2 — Phase 4 only consumes them.

**Next: Phase 5** — layout + viz (scene-graph), per spec §10.3.
