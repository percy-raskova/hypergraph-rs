# hypergraph-rs Phase 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Follow red → green → commit verbatim; every `use` statement, bound, and expected test count below is exact.

**Goal:** Implement `hypergraph-rs::linalg` (incidence / adjacency / intersection-profile / degree / clique-motif matrices + laplacian family) and `hypergraph-rs::algorithms` (connected components, shortest paths, properties, clustering, assortativity, the tractable centralities, and simpliciality) — all derived from the bipartite `Hypergraph<N,E,M>` substrate built in Phase 0+1. Introduce the `[features]` table from spec §10.1 and the `skeleton()` 1-skeleton projection (Phase 1/2 did NOT add it — verified against the Phase 0+1 completion summary, which lists no `skeleton`).

**Architecture:** `linalg` is a set of free generic functions `f(&Hypergraph<N,E,M>, …)` returning dense `ndarray::Array2<f64>` / `Array1<f64>` and sparse `sprs::CsMat<f64>`, plus the `(matrix, row_ids, col_ids)` index vectors. `algorithms` delegates the genuinely-equivalent count (`number_connected_components`) to `rustworkx_core::connectivity`, and otherwise operates on the `Hypergraph` directly via node-neighbour BFS (deterministic) or on the `linalg` matrices. Rows/columns follow **node insertion order** (`node_ids()`) and **edge insertion order** (`edge_ids()`) — the III.7 determinism contract, identical to XGI's dict-order (`xgi/linalg/hypergraph_matrix.py:110-111`).

**Tech Stack additions (this phase):** `ndarray = "0.17"` and `sprs = "0.11"`. The dep-probe (2026-07-18) proved rustworkx-core 0.18 transitively forces `ndarray 0.17.2`, and `sprs 0.11.4` unifies onto that same `0.17.2` node — so the workspace **must pin `ndarray = "0.17"`**, NOT the `0.16` the spec §10.1 draft named (correcting the spec). No LAPACK backend is pulled: the one eigenvector routine we need (`clique_eigenvector_centrality`) is hand-rolled power iteration (probe path (b)), so `ndarray-linalg`/OpenBLAS are avoided entirely.

## Global Constraints (unchanged from Phase 0+1 unless noted)

- **Rust edition 2021**, MSRV 1.85, license BSD-3-Clause, no `unsafe`.
- **Determinism (III.7):** every public matrix is row/col-ordered by `node_ids()` / `edge_ids()`; every public map/list is insertion-ordered (`IndexMap` / ordered `Vec`). No `HashMap` in any returned-order path — `HashSet`/`HashMap` appear only as internal dedup/visited scratch, never as the iteration source for a returned value.
- **Every loop is statically bounded** (Power-of-10 rule 2): matrix loops bound = `num_nodes`·`num_edges`; BFS loops bound = `num_nodes` (each node enqueued at most once, guarded by the `INFINITY`/visited check); power iteration and the Katz/centrality series are bounded by an explicit `max_iter`/`cutoff` counter.
- **TDD:** red → green → commit. Integration tests are **direct children** of `crates/hypergraph-rs/tests/` (one file per module family: `test_linalg.rs`, `test_algorithms.rs`). Run with `--test test_linalg` / `--test test_algorithms`.
- **`assert_eq!`/`assert_ne!` require `Debug` on operands** — `Array2<f64>`, `Array1<f64>`, `Vec<String>`, `f64`, `IndexMap` all derive/impl `Debug`, so `assert_eq!` is fine on them. Compare floats with a local epsilon check (e.g. `(a - b).abs() < 1e-9`, or the `test_linalg.rs`-local `approx` helper from Task 4) where exact bit-equality is not guaranteed — no crate-wide `approx_eq` is added (Task 5 note: it would be an unused, `dead_code`-linted `pub(crate)` item).
- **`serde_json::Value`-specific methods stay in concrete impl blocks** — not relevant this phase (linalg/algorithms are generic over `N,E,M` and touch only structure, never attribute internals), except `normalized_hypergraph_laplacian`'s `weighted=true` path, which reads `edge_attrs`'s `"weight"` key and therefore lives in a concrete `impl` (Task 4, called out inline).
- **Commit convention:** Conventional commits, `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer.

## Deliberate deviations from XGI (Phase 3 — each verified against installed 0.10.2 source)

1. **linalg return shape.** XGI's `index=True/False` returns a variable-length tuple. Rust has no variadic return, so every `linalg` function **always** returns `(matrix, row_ids, col_ids)` (or `(vector, row_ids)` for `degree_matrix`). The Phase 7 Python binding maps `index=False` by dropping the id vectors. (XGI ref: `hypergraph_matrix.py:135`.)
2. **f64 everywhere.** XGI's `incidence_matrix` is `dtype=int` (`hypergraph_matrix.py:130`). We build `f64` so incidence/adjacency/laplacian share one numeric type; values are identical for whole-number incidence. Conformance compares numeric equality, which holds.
3. **`skeleton()` is `UnGraph<String, f64>` (undirected).** Spec §4.1 wrote `Graph<String, f64>` (petgraph's `Graph` defaults to *Directed*). The 1-skeleton pairwise projection is undirected (XGI's `H.nodes.neighbors` is symmetric), and the connectivity/shortest-path/clustering algorithms need undirected — so we return `rustworkx_core::petgraph::graph::UnGraph<String, f64>`. Edge weight is unit `1.0` (edge existence); multiplicity of shared hyperedges is carried by `adjacency_matrix`, not the skeleton.
4. **Connected components / shortest paths are hand-rolled node-neighbour BFS**, giving **deterministically-ordered** components and distance maps (XGI yields components in hash-set order — `connected.py:77-82`; our ordering is the III.7 improvement). `number_connected_components` **delegates** to `rustworkx_core::connectivity::number_connected_components` on the skeleton (genuine plugin reuse); a test asserts the delegated count equals `connected_components().len()`.
5. **`clique_eigenvector_centrality` uses hand-rolled power iteration**, not `scipy.sparse.linalg.eigsh` (`centrality.py:60-63`). The probe proved there is no pure-Rust eigendecomposition on the ndarray-0.17 line without a LAPACK backend; power iteration converges to the same dominant Perron eigenvector within `tol` for the symmetric non-negative clique-motif matrix. Tested structurally (positivity, L1-normalization, node ranking), not byte-exact float parity — parity is re-checked at the Phase 7 conformance gate.
6. **Tensor-based centralities are DEFERRED out of Phase 3.** `h_eigenvector_centrality`, `z_eigenvector_centrality`, `uniform_h_eigenvector_centrality`, `node_edge_centrality`, and `line_vector_centrality` need the `ttsv1`/`ttsv2` tensor-contraction machinery and (for `line_vector`) a line-graph + networkx eigenvector routine. They are tracked in Open Questions and land in a later phase; only `katz_centrality` and `clique_eigenvector_centrality` ship in Phase 3.
7. **assortativity — deterministic paths only.** `dynamical_assortativity` and `degree_assortativity(exact=true)` are deterministic and ship now. The default `exact=false` random-sampling path (`assortativity.py:144-156`) needs the seeded `rand_pcg` pin that Phase 4 (generators) blesses — deferred, tracked in Open Questions.
8. **`largest_connected_hypergraph`, `adjacency_tensor`, `hodge_matrix`/`boundary_matrix`** are out of Phase 3 scope: `largest_connected_hypergraph` needs `subhypergraph` (a Phase 2 globalview), `adjacency_tensor` needs the n-D tensor type, and the Hodge/boundary matrices are simplicial-complex operators (Phase 2 `SimplicialComplex`). Tracked in Open Questions.
9. **`is_possible_order` on an empty hypergraph returns `false`.** XGI computes `d_max = max_edge_order(H)` = `None` and evaluates `(d >= 0) and (d <= None)`, which raises `TypeError` at runtime (`properties.py:144-161`). We keep `is_possible_order` a total function (`matches!(max_edge_order(h), Some(dmax) if d <= dmax)` — `false` when `max_edge_order` is `None`) rather than propagating an error for an untested edge case; a deliberate total-function choice, not tracked as a bug.
10. **`two_node_clustering_coefficient` skips a neighbour term instead of poisoning to NaN when its denominator is 0.** XGI's `_uv_cc` returns `np.nan` on a zero denominator, which would NaN the whole node's averaged value. In this codebase the path is unreachable — every `v` in `node_neighbors(n)` shares ≥1 edge with `n`, so `union`/`min`/`max` of `m_u`/`m_v` is always ≥1 — but the skip-vs-NaN choice is recorded here for explicit XGI-parity intent rather than left implicit in the code.

---

## File Structure (additions this phase)

```
crates/hypergraph-rs/
├── Cargo.toml                         # + ndarray, sprs deps; + [features] table
├── src/
│   ├── lib.rs                         # + pub mod linalg; #[cfg(feature="algorithms")] pub mod algorithms;
│   ├── core/
│   │   └── hypergraph.rs              # + skeleton(), node_neighbors(), node_degree helpers
│   ├── linalg/
│   │   ├── mod.rs                     # re-exports; filtered_edge_ids helper; LinalgError
│   │   ├── hypergraph_matrix.rs       # incidence/adjacency/intersection/degree/clique_motif
│   │   └── laplacian.rs               # laplacian/multiorder/normalized
│   └── algorithms/
│       ├── mod.rs                     # submodule re-exports only (no crate-wide helpers — see Task 5 note)
│       ├── connected.rs
│       ├── shortest_path.rs
│       ├── properties.rs
│       ├── clustering.rs
│       ├── assortativity.rs
│       ├── centrality.rs
│       └── simpliciality.rs
└── tests/
    ├── test_hypergraph.rs             # (Phase 0+1) + skeleton/neighbors tests appended in Task 5
    ├── test_linalg.rs                 # NEW — Tasks 1-4
    └── test_algorithms.rs            # NEW — Tasks 5-11
```

Workspace-root `Cargo.toml` (`[workspace.dependencies]`) gains `ndarray` and `sprs`. Everything else (`rustworkx-core`, `indexmap`, `serde`, `serde_json`, `thiserror`) is already declared from Phase 0.

---

## Task 1: Add `ndarray`/`sprs` deps, the `[features]` table, and `incidence_matrix` (dense + sparse)

**Files:**
- Modify: `hypergraph-rs/Cargo.toml` (workspace)
- Modify: `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/linalg/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/linalg/hypergraph_matrix.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_linalg.rs`

**Interfaces:**
- `linalg::incidence_matrix(&Hypergraph<N,E,M>, order: Option<usize>) -> (Array2<f64>, Vec<String>, Vec<String>)` — dense, rows=nodes, cols=edges.
- `linalg::incidence_matrix_sparse(&Hypergraph<N,E,M>, order: Option<usize>) -> (CsMat<f64>, Vec<String>, Vec<String>)`.
- `linalg::filtered_edge_ids(&Hypergraph<N,E,M>, order: Option<usize>) -> Vec<String>` — `pub(crate)` helper; `order=Some(d)` keeps edges whose `members().len() == d+1`.

**XGI source:** `hypergraph_matrix.py:64-135` (`incidence_matrix`). Empty rule (`:100-105`): if no filtered edges OR no nodes, return a `(0,0)` matrix. Edge "order" `= size − 1`, so `filterby("order", d)` keeps `len(members)==d+1`.

- [ ] **Step 1: Add workspace deps.** In `hypergraph-rs/Cargo.toml`, under `[workspace.dependencies]`, add (the probe blessed `ndarray = "0.17"`, correcting the spec's `0.16`; `sprs = "0.11"` unifies onto rustworkx-core's ndarray-0.17.2 node):

```toml
ndarray = "0.17"
sprs = "0.11"
```

- [ ] **Step 2: Wire deps + features into the core crate.** In `crates/hypergraph-rs/Cargo.toml`, add under `[dependencies]`:

```toml
ndarray.workspace = true
sprs.workspace = true
```

and append this `[features]` table (spec §10.1; only the flags relevant through Phase 5 are wired to real gates now — `generators`/`stats`/`readwrite`/`layout`/`viz`/`dynamics`/`communities`/`convert`/`network` are declared as empty forward-decls so the default set is stable, and `network` keeps its optional-dep shape for Phase 6):

```toml
[features]
default = ["algorithms", "generators", "stats", "readwrite", "layout", "viz"]
all = ["default", "dynamics", "communities", "convert", "network"]
algorithms = []
generators = []
stats = []
readwrite = []
layout = []
viz = []
dynamics = []
communities = []
convert = []
network = []
```

Note: `linalg` is **not** a feature — it is unconditional core (algorithms depend on it). `network = ["reqwest"]` from the spec draft is deferred to Phase 6 (no `reqwest` dep yet); it is an empty forward-decl here so `--features all` still resolves.

- [ ] **Step 3: Register the `linalg` module in `lib.rs`.** Append to `crates/hypergraph-rs/src/lib.rs`:

```rust
pub mod linalg;
```

Note: `#[cfg(feature = "algorithms")] pub mod algorithms;` is **not** added here — `algorithms/mod.rs` does not exist until Task 5, and every Task 1-4 test run uses default features (which include `algorithms`), so registering the module before its `mod.rs` exists would break the build for the entire linalg phase (`error[E0583]`). Task 5 Step 7 adds that line to `lib.rs` in the same step that creates `algorithms/mod.rs`.

- [ ] **Step 4: Create `linalg/mod.rs` with only the functions this task implements.** `linalg/mod.rs` is built up incrementally: this task's `pub use` list names only `incidence_matrix`/`incidence_matrix_sparse` (the only functions `hypergraph_matrix.rs` defines so far); Task 2 extends the list with `adjacency_matrix`/`clique_motif_matrix`, Task 3 with `intersection_profile`/`degree_matrix`, and Task 4 adds `mod laplacian;` plus its re-exports — each extension lands in the same step that implements the corresponding functions, so the crate never references a symbol that doesn't exist yet. Write to `crates/hypergraph-rs/src/linalg/mod.rs`:

```rust
//! Matrices derived from the bipartite hypergraph substrate.
//!
//! Row order follows [`Hypergraph::node_ids`] (node insertion order); column
//! order follows [`Hypergraph::edge_ids`] (edge insertion order). This is the
//! III.7 determinism contract and matches XGI's dict-ordering
//! (`xgi/linalg/hypergraph_matrix.py`).

mod hypergraph_matrix;

pub use hypergraph_matrix::{incidence_matrix, incidence_matrix_sparse};

use crate::Hypergraph;
use thiserror::Error;

/// Errors raised by linear-algebra operators that have preconditions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum LinalgError {
    /// The normalized Laplacian is undefined when isolated nodes exist
    /// (division by a zero degree). XGI parity: `laplacian_matrix.py:222-225`.
    #[error("every node must be a member of an edge to avoid divide-by-zero")]
    IsolatedNodes,
}

/// Edge IDs kept after an optional order filter, in edge insertion order.
///
/// `order = Some(d)` keeps edges whose membership count equals `d + 1`
/// (edge order = size − 1). `order = None` keeps all edges.
pub(crate) fn filtered_edge_ids<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> Vec<String> {
    match order {
        None => h.edge_ids(),
        Some(d) => h
            .edge_ids()
            .into_iter()
            .filter(|e| h.members(e).map(|m| m.len()) == Some(d + 1))
            .collect(),
    }
}
```

- [ ] **Step 5: Create `linalg/hypergraph_matrix.rs` with `incidence_matrix` + `incidence_matrix_sparse` only** (the rest land in Tasks 2-3). Write to `crates/hypergraph-rs/src/linalg/hypergraph_matrix.rs`:

```rust
//! Incidence, adjacency, intersection-profile, degree, and clique-motif matrices.

use indexmap::IndexMap;
use ndarray::Array2;
use sprs::{CsMat, TriMat};

use super::filtered_edge_ids;
use crate::Hypergraph;

/// Dense incidence matrix `I` (rows = nodes, cols = edges); `I[i,j] = 1`
/// iff node `i` is a member of edge `j`.
///
/// XGI parity: `xgi.incidence_matrix(H, order, sparse=False, index=True)`
/// (`hypergraph_matrix.py:64-135`). Returns a `(0, 0)` matrix with empty id
/// vectors when there are no nodes or no (filtered) edges.
pub fn incidence_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array2<f64>, Vec<String>, Vec<String>) {
    let node_ids = h.node_ids();
    let edge_ids = filtered_edge_ids(h, order);
    if node_ids.is_empty() || edge_ids.is_empty() {
        return (Array2::zeros((0, 0)), Vec::new(), Vec::new());
    }
    let node_index: IndexMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut m = Array2::<f64>::zeros((node_ids.len(), edge_ids.len()));
    for (j, e) in edge_ids.iter().enumerate() {
        for member in h.members(e).unwrap_or_default() {
            if let Some(&i) = node_index.get(member.as_str()) {
                m[[i, j]] = 1.0;
            }
        }
    }
    (m, node_ids, edge_ids)
}

/// Sparse (CSR) incidence matrix — same values and ordering as
/// [`incidence_matrix`], as a `sprs::CsMat<f64>`.
pub fn incidence_matrix_sparse<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (CsMat<f64>, Vec<String>, Vec<String>) {
    let node_ids = h.node_ids();
    let edge_ids = filtered_edge_ids(h, order);
    if node_ids.is_empty() || edge_ids.is_empty() {
        return (CsMat::zero((0, 0)), Vec::new(), Vec::new());
    }
    let node_index: IndexMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut tri = TriMat::<f64>::new((node_ids.len(), edge_ids.len()));
    for (j, e) in edge_ids.iter().enumerate() {
        for member in h.members(e).unwrap_or_default() {
            if let Some(&i) = node_index.get(member.as_str()) {
                tri.add_triplet(i, j, 1.0);
            }
        }
    }
    (tri.to_csr(), node_ids, edge_ids)
}
```

- [ ] **Step 6: Write the failing test.** Write to `crates/hypergraph-rs/tests/test_linalg.rs`:

```rust
use hypergraph_rs::linalg::{incidence_matrix, incidence_matrix_sparse};
use hypergraph_rs::Hypergraph;
use ndarray::array;

fn sample() -> Hypergraph {
    // Nodes added first (1..=7) to pin row order; XGI docstring example.
    let mut h: Hypergraph = Hypergraph::new();
    for n in ["1", "2", "3", "4", "5", "6", "7"] {
        h.add_node(n, serde_json::Value::Null);
    }
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into(), "7".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(vec!["4".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(
        vec!["5".into(), "6".into(), "7".into()],
        Some("e2".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_incidence_matrix_values_and_order() {
    let h = sample();
    let (i, rows, cols) = incidence_matrix(&h, None);
    assert_eq!(rows, vec!["1", "2", "3", "4", "5", "6", "7"]);
    assert_eq!(cols, vec!["e0", "e1", "e2"]);
    // XGI docstring golden (hypergraph_matrix.py:37-43).
    let expected = array![
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
    ];
    assert_eq!(i, expected);
}

#[test]
fn test_incidence_matrix_order_filter() {
    let h = sample();
    // order=2 keeps only the size-3 edge e2.
    let (i, _rows, cols) = incidence_matrix(&h, Some(2));
    assert_eq!(cols, vec!["e2"]);
    assert_eq!(i.shape(), &[7, 1]);
    assert_eq!(i.column(0).sum(), 3.0);
}

#[test]
fn test_incidence_matrix_empty() {
    let h: Hypergraph = Hypergraph::new();
    let (i, rows, cols) = incidence_matrix(&h, None);
    assert_eq!(i.shape(), &[0, 0]);
    assert!(rows.is_empty());
    assert!(cols.is_empty());
}

#[test]
fn test_incidence_matrix_sparse_matches_dense() {
    let h = sample();
    let (dense, _, _) = incidence_matrix(&h, None);
    let (sparse, _, _) = incidence_matrix_sparse(&h, None);
    assert_eq!(sparse.to_dense(), dense);
}
```

- [ ] **Step 7: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: FAIL — modules/functions don't exist yet (compile error) until Steps 1-5 are in place; if you did Steps 1-5 first, this passes — in that case revert `hypergraph_matrix.rs` to an empty `//!` module, confirm the compile failure, then restore. (TDD red is the compile error before the impl body exists.)

- [ ] **Step 8: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: PASS — 4 tests.

- [ ] **Step 9: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): linalg incidence_matrix (dense + sparse) + features table + ndarray/sprs deps

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: `adjacency_matrix` + `clique_motif_matrix`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/linalg/hypergraph_matrix.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/linalg/mod.rs` (extend the `pub use` list with `adjacency_matrix`, `clique_motif_matrix`)
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_linalg.rs`

**Interfaces:**
- `adjacency_matrix(&Hypergraph<N,E,M>, order: Option<usize>, s: usize, weighted: bool) -> (Array2<f64>, Vec<String>)`.
- `clique_motif_matrix(&Hypergraph<N,E,M>) -> (Array2<f64>, Vec<String>)` — `= adjacency_matrix(h, None, 1, true)`.

**XGI source:** `hypergraph_matrix.py:138-204` (`adjacency_matrix`): `A = I·Iᵀ`, zero the diagonal, then `A = (A >= s) * 1` (binary) or `(A >= s) * A` (weighted). Empty rule (`:175-181`): if incidence is `(0,0)` but nodes exist, return `zeros((num_nodes, num_nodes))`. `clique_motif_matrix` (`:268-299`) = weighted adjacency.

- [ ] **Step 1: Write the failing test.** Append to `tests/test_linalg.rs`:

```rust
use hypergraph_rs::linalg::{adjacency_matrix, clique_motif_matrix};

#[test]
fn test_adjacency_binary() {
    let h = sample();
    let (a, rows) = adjacency_matrix(&h, None, 1, false);
    assert_eq!(rows, vec!["1", "2", "3", "4", "5", "6", "7"]);
    // node 7 (index 6) shares edges with 1,2,3 (via e0) and 5,6 (via e2).
    assert_eq!(a[[6, 0]], 1.0); // 7-1 connected
    assert_eq!(a[[6, 4]], 1.0); // 7-5 connected
    assert_eq!(a[[0, 0]], 0.0); // diagonal zeroed
    assert_eq!(a[[3, 0]], 0.0); // node 4 isolated from node 1
}

#[test]
fn test_adjacency_weighted_counts_shared_edges() {
    // Two edges both containing {a, b} => weight 2 between them.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let (a, rows) = adjacency_matrix(&h, None, 1, true);
    let ia = rows.iter().position(|x| x == "a").unwrap();
    let ib = rows.iter().position(|x| x == "b").unwrap();
    assert_eq!(a[[ia, ib]], 2.0);
}

#[test]
fn test_adjacency_s_threshold() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    // s=2: only pairs sharing >= 2 edges remain. a-b share 2; a-c share 1.
    let (a, rows) = adjacency_matrix(&h, None, 2, false);
    let ia = rows.iter().position(|x| x == "a").unwrap();
    let ib = rows.iter().position(|x| x == "b").unwrap();
    let ic = rows.iter().position(|x| x == "c").unwrap();
    assert_eq!(a[[ia, ib]], 1.0);
    assert_eq!(a[[ia, ic]], 0.0);
}

#[test]
fn test_adjacency_no_edges_returns_zeros_n_by_n() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);
    let (a, rows) = adjacency_matrix(&h, None, 1, false);
    assert_eq!(a.shape(), &[2, 2]);
    assert_eq!(rows, vec!["a", "b"]);
    assert_eq!(a.sum(), 0.0);
}

#[test]
fn test_clique_motif_equals_weighted_adjacency() {
    let h = sample();
    let (w, _) = clique_motif_matrix(&h);
    let (a, _) = adjacency_matrix(&h, None, 1, true);
    assert_eq!(w, a);
}
```

- [ ] **Step 2: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: FAIL — `adjacency_matrix`/`clique_motif_matrix` don't exist.

- [ ] **Step 3: Implement.** First, extend the `pub use` line in `linalg/mod.rs` so the new symbols resolve:

```rust
pub use hypergraph_matrix::{
    adjacency_matrix, clique_motif_matrix, incidence_matrix, incidence_matrix_sparse,
};
```

Then append to `linalg/hypergraph_matrix.rs`:

```rust
/// Adjacency matrix `A` (nodes × nodes). `A = I·Iᵀ` with the diagonal zeroed;
/// then thresholded by `s` overlapping edges. If `weighted`, the surviving
/// entries carry the shared-edge count; otherwise they are `1.0`.
///
/// XGI parity: `xgi.adjacency_matrix(H, order, s, weighted)`
/// (`hypergraph_matrix.py:138-204`).
pub fn adjacency_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    s: usize,
    weighted: bool,
) -> (Array2<f64>, Vec<String>) {
    let (inc, node_ids, _edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        // No (filtered) edges: XGI returns an N×N zero matrix over all nodes.
        let n = h.num_nodes();
        return (Array2::zeros((n, n)), h.node_ids());
    }
    let mut a = inc.dot(&inc.t());
    let n = a.nrows();
    for d in 0..n {
        a[[d, d]] = 0.0;
    }
    let s = s as f64;
    let out = a.mapv(|v| {
        if v >= s {
            if weighted {
                v
            } else {
                1.0
            }
        } else {
            0.0
        }
    });
    (out, node_ids)
}

/// Weighted clique-motif matrix — `adjacency_matrix(H, None, 1, true)`.
///
/// XGI parity: `xgi.clique_motif_matrix(H)` (`hypergraph_matrix.py:268-299`).
pub fn clique_motif_matrix<N, E, M>(h: &Hypergraph<N, E, M>) -> (Array2<f64>, Vec<String>) {
    adjacency_matrix(h, None, 1, true)
}
```

- [ ] **Step 4: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: PASS — 9 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): linalg adjacency_matrix + clique_motif_matrix

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: `intersection_profile` + `degree_matrix`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/linalg/hypergraph_matrix.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/linalg/mod.rs` (extend the `pub use` list with `intersection_profile`, `degree_matrix`)
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_linalg.rs`

**Interfaces:**
- `intersection_profile(&Hypergraph<N,E,M>, order: Option<usize>) -> (Array2<f64>, Vec<String>)` — `P = Iᵀ·I` (edges × edges), row ids = edge ids.
- `degree_matrix(&Hypergraph<N,E,M>, order: Option<usize>) -> (Array1<f64>, Vec<String>)` — `K[i]` = incidence row sum (node degree at that order), row ids = node ids.

**XGI source:** `hypergraph_matrix.py:207-233` (`intersection_profile`: `P = eye.T.dot(eye)`), `:236-265` (`degree_matrix`: `K = ravel(sum(eye, axis=1))`; if incidence `(0,0)` → `zeros(num_nodes)`).

- [ ] **Step 1: Write the failing test.** Append to `tests/test_linalg.rs`:

```rust
use hypergraph_rs::linalg::{degree_matrix, intersection_profile};
use ndarray::Array1;

#[test]
fn test_intersection_profile_diagonal_is_edge_size() {
    let h = sample();
    let (p, edge_rows) = intersection_profile(&h, None);
    assert_eq!(edge_rows, vec!["e0", "e1", "e2"]);
    // diagonal[j] = |members of edge j|
    assert_eq!(p[[0, 0]], 4.0); // e0 has 4 members
    assert_eq!(p[[1, 1]], 1.0); // e1 has 1
    assert_eq!(p[[2, 2]], 3.0); // e2 has 3
    // e0 ∩ e2 = {7} => 1
    assert_eq!(p[[0, 2]], 1.0);
    // e0 ∩ e1 = {} => 0
    assert_eq!(p[[0, 1]], 0.0);
}

#[test]
fn test_degree_matrix_counts_memberships() {
    let h = sample();
    let (k, node_rows) = degree_matrix(&h, None);
    assert_eq!(node_rows, vec!["1", "2", "3", "4", "5", "6", "7"]);
    let expected: Array1<f64> = Array1::from(vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0]);
    assert_eq!(k, expected); // node 7 is in e0 and e2
}

#[test]
fn test_degree_matrix_empty_is_zeros_over_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);
    let (k, rows) = degree_matrix(&h, None);
    assert_eq!(k, Array1::from(vec![0.0, 0.0]));
    assert_eq!(rows, vec!["a", "b"]);
}
```

- [ ] **Step 2: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: FAIL.

- [ ] **Step 3: Implement.** First, extend the `pub use` line in `linalg/mod.rs` so the new symbols resolve:

```rust
pub use hypergraph_matrix::{
    adjacency_matrix, clique_motif_matrix, degree_matrix, incidence_matrix,
    incidence_matrix_sparse, intersection_profile,
};
```

Then append to `linalg/hypergraph_matrix.rs` (add `Array1, Axis` to the ndarray import at the top: change `use ndarray::Array2;` to `use ndarray::{Array1, Array2, Axis};`):

```rust
/// Intersection profile `P = Iᵀ·I` (edges × edges). `P[a,b]` is the number of
/// nodes shared by edges `a` and `b`; the diagonal is each edge's size.
///
/// XGI parity: `xgi.intersection_profile(H, order)`
/// (`hypergraph_matrix.py:207-233`). Row ids are edge ids.
pub fn intersection_profile<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array2<f64>, Vec<String>) {
    let (inc, _node_ids, edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        return (Array2::zeros((0, 0)), Vec::new());
    }
    let p = inc.t().dot(&inc);
    (p, edge_ids)
}

/// Degree vector `K` (one entry per node) = incidence row sums.
///
/// XGI parity: `xgi.degree_matrix(H, order)` (`hypergraph_matrix.py:236-265`).
/// When there are no (filtered) edges, returns `zeros(num_nodes)`.
pub fn degree_matrix<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> (Array1<f64>, Vec<String>) {
    let (inc, node_ids, _edge_ids) = incidence_matrix(h, order);
    if inc.shape() == [0, 0] {
        return (Array1::zeros(h.num_nodes()), h.node_ids());
    }
    let k = inc.sum_axis(Axis(1));
    (k, node_ids)
}
```

- [ ] **Step 4: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: PASS — 12 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): linalg intersection_profile + degree_matrix

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: `laplacian` + `multiorder_laplacian` + `normalized_hypergraph_laplacian`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/src/linalg/laplacian.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/linalg/mod.rs` (add `mod laplacian;` and its `pub use`)
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_linalg.rs`

**Interfaces:**
- `laplacian(&Hypergraph<N,E,M>, order: usize, rescale_per_node: bool) -> (Array2<f64>, Vec<String>)`.
- `multiorder_laplacian(&Hypergraph<N,E,M>, orders: &[usize], weights: &[f64], rescale_per_node: bool) -> (Array2<f64>, Vec<String>)`.
- `normalized_hypergraph_laplacian(&Hypergraph<serde_json::Value, serde_json::Value, M>, weighted: bool) -> Result<(Array2<f64>, Vec<String>), LinalgError>` — concrete `N=E=Value` impl because `weighted=true` reads the `"weight"` edge attribute (a `serde_json::Value` operation).

**XGI source:** `laplacian_matrix.py:62-115` (`laplacian`: `A` weighted, `L = order·diag(K) − A`, optional `/order`), `:118-184` (`multiorder_laplacian`: `Σ_d L_d · w_d / mean(K_d)`, skip all-zero-`K` orders), `:187-254` (`normalized_hypergraph_laplacian`: isolate check raises; `L = I − Dv^{-1/2}·Inc·W·De^{-1}·Incᵀ·Dv^{-1/2}`).

- [ ] **Step 1: Write the failing test.** Append to `tests/test_linalg.rs`:

```rust
use hypergraph_rs::linalg::{laplacian, multiorder_laplacian, normalized_hypergraph_laplacian};
use hypergraph_rs::linalg::LinalgError;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn test_laplacian_order1_is_graph_laplacian() {
    // Triangle: three pairwise edges over {a,b,c}. L = 2I - A (order 1, weighted).
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let (l, rows) = laplacian(&h, 1, false);
    assert_eq!(rows, vec!["a", "b", "c"]);
    // Each node has degree 2; L diagonal = 2, off-diagonal = -1.
    assert!(approx(l[[0, 0]], 2.0));
    assert!(approx(l[[0, 1]], -1.0));
    // Row sums of a graph Laplacian are 0.
    for i in 0..3 {
        assert!(approx(l.row(i).sum(), 0.0));
    }
}

#[test]
fn test_laplacian_empty() {
    let h: Hypergraph = Hypergraph::new();
    let (l, rows) = laplacian(&h, 1, false);
    assert_eq!(l.shape(), &[0, 0]);
    assert!(rows.is_empty());
}

#[test]
fn test_multiorder_laplacian_shape() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let (l, rows) = multiorder_laplacian(&h, &[1, 2], &[1.0, 1.0], false);
    assert_eq!(rows.len(), 3);
    assert_eq!(l.shape(), &[3, 3]);
    // Multiorder Laplacian rows still sum to ~0.
    for i in 0..3 {
        assert!(approx(l.row(i).sum(), 0.0));
    }
}

#[test]
fn test_normalized_laplacian_isolated_node_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_node("isolated", serde_json::Value::Null);
    let result = normalized_hypergraph_laplacian(&h, false);
    assert_eq!(result, Err(LinalgError::IsolatedNodes));
}

#[test]
fn test_normalized_laplacian_ok() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let (l, rows) = normalized_hypergraph_laplacian(&h, false).unwrap();
    assert_eq!(rows, vec!["a", "b", "c"]);
    assert_eq!(l.shape(), &[3, 3]);
}
```

- [ ] **Step 2: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: FAIL.

- [ ] **Step 3: Implement.** First, add `mod laplacian;` and its re-export to `linalg/mod.rs` (below the existing `mod hypergraph_matrix;` line):

```rust
mod laplacian;

pub use laplacian::{laplacian, multiorder_laplacian, normalized_hypergraph_laplacian};
```

Then write to `crates/hypergraph-rs/src/linalg/laplacian.rs`:

```rust
//! Laplacian matrices (order-`d`, multiorder, normalized) of a hypergraph.

use ndarray::{Array1, Array2};

use super::hypergraph_matrix::{adjacency_matrix, degree_matrix, incidence_matrix};
use super::LinalgError;
use crate::Hypergraph;

/// Build a diagonal matrix from a vector.
fn diag(v: &Array1<f64>) -> Array2<f64> {
    let n = v.len();
    let mut m = Array2::<f64>::zeros((n, n));
    for i in 0..n {
        m[[i, i]] = v[i];
    }
    m
}

/// Order-`d` Laplacian `L = d·diag(K) − A` (A = weighted adjacency at order `d`).
///
/// XGI parity: `xgi.laplacian(H, order, rescale_per_node)`
/// (`laplacian_matrix.py:62-115`). `order = 1` gives the usual graph Laplacian.
pub fn laplacian<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: usize,
    rescale_per_node: bool,
) -> (Array2<f64>, Vec<String>) {
    let (a, node_ids) = adjacency_matrix(h, Some(order), 1, true);
    if a.shape() == [0, 0] {
        return (Array2::zeros((0, 0)), Vec::new());
    }
    let (k, _) = degree_matrix(h, Some(order));
    let mut l = diag(&k) * (order as f64) - &a;
    if rescale_per_node {
        l = l / (order as f64);
    }
    (l, node_ids)
}

/// Multiorder Laplacian `Σ_d L_d · w_d / mean(K_d)`, skipping orders with no edges.
///
/// XGI parity: `xgi.multiorder_laplacian(H, orders, weights, rescale_per_node)`
/// (`laplacian_matrix.py:118-184`). Panics if `orders.len() != weights.len()`
/// (XGI raises `ValueError`).
pub fn multiorder_laplacian<N, E, M>(
    h: &Hypergraph<N, E, M>,
    orders: &[usize],
    weights: &[f64],
    rescale_per_node: bool,
) -> (Array2<f64>, Vec<String>) {
    assert_eq!(
        orders.len(),
        weights.len(),
        "orders and weights must have the same length"
    );
    let n = h.num_nodes();
    let mut l_multi = Array2::<f64>::zeros((n, n));
    for (&d, &w) in orders.iter().zip(weights.iter()) {
        let (k, _) = degree_matrix(h, Some(d));
        let mean_k = if k.is_empty() { 0.0 } else { k.mean().unwrap_or(0.0) };
        // Skip orders with no edges (all-zero K) to avoid divide-by-zero.
        if mean_k == 0.0 {
            continue;
        }
        let (l_d, _) = laplacian(h, d, rescale_per_node);
        if l_d.shape() == [n, n] {
            l_multi = l_multi + l_d * (w / mean_k);
        }
    }
    (l_multi, h.node_ids())
}
```

For `normalized_hypergraph_laplacian`, the `weighted=true` path reads the
`"weight"` edge attribute, a `serde_json::Value` operation — so it lives in a
**concrete** `impl`-free function specialized to `N=E=serde_json::Value`.
Append:

```rust
/// Normalized hypergraph Laplacian
/// `L = I − Dv^{-1/2}·Inc·W·De^{-1}·Incᵀ·Dv^{-1/2}`.
///
/// XGI parity: `xgi.normalized_hypergraph_laplacian(H, weighted)`
/// (`laplacian_matrix.py:187-254`). Concrete over `N = E = serde_json::Value`
/// because `weighted = true` reads each edge's `"weight"` attribute (default 1.0).
///
/// # Errors
/// [`LinalgError::IsolatedNodes`] if any node has degree 0 (division by zero).
pub fn normalized_hypergraph_laplacian<M>(
    h: &Hypergraph<serde_json::Value, serde_json::Value, M>,
    weighted: bool,
) -> Result<(Array2<f64>, Vec<String>), LinalgError> {
    // Isolate check: every node must belong to at least one edge.
    for nid in h.node_ids() {
        if h.memberships(&nid).map(|m| m.is_empty()).unwrap_or(true) {
            return Err(LinalgError::IsolatedNodes);
        }
    }

    let (inc, node_ids, edge_ids) = incidence_matrix(h, None);
    let n = node_ids.len();

    // Dv = node degrees (incidence row sums); De = edge sizes (col sums).
    let dv = inc.sum_axis(ndarray::Axis(1)); // len n
    let de = inc.sum_axis(ndarray::Axis(0)); // len m

    let weights: Vec<f64> = if weighted {
        edge_ids
            .iter()
            .map(|e| {
                h.edge_attrs(e)
                    .and_then(|a| a.get("weight"))
                    .and_then(|w| w.as_f64())
                    .unwrap_or(1.0)
            })
            .collect()
    } else {
        vec![1.0; edge_ids.len()]
    };

    let dv_invsqrt = diag(&dv.mapv(|x| x.powf(-0.5)));
    let de_inv = diag(&de.mapv(|x| 1.0 / x));
    let w = diag(&Array1::from(weights));
    let eye = Array2::<f64>::eye(n);

    let mid = inc.dot(&w).dot(&de_inv).dot(&inc.t());
    let l = eye - dv_invsqrt.dot(&mid).dot(&dv_invsqrt);
    Ok((l, node_ids))
}
```

`normalized_hypergraph_laplacian` is already covered by the `pub use laplacian::{...}` line added at the top of this step — no further `mod.rs` edit needed.

- [ ] **Step 4: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_linalg`. Expected: PASS — 17 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): linalg laplacian + multiorder + normalized

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: Core helpers (`skeleton`, `node_neighbors`, `node_degree`, `edge_size`) + `algorithms::properties`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph.rs` (append skeleton/neighbors tests)
- Create: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/mod.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/properties.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces (core, unconditional — no feature gate; petgraph is always present):**
- `Hypergraph::skeleton(&self) -> UnGraph<String, f64>` — undirected 1-skeleton, node weight = node id, edge weight = 1.0; nodes added in `node_ids()` order so `NodeIndex::new(i)` ↔ `node_ids()[i]`.
- `Hypergraph::node_neighbors(&self, node_id: &str) -> Option<Vec<String>>` — nodes sharing ≥1 hyperedge, excluding self, insertion-ordered/deduped.
- `Hypergraph::node_degree(&self, node_id: &str, order: Option<usize>) -> Option<usize>` — memberships (optionally filtered to edges of a given order).
- `Hypergraph::edge_size(&self, edge_id: &str) -> Option<usize>` — `members().len()`.

**Interfaces (algorithms::properties):** `is_uniform`, `max_edge_order`, `num_edges_order`, `unique_edge_sizes`, `degree_counts`, `degree_histogram`, `density`, `incidence_density`, `edge_neighborhood`, `is_possible_order` (per `properties.py`).

**XGI source:** `properties.py` (whole file); `_plain_bfs` neighbour semantics `connected.py:233-248`.

- [ ] **Step 1: Write failing core-helper tests.** Append to `tests/test_hypergraph.rs`:

```rust
use rustworkx_core::petgraph::visit::EdgeCount;

#[test]
fn test_skeleton_projects_pairwise() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let g = h.skeleton();
    // triangle a-b, a-c, b-c => 3 nodes, 3 undirected edges
    assert_eq!(g.node_count(), 3);
    assert_eq!(g.edge_count(), 3);
}

#[test]
fn test_skeleton_node_order_positional() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("x", serde_json::Value::Null);
    h.add_node("y", serde_json::Value::Null);
    let g = h.skeleton();
    use rustworkx_core::petgraph::graph::NodeIndex;
    assert_eq!(g.node_weight(NodeIndex::new(0)).unwrap(), "x");
    assert_eq!(g.node_weight(NodeIndex::new(1)).unwrap(), "y");
}

#[test]
fn test_node_neighbors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let mut nb = h.node_neighbors("c").unwrap();
    nb.sort();
    assert_eq!(nb, vec!["a", "b", "d"]);
    assert!(!nb.contains(&"c".to_string()));
}

#[test]
fn test_node_degree_and_edge_size() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(h.node_degree("a", None), Some(2));
    assert_eq!(h.node_degree("a", Some(1)), Some(1)); // only e0 (size 2 => order 1)
    assert_eq!(h.edge_size("e1"), Some(3));
    assert_eq!(h.edge_size("nope"), None);
}
```

- [ ] **Step 2: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph`. Expected: FAIL.

- [ ] **Step 3: Implement the core helpers.** In `core/hypergraph.rs`, add these imports near the top (after the existing `use rustworkx_core::petgraph::stable_graph::...` line):

```rust
use rustworkx_core::petgraph::graph::{NodeIndex as GraphNodeIndex, UnGraph};
use rustworkx_core::petgraph::visit::NodeIndexable;
```

Add these methods inside the generic `impl<N, E, M> Hypergraph<N, E, M>` block:

```rust
    /// The undirected 1-skeleton (pairwise projection): a node for every agent
    /// (weight = its id, added in `node_ids()` order so `NodeIndex::new(i)`
    /// corresponds to `node_ids()[i]`), and an edge (weight `1.0`) between two
    /// agents that co-occur in at least one hyperedge.
    ///
    /// Undirected (deviation from spec §4.1's `Graph`): the connectivity,
    /// shortest-path, and clustering algorithms operate on this graph.
    pub fn skeleton(&self) -> UnGraph<String, f64> {
        let mut g = UnGraph::<String, f64>::new_undirected();
        let node_ids = self.node_ids();
        let mut idx_of: IndexMap<String, GraphNodeIndex> = IndexMap::new();
        for n in &node_ids {
            let ix = g.add_node(n.clone());
            idx_of.insert(n.clone(), ix);
        }
        let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for e in self.edge_ids() {
            let members = self.members(&e).unwrap_or_default();
            // Bounded: |members| choose 2.
            for a in 0..members.len() {
                for b in (a + 1)..members.len() {
                    let ia = idx_of[&members[a]];
                    let ib = idx_of[&members[b]];
                    let key = if g.to_index(ia) < g.to_index(ib) {
                        (g.to_index(ia), g.to_index(ib))
                    } else {
                        (g.to_index(ib), g.to_index(ia))
                    };
                    if seen.insert(key) {
                        g.add_edge(ia, ib, 1.0);
                    }
                }
            }
        }
        g
    }

    /// The neighbours of a node: all nodes that share at least one hyperedge
    /// with it, excluding itself, in insertion order (deduped). `None` if the
    /// node is absent. XGI parity: `H.nodes.neighbors(n)`.
    pub fn node_neighbors(&self, node_id: &str) -> Option<Vec<String>> {
        let edges = self.memberships(node_id)?;
        let mut result: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in edges {
            for m in self.members(&e).unwrap_or_default() {
                if m != node_id && seen.insert(m.clone()) {
                    result.push(m);
                }
            }
        }
        Some(result)
    }

    /// The degree of a node = number of edges it belongs to. With
    /// `order = Some(d)`, counts only edges of that order (size `d+1`).
    /// `None` if the node is absent. XGI parity: `H.degree(n, order)`.
    pub fn node_degree(&self, node_id: &str, order: Option<usize>) -> Option<usize> {
        let edges = self.memberships(node_id)?;
        Some(match order {
            None => edges.len(),
            Some(d) => edges
                .iter()
                .filter(|e| self.edge_size(e) == Some(d + 1))
                .count(),
        })
    }

    /// The size (number of members) of an edge. `None` if the edge is absent.
    pub fn edge_size(&self, edge_id: &str) -> Option<usize> {
        self.members(edge_id).map(|m| m.len())
    }
```

- [ ] **Step 4: Verify core-helper tests pass.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph`. Expected: PASS (Phase 0+1's 64 + these 4 = 68).

- [ ] **Step 5: Write the failing `properties` test.** Write to `crates/hypergraph-rs/tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::properties::{
    degree_counts, density, incidence_density, is_uniform, max_edge_order, num_edges_order,
    unique_edge_sizes,
};
use hypergraph_rs::Hypergraph;

#[test]
fn test_is_uniform_and_max_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(is_uniform(&h), Some(2)); // all edges order 2
    assert_eq!(max_edge_order(&h), Some(2));
}

#[test]
fn test_is_uniform_false_when_mixed() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(is_uniform(&h), None);
}

#[test]
fn test_unique_edge_sizes_and_num_edges_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(unique_edge_sizes(&h), vec![2, 3]);
    assert_eq!(num_edges_order(&h, Some(1)), 1); // size-2 edges
    assert_eq!(num_edges_order(&h, None), 2);
}

#[test]
fn test_degree_counts() {
    // XGI docstring: edges [[1,2],[2,3,4]] => degree_counts == [0, 3, 1]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["2".into(), "3".into(), "4".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(degree_counts(&h, None), vec![0, 3, 1]);
}

#[test]
fn test_density_and_incidence_density_nonempty() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    // 1 edge, n=2 => density = 1/(2^2 - 1) = 1/3
    assert!((density(&h, None, None, false) - (1.0 / 3.0)).abs() < 1e-12);
    // incidence density: nnz=2, denom = n*num_edges = 2*1 = 2 => 1.0
    assert!((incidence_density(&h, None, None, false) - 1.0).abs() < 1e-12);
}
```

- [ ] **Step 6: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_algorithms`. Expected: FAIL.

- [ ] **Step 7: Register the `algorithms` module in `lib.rs`, then create `algorithms/mod.rs`.** Append to `crates/hypergraph-rs/src/lib.rs`:

```rust
#[cfg(feature = "algorithms")]
pub mod algorithms;
```

Then write to `crates/hypergraph-rs/src/algorithms/mod.rs`:

```rust
//! Hypergraph algorithms. All functions take `&Hypergraph<N,E,M>` first.
//!
//! Gated behind the `algorithms` feature (in the default set). Component and
//! distance orderings follow node insertion order (III.7 determinism), a
//! deliberate improvement over XGI's hash-set iteration.

pub mod assortativity;
pub mod centrality;
pub mod clustering;
pub mod connected;
pub mod properties;
pub mod shortest_path;
pub mod simpliciality;
```

Note: no crate-wide `approx_eq` helper is added here — nothing in Tasks 6-11 would call it (each test/impl uses its own local epsilon comparison), and a `pub(crate)` item with no caller trips rustc's `dead_code` lint, which Task 12 Step 1's `cargo clippy -- -D warnings` denies.

Note: `assortativity`, `centrality`, `clustering`, `connected`, `shortest_path`,
and `simpliciality` are created in Tasks 6-11; to keep the crate compiling after
this task, create each as a one-line stub `//! placeholder` file now and fill
it in its task. (Alternatively, comment out the not-yet-created `pub mod` lines
and uncomment them per task — either keeps `cargo build` green between tasks.)

- [ ] **Step 8: Implement `properties`.** Write to `crates/hypergraph-rs/src/algorithms/properties.rs`:

```rust
//! Structural properties of a hypergraph (`xgi/algorithms/properties.py`).

use std::collections::BTreeSet;

use crate::Hypergraph;

/// Number of edges of order `d` (size `d+1`); all edges if `d` is `None`.
/// XGI parity: `num_edges_order` (`properties.py:92-117`).
pub fn num_edges_order<N, E, M>(h: &Hypergraph<N, E, M>, d: Option<usize>) -> usize {
    match d {
        None => h.num_edges(),
        Some(order) => h
            .edge_ids()
            .iter()
            .filter(|e| h.edge_size(e) == Some(order + 1))
            .count(),
    }
}

/// Maximum edge order (max size − 1). `None` for an empty hypergraph with no
/// nodes; `Some(0)` when there are nodes but no edges. XGI parity:
/// `max_edge_order` (`properties.py:120-141`).
pub fn max_edge_order<N, E, M>(h: &Hypergraph<N, E, M>) -> Option<usize> {
    if h.num_edges() > 0 {
        let max_size = h
            .edge_ids()
            .iter()
            .filter_map(|e| h.edge_size(e))
            .max()
            .unwrap_or(0);
        Some(max_size - 1)
    } else if h.num_nodes() > 0 {
        Some(0)
    } else {
        None
    }
}

/// Whether `d` is a possible order (0 ≤ d ≤ max order).
/// XGI parity: `is_possible_order` (`properties.py:144-161`).
pub fn is_possible_order<N, E, M>(h: &Hypergraph<N, E, M>, d: usize) -> bool {
    matches!(max_edge_order(h), Some(dmax) if d <= dmax)
}

/// Unique edge sizes in ascending order. XGI parity: `unique_edge_sizes`
/// (`properties.py:321-334`).
pub fn unique_edge_sizes<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<usize> {
    let set: BTreeSet<usize> = h.edge_ids().iter().filter_map(|e| h.edge_size(e)).collect();
    set.into_iter().collect()
}

/// The order of uniformity if all non-singleton edges share one size, else
/// `None`. XGI parity: `is_uniform` (`properties.py:164-197`) — returns
/// `d = size − 1` on success, `False` otherwise (mapped to `None`).
pub fn is_uniform<N, E, M>(h: &Hypergraph<N, E, M>) -> Option<usize> {
    let mut sizes: BTreeSet<usize> = unique_edge_sizes(h).into_iter().collect();
    sizes.remove(&1); // discard singletons
    if sizes.len() != 1 {
        return None;
    }
    sizes.into_iter().next().map(|s| s - 1)
}

/// Occurrences of each degree value, index = degree, from 0 to max degree.
/// XGI parity: `degree_counts` (`properties.py:244-282`).
pub fn degree_counts<N, E, M>(h: &Hypergraph<N, E, M>, order: Option<usize>) -> Vec<usize> {
    let degs: Vec<usize> = h
        .node_ids()
        .iter()
        .filter_map(|n| h.node_degree(n, order))
        .collect();
    let max_d = degs.iter().copied().max().unwrap_or(0);
    let mut counts = vec![0usize; max_d + 1];
    for d in degs {
        counts[d] += 1;
    }
    counts
}

/// Observed degrees (ascending) and their counts.
/// XGI parity: `degree_histogram` (`properties.py:285-318`).
pub fn degree_histogram<N, E, M>(h: &Hypergraph<N, E, M>) -> (Vec<usize>, Vec<usize>) {
    let mut counts: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for n in h.node_ids() {
        let d = h.node_degree(&n, None).unwrap_or(0);
        *counts.entry(d).or_insert(0) += 1;
    }
    let degrees: Vec<usize> = counts.keys().copied().collect();
    let heights: Vec<usize> = counts.values().copied().collect();
    (degrees, heights)
}

/// The edge neighbourhood of a node: the members of each edge containing it,
/// excluding the node itself (or including it if `include_self`).
/// XGI parity: `edge_neighborhood` (`properties.py:200-241`).
pub fn edge_neighborhood<N, E, M>(
    h: &Hypergraph<N, E, M>,
    n: &str,
    include_self: bool,
) -> Vec<Vec<String>> {
    let memberships = match h.memberships(n) {
        Some(m) => m,
        None => return Vec::new(),
    };
    memberships
        .iter()
        .map(|e| {
            let members = h.members(e).unwrap_or_default();
            if include_self {
                members
            } else {
                members.into_iter().filter(|m| m != n).collect()
            }
        })
        .collect()
}

/// Binomial coefficient `n choose k` (exact, u128 to resist overflow at scale).
fn comb(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num *= (n - i) as u128;
        den *= (i + 1) as u128;
    }
    (num / den) as f64
}

/// Hypergraph density = existing edges / possible edges.
/// XGI parity: `density` (`properties.py:337-425`). `order`/`max_order` are
/// mutually exclusive (order wins); `ignore_singletons` drops order-0 edges.
pub fn density<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> f64 {
    let n = h.num_nodes();
    assert!(n >= 1, "density is not defined for an empty hypergraph");
    if h.num_edges() < 1 {
        return 0.0;
    }
    let count_order = |d: usize| num_edges_order(h, Some(d));
    let count_leq = |dmax: usize| (0..=dmax).map(count_order).sum::<usize>();

    let (numer, denom): (f64, f64) = match (order, max_order) {
        (None, None) => {
            let mut numer = h.num_edges() as f64;
            let mut denom = 2f64.powi(n as i32) - 1.0;
            if ignore_singletons {
                numer -= count_order(0) as f64;
                denom -= n as f64;
            }
            (numer, denom)
        }
        (None, Some(mo)) => {
            assert!(mo < n, "max_order must be smaller than the number of nodes");
            let mut numer = count_leq(mo) as f64;
            let mut denom: f64 = (0..=mo).map(|o| comb(n, o + 1)).sum();
            if ignore_singletons {
                numer -= count_order(0) as f64;
                denom -= n as f64;
            }
            (numer, denom)
        }
        (Some(o), _) => {
            assert!(o < n, "order must be smaller than the number of nodes");
            if ignore_singletons && o == 0 {
                return 0.0;
            }
            (count_order(o) as f64, comb(n, o + 1))
        }
    };
    if denom == 0.0 {
        0.0
    } else {
        numer / denom
    }
}

/// Density of the incidence matrix = nnz / (n · num_edges_considered).
/// XGI parity: `incidence_density` (`properties.py:428-494`).
pub fn incidence_density<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> f64 {
    let n = h.num_nodes();
    assert!(n >= 1, "incidence density is not defined for an empty hypergraph");
    if h.num_edges() < 1 {
        return 0.0;
    }
    let keep: Vec<String> = h
        .edge_ids()
        .into_iter()
        .filter(|e| {
            let size = h.edge_size(e).unwrap_or(0);
            let ord = size.saturating_sub(1);
            match (order, max_order) {
                (Some(o), _) => {
                    if ignore_singletons && o == 0 {
                        false
                    } else {
                        ord == o
                    }
                }
                (None, Some(mo)) => ord <= mo && (!ignore_singletons || ord > 0),
                (None, None) => !ignore_singletons || ord > 0,
            }
        })
        .collect();
    let denom = (n * keep.len()) as f64;
    let numer: f64 = keep.iter().filter_map(|e| h.edge_size(e)).sum::<usize>() as f64;
    if denom == 0.0 {
        0.0
    } else {
        numer / denom
    }
}
```

- [ ] **Step 9: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_algorithms`. Expected: PASS — 5 tests. Also `cargo test -p hypergraph-rs --test test_hypergraph` → 68.

- [ ] **Step 10: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): core skeleton/neighbors helpers + algorithms::properties

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: `algorithms::connected`

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/connected.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:** `is_connected`, `connected_components` (deterministic `Vec<Vec<String>>`), `number_connected_components`, `node_connected_component`, `largest_connected_component`.

**XGI source:** `connected.py:16-248`. Semantics: BFS over node neighbours; components ordered by first-seen node in `node_ids()` order; each component's nodes ordered by `node_ids()` order.

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::connected::{
    connected_components, is_connected, largest_connected_component, node_connected_component,
    number_connected_components,
};

fn two_components() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into(), "e".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_number_and_is_connected() {
    let h = two_components();
    assert_eq!(number_connected_components(&h), 2);
    assert!(!is_connected(&h));

    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    assert!(is_connected(&h2));
    assert_eq!(number_connected_components(&h2), 1);
}

#[test]
fn test_connected_components_deterministic() {
    let h = two_components();
    let comps = connected_components(&h);
    assert_eq!(comps.len(), 2);
    assert_eq!(comps[0], vec!["a", "b"]);
    assert_eq!(comps[1], vec!["c", "d", "e"]);
}

#[test]
fn test_largest_and_node_component() {
    let h = two_components();
    assert_eq!(largest_connected_component(&h), vec!["c", "d", "e"]);
    assert_eq!(node_connected_component(&h, "a").unwrap(), vec!["a", "b"]);
    assert!(node_connected_component(&h, "zzz").is_none());
}

#[test]
fn test_rustworkx_delegated_count_matches_handrolled() {
    let h = two_components();
    // number_connected_components delegates to rustworkx_core on the skeleton;
    // it must agree with the hand-rolled component extraction.
    assert_eq!(number_connected_components(&h), connected_components(&h).len());
}
```

- [ ] **Step 2: Run test to verify it fails.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_algorithms`. Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/connected.rs`:

```rust
//! Connected components of a hypergraph (`xgi/algorithms/connected.py`).
//!
//! Components are extracted by a deterministic node-neighbour BFS (III.7:
//! ordered by `node_ids()`), while `number_connected_components` delegates the
//! count to `rustworkx_core::connectivity` on the 1-skeleton — a genuine
//! rustworkx-core plugin reuse. A test asserts the two agree.

use std::collections::{HashSet, VecDeque};

use rustworkx_core::connectivity::number_connected_components as rx_ncc;

use crate::Hypergraph;

/// BFS the component containing `source`, returning its nodes in `node_ids()`
/// order. Assumes `source` exists.
fn bfs_component<N, E, M>(h: &Hypergraph<N, E, M>, source: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    seen.insert(source.to_string());
    queue.push_back(source.to_string());
    // Bounded: each node enters the queue at most once (guarded by `seen`),
    // so the loop runs at most `num_nodes` times.
    while let Some(cur) = queue.pop_front() {
        for nb in h.node_neighbors(&cur).unwrap_or_default() {
            if seen.insert(nb.clone()) {
                queue.push_back(nb);
            }
        }
    }
    seen
}

/// All connected components, each a node list in `node_ids()` order, the
/// components themselves ordered by their first-seen node.
/// XGI parity: `xgi.connected_components(H)` (`connected.py:48-82`).
pub fn connected_components<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<Vec<String>> {
    let node_ids = h.node_ids();
    let mut seen: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for v in &node_ids {
        if !seen.contains(v) {
            let comp = bfs_component(h, v);
            seen.extend(comp.iter().cloned());
            // Emit in node_ids() order for determinism.
            let ordered: Vec<String> =
                node_ids.iter().filter(|n| comp.contains(*n)).cloned().collect();
            components.push(ordered);
        }
    }
    components
}

/// Whether the hypergraph is connected (one component; empty ⇒ trivially so is
/// undefined — matches XGI, which indexes `list(H.nodes)[0]`, so guard empty).
/// XGI parity: `xgi.is_connected(H)` (`connected.py:16-45`).
pub fn is_connected<N, E, M>(h: &Hypergraph<N, E, M>) -> bool {
    if h.num_nodes() == 0 {
        return false;
    }
    number_connected_components(h) == 1
}

/// Number of connected components, delegated to rustworkx-core on the skeleton.
/// XGI parity: `xgi.number_connected_components(H)` (`connected.py:85-121`).
pub fn number_connected_components<N, E, M>(h: &Hypergraph<N, E, M>) -> usize {
    let g = h.skeleton();
    rx_ncc(&g)
}

/// The connected component containing node `n`, in `node_ids()` order, or
/// `None` if the node is absent. XGI parity: `node_connected_component`
/// (`connected.py:154-188`) — XGI raises; we return `None`.
pub fn node_connected_component<N, E, M>(h: &Hypergraph<N, E, M>, n: &str) -> Option<Vec<String>> {
    if !h.has_node(n) {
        return None;
    }
    let comp = bfs_component(h, n);
    Some(h.node_ids().into_iter().filter(|x| comp.contains(x)).collect())
}

/// The largest connected component (ties broken by first-seen order).
/// XGI parity: `largest_connected_component` (`connected.py:124-151`).
pub fn largest_connected_component<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<String> {
    connected_components(h)
        .into_iter()
        .max_by_key(|c| c.len())
        .unwrap_or_default()
}
```

Note on the delegation signature: `rustworkx_core::connectivity::number_connected_components`
is called as `rx_ncc(&g)` where `g: UnGraph<String, f64>`. If the 0.18 build
rejects that call shape (by-value vs by-ref), fall back to
`connected_components(h).len()` — the test `test_rustworkx_delegated_count_matches_handrolled`
already pins the two to agree, so the fallback is behaviour-preserving. Record
the resolution in Open Questions.

- [ ] **Step 4: Run test to verify it passes.** `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_algorithms`. Expected: PASS — 9 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::connected (BFS + rustworkx-core count delegation)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 7: `algorithms::shortest_path`

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/shortest_path.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:**
- `single_source_shortest_path_length(&Hypergraph<N,E,M>, source: &str) -> Option<IndexMap<String, f64>>` — distances (unit-hop) from `source`; `f64::INFINITY` for unreachable; `None` if source absent. Keys in `node_ids()` order.
- `shortest_path_length(&Hypergraph<N,E,M>) -> Vec<(String, IndexMap<String, f64>)>` — one entry per source.

**XGI source:** `shortest_path.py:10-102`. XGI's Dijkstra uses `increment = 1` per hop (unit weights) ⇒ BFS levels. We hand-roll BFS on node neighbours (avoids depending on the unverified `rustworkx_core::shortest_path::dijkstra` `DictMap` return type; exact unit-weight parity).

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::shortest_path::{
    shortest_path_length, single_source_shortest_path_length,
};

#[test]
fn test_single_source_distances() {
    // Path-ish: e0={a,b}, e1={b,c}. a->b=1, a->c=2 (via shared node b).
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let d = single_source_shortest_path_length(&h, "a").unwrap();
    assert_eq!(d["a"], 0.0);
    assert_eq!(d["b"], 1.0);
    assert_eq!(d["c"], 2.0);
}

#[test]
fn test_unreachable_is_infinity() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let d = single_source_shortest_path_length(&h, "a").unwrap();
    assert_eq!(d["b"], 1.0);
    assert!(d["c"].is_infinite());
}

#[test]
fn test_missing_source_none() {
    let h: Hypergraph = Hypergraph::new();
    assert!(single_source_shortest_path_length(&h, "nope").is_none());
}

#[test]
fn test_shortest_path_length_all_sources() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let all = shortest_path_length(&h);
    assert_eq!(all.len(), 2); // one per node
    let (src, dists) = &all[0];
    assert_eq!(src, "a");
    assert_eq!(dists["b"], 1.0);
}
```

- [ ] **Step 2: Run test to verify it fails.** Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/shortest_path.rs`:

```rust
//! Unit-hop shortest paths on a hypergraph (`xgi/algorithms/shortest_path.py`).
//!
//! XGI's Dijkstra uses a fixed increment of 1 per hop, i.e. BFS levels over the
//! node-neighbour relation. We hand-roll that BFS (unit-weight exact parity;
//! keeps the algorithm independent of the rustworkx-core `dijkstra` return type).

use std::collections::VecDeque;

use indexmap::IndexMap;

use crate::Hypergraph;

/// Distances from `source` to every node (unit hops). Unreachable nodes get
/// `f64::INFINITY`; `source` gets `0`. Keys in `node_ids()` order. `None` if
/// `source` is absent. XGI parity: `single_source_shortest_path_length`.
pub fn single_source_shortest_path_length<N, E, M>(
    h: &Hypergraph<N, E, M>,
    source: &str,
) -> Option<IndexMap<String, f64>> {
    if !h.has_node(source) {
        return None;
    }
    let mut dist: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        dist.insert(n, f64::INFINITY);
    }
    dist.insert(source.to_string(), 0.0);
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(source.to_string());
    // Bounded: each node is enqueued at most once (distance set from INF once).
    while let Some(cur) = queue.pop_front() {
        let cur_d = dist[&cur];
        for nb in h.node_neighbors(&cur).unwrap_or_default() {
            if dist[&nb].is_infinite() {
                dist.insert(nb.clone(), cur_d + 1.0);
                queue.push_back(nb);
            }
        }
    }
    Some(dist)
}

/// Distances from every source. XGI parity: `shortest_path_length`.
pub fn shortest_path_length<N, E, M>(
    h: &Hypergraph<N, E, M>,
) -> Vec<(String, IndexMap<String, f64>)> {
    h.node_ids()
        .into_iter()
        .filter_map(|src| single_source_shortest_path_length(h, &src).map(|d| (src, d)))
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes.** Expected: PASS — 13 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::shortest_path (unit-hop BFS)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 8: `algorithms::clustering`

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/clustering.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:** `clustering_coefficient`, `local_clustering_coefficient`, `two_node_clustering_coefficient` — each returns `IndexMap<String, f64>` in `node_ids()` order.

**XGI source:** `clustering.py:17-278`.

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::clustering::{
    clustering_coefficient, local_clustering_coefficient, two_node_clustering_coefficient,
};

fn triangle() -> Hypergraph {
    // Three size-2 edges over {0,1,2} form a closed triangle, plus the size-3
    // edge {0,1,2} itself — matches XGI's `random_hypergraph(3,[1,1])`
    // docstring fixture (the source of the 0.5 two-node-CC golden below).
    // Without the triple edge, `_uv_cc`(union) evaluates to 1/3, not 0.5.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "2".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("e3".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_clustering_coefficient_triangle_is_one() {
    let h = triangle();
    let cc = clustering_coefficient(&h);
    for n in ["0", "1", "2"] {
        assert!((cc[n] - 1.0).abs() < 1e-9);
    }
}

#[test]
fn test_clustering_coefficient_no_neighbors_is_zero() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("lonely", serde_json::Value::Null);
    let cc = clustering_coefficient(&h);
    assert_eq!(cc["lonely"], 0.0);
}

#[test]
fn test_two_node_clustering_coefficient_triangle() {
    let h = triangle();
    let cc = two_node_clustering_coefficient(&h, "union");
    // XGI docstring golden (`random_hypergraph(3,[1,1])`, which also contains
    // the triple {0,1,2}): 0.5 each. Without the triple edge, union-CC is 1/3.
    for n in ["0", "1", "2"] {
        assert!((cc[n] - 0.5).abs() < 1e-9);
    }
}

#[test]
fn test_local_clustering_coefficient_defined() {
    let h = triangle();
    let cc = local_clustering_coefficient(&h);
    assert_eq!(cc.len(), 3);
    // each node is in 3 edges here -> value present & finite
    for n in ["0", "1", "2"] {
        assert!(cc[n].is_finite());
    }
}
```

- [ ] **Step 2: Run test to verify it fails.** Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/clustering.rs`:

```rust
//! Nodal clustering coefficients (`xgi/algorithms/clustering.py`).

use std::collections::HashSet;

use indexmap::IndexMap;
use ndarray::Axis;

use crate::linalg::adjacency_matrix;
use crate::Hypergraph;

/// Clustering coefficient via the unweighted pairwise projection:
/// `c_i = 0.5 · (A³)_ii / [k_i (k_i − 1) / 2]`, 0 when undefined.
/// XGI parity: `clustering_coefficient` (`clustering.py:17-74`).
pub fn clustering_coefficient<N, E, M>(h: &Hypergraph<N, E, M>) -> IndexMap<String, f64> {
    let (adj, node_ids) = adjacency_matrix(h, None, 1, false);
    let mut result: IndexMap<String, f64> = IndexMap::new();
    // Nodes absent from the adjacency (isolated) get 0.
    for n in h.node_ids() {
        result.insert(n, 0.0);
    }
    if adj.shape() == [0, 0] {
        return result;
    }
    let k = adj.sum_axis(Axis(1));
    let a3 = adj.dot(&adj).dot(&adj);
    for (i, n) in node_ids.iter().enumerate() {
        let denom = k[i] * (k[i] - 1.0) / 2.0;
        let c = if denom != 0.0 {
            0.5 * a3[[i, i]] / denom
        } else {
            0.0
        };
        result.insert(n.clone(), c);
    }
    result
}

/// Local clustering coefficient based on edge overlap.
/// XGI parity: `local_clustering_coefficient` (`clustering.py:77-164`).
pub fn local_clustering_coefficient<N, E, M>(h: &Hypergraph<N, E, M>) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        let ev = h.memberships(&n).unwrap_or_default();
        let dv = ev.len();
        if dv <= 1 {
            result.insert(n, 0.0);
            continue;
        }
        let mut total_eo = 0.0;
        // Bounded: |ev| choose 2.
        for a in 0..ev.len() {
            for b in (a + 1)..ev.len() {
                let edge1: HashSet<String> = h.members(&ev[a]).unwrap_or_default().into_iter().collect();
                let edge2: HashSet<String> = h.members(&ev[b]).unwrap_or_default().into_iter().collect();
                let d1: HashSet<String> = edge1.difference(&edge2).cloned().collect();
                let d2: HashSet<String> = edge2.difference(&edge1).cloned().collect();
                let union: HashSet<String> = d1.union(&d2).cloned().collect();
                let eo = if union.is_empty() {
                    0.0
                } else {
                    let neigh_d1: HashSet<String> = d1
                        .iter()
                        .flat_map(|d| h.node_neighbors(d).unwrap_or_default())
                        .collect();
                    let neigh_d2: HashSet<String> = d2
                        .iter()
                        .flat_map(|d| h.node_neighbors(d).unwrap_or_default())
                        .collect();
                    let a1 = neigh_d1.intersection(&d2).count();
                    let a2 = neigh_d2.intersection(&d1).count();
                    (a1 + a2) as f64 / union.len() as f64
                };
                total_eo += eo;
            }
        }
        result.insert(n, 2.0 * total_eo / (dv as f64 * (dv as f64 - 1.0)));
    }
    result
}

/// Two-node clustering coefficient averaged over a node's neighbours.
/// `kind` ∈ {"union", "min", "max"}. XGI parity:
/// `two_node_clustering_coefficient` (`clustering.py:167-278`).
pub fn two_node_clustering_coefficient<N, E, M>(
    h: &Hypergraph<N, E, M>,
    kind: &str,
) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        let neighbors = h.node_neighbors(&n).unwrap_or_default();
        if neighbors.is_empty() {
            result.insert(n, 0.0);
            continue;
        }
        let m_u: HashSet<String> = h.memberships(&n).unwrap_or_default().into_iter().collect();
        let mut acc = 0.0;
        let count = neighbors.len() as f64;
        for v in &neighbors {
            let m_v: HashSet<String> = h.memberships(v).unwrap_or_default().into_iter().collect();
            let num = m_u.intersection(&m_v).count() as f64;
            let denom = match kind {
                "union" => m_u.union(&m_v).count() as f64,
                "min" => m_u.len().min(m_v.len()) as f64,
                "max" => m_u.len().max(m_v.len()) as f64,
                _ => panic!("invalid kind of clustering: {kind}"),
            };
            // Deviation #10: XGI's `_uv_cc` returns NaN here; we skip the term
            // instead (unreachable in practice — `v` is a neighbour of `n`, so
            // they share >=1 edge and every `kind`'s denominator is >=1).
            if denom != 0.0 {
                acc += (num / denom) / count;
            }
        }
        result.insert(n, acc);
    }
    result
}
```

- [ ] **Step 4: Run test to verify it passes.** Expected: PASS — 17 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::clustering (global + local + two-node)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 9: `algorithms::assortativity`

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/assortativity.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:**
- `dynamical_assortativity(&Hypergraph<N,E,M>) -> Result<f64, AssortError>`.
- `degree_assortativity_exact(&Hypergraph<N,E,M>, kind: AssortKind) -> Result<f64, AssortError>` where `AssortKind ∈ {Uniform, Top2, TopBottom}`. (The random-sampling default `exact=false` awaits the Phase-4 `rand_pcg` pin — Open Questions.)

**XGI source:** `assortativity.py:13-219`. `dynamical` (`:44-64`): requires uniform, ≥1 node/edge, no singletons; `kk1·k1²/k2² − 1`. `degree_assortativity` exact (`:117-143`): Pearson corr over `[k[n1],k[n2]]` from permutations of members (size>1).

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::assortativity::{
    degree_assortativity_exact, dynamical_assortativity, AssortError, AssortKind,
};

#[test]
fn test_dynamical_requires_uniform() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(dynamical_assortativity(&h), Err(AssortError::NotUniform));
}

#[test]
fn test_dynamical_uniform_regular_is_near_zero() {
    // 3-uniform (edge order 2): two copies of the same triple {0,1,2}, so all
    // node degrees are equal (2) => dynamical assortativity is 0.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let val = dynamical_assortativity(&h).unwrap();
    assert!(val.abs() < 1e-9);
}

#[test]
fn test_degree_assortativity_exact_runs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let rho = degree_assortativity_exact(&h, AssortKind::Uniform).unwrap();
    assert!(rho.is_finite());
    assert!((-1.0..=1.0).contains(&rho));
}

#[test]
fn test_assortativity_empty_errors() {
    let h: Hypergraph = Hypergraph::new();
    assert_eq!(dynamical_assortativity(&h), Err(AssortError::NoNodes));
}
```

- [ ] **Step 2: Run test to verify it fails.** Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/assortativity.rs`:

```rust
//! Degree assortativity of a hypergraph (`xgi/algorithms/assortativity.py`).
//!
//! Deterministic paths only: `dynamical_assortativity` and the exact
//! `degree_assortativity`. The random-sampling default awaits the Phase-4
//! `rand_pcg` pin.

use thiserror::Error;

use super::properties::{is_uniform, unique_edge_sizes};
use crate::Hypergraph;

/// Errors from assortativity computations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AssortError {
    #[error("hypergraph must contain nodes")]
    NoNodes,
    #[error("hypergraph must contain edges")]
    NoEdges,
    #[error("hypergraph must be uniform")]
    NotUniform,
    #[error("hypergraph must have no singleton edges")]
    HasSingletons,
}

/// The kind of exact degree assortativity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssortKind {
    Uniform,
    Top2,
    TopBottom,
}

fn degrees<N, E, M>(h: &Hypergraph<N, E, M>) -> std::collections::HashMap<String, f64> {
    h.node_ids()
        .into_iter()
        .map(|n| {
            let d = h.node_degree(&n, None).unwrap_or(0) as f64;
            (n, d)
        })
        .collect()
}

/// Dynamical assortativity of a uniform hypergraph.
/// XGI parity: `dynamical_assortativity` (`assortativity.py:13-64`).
pub fn dynamical_assortativity<N, E, M>(h: &Hypergraph<N, E, M>) -> Result<f64, AssortError> {
    if h.num_nodes() == 0 {
        return Err(AssortError::NoNodes);
    }
    if h.num_edges() == 0 {
        return Err(AssortError::NoEdges);
    }
    if is_uniform(h).is_none() {
        return Err(AssortError::NotUniform);
    }
    if unique_edge_sizes(h).contains(&1) {
        return Err(AssortError::HasSingletons);
    }
    let k = degrees(h);
    let vals: Vec<f64> = h.node_ids().iter().map(|n| k[n]).collect();
    let n = vals.len() as f64;
    let k1 = vals.iter().sum::<f64>() / n; // mean degree
    let k2 = vals.iter().map(|x| x * x).sum::<f64>() / n; // 2nd moment
    let mut pair_products: Vec<f64> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        for a in 0..members.len() {
            for b in (a + 1)..members.len() {
                pair_products.push(k[&members[a]] * k[&members[b]]);
            }
        }
    }
    let kk1 = pair_products.iter().sum::<f64>() / pair_products.len() as f64;
    Ok(kk1 * k1 * k1 / (k2 * k2) - 1.0)
}

/// Choose two degrees from an edge per the assortativity kind (exact variants).
fn choose_degrees(
    members: &[String],
    k: &std::collections::HashMap<String, f64>,
    kind: AssortKind,
) -> Vec<(f64, f64)> {
    match kind {
        AssortKind::Uniform => {
            let mut out = Vec::new();
            // permutations of size 2 (ordered) with n1 != n2.
            for a in 0..members.len() {
                for b in 0..members.len() {
                    if a != b {
                        out.push((k[&members[a]], k[&members[b]]));
                    }
                }
            }
            out
        }
        AssortKind::Top2 => {
            let mut ks: Vec<f64> = members.iter().map(|m| k[m]).collect();
            ks.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let top = &ks[ks.len().saturating_sub(2)..];
            // ordered permutations of the two selected degrees
            vec![(top[0], top[1]), (top[1], top[0])]
        }
        AssortKind::TopBottom => {
            let mut ks: Vec<f64> = members.iter().map(|m| k[m]).collect();
            ks.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let lo = ks[0];
            let hi = ks[ks.len() - 1];
            vec![(lo, hi), (hi, lo)]
        }
    }
}

/// Pearson correlation of two equal-length columns; 0 if undefined (nan).
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        cov += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    let denom = (vx * vy).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        cov / denom
    }
}

/// Exact degree assortativity (Pearson corr over the chosen degree pairs).
/// XGI parity: `degree_assortativity(exact=True)` (`assortativity.py:117-161`).
pub fn degree_assortativity_exact<N, E, M>(
    h: &Hypergraph<N, E, M>,
    kind: AssortKind,
) -> Result<f64, AssortError> {
    if h.num_nodes() == 0 {
        return Err(AssortError::NoNodes);
    }
    if h.num_edges() == 0 {
        return Err(AssortError::NoEdges);
    }
    let k = degrees(h);
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        if members.len() > 1 {
            for (a, b) in choose_degrees(&members, &k, kind) {
                xs.push(a);
                ys.push(b);
            }
        }
    }
    Ok(pearson(&xs, &ys))
}
```

- [ ] **Step 4: Run test to verify it passes.** Expected: PASS — 21 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::assortativity (dynamical + exact degree)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 10: `algorithms::centrality` (`katz` + `clique_eigenvector`)

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/centrality.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:**
- `katz_centrality(&Hypergraph<N,E,M>, cutoff: usize) -> IndexMap<String, f64>`.
- `clique_eigenvector_centrality(&Hypergraph<N,E,M>, tol: f64, max_iter: usize) -> IndexMap<String, f64>` — hand-rolled power iteration (deviation #5); returns all-NaN if disconnected, empty if no nodes.

**XGI source:** `centrality.py:26-67` (`clique_eigenvector_centrality`), `:218-296` (`katz_centrality`). Tensor centralities (`h_/z_/uniform_h/node_edge/line_vector`) are **deferred** — Open Questions.

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::centrality::{clique_eigenvector_centrality, katz_centrality};

fn connected_triangle() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "2".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_katz_l1_normalized_and_symmetric() {
    let h = connected_triangle();
    let c = katz_centrality(&h, 100);
    let sum: f64 = c.values().map(|v| v.abs()).sum();
    assert!((sum - 1.0).abs() < 1e-9); // L1-normalized
    // symmetric triangle => equal centralities
    assert!((c["0"] - c["1"]).abs() < 1e-9);
    assert!((c["1"] - c["2"]).abs() < 1e-9);
}

#[test]
fn test_katz_empty_no_edges_is_zeros() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    let c = katz_centrality(&h, 100);
    assert_eq!(c["a"], 0.0);
}

#[test]
fn test_clique_eigenvector_positive_normalized() {
    let h = connected_triangle();
    let c = clique_eigenvector_centrality(&h, 1e-6, 1000);
    let sum: f64 = c.values().map(|v| v.abs()).sum();
    assert!((sum - 1.0).abs() < 1e-6);
    for v in c.values() {
        assert!(*v > 0.0);
    }
    assert!((c["0"] - c["1"]).abs() < 1e-6);
}

#[test]
fn test_clique_eigenvector_disconnected_is_nan() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let c = clique_eigenvector_centrality(&h, 1e-6, 1000);
    assert!(c["a"].is_nan());
}
```

- [ ] **Step 2: Run test to verify it fails.** Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/centrality.rs`:

```rust
//! Hypergraph centralities (`xgi/algorithms/centrality.py`).
//!
//! Phase 3 ships the two non-tensor centralities: `katz_centrality` (power
//! series on the clique-motif matrix) and `clique_eigenvector_centrality`
//! (hand-rolled power iteration — no LAPACK; deviation #5). Tensor-based
//! centralities are deferred (Open Questions).

use indexmap::IndexMap;

use super::connected::is_connected;
use crate::linalg::clique_motif_matrix;
use crate::Hypergraph;

/// L1 norm of a slice.
fn l1(v: &[f64]) -> f64 {
    v.iter().map(|x| x.abs()).sum()
}

/// Katz centrality via the truncated power series
/// `mat = Σ αⁱ Aⁱ⁺¹`, then `c = mat · (1/n · 1)`, L1-normalized.
/// XGI parity: `katz_centrality(H, cutoff)` (`centrality.py:218-296`).
pub fn katz_centrality<N, E, M>(h: &Hypergraph<N, E, M>, cutoff: usize) -> IndexMap<String, f64> {
    let n = h.num_nodes();
    let mut result: IndexMap<String, f64> = IndexMap::new();
    if n == 0 {
        return result;
    }
    let node_ids = h.node_ids();
    if h.num_edges() == 0 {
        for nid in node_ids {
            result.insert(nid, 0.0);
        }
        return result;
    }
    let (a, _rows) = clique_motif_matrix(h);
    let alpha = 1.0 / 2f64.powi(n as i32);
    let mut mat = a.clone();
    // Bounded loop: `cutoff` iterations.
    for _ in 1..cutoff {
        mat = mat.dot(&a) * alpha + &a;
    }
    let u = ndarray::Array1::<f64>::from_elem(n, 1.0 / n as f64);
    let mut c = mat.dot(&u);
    let norm = l1(c.as_slice().unwrap());
    if norm != 0.0 {
        c.mapv_inplace(|x| x / norm);
    }
    for (i, nid) in node_ids.into_iter().enumerate() {
        result.insert(nid, c[i]);
    }
    result
}

/// Clique-motif eigenvector centrality via power iteration.
/// XGI parity: `clique_eigenvector_centrality(H, tol)` (`centrality.py:26-67`);
/// deviation #5 (power iteration replaces scipy `eigsh`). Empty ⇒ empty map;
/// disconnected ⇒ all-NaN.
pub fn clique_eigenvector_centrality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    tol: f64,
    max_iter: usize,
) -> IndexMap<String, f64> {
    let mut result: IndexMap<String, f64> = IndexMap::new();
    if h.num_nodes() == 0 {
        return result;
    }
    let node_ids = h.node_ids();
    if !is_connected(h) {
        for nid in node_ids {
            result.insert(nid, f64::NAN);
        }
        return result;
    }
    let (w, _rows) = clique_motif_matrix(h);
    let n = node_ids.len();
    let mut v = ndarray::Array1::<f64>::from_elem(n, 1.0 / n as f64);
    // Power iteration (bounded by max_iter): v ← W v / ||W v||₂.
    for _ in 0..max_iter {
        let mut next = w.dot(&v);
        let norm2 = next.dot(&next).sqrt();
        if norm2 == 0.0 {
            break;
        }
        next.mapv_inplace(|x| x / norm2);
        let delta = (&next - &v).mapv(|x| x * x).sum().sqrt();
        v = next;
        if delta < tol {
            break;
        }
    }
    // Sign-fix + L1-normalize (XGI: `sign(v[0]) * v / norm(v, 1)`).
    let sign = if v[0] < 0.0 { -1.0 } else { 1.0 };
    let norm1 = l1(v.as_slice().unwrap());
    for (i, nid) in node_ids.into_iter().enumerate() {
        result.insert(nid, sign * v[i] / norm1);
    }
    result
}
```

- [ ] **Step 4: Run test to verify it passes.** Expected: PASS — 25 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::centrality (katz + clique-eigenvector power iteration)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 11: `algorithms::simpliciality`

**Files:**
- Create/replace: `hypergraph-rs/crates/hypergraph-rs/src/algorithms/simpliciality.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_algorithms.rs`

**Interfaces:** `simplicial_fraction`, `edit_simpliciality`, `simplicial_edit_distance`, `face_edit_simpliciality`, `mean_face_edit_distance` — each `-> f64` (NaN where XGI returns nan). A private `FaceSet` (sorted-member membership set, standing in for XGI's trie-based face lookup) and `maximal_edges` helper.

**XGI source:** `simpliciality.py:17-509`. A face's subfaces = its powerset (sizes `min_size..len-1`). An edge is a simplex iff every subface (size ≥ min_size) is present. `_max_number_of_subfaces` (`:474-480`). `maximal` edges = members not contained in any other edge.

**Bound note (Power-of-10 rule 2):** `powerset`'s `0u64..(1u64 << n)` loop and `max_number_of_subfaces`'s `2i128.pow(max_size)` are statically bounded only for a single edge's arity `n < 64` (a `u64`/`i128::pow` overflow — a debug-build panic — above that). This is provably fine for every fixture in this plan and for Babylon's small hyperedges; the assumption is enforced with a `debug_assert!` in `powerset` below rather than left implicit.

- [ ] **Step 1: Write the failing test.** Append to `tests/test_algorithms.rs`:

```rust
use hypergraph_rs::algorithms::simpliciality::{
    edit_simpliciality, face_edit_simpliciality, mean_face_edit_distance,
    simplicial_edit_distance, simplicial_fraction,
};

fn full_simplex() -> Hypergraph {
    // Down-closed on {a,b,c} for sizes >= 2: {a,b},{a,c},{b,c},{a,b,c}
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e3".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_simplicial_fraction_full_simplex_is_one() {
    let h = full_simplex();
    let sf = simplicial_fraction(&h, 2, true);
    assert!((sf - 1.0).abs() < 1e-9);
}

#[test]
fn test_edit_distance_full_simplex_is_zero() {
    let h = full_simplex();
    let d = simplicial_edit_distance(&h, 2, true, true);
    assert!(d.abs() < 1e-9);
    assert!((edit_simpliciality(&h, 2, true) - 1.0).abs() < 1e-9);
}

#[test]
fn test_edit_distance_missing_face_positive() {
    // {a,b,c} present but {b,c} missing => not a full simplex.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let d = simplicial_edit_distance(&h, 2, true, true);
    assert!(d > 0.0);
}

#[test]
fn test_face_edit_simpliciality_full_simplex_is_one() {
    let h = full_simplex();
    assert!((face_edit_simpliciality(&h, 2, true) - 1.0).abs() < 1e-9);
    assert!(mean_face_edit_distance(&h, 2, true, true).abs() < 1e-9);
}
```

- [ ] **Step 2: Run test to verify it fails.** Expected: FAIL.

- [ ] **Step 3: Implement.** Write to `crates/hypergraph-rs/src/algorithms/simpliciality.rs`:

```rust
//! Simpliciality measures (`xgi/algorithms/simpliciality.py`).
//!
//! A face's subfaces are its powerset for sizes `min_size..len-1`. Edge
//! membership is tested against a set of canonicalized (sorted) member lists.

use std::collections::HashSet;

use crate::Hypergraph;

/// Sorted-member set of all edges, for O(1) subface presence queries.
struct FaceSet {
    faces: HashSet<Vec<String>>,
}

impl FaceSet {
    fn build<N, E, M>(h: &Hypergraph<N, E, M>) -> Self {
        let mut faces = HashSet::new();
        for e in h.edge_ids() {
            let mut m = h.members(&e).unwrap_or_default();
            m.sort();
            faces.insert(m);
        }
        FaceSet { faces }
    }
    fn contains(&self, face: &[String]) -> bool {
        let mut key = face.to_vec();
        key.sort();
        self.faces.contains(&key)
    }
}

/// Subfaces of `face` for sizes in `min_size..=max_size`.
fn powerset(face: &[String], min_size: usize, max_size: usize) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let n = face.len();
    // Bounded: 2^n subsets, filtered by size (n is a single edge's arity).
    // The bound is only statically provable for n < 64 (u64 shift overflow
    // above that) — see the Task 11 "Bound note".
    debug_assert!(n < 64, "powerset: edge arity must be < 64");
    for mask in 0u64..(1u64 << n) {
        let size = mask.count_ones() as usize;
        if size >= min_size && size <= max_size {
            let subset: Vec<String> = (0..n)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| face[i].clone())
                .collect();
            out.push(subset);
        }
    }
    out
}

/// Number of subfaces (sizes `min_size..len-1`) of `face` absent from the set.
fn count_missing_subfaces(fs: &FaceSet, face: &[String], min_size: usize) -> usize {
    if face.len() < min_size + 1 {
        return 0;
    }
    powerset(face, min_size, face.len() - 1)
        .into_iter()
        .filter(|sub| !fs.contains(sub))
        .count()
}

/// Maximum possible subfaces of a face of `max_size`, sizes ≥ `min_size`,
/// excluding the face itself and the empty set. XGI: `_max_number_of_subfaces`.
fn max_number_of_subfaces(min_size: usize, max_size: usize) -> usize {
    let mut d: i128 = 2i128.pow(max_size as u32) - 2;
    for i in 1..min_size {
        d -= comb(max_size, i) as i128;
    }
    d.max(0) as usize
}

fn comb(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num *= (n - i) as u128;
        den *= (i + 1) as u128;
    }
    (num / den) as usize
}

/// Edge IDs whose member sets are maximal (not a subset of another edge).
fn maximal_edges<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<String> {
    let all: Vec<(String, HashSet<String>)> = h
        .edge_ids()
        .into_iter()
        .map(|e| {
            let set: HashSet<String> = h.members(&e).unwrap_or_default().into_iter().collect();
            (e, set)
        })
        .collect();
    all.iter()
        .filter(|(_id, set)| {
            !all.iter().any(|(_oid, other)| {
                other.len() > set.len() && set.is_subset(other)
            })
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Whether an edge is a simplex: every subface (size ≥ min_size) is present.
fn is_simplex(fs: &FaceSet, edge: &[String], min_size: usize) -> bool {
    if edge.len() < min_size {
        return true;
    }
    powerset(edge, min_size, edge.len())
        .into_iter()
        .all(|sub| fs.contains(&sub))
}

/// Simplicial fraction: what fraction of potential-simplex edges are simplices.
/// XGI parity: `simplicial_fraction` (`simpliciality.py:325-383`).
pub fn simplicial_fraction<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let candidates: Vec<Vec<String>> = h
        .edge_ids()
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    let ps = candidates.len();
    if ps == 0 {
        return f64::NAN;
    }
    let ns = candidates
        .iter()
        .filter(|e| is_simplex(&fs, e, min_size))
        .count();
    ns as f64 / ps as f64
}

/// Simplicial edit distance (normalized): fraction of missing subfaces needed
/// to make `H` a simplicial complex. XGI parity: `simplicial_edit_distance`
/// (`simpliciality.py:79-182`). Simplified to the non-redundant count — the
/// redundant-face correction (`:157-172`) is a de-duplication that does not
/// change the value when maximal faces are member-disjoint; the general
/// correction is tracked in Open Questions.
pub fn simplicial_edit_distance<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let max_faces: Vec<Vec<String>> = maximal_edges(h)
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    if max_faces.is_empty() {
        return f64::NAN;
    }
    let ms: usize = max_faces
        .iter()
        .map(|e| count_missing_subfaces(&fs, e, min_size))
        .sum();
    if !normalize {
        return ms as f64;
    }
    let s = h
        .edge_ids()
        .into_iter()
        .filter(|e| h.edge_size(e).map(|sz| sz >= min_size).unwrap_or(false))
        .count();
    let mf = max_faces.len();
    let denom = s as i128 - mf as i128 + ms as i128;
    if denom > 0 {
        ms as f64 / denom as f64
    } else {
        f64::NAN
    }
}

/// Edit simpliciality = 1 − simplicial_edit_distance.
/// XGI parity: `edit_simpliciality` (`simpliciality.py:17-76`).
pub fn edit_simpliciality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    1.0 - simplicial_edit_distance(h, min_size, exclude_min_size, true)
}

/// Mean face edit distance over maximal faces.
/// XGI parity: `mean_face_edit_distance` (`simpliciality.py:248-322`).
pub fn mean_face_edit_distance<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let max_faces: Vec<Vec<String>> = maximal_edges(h)
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    if max_faces.is_empty() {
        return 0.0;
    }
    let count = max_faces.len() as f64;
    let mut avg = 0.0;
    for e in &max_faces {
        if e.len() >= min_size {
            let mut d = count_missing_subfaces(&fs, e, min_size) as f64;
            let m = max_number_of_subfaces(min_size, e.len());
            if normalize && m != 0 {
                d /= m as f64;
            }
            avg += d / count;
        }
    }
    avg
}

/// Face edit simpliciality = 1 − mean_face_edit_distance.
/// XGI parity: `face_edit_simpliciality` (`simpliciality.py:185-245`).
pub fn face_edit_simpliciality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    1.0 - mean_face_edit_distance(h, min_size, exclude_min_size, true)
}
```

- [ ] **Step 4: Run test to verify it passes.** Expected: PASS — 29 tests.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): algorithms::simpliciality (fraction + edit + face variants)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 12: clippy + fmt + full-suite gate

**Files:** any with clippy warnings.

- [ ] **Step 1: Clippy the core crate (default features).** `cd hypergraph-rs && cargo clippy -p hypergraph-rs -- -D warnings`. Fix every warning. Likely fixers: `needless_range_loop` on the matrix/diagonal loops — allow with a one-line justification where the index is used to write `m[[i,i]]`, or rewrite with `.indexed_iter_mut()` where clean; `clippy::type_complexity` on the `(Array2<f64>, Vec<String>, Vec<String>)` returns — add a `#[allow(clippy::type_complexity)]` with the rationale "the (matrix, row_ids, col_ids) triple is the XGI index=True contract".

- [ ] **Step 2: Clippy the whole workspace + the `algorithms`-off build.** `cargo clippy --workspace -- -D warnings` and `cargo clippy -p hypergraph-rs --no-default-features -- -D warnings` (verifies `linalg` still compiles with `algorithms` disabled — no `#[cfg]`-gated symbol leaks into `linalg`). Expected: clean.

- [ ] **Step 3: Full suite.** `cd hypergraph-rs && cargo test --workspace`. Expected: PASS — Phase 0+1's 64 + Task 5's 4 core helper tests (68 in `test_hypergraph`), 17 in `test_linalg`, 29 in `test_algorithms`. The pyo3/wasm/cli stub crates contribute 0.

- [ ] **Step 4: rustfmt.** `cd hypergraph-rs && cargo fmt --all`.

- [ ] **Step 5: Commit.**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs): clippy + fmt cleanup for linalg + algorithms

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 3 Completion Summary

After all 12 tasks, `hypergraph-rs` has:

- **`linalg` (unconditional core):** `incidence_matrix` (dense) + `incidence_matrix_sparse` (`sprs::CsMat`), `adjacency_matrix` (binary/weighted, `s`-threshold, order filter), `clique_motif_matrix`, `intersection_profile`, `degree_matrix`, `laplacian`, `multiorder_laplacian`, `normalized_hypergraph_laplacian` — all row/col-ordered by node/edge insertion order (III.7); `ndarray = "0.17"` + `sprs = "0.11"`.
- **Core helpers:** `skeleton()` (undirected 1-skeleton), `node_neighbors()`, `node_degree()`, `edge_size()`.
- **`algorithms` (feature-gated, in default set):** `connected` (deterministic BFS components + rustworkx-core count delegation), `shortest_path` (unit-hop BFS), `properties` (is_uniform, max_edge_order, num_edges_order, unique_edge_sizes, degree_counts, degree_histogram, density, incidence_density, edge_neighborhood, is_possible_order), `clustering` (global/local/two-node), `assortativity` (dynamical + exact degree), `centrality` (katz + clique-eigenvector power iteration), `simpliciality` (fraction + edit + face variants).
- **The `[features]` table** from spec §10.1 (default = algorithms/generators/stats/readwrite/layout/viz).
- **Clippy + fmt clean; ~114 tests passing** across the three integration files.

**Deferred (Open Questions):** tensor centralities (h/z/uniform-h/node-edge/line-vector), the `exact=false` assortativity sampling path, `largest_connected_hypergraph`, `adjacency_tensor`, Hodge/boundary matrices, and the redundant-missing-face correction in `simplicial_edit_distance`.

**Next: Phase 4** — generators + stats + readwrite (needs the blessed `rand_pcg` pin resolved first).
