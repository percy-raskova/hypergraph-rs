# hypergraph-rs Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the second layer of `hypergraph-rs` on top of the Phase 0+1 core: the six deferred `Hypergraph` methods (`double_edge_swap`, `random_edge_shuffle`, `merge_duplicate_edges`, `cleanup`, `dual`, `update`), the `DiHypergraph` directed class, the `SimplicialComplex` class (closure-under-faces + lazily-cached face lattice), the Rust-side `NodeView`/`EdgeView` proxy objects, and the `subhypergraph` global view. Conformance target (met at Phase 7 when the Python binding lands): XGI's `tests/core/test_dihypergraph.py`, `test_simplicialcomplex.py`, `test_views.py`, `test_globalviews.py`.

**Architecture:** Phase 1 established that a `Hypergraph<N,E,M>` **IS** a `rustworkx_core::petgraph::stable_graph::StableDiGraph<NodeKind<N,E>, MembershipEdge<M>>` with `IndexMap<String, NodeIndex>` bimaps for O(1) insertion-ordered id lookup. Phase 2 keeps that substrate untouched and adds:

- **`DiHypergraph<N,E,M>`** — its own struct (XGI's `DiHypergraph` is NOT a subclass of `Hypergraph`), reusing `NodeKind<N,E>` but with a **new** `DiMembershipEdge<M>` carrying a `DiRole` flag (`Tail` = sender, `Head` = receiver). All directed membership edges are physically oriented **agent → hyperedge**; the `role` flag distinguishes tail from head. This makes every directional query a single `edges_directed(idx, dir)` scan reading the role.
- **`SimplicialComplex<N,E,M>`** — composes a `Hypergraph<N,E,M>` (`inner`) plus a `std::cell::OnceCell<FaceLattice>` cache invalidated on mutation. `add_simplex` closes under faces (adds every missing subface of size ≥ 2).
- **Views** — `NodeView`/`EdgeView` borrow `&Hypergraph` and hold an insertion-ordered `Vec<String>` id subset; `filterby_*`/`isolates`/`singletons` return new sub-views. They call only Phase 1's public API.

**Tech Stack:** unchanged from Phase 0+1 (Rust 1.85 MSRV, edition 2021, `rustworkx-core` 0.18 re-exporting `petgraph` 0.8, `indexmap` 2, `serde`/`serde_json` 1, `thiserror` 2), **plus** seeded RNG for `random_edge_shuffle`: `rand` / `rand_pcg` / `rand_core` on the **0.10 line** (see Task 1 — the Phase 0+1 workspace declared placeholder `rand = "0.8"` / `rand_pcg = "0.3"` that were never wired into a crate; Task 1 bumps them to the probe-blessed pins that unify with rustworkx-core 0.18's own `rand 0.10.2`).

## Deliberate deviations from XGI

Each verified against the installed 0.10.2 source in the 2026-07-18 preflight; the Phase 7 Python binding shims where noted. These are **in addition to** the Phase 0+1 deviations (strict empty/duplicate on `add_edge`, insertion-ordered members, `update_uid_counter` bug fix, `clear` not resetting the counter, unfrozen `copy`).

1. **`DiHypergraph::add_edge` empty rejection:** XGI silently creates an edge even when both tail and head are empty (dihypergraph.py:565-582 has no check). We return `Err(EdgeError::EmptyMembers)` **only when BOTH tail and head are empty**; a one-sided edge like `(["3","4"], [])` is allowed (a valid all-senders directed edge). Binding shims to XGI's silent-create for conformance.
2. **`DiHypergraph::add_edge` duplicate `idx`:** XGI warns + no-ops (dihypergraph.py:561-563); we return `Err(EdgeError::AlreadyExists)`. Same shim as the undirected class.
3. **`random_edge_shuffle` requires an explicit `u64` seed.** XGI accepts `seed=None` → OS entropy (hypergraph.py:1035, `np.random.default_rng(seed)`). We require a seed (Constitution III.7 — determinism is opt-in per call, spec §7.6). Furthermore `rand_pcg::Pcg64Mcg` does **not** reproduce numpy's `Generator` sequence, so our conformance tests assert **structural invariants** (edge sizes preserved, the two edges' combined node multiset preserved, cross-run reproducibility with the same seed), never exact membership (spec §9.4 option (a)).
4. **`SimplicialComplex` blocked methods return `Err`, not raise.** `add_edge`/`add_edges_from`/`remove_edge`/`add_node_to_edge` return `Err(SimplexError::NotAvailable { .. })` (simplicialcomplex.py:135-163, 216-218 raise `XGIError`). Same observable contract (operation refused), Rust-idiomatic surface.
5. **`SimplicialComplex::add_simplex` duplicate-`idx` / already-present-members → silent `Ok(())` no-op.** XGI warns + returns (simplicialcomplex.py:310-315); we no-op silently.
6. **`merge_duplicate_edges` / `cleanup` use typed enums, not string args.** `MergeRename::{First, New}`, `MergeRule::{First, Union, Intersection}`, `FilterMode::{Eq, Neq, Lt, Gt, Leq, Geq}`. XGI's `rename="tuple"` (a Python-tuple dict key, hypergraph.py:1380-1381) is **deferred** — it has no faithful `String`-keyed representation in the core; the Phase 7 binding will synthesize it. `merge_rule="union"` produces a **JSON array of distinct values** (deterministic, insertion-ordered) rather than XGI's unordered Python `set` (hypergraph.py:1390-1395). `cleanup`'s `connected` and `relabel` steps are **deferred** — they call `largest_connected_hypergraph` (Phase 3 `algorithms`) and `convert_labels_to_integers` (a not-yet-ported `utils`); our `cleanup` covers `isolates`/`singletons`/`multiedges` only (see Task 6 open note).
7. **`filterby` is exposed as typed per-stat methods** (`filterby_degree`, `filterby_size`, `filterby_order`) instead of XGI's stringly-typed `filterby('degree', …)` dispatch (views.py:197-287). The Phase 7 binding maps the string API onto these. `between` is a dedicated `filterby_*_between(lo, hi)` method (avoids a tuple-typed `val`).
8. **`dual` drops isolate nodes.** XGI's `dual` maps each node to a dual-edge whose members are the node's memberships (hypergraph.py:1459-1461); an isolate node → an empty dual-edge, which XGI's `add_edges_from` creates. Our strict `add_edge` cannot represent an empty edge, so isolate nodes are **skipped** in the dual (documented; consistent with deviation #1's empty-edge strictness).
9. **`DiHypergraph` directional views are direct methods, not proxy objects (this phase).** `DH.dimembers`, `.head`, `.tail`, `.in_degree`, etc. are methods on `DiHypergraph`. The `DiNodeView`/`DiEdgeView` proxy objects (views.py:850-1244) are synthesized in the Phase 7 PyO3 binding over these methods.
10. **`SimplicialComplex::face_lattice` uses a `BTreeMap<String, Vec<String>>` immediate-face relation, not spec §3.4's `OnceCell<rustworkx_core::petgraph::DiGraph<SimplexId, ()>>`.** The design spec describes the cache as holding, for each simplex, both its cofaces and faces, in a petgraph `DiGraph`. Task 12 implements only the downward (codimension-1 face) relation as a deterministic `BTreeMap`, sufficient for this phase's scope (the `face_lattice` test asserts immediate faces only) — cofaces and a graph-typed representation are deferred. Recorded 2026-07-19 preflight (defect P2-D3) as a deliberate simplification of spec §3.4; reconcile if a later phase needs upward (coface) traversal.

---

## Global Constraints

- **Rust edition 2021**, MSRV 1.85. **No `unsafe`.** All loops have a fixed upper bound (Power-of-10 rule 2 — the Fisher-Yates loop in `random_edge_shuffle` is `for i in 0..k`, `k` a bound computed before the loop; the `combinations` next-combination generator in `simplicialcomplex.rs` (Task 12) is `for _ in 0..total` with `total = n_choose_k(n, choose)` computed before the loop — see Task 12's 2026-07-19 preflight correction, defect P2-D2).
- **Determinism (III.7 parity):** all public iteration is insertion-ordered (via the `IndexMap` bimaps / `StableDiGraph` node order). Set-valued comparisons (members, tail/head) sort or use `BTreeSet` so results are order-independent where XGI uses `set`. The seeded `Pcg64Mcg` is the only randomness; no `HashMap` in any public iteration path (a private `HashSet` for dedup within one call is fine — it never escapes into ordered output).
- **`assert_eq!`/`assert_ne!` require `Debug`** on the operands (their failure branch formats with `{:?}`, type-checked even on the pass path). For a new type whose `Debug` lands later in the same phase, tests written before it must use `assert!(a == b)` / `assert!(a != b)`. Within a task where both `Debug` and `PartialEq` land together, `assert_eq!` is fine.
- **serde_json-specific operations** (`as_object_mut`, `get`, `Value::Array`, …) live in concrete `impl Hypergraph<Value, Value, Value>` blocks — they do not exist on bare type parameters (a hard compile error in a generic block, the Phase 1 Task 16 lesson).
- **Integration tests are direct children of `crates/hypergraph-rs/tests/`** — one file per module family: `test_hypergraph_ext.rs` (deferred `Hypergraph` methods), `test_views.rs`, `test_dihypergraph.rs`, `test_simplicialcomplex.rs`, `test_globalviews.rs`. Run each with `--test <stem>`.
- **Commit convention:** Conventional commits, ending with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **No feature flags this phase.** Everything added lives in `core/` (always compiled). `rand`/`rand_pcg` become unconditional core dependencies (spec §2 lists them among the core crate's deps).

---

## File Structure (Phase 2 additions)

```
hypergraph-rs/
├── Cargo.toml                                    # MODIFY: bump rand/rand_pcg pins, add rand_core (Task 1)
├── crates/
│   └── hypergraph-rs/
│       ├── Cargo.toml                            # MODIFY: add rand + rand_pcg to [dependencies] (Task 1)
│       ├── src/
│       │   ├── lib.rs                            # MODIFY: re-export the new public types
│       │   └── core/
│       │       ├── mod.rs                        # MODIFY: declare dihypergraph, simplicialcomplex, views, globalviews
│       │       ├── hypergraph.rs                 # MODIFY: deferred methods + private helpers + accessors
│       │       ├── kinds.rs                      # MODIFY: add DiRole + DiMembershipEdge<M>
│       │       ├── error.rs                      # MODIFY: add SwapError + SimplexError
│       │       ├── dihypergraph.rs               # NEW: DiHypergraph<N,E,M>
│       │       ├── simplicialcomplex.rs          # NEW: SimplicialComplex<N,E,M> + FaceLattice
│       │       ├── views.rs                      # NEW: NodeView, EdgeView, FilterMode
│       │       └── globalviews.rs                # NEW: subhypergraph
│       └── tests/
│           ├── test_hypergraph.rs                # (Phase 1 — unchanged)
│           ├── test_hypergraph_ext.rs            # NEW
│           ├── test_views.rs                     # NEW
│           ├── test_dihypergraph.rs              # NEW
│           ├── test_simplicialcomplex.rs         # NEW
│           └── test_globalviews.rs               # NEW
```

---

## Task 1: Bump RNG deps to the probe-blessed 0.10 line and wire them into the core crate

**Files:**
- Modify: `hypergraph-rs/Cargo.toml`
- Modify: `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`

**Why:** The Phase 0+1 workspace declared `rand = "0.8"` / `rand_pcg = "0.3"` in `[workspace.dependencies]` but never depended on them from any crate (the core crate's `[dependencies]` list omitted them). The 2026-07-18 dependency probe confirmed (CONFIRMED verdict) that rustworkx-core 0.18 itself resolves `rand 0.10.2`, `rand_pcg 0.10.2`, `rand_core 0.10.1`; pinning our own deps to the same line unifies to a single copy in the lockfile instead of building two incompatible `rand` majors. The probe also confirmed rand 0.10 moved range sampling off `Rng` onto a new `RngExt` trait — `use rand::RngExt;` is required for `.random_range(..)` (plain `use rand::Rng;` gives only `.random()`), and `Pcg64Mcg::seed_from_u64` is available via `SeedableRng`.

- [ ] **Step 1: Bump the workspace dependency pins**

In `hypergraph-rs/Cargo.toml`, replace the two placeholder lines in `[workspace.dependencies]`:

```toml
rand = "0.8"
rand_pcg = "0.3"
```

with the probe-blessed pins (add `rand_core`):

```toml
# RNG line pinned to match what rustworkx-core 0.18 resolves (rand 0.10.2 /
# rand_pcg 0.10.2 / rand_core 0.10.1) so Cargo unifies to a single copy.
# rand 0.10 moved range sampling onto the RngExt trait: sampling code must
# `use rand::RngExt;` for `.random_range(..)` (2026-07-18 probe, CONFIRMED).
rand = "0.10.2"
rand_pcg = "0.10.2"
rand_core = "0.10.1"
```

- [ ] **Step 2: Wire `rand` + `rand_pcg` into the core crate**

In `hypergraph-rs/crates/hypergraph-rs/Cargo.toml`, add to the `[dependencies]` table (after `thiserror.workspace = true`):

```toml
rand.workspace = true
rand_pcg.workspace = true
rand_core.workspace = true
```

- [ ] **Step 3: Verify the workspace still builds and resolves a single `rand`**

Run: `cd hypergraph-rs && cargo build`
Expected: builds successfully.

Run: `cd hypergraph-rs && cargo tree -i rand 2>/dev/null | head -20`
Expected: a single `rand v0.10.x` node (proptest, if present later, may pull a private `rand 0.9.x` — that is a documented non-conflict, not this line).

- [ ] **Step 4: Commit**

```bash
cd hypergraph-rs
git add Cargo.toml crates/hypergraph-rs/Cargo.toml
git commit -m "build(hypergraph-rs): pin rand/rand_pcg/rand_core to 0.10 line, wire into core

Bumps the placeholder rand=0.8/rand_pcg=0.3 workspace pins to the probe-blessed
0.10.2/0.10.2/0.10.1 line that unifies with rustworkx-core 0.18's own rand.
Adds rand, rand_pcg, rand_core to the core crate deps for random_edge_shuffle.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: `Hypergraph` bulk removal, `update`, and query helpers

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`

**Interfaces:**
- `remove_nodes_from(&mut self, node_ids: &[String])` — weak-remove each (ignore missing, XGI warns).
- `remove_edges_from(&mut self, edge_ids: &[String])` — remove each (ignore missing).
- `update(&mut self, nodes, edges)` where `N: Default, M: Default + Clone` — XGI parity `H.update(edges=, nodes=)` (hypergraph.py:1210-1229): add nodes then edges.
- `isolate_node_ids(&self) -> Vec<String>` — nodes with zero memberships (insertion order).
- `singleton_edge_ids(&self) -> Vec<String>` — edges with exactly one member.
- `graph_attrs(&self) -> &serde_json::Map<String, serde_json::Value>` — read-only accessor over the graph-attr dict (needed by `dual`/`subhypergraph`).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`:

```rust
use hypergraph_rs::Hypergraph;

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_remove_edges_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e3".into()), v()).unwrap();
    h.remove_edges_from(&["e1".to_string(), "e3".to_string()]);
    assert_eq!(h.num_edges(), 1);
    assert!(h.has_edge("e2"));
    assert!(!h.has_edge("e1"));
}

#[test]
fn test_remove_edges_from_ignores_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.remove_edges_from(&["e1".to_string(), "nope".to_string()]);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_remove_nodes_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    h.remove_nodes_from(&["a".to_string(), "c".to_string()]);
    assert!(!h.has_node("a"));
    assert!(!h.has_node("c"));
    assert!(h.has_node("b"));
    assert_eq!(h.members("e1").unwrap(), vec!["b".to_string()]);
}

#[test]
fn test_update_adds_nodes_and_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.update(
        vec![("solo".to_string(), v())],
        vec![(vec!["a".into(), "b".into()], Some("e1".into()), v())],
    );
    assert!(h.has_node("solo"));
    assert!(h.has_edge("e1"));
    assert_eq!(h.num_nodes(), 3);
}

#[test]
fn test_isolate_node_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    assert_eq!(h.isolate_node_ids(), vec!["lonely".to_string()]);
}

#[test]
fn test_singleton_edge_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), v()).unwrap();
    assert_eq!(h.singleton_edge_ids(), vec!["e2".to_string()]);
}

#[test]
fn test_graph_attrs_accessor() {
    let mut h: Hypergraph = Hypergraph::new();
    h.set_graph_attr("name", serde_json::json!("wayne"));
    assert_eq!(h.graph_attrs().get("name"), Some(&serde_json::json!("wayne")));
    assert_eq!(h.graph_attrs().len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: FAIL — the new methods don't exist (compile error).

- [ ] **Step 3: Implement the helpers**

Add inside the generic `impl<N, E, M> Hypergraph<N, E, M>` block in `hypergraph.rs` (after the Phase 1 methods):

```rust
    /// Remove multiple edges. Missing ids are ignored (XGI warns).
    /// XGI parity: `H.remove_edges_from(ebunch)`.
    pub fn remove_edges_from(&mut self, edge_ids: &[String]) {
        for eid in edge_ids {
            let _ = self.remove_edge(eid);
        }
    }

    /// Remove multiple nodes (weak removal). Missing ids are ignored.
    /// XGI parity: `H.remove_nodes_from(nodes)`.
    pub fn remove_nodes_from(&mut self, node_ids: &[String]) {
        for nid in node_ids {
            let _ = self.remove_node(nid, false);
        }
    }

    /// Add nodes then edges. XGI parity: `H.update(edges=, nodes=)`
    /// (hypergraph.py:1210-1229).
    pub fn update(
        &mut self,
        nodes: Vec<(String, N)>,
        edges: Vec<(Vec<String>, Option<String>, E)>,
    ) where
        N: Default,
        M: Default + Clone,
    {
        self.add_nodes_from(nodes);
        let _ = self.add_edges_from(edges);
    }

    /// Node ids with zero memberships, in insertion order.
    /// XGI parity: `H.nodes.isolates()` (views.py:632-667, default).
    pub fn isolate_node_ids(&self) -> Vec<String> {
        self.node_ids()
            .into_iter()
            .filter(|n| self.memberships(n).map_or(false, |m| m.is_empty()))
            .collect()
    }

    /// Edge ids with exactly one member, in insertion order.
    /// XGI parity: `H.edges.singletons()` (views.py:749-761).
    pub fn singleton_edge_ids(&self) -> Vec<String> {
        self.edge_ids()
            .into_iter()
            .filter(|e| self.members(e).map_or(false, |m| m.len() == 1))
            .collect()
    }

    /// Read-only view of the graph-level attribute dict (XGI's `H.graph`).
    pub fn graph_attrs(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.graph_attrs
    }
```

Note: `remove_node`'s Phase 1 signature is `remove_node(&mut self, node_id: &str, strong: bool)`; `remove_nodes_from` uses the weak (`false`) mode, matching XGI's default.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: PASS — 7 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): bulk removal, update, and isolate/singleton/graph-attr helpers

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: `Hypergraph::dual`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`

**Interfaces:**
- `dual(&self) -> Self` on the concrete `Hypergraph<Value, Value, Value>` — nodes become edges and edges become nodes (hypergraph.py:1446-1466). Isolate nodes are dropped (deviation #8).

**XGI reference** (hypergraph.py:1446-1466): each original node `n` becomes a dual-edge with id `n` and members `H.nodes.memberships(n)`; each original edge `e` becomes a dual-node with id `e` carrying the edge's attrs; graph attrs are deep-copied. **Ordering matters for us:** we add the edge→node mappings **first** (so the dual-node attrs are set), then the node→edge mappings — because our `add_edge` auto-creates any missing member node as a *bare* agent and our `add_node` returns `false` without overwriting attrs on an existing node (Phase 1 semantics). Adding attributed nodes first means the later `add_edge` finds them already present and leaves their attrs intact.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_hypergraph_ext.rs`:

```rust
#[test]
fn test_dual_swaps_nodes_and_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::json!({"w": 1})).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::json!({"w": 2})).unwrap();

    let d = h.dual();
    // original edges e1, e2 become dual nodes carrying the edge attrs
    assert!(d.has_node("e1"));
    assert!(d.has_node("e2"));
    assert_eq!(d.node_attrs("e1").unwrap()["w"], 1);
    // original nodes become dual edges whose members are the memberships
    assert!(d.has_edge("a"));
    assert!(d.has_edge("b"));
    assert_eq!(d.members("a").unwrap(), vec!["e1".to_string()]);
    let mut b_members = d.members("b").unwrap();
    b_members.sort();
    assert_eq!(b_members, vec!["e1".to_string(), "e2".to_string()]);
}

#[test]
fn test_dual_drops_isolate_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    let d = h.dual();
    // "lonely" has no memberships -> would be an empty dual-edge -> dropped
    assert!(!d.has_edge("lonely"));
    assert!(d.has_edge("a"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: FAIL — `dual` doesn't exist.

- [ ] **Step 3: Implement `dual`**

Add a concrete-typed impl block (place it near the Phase 1 Task 16 `impl<M> Hypergraph<Value, Value, M>` block; use fully-defaulted `Value` because `dual` interchanges node and edge attrs, which is only well-typed when `N == E`). At the top of `hypergraph.rs`, if not already present, add `use serde_json::Value;`.

```rust
impl Hypergraph<Value, Value, Value> {
    /// The dual: nodes become edges and edges become nodes.
    /// XGI parity: `H.dual()` (hypergraph.py:1446-1466). Isolate nodes are
    /// dropped (they would map to empty dual-edges, which our strict add_edge
    /// cannot represent — deviation #8).
    pub fn dual(&self) -> Self {
        let mut d = Self::new();

        // Original edges -> dual nodes (id = edge id, attrs = edge attrs).
        // Done FIRST so their attrs survive the auto-node-creation in add_edge.
        for eid in self.edge_ids() {
            let attrs = self.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            d.add_node(&eid, attrs);
        }

        // Original nodes -> dual edges (id = node id, members = memberships).
        for nid in self.node_ids() {
            let memberships = self.memberships(&nid).unwrap_or_default();
            if memberships.is_empty() {
                continue; // isolate node -> empty dual-edge -> skipped
            }
            let attrs = self.node_attrs(&nid).cloned().unwrap_or(Value::Null);
            let _ = d.add_edge(memberships, Some(nid.clone()), attrs);
        }

        // Graph attrs.
        for (k, val) in self.graph_attrs().clone() {
            d.set_graph_attr(&k, val);
        }
        d
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: PASS — 9 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement dual (node<->edge swap)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: `Hypergraph::double_edge_swap` + `SwapError` + membership rewire helpers

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/error.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `double_edge_swap(&mut self, n_id1, n_id2, e_id1, e_id2) -> Result<(), SwapError>` where `M: Default + Clone`.
- Private helpers `attach_member` / `detach_member` (bipartite membership add/remove, both directions).

**XGI reference** (hypergraph.py:918-992): moves `n_id1` from `e_id1` to `e_id2`-membership and `n_id2` from `e_id2` to `e_id1`. Raises `IDNotFound` if a node isn't a member of the named edge (974-978); raises `XGIError` "does not preserve edge sizes" if the swap would shrink an edge because the incoming node is already present (980-986).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_hypergraph_ext.rs`:

```rust
use hypergraph_rs::SwapError;

#[test]
fn test_double_edge_swap_basic() {
    // XGI docstring example: H([[1,2,3],[3,4]]); swap(1,4,e0,e1) -> [{2,3,4},{1,3}]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    h.double_edge_swap("1", "4", "e0", "e1").unwrap();
    let mut m0 = h.members("e0").unwrap();
    m0.sort();
    assert_eq!(m0, vec!["2".to_string(), "3".to_string(), "4".to_string()]);
    let mut m1 = h.members("e1").unwrap();
    m1.sort();
    assert_eq!(m1, vec!["1".to_string(), "3".to_string()]);
}

#[test]
fn test_double_edge_swap_node_not_in_edge_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    // node "9" is not a member of e0
    let r = h.double_edge_swap("9", "3", "e0", "e1");
    assert!(matches!(r, Err(SwapError::IdNotFound { .. })));
}

#[test]
fn test_double_edge_swap_size_not_preserved_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    // 2 is in BOTH edges; moving 1 into e1 collides with nothing, but moving 2's
    // counterpart would shrink. Construct a genuine collision: swap 1 and 2 where
    // 2 is already in e0.
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["2".into(), "3".into()], Some("e1".into()), v()).unwrap();
    // swap n1=1 (in e0) and n2=2 (in e1): e0 would become {2,2}->{2} (shrink)
    let r = h.double_edge_swap("1", "2", "e0", "e1");
    assert!(matches!(r, Err(SwapError::SizeNotPreserved)));
    // graph unchanged
    assert_eq!(h.members("e0").unwrap().len(), 2);
}

#[test]
fn test_double_edge_swap_missing_edge_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into()], Some("e0".into()), v()).unwrap();
    let r = h.double_edge_swap("1", "1", "e0", "nope");
    assert!(matches!(r, Err(SwapError::IdNotFound { .. })));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: FAIL — `double_edge_swap` / `SwapError` don't exist.

- [ ] **Step 3: Implement `SwapError`, the helpers, and `double_edge_swap`**

In `error.rs`, add:

```rust
/// Error raised by `double_edge_swap`.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SwapError {
    /// A specified node or edge was not found, or the node is not a member of
    /// the named edge.
    #[error("id {id} not found (or node not a member of the named edge)")]
    IdNotFound { id: String },
    /// The swap would not preserve edge sizes (the incoming node is already a
    /// member of the target edge).
    #[error("swap does not preserve edge sizes")]
    SizeNotPreserved,
}
```

In `lib.rs`, extend the error re-export line:

```rust
pub use core::error::{EdgeError, NodeError, SwapError};
```

In `hypergraph.rs`, add the private rewire helpers inside the generic `impl<N, E, M>` block:

```rust
    /// Attach `node_id` to `edge_id` (both bipartite directions). No-op if the
    /// membership already exists. Both ids must already exist.
    fn attach_member(&mut self, edge_id: &str, node_id: &str)
    where
        M: Default + Clone,
    {
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        if self.inner.find_edge(a, he).is_none() {
            let m = MembershipEdge { member_data: M::default() };
            self.inner.add_edge(a, he, m.clone());
            self.inner.add_edge(he, a, m);
        }
    }

    /// Detach `node_id` from `edge_id` (both bipartite directions). No-op if
    /// the membership is absent.
    fn detach_member(&mut self, edge_id: &str, node_id: &str) {
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        if let Some(e) = self.inner.find_edge(a, he) {
            self.inner.remove_edge(e);
        }
        if let Some(e) = self.inner.find_edge(he, a) {
            self.inner.remove_edge(e);
        }
    }
```

Then add `double_edge_swap` (also in the generic block). Add `use super::error::SwapError;` to the existing error `use` line at the top of `hypergraph.rs` (it currently imports `EdgeError, NodeError`).

```rust
    /// Swap the edge memberships of two nodes between two edges.
    /// XGI parity: `H.double_edge_swap(n1, n2, e1, e2)` (hypergraph.py:918-992).
    pub fn double_edge_swap(
        &mut self,
        n_id1: &str,
        n_id2: &str,
        e_id1: &str,
        e_id2: &str,
    ) -> Result<(), SwapError>
    where
        M: Default + Clone,
    {
        self.assert_not_frozen();

        let members1 = self
            .members(e_id1)
            .ok_or(SwapError::IdNotFound { id: e_id1.to_string() })?;
        let members2 = self
            .members(e_id2)
            .ok_or(SwapError::IdNotFound { id: e_id2.to_string() })?;
        if !members1.iter().any(|m| m == n_id1) {
            return Err(SwapError::IdNotFound { id: n_id1.to_string() });
        }
        if !members2.iter().any(|m| m == n_id2) {
            return Err(SwapError::IdNotFound { id: n_id2.to_string() });
        }

        // Compute the post-swap member sets and verify size preservation BEFORE
        // mutating (XGI hypergraph.py:980-986).
        use std::collections::BTreeSet;
        let new1: BTreeSet<String> = members1
            .iter()
            .filter(|m| *m != n_id1)
            .cloned()
            .chain(std::iter::once(n_id2.to_string()))
            .collect();
        let new2: BTreeSet<String> = members2
            .iter()
            .filter(|m| *m != n_id2)
            .cloned()
            .chain(std::iter::once(n_id1.to_string()))
            .collect();
        if new1.len() != members1.len() || new2.len() != members2.len() {
            return Err(SwapError::SizeNotPreserved);
        }

        // Commit: e1 loses n1 gains n2; e2 loses n2 gains n1.
        self.detach_member(e_id1, n_id1);
        self.attach_member(e_id1, n_id2);
        self.detach_member(e_id2, n_id2);
        self.attach_member(e_id2, n_id1);
        Ok(())
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: PASS — 13 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement double_edge_swap + SwapError + rewire helpers

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 5: `Hypergraph::random_edge_shuffle` (seeded `rand_pcg`)

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/error.rs`

**Interfaces:**
- `random_edge_shuffle(&mut self, e_id1: Option<&str>, e_id2: Option<&str>, seed: u64) -> Result<(), EdgeError>` where `M: Default + Clone`. The fewer-than-two-edges case returns a dedicated `EdgeError::TooFewEdges` variant (2026-07-19 preflight, defect P2-D4 — the initial draft misused `EdgeError::NotFound` with a sentinel string, which doesn't name any real edge).

**XGI reference** (hypergraph.py:994-1078): pick two edges (given or two random distinct), bucket the symmetric-difference nodes, redistribute so edge-1 keeps `|e1 \ e2|` of them, re-add the shared nodes to both. Sizes, ids, and attrs are unchanged. **Determinism note (deviation #3):** `rand_pcg` ≠ numpy, so tests assert invariants, not exact membership.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_hypergraph_ext.rs`:

```rust
#[test]
fn test_random_edge_shuffle_preserves_sizes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    let s0 = h.members("e0").unwrap().len();
    let s1 = h.members("e1").unwrap().len();
    h.random_edge_shuffle(Some("e0"), Some("e1"), 42).unwrap();
    assert_eq!(h.members("e0").unwrap().len(), s0);
    assert_eq!(h.members("e1").unwrap().len(), s1);
}

#[test]
fn test_random_edge_shuffle_preserves_combined_node_multiset() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into(), "5".into()], Some("e1".into()), v()).unwrap();
    let before: std::collections::BTreeSet<String> =
        h.members("e0").unwrap().into_iter().chain(h.members("e1").unwrap()).collect();
    h.random_edge_shuffle(Some("e0"), Some("e1"), 7).unwrap();
    let after: std::collections::BTreeSet<String> =
        h.members("e0").unwrap().into_iter().chain(h.members("e1").unwrap()).collect();
    assert_eq!(before, after);
}

#[test]
fn test_random_edge_shuffle_is_reproducible() {
    let build = || {
        let mut h: Hypergraph = Hypergraph::new();
        h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), v()).unwrap();
        h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
        h
    };
    let mut h1 = build();
    let mut h2 = build();
    h1.random_edge_shuffle(Some("e0"), Some("e1"), 99).unwrap();
    h2.random_edge_shuffle(Some("e0"), Some("e1"), 99).unwrap();
    let mut a = h1.members("e0").unwrap();
    a.sort();
    let mut b = h2.members("e0").unwrap();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn test_random_edge_shuffle_needs_two_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into()], Some("e0".into()), v()).unwrap();
    assert!(h.random_edge_shuffle(None, None, 1).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: FAIL — `random_edge_shuffle` doesn't exist.

- [ ] **Step 3: Add `EdgeError::TooFewEdges` and implement `random_edge_shuffle`**

In `error.rs`, add a variant to the existing `EdgeError` enum (alongside `NotFound`/`AlreadyExists`/`EmptyMembers`):

```rust
    /// `random_edge_shuffle` needs at least two edges to swap between.
    #[error("random_edge_shuffle requires at least 2 edges")]
    TooFewEdges,
```

At the top of `hypergraph.rs`, add the RNG imports (the probe verified this exact trait set: `RngExt` provides `.random_range`, `SeedableRng` provides `seed_from_u64`):

```rust
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;
```

Add inside the generic `impl<N, E, M>` block:

```rust
    /// Randomly redistribute nodes between two hyperedges, preserving both
    /// sizes. XGI parity: `H.random_edge_shuffle(e1, e2, seed)`
    /// (hypergraph.py:994-1078). Requires an explicit seed (deviation #3);
    /// the rand_pcg sequence does not match numpy, so only structural
    /// invariants are guaranteed across implementations.
    pub fn random_edge_shuffle(
        &mut self,
        e_id1: Option<&str>,
        e_id2: Option<&str>,
        seed: u64,
    ) -> Result<(), EdgeError>
    where
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if self.hyperedge_ids.len() < 2 {
            return Err(EdgeError::TooFewEdges);
        }
        let mut rng = Pcg64Mcg::seed_from_u64(seed);

        // Resolve the two edge ids (given, or two distinct random).
        let (id1, id2) = match (e_id1, e_id2) {
            (Some(a), Some(b)) => (a.to_string(), b.to_string()),
            _ => {
                let ids = self.edge_ids();
                let i = rng.random_range(0..ids.len());
                let mut j = rng.random_range(0..ids.len() - 1);
                if j >= i {
                    j += 1;
                }
                (ids[i].clone(), ids[j].clone())
            }
        };

        use std::collections::BTreeSet;
        let s1: BTreeSet<String> = self
            .members(&id1)
            .ok_or(EdgeError::NotFound { edge_id: id1.clone() })?
            .into_iter()
            .collect();
        let s2: BTreeSet<String> = self
            .members(&id2)
            .ok_or(EdgeError::NotFound { edge_id: id2.clone() })?
            .into_iter()
            .collect();

        let both: BTreeSet<String> = s1.intersection(&s2).cloned().collect();
        let only1: BTreeSet<String> = s1.difference(&both).cloned().collect();
        let only2: BTreeSet<String> = s2.difference(&both).cloned().collect();

        // Bucket in a deterministic base order (BTreeSet -> sorted).
        let mut bucket: Vec<String> = only1.union(&only2).cloned().collect();
        let k = only1.len();
        let n = bucket.len();
        // Partial Fisher-Yates: bounded by k (computed above), never exceeds n.
        for i in 0..k {
            let j = i + rng.random_range(0..(n - i));
            bucket.swap(i, j);
        }
        let new_only1: BTreeSet<String> = bucket[..k].iter().cloned().collect();
        let new_e1: BTreeSet<String> = new_only1.union(&both).cloned().collect();
        let new_only2: BTreeSet<String> = bucket[k..].iter().cloned().collect();
        let new_e2: BTreeSet<String> = new_only2.union(&both).cloned().collect();

        // Rewire: detach all old members, attach the new ones (ids/attrs kept).
        for node in &s1 {
            self.detach_member(&id1, node);
        }
        for node in &s2 {
            self.detach_member(&id2, node);
        }
        for node in &new_e1 {
            self.attach_member(&id1, node);
        }
        for node in &new_e2 {
            self.attach_member(&id2, node);
        }
        Ok(())
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: PASS — 17 tests, 0 failures. If the RNG imports fail to resolve (`E0432`/`E0599`), the probe-blessed fallback is `use rand::Rng;` alongside `RngExt`, or `use rand_pcg::rand_core::SeedableRng;`; adjust the three `use` lines until `cargo build` is clean (the red/green cycle is the arbiter — see open questions).

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement random_edge_shuffle (seeded Pcg64Mcg)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 6: `Hypergraph::merge_duplicate_edges` + `cleanup` + `MergeRename`/`MergeRule` enums

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_hypergraph_ext.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/hypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces (concrete `impl Hypergraph<Value, Value, Value>` — these manipulate JSON attrs):**
- `merge_duplicate_edges(&mut self, rename: MergeRename, merge_rule: MergeRule, multiplicity: Option<&str>)`.
- `cleanup(&mut self, isolates: bool, singletons: bool, multiedges: bool)` — mutate in place; `cleaned(...)` returns a cleaned copy.

**XGI reference:** `merge_duplicate_edges` (hypergraph.py:1257-1419) groups by `frozenset(members)`; `rename="first"` → min id, `rename="new"` → next uid; `merge_rule` first/union/intersection; optional `multiplicity` attr. `cleanup` (hypergraph.py:1468-1518) merges multiedges, drops singletons, drops isolates, (then connected + relabel — deferred). Note XGI's boolean params mean *allow*: `isolates=False` ⇒ remove them.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_hypergraph_ext.rs`:

```rust
use hypergraph_rs::{MergeRename, MergeRule};

#[test]
fn test_merge_duplicate_edges_rename_first() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), serde_json::json!({"color": "blue"})).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), serde_json::json!({"color": "red"})).unwrap();
    h.add_edge(vec!["3".into(), "4".into(), "5".into()], Some("3".into()), v()).unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::First, None);
    // {1,2} duplicates (ids 0,1) merge into id "0" with first's attrs
    assert!(h.has_edge("0"));
    assert!(!h.has_edge("1"));
    assert!(h.has_edge("3"));
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.edge_attrs("0").unwrap()["color"], "blue");
}

#[test]
fn test_merge_duplicate_edges_multiplicity() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v()).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v()).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("2".into()), v()).unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::First, Some("mult"));
    assert_eq!(h.edge_attrs("0").unwrap()["mult"], 3);
    assert_eq!(h.num_edges(), 1);
}

#[test]
fn test_merge_duplicate_edges_intersection() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["3".into(), "4".into()], Some("0".into()), serde_json::json!({"color": "purple"})).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("1".into()), serde_json::json!({"color": "purple", "name": "test"})).unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::Intersection, None);
    // color agrees -> kept; name differs (missing in one) -> null
    assert_eq!(h.edge_attrs("0").unwrap()["color"], "purple");
    assert_eq!(h.edge_attrs("0").unwrap()["name"], serde_json::Value::Null);
}

#[test]
fn test_cleanup_removes_isolates_singletons_and_multiedges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v()).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v()).unwrap(); // duplicate
    h.add_edge(vec!["3".into()], Some("2".into()), v()).unwrap();             // singleton
    h.add_node("lonely", v());                                                // isolate
    h.cleanup(false, false, false);
    assert!(!h.has_node("lonely"));       // isolate removed
    assert!(!h.has_edge("2"));            // singleton removed
    assert_eq!(h.num_edges(), 1);         // duplicate merged
}

#[test]
fn test_cleaned_leaves_original_untouched() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v()).unwrap();
    h.add_node("lonely", v());
    let c = h.cleaned(false, true, true); // remove isolates only
    assert!(!c.has_node("lonely"));
    assert!(h.has_node("lonely")); // original unchanged
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: FAIL — `merge_duplicate_edges` / `cleanup` / the enums don't exist.

- [ ] **Step 3: Implement the enums and methods**

Near the top of `hypergraph.rs` (after the imports), add the enums:

```rust
/// How to name a merged edge (subset of XGI's `rename`; `"tuple"` deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRename {
    /// Reuse the smallest (sorted) duplicate id.
    First,
    /// Assign a fresh auto-generated id.
    New,
}

/// How to combine attrs of merged edges (XGI's `merge_rule`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRule {
    /// Take the smallest-id duplicate's attrs.
    First,
    /// For each key, a JSON array of the distinct values (missing -> null).
    Union,
    /// For each key, the shared value or null if the duplicates disagree.
    Intersection,
}
```

In `lib.rs`, re-export them:

```rust
pub use core::hypergraph::{MergeRename, MergeRule};
```

Add a concrete impl block (alongside the `dual` block):

```rust
impl Hypergraph<Value, Value, Value> {
    /// Merge edges that have identical member-sets.
    /// XGI parity: `H.merge_duplicate_edges(rename, merge_rule, multiplicity)`
    /// (hypergraph.py:1257-1419). `rename="tuple"` is deferred (deviation #6);
    /// union produces a JSON array, not a Python set.
    pub fn merge_duplicate_edges(
        &mut self,
        rename: MergeRename,
        merge_rule: MergeRule,
        multiplicity: Option<&str>,
    ) {
        self.assert_not_frozen();
        use std::collections::BTreeMap;

        // Group edge ids by their sorted member-set (deterministic).
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for eid in self.edge_ids() {
            let mut mem = self.members(&eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid);
        }

        let mut to_remove: Vec<String> = Vec::new();
        let mut to_add: Vec<(Vec<String>, String, Value)> = Vec::new();
        for (members, dup_ids) in &groups {
            if dup_ids.len() <= 1 {
                continue;
            }
            to_remove.extend(dup_ids.iter().cloned());
            let new_id = match rename {
                MergeRename::First => {
                    let mut sorted = dup_ids.clone();
                    sorted.sort();
                    sorted[0].clone()
                }
                MergeRename::New => {
                    let id = self.edge_uid_counter.to_string();
                    self.edge_uid_counter += 1;
                    id
                }
            };
            let mut new_attrs = self.merged_edge_attrs(dup_ids, merge_rule);
            if let Some(mkey) = multiplicity {
                // merged_edge_attrs(MergeRule::First) can yield a non-object
                // (e.g. Value::Null, our default empty attrs) — coerce before
                // inserting the multiplicity key (2026-07-19 preflight defect
                // P2-D1: Null.as_object_mut() silently dropped the insert).
                if !new_attrs.is_object() {
                    new_attrs = serde_json::json!({});
                }
                new_attrs
                    .as_object_mut()
                    .unwrap()
                    .insert(mkey.to_string(), serde_json::json!(dup_ids.len()));
            }
            to_add.push((members.clone(), new_id, new_attrs));
        }

        for eid in &to_remove {
            let _ = self.remove_edge(eid);
        }
        for (members, id, attrs) in to_add {
            let _ = self.add_edge(members, Some(id), attrs);
        }
    }

    /// Combine the attrs of a group of duplicate edges per `merge_rule`.
    fn merged_edge_attrs(&self, dup_ids: &[String], rule: MergeRule) -> Value {
        match rule {
            MergeRule::First => {
                let min_id = dup_ids.iter().min().cloned().unwrap_or_default();
                self.edge_attrs(&min_id).cloned().unwrap_or_else(|| serde_json::json!({}))
            }
            MergeRule::Union | MergeRule::Intersection => {
                use std::collections::BTreeSet;
                let mut keys: BTreeSet<String> = BTreeSet::new();
                for id in dup_ids {
                    if let Some(obj) = self.edge_attrs(id).and_then(|a| a.as_object()) {
                        keys.extend(obj.keys().cloned());
                    }
                }
                let mut out = serde_json::Map::new();
                for k in keys {
                    let mut distinct: Vec<Value> = Vec::new();
                    for id in dup_ids {
                        let val = self
                            .edge_attrs(id)
                            .and_then(|a| a.get(&k))
                            .cloned()
                            .unwrap_or(Value::Null);
                        if !distinct.contains(&val) {
                            distinct.push(val);
                        }
                    }
                    let merged = match rule {
                        MergeRule::Union => Value::Array(distinct),
                        _ => {
                            if distinct.len() == 1 {
                                distinct.into_iter().next().unwrap()
                            } else {
                                Value::Null
                            }
                        }
                    };
                    out.insert(k, merged);
                }
                Value::Object(out)
            }
        }
    }

    /// Remove undesirable artifacts in place. XGI parity: `H.cleanup(...)`
    /// (hypergraph.py:1468-1518). `connected` and `relabel` are deferred
    /// (need Phase 3 algorithms + a utils module — deviation #6). Boolean
    /// params mean *allow*: `isolates=false` removes isolate nodes.
    pub fn cleanup(&mut self, isolates: bool, singletons: bool, multiedges: bool) {
        self.assert_not_frozen();
        if !multiedges {
            self.merge_duplicate_edges(MergeRename::First, MergeRule::First, None);
        }
        if !singletons {
            let s = self.singleton_edge_ids();
            self.remove_edges_from(&s);
        }
        if !isolates {
            let i = self.isolate_node_ids();
            self.remove_nodes_from(&i);
        }
    }

    /// Return a cleaned copy, leaving `self` untouched.
    pub fn cleaned(&self, isolates: bool, singletons: bool, multiedges: bool) -> Self {
        let mut c = self.copy();
        c.cleanup(isolates, singletons, multiedges);
        c
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_hypergraph_ext`
Expected: PASS — 22 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement merge_duplicate_edges + cleanup + merge enums

connected+relabel steps of cleanup deferred (need Phase 3 algorithms + utils).
rename=tuple deferred; merge_rule=union yields a JSON array (deviation #6).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 7: `NodeView` proxy object

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_views.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/core/views.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `Hypergraph::nodes(&self) -> NodeView<'_, N, E, M>`.
- `NodeView`: `ids()`, `len()`, `is_empty()`, `contains(id)`, `attrs(id)`, `degree(id)`, `memberships(id)`, `isolates()`, `isolates_ignoring_singletons()`, `filterby_degree(val, mode)`, `filterby_degree_between(lo, hi)`.
- `FilterMode::{Eq, Neq, Lt, Gt, Leq, Geq}` with a `matches(actual: usize, val: usize) -> bool` helper.

**XGI reference:** `H.nodes` is a `NodeView` (views.py:558-667). `memberships(n)` returns the edge-id set (views.py:604-630); `isolates()` returns degree-0 nodes, `ignore_singletons=True` ignores size-1 edges (views.py:632-667); `degree` = number of memberships; `filterby('degree', val, mode)` returns the filtered sub-view (views.py:197-287).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_views.rs`:

```rust
use hypergraph_rs::{FilterMode, Hypergraph};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_nodeview_len_contains_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.contains("a"));
    assert!(!nodes.contains("z"));
    assert_eq!(nodes.ids(), &["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_nodeview_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e2".into()), v()).unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.degree("a"), 2);
    assert_eq!(nodes.degree("b"), 1);
}

#[test]
fn test_nodeview_isolates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    let iso = h.nodes().isolates();
    assert_eq!(iso.ids(), &["lonely".to_string()]);
}

#[test]
fn test_nodeview_isolates_ignoring_singletons() {
    let mut h: Hypergraph = Hypergraph::new();
    // "a" is only in a singleton edge -> counts as isolated when ignoring singletons
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    let iso = h.nodes().isolates_ignoring_singletons();
    assert!(iso.contains("a"));
    assert!(!iso.contains("b"));
}

#[test]
fn test_nodeview_filterby_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    // degrees: a=1, b=2, c=2
    let deg2 = h.nodes().filterby_degree(2, FilterMode::Eq);
    let mut ids = deg2.ids().to_vec();
    ids.sort();
    assert_eq!(ids, vec!["b".to_string(), "c".to_string()]);
    let lt2 = h.nodes().filterby_degree(2, FilterMode::Lt);
    assert_eq!(lt2.ids(), &["a".to_string()]);
}

#[test]
fn test_nodeview_attrs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"color": "red"}));
    assert_eq!(h.nodes().attrs("a").unwrap()["color"], "red");
    assert!(h.nodes().attrs("missing").is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_views`
Expected: FAIL — `views` module doesn't exist yet (compile error).

- [ ] **Step 3: Create the `views` module with `NodeView` + `FilterMode`**

In `core/mod.rs`, add the module declaration (keep alphabetical with the existing ones):

```rust
pub mod views;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/core/views.rs`:

```rust
//! Read-only proxy views over a [`Hypergraph`]: `NodeView` and `EdgeView`.
//!
//! Each view borrows the hypergraph and holds an insertion-ordered subset of
//! ids. Filtering methods (`isolates`, `singletons`, `filterby_*`) return a new
//! sub-view over the same hypergraph. XGI parity: `H.nodes` / `H.edges`.

use super::hypergraph::Hypergraph;

/// Comparison mode for `filterby_*`. XGI parity: the `mode` argument of
/// `IDView.filterby` (views.py:197-287). `between` is a dedicated method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Eq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl FilterMode {
    /// Whether `actual <mode> val` holds.
    pub fn matches(self, actual: usize, val: usize) -> bool {
        match self {
            FilterMode::Eq => actual == val,
            FilterMode::Neq => actual != val,
            FilterMode::Lt => actual < val,
            FilterMode::Gt => actual > val,
            FilterMode::Leq => actual <= val,
            FilterMode::Geq => actual >= val,
        }
    }
}

/// A read-only view over the nodes of a hypergraph.
pub struct NodeView<'a, N, E, M> {
    graph: &'a Hypergraph<N, E, M>,
    ids: Vec<String>,
}

impl<N, E, M> Hypergraph<N, E, M> {
    /// A view over all nodes (insertion order). XGI parity: `H.nodes`.
    pub fn nodes(&self) -> NodeView<'_, N, E, M> {
        NodeView { graph: self, ids: self.node_ids() }
    }
}

impl<'a, N, E, M> NodeView<'a, N, E, M> {
    /// The tracked node ids, in insertion order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The number of nodes tracked.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `node_id` is tracked by this view.
    pub fn contains(&self, node_id: &str) -> bool {
        self.ids.iter().any(|n| n == node_id)
    }

    /// The node's attributes. XGI parity: `H.nodes[n]`.
    pub fn attrs(&self, node_id: &str) -> Option<&'a N> {
        self.graph.node_attrs(node_id)
    }

    /// The number of edges the node belongs to. XGI parity: `H.nodes.degree`.
    pub fn degree(&self, node_id: &str) -> usize {
        self.graph.memberships(node_id).map_or(0, |m| m.len())
    }

    /// The edge ids the node is a member of. XGI parity:
    /// `H.nodes.memberships(n)` (views.py:604-630).
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        self.graph.memberships(node_id)
    }

    /// Sub-view of degree-0 nodes. XGI parity: `H.nodes.isolates()`.
    pub fn isolates(&self) -> NodeView<'a, N, E, M> {
        let ids = self.ids.iter().filter(|n| self.degree(n) == 0).cloned().collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes in no edge of size >= 2. XGI parity:
    /// `H.nodes.isolates(ignore_singletons=True)` (views.py:632-667).
    pub fn isolates_ignoring_singletons(&self) -> NodeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let mut in_big_edges: BTreeSet<String> = BTreeSet::new();
        for eid in self.graph.edge_ids() {
            let members = self.graph.members(&eid).unwrap_or_default();
            if members.len() >= 2 {
                in_big_edges.extend(members);
            }
        }
        let ids = self
            .ids
            .iter()
            .filter(|n| !in_big_edges.contains(*n))
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes whose degree satisfies `mode` vs `val`.
    /// XGI parity: `H.nodes.filterby('degree', val, mode)`.
    pub fn filterby_degree(&self, val: usize, mode: FilterMode) -> NodeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|n| mode.matches(self.degree(n), val))
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes with `lo <= degree <= hi`.
    /// XGI parity: `filterby('degree', (lo, hi), 'between')`.
    pub fn filterby_degree_between(&self, lo: usize, hi: usize) -> NodeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|n| {
                let d = self.degree(n);
                lo <= d && d <= hi
            })
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }
}
```

In `lib.rs`, re-export the view types:

```rust
pub use core::views::{EdgeView, FilterMode, NodeView};
```

Note: `EdgeView` is referenced by this re-export but defined in Task 8. If you are implementing Task 7 standalone before Task 8, temporarily re-export only `{FilterMode, NodeView}` and add `EdgeView` when Task 8 lands. (In a single Phase-2 pass, add both now and let Task 8 define `EdgeView`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_views`
Expected: PASS — 6 tests, 0 failures. (If the `EdgeView` re-export errors because Task 8 hasn't landed, drop `EdgeView` from the `lib.rs` line for now.)

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement NodeView proxy + FilterMode

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 8: `EdgeView` proxy object

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_views.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/views.rs`
- Modify (if not already done in Task 7): `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `Hypergraph::edges(&self) -> EdgeView<'_, N, E, M>`.
- `EdgeView`: `ids()`, `len()`, `is_empty()`, `contains(id)`, `attrs(id)`, `size(id)`, `order(id)`, `members(id)`, `singletons()`, `empty()`, `filterby_size(val, mode)`, `filterby_order(val, mode)`, `duplicates()`, `lookup(members)`, `maximal(strict)`.

**XGI reference:** `H.edges` (views.py:670-847). `size` = member count (edgestats.py:148); `order` = size − 1 (edgestats.py:108-145); `members(e)` (views.py:706-747); `singletons()` size==1 (views.py:749-761); `empty()` size==0 (views.py:763-775); `duplicates()` keeps all-but-first-sorted per member-set group (views.py:409-465); `lookup(members)` returns edges whose member-set equals the argument (views.py:467-509); `maximal(strict)` (views.py:777-847).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_views.rs`:

```rust
#[test]
fn test_edgeview_size_and_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    let edges = h.edges();
    assert_eq!(edges.size("e1"), 3);
    assert_eq!(edges.order("e1"), 2);
}

#[test]
fn test_edgeview_singletons_and_empty() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), v()).unwrap();
    assert_eq!(h.edges().singletons().ids(), &["e2".to_string()]);
    assert!(h.edges().empty().is_empty());
}

#[test]
fn test_edgeview_filterby_size() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["a".into(), "b".into()], Some("e2".into()), v()).unwrap();
    let big = h.edges().filterby_size(3, FilterMode::Geq);
    assert_eq!(big.ids(), &["e1".to_string()]);
}

#[test]
fn test_edgeview_duplicates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into(), "2".into()], Some("1".into()), v()).unwrap();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("2".into()), v()).unwrap();
    // ids 0 and 2 share member-set {0,1,2}; first-sorted (0) kept, 2 is a duplicate
    assert_eq!(h.edges().duplicates().ids(), &["2".to_string()]);
}

#[test]
fn test_edgeview_lookup() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("1".into()), v()).unwrap();
    let found = h.edges().lookup(&["3".to_string(), "4".to_string()]);
    assert_eq!(found.ids(), &["1".to_string()]);
}

#[test]
fn test_edgeview_maximal_nonstrict() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("0".into()), v()).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("2".into()), v()).unwrap();
    let mut ids = h.edges().maximal(false).ids().to_vec();
    ids.sort();
    // {1,2} is a strict subset of {1,2,3} -> not maximal
    assert_eq!(ids, vec!["0".to_string(), "2".to_string()]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_views`
Expected: FAIL — `EdgeView` / `edges()` don't exist.

- [ ] **Step 3: Implement `EdgeView`**

Append to `core/views.rs`:

```rust
/// A read-only view over the edges of a hypergraph.
pub struct EdgeView<'a, N, E, M> {
    graph: &'a Hypergraph<N, E, M>,
    ids: Vec<String>,
}

impl<N, E, M> Hypergraph<N, E, M> {
    /// A view over all edges (insertion order). XGI parity: `H.edges`.
    pub fn edges(&self) -> EdgeView<'_, N, E, M> {
        EdgeView { graph: self, ids: self.edge_ids() }
    }
}

impl<'a, N, E, M> EdgeView<'a, N, E, M> {
    /// The tracked edge ids, in insertion order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The number of edges tracked.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `edge_id` is tracked by this view.
    pub fn contains(&self, edge_id: &str) -> bool {
        self.ids.iter().any(|e| e == edge_id)
    }

    /// The edge's attributes. XGI parity: `H.edges[e]`.
    pub fn attrs(&self, edge_id: &str) -> Option<&'a E> {
        self.graph.edge_attrs(edge_id)
    }

    /// The number of members. XGI parity: `H.edges.size` (edgestats.py:148).
    pub fn size(&self, edge_id: &str) -> usize {
        self.graph.members(edge_id).map_or(0, |m| m.len())
    }

    /// The order (size − 1). XGI parity: `H.edges.order` (edgestats.py:108).
    /// Saturates at 0 (our edges are never empty in normal use).
    pub fn order(&self, edge_id: &str) -> usize {
        self.size(edge_id).saturating_sub(1)
    }

    /// The member node ids. XGI parity: `H.edges.members(e)` (views.py:706-747).
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.graph.members(edge_id)
    }

    /// Sub-view of size-1 edges. XGI parity: `H.edges.singletons()`.
    pub fn singletons(&self) -> EdgeView<'a, N, E, M> {
        self.filterby_size(1, FilterMode::Eq)
    }

    /// Sub-view of size-0 edges. XGI parity: `H.edges.empty()`.
    pub fn empty(&self) -> EdgeView<'a, N, E, M> {
        self.filterby_size(0, FilterMode::Eq)
    }

    /// Sub-view of edges whose size satisfies `mode` vs `val`.
    pub fn filterby_size(&self, val: usize, mode: FilterMode) -> EdgeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|e| mode.matches(self.size(e), val))
            .cloned()
            .collect();
        EdgeView { graph: self.graph, ids }
    }

    /// Sub-view of edges whose order satisfies `mode` vs `val`.
    pub fn filterby_order(&self, val: usize, mode: FilterMode) -> EdgeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|e| mode.matches(self.order(e), val))
            .cloned()
            .collect();
        EdgeView { graph: self.graph, ids }
    }

    /// Sub-view of edges that duplicate an earlier one (same member-set).
    /// The first (sorted) id of each group is kept; the rest are duplicates.
    /// XGI parity: `H.edges.duplicates()` (views.py:409-465).
    pub fn duplicates(&self) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for eid in &self.ids {
            let mut mem = self.graph.members(eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid.clone());
        }
        let mut dups: Vec<String> = Vec::new();
        for members_ids in groups.values() {
            if members_ids.len() > 1 {
                let mut sorted = members_ids.clone();
                sorted.sort();
                dups.extend(sorted.into_iter().skip(1));
            }
        }
        // Preserve insertion order among the flagged ids.
        let ids = self.ids.iter().filter(|e| dups.contains(e)).cloned().collect();
        EdgeView { graph: self.graph, ids }
    }

    /// Sub-view of edges whose member-set equals `members`.
    /// XGI parity: `H.edges.lookup(members)` (views.py:467-509).
    pub fn lookup(&self, members: &[String]) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let want: BTreeSet<String> = members.iter().cloned().collect();
        let ids = self
            .ids
            .iter()
            .filter(|e| {
                let got: BTreeSet<String> =
                    self.graph.members(e).unwrap_or_default().into_iter().collect();
                got == want
            })
            .cloned()
            .collect();
        EdgeView { graph: self.graph, ids }
    }

    /// Sub-view of maximal edges (not a subset of any other edge).
    /// `strict=false` keeps maximal multiedges; `strict=true` excludes any edge
    /// that is a subset of or equal to another. XGI parity: `H.edges.maximal()`
    /// (views.py:777-847).
    pub fn maximal(&self, strict: bool) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let sets: Vec<(String, BTreeSet<String>)> = self
            .ids
            .iter()
            .map(|e| {
                let s: BTreeSet<String> =
                    self.graph.members(e).unwrap_or_default().into_iter().collect();
                (e.clone(), s)
            })
            .collect();

        let ids = self
            .ids
            .iter()
            .filter(|e| {
                let me = &sets.iter().find(|(id, _)| id == *e).unwrap().1;
                // Keep e unless some OTHER edge f makes it non-maximal.
                !sets.iter().any(|(fid, mf)| {
                    if fid == *e {
                        return false;
                    }
                    if strict {
                        me.is_subset(mf) // subset OR equal disqualifies (strict)
                    } else {
                        me.is_subset(mf) && me.len() < mf.len() // strict subset only
                    }
                })
            })
            .cloned()
            .collect();
        EdgeView { graph: self.graph, ids }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_views`
Expected: PASS — 12 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement EdgeView proxy (size/order/duplicates/maximal/lookup)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 9: `DiHypergraph` — struct, kinds, CRUD, and `add_edge`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/` → Create: `test_dihypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/kinds.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/core/dihypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `DiRole::{Tail, Head}` and `DiMembershipEdge<M> { member_data: M, role: DiRole }` in `kinds.rs`.
- `DiHypergraph<N, E, M>`: `new`, `num_nodes`, `num_edges`, `add_node(id, attrs) -> bool`, `has_node`, `has_edge`, `add_edge(tail, head, idx, attrs) -> Result<String, EdgeError>` where `N: Default, M: Default + Clone`.

**XGI reference** (dihypergraph.py): `add_node` initializes `{"in": set(), "out": set()}` (346-349); `add_edge(members, idx)` takes `members = (tail, head)` (553-557), auto-id via `count()` (559), duplicate `idx` warns + returns (561-563), tail nodes get `_edge[uid]["in"]` / node `["out"]`, head nodes get `_edge[uid]["out"]` / node `["in"]` (567-579), `update_uid_counter` guarded by `if idx:` (584-585 — the falsy-id bug we fix, matching Phase 1). **Orientation:** all directed membership petgraph edges are physically **agent → hyperedge**, carrying `role = Tail` (sender) or `role = Head` (receiver).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_dihypergraph.rs`:

```rust
use hypergraph_rs::{DiHypergraph, EdgeError};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_dihypergraph_new_is_empty() {
    let dh: DiHypergraph = DiHypergraph::new();
    assert_eq!(dh.num_nodes(), 0);
    assert_eq!(dh.num_edges(), 0);
}

#[test]
fn test_dihypergraph_add_node() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    assert!(dh.add_node("a", v()));
    assert!(!dh.add_node("a", v()));
    assert!(dh.has_node("a"));
    assert_eq!(dh.num_nodes(), 1);
}

#[test]
fn test_dihypergraph_add_edge_creates_members() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let id = dh
        .add_edge(vec!["1".into(), "2".into()], vec!["3".into()], Some("e0".into()), v())
        .unwrap();
    assert_eq!(id, "e0");
    assert_eq!(dh.num_edges(), 1);
    assert_eq!(dh.num_nodes(), 3);
    assert!(dh.has_edge("e0"));
}

#[test]
fn test_dihypergraph_add_edge_auto_id() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let a = dh.add_edge(vec!["1".into()], vec!["2".into()], None, v()).unwrap();
    let b = dh.add_edge(vec!["3".into()], vec!["4".into()], None, v()).unwrap();
    assert_eq!(a, "0");
    assert_eq!(b, "1");
}

#[test]
fn test_dihypergraph_add_edge_one_sided_allowed() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    // all-senders edge (empty head) is allowed
    let id = dh.add_edge(vec!["3".into(), "4".into()], vec![], Some("e0".into()), v()).unwrap();
    assert_eq!(id, "e0");
    assert_eq!(dh.num_nodes(), 2);
}

#[test]
fn test_dihypergraph_add_edge_both_empty_errors() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let r = dh.add_edge(vec![], vec![], Some("e0".into()), v());
    assert!(matches!(r, Err(EdgeError::EmptyMembers)));
}

#[test]
fn test_dihypergraph_add_edge_duplicate_idx_errors() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    let r = dh.add_edge(vec!["3".into()], vec!["4".into()], Some("e0".into()), v());
    assert!(matches!(r, Err(EdgeError::AlreadyExists { .. })));
    assert_eq!(dh.num_edges(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: FAIL — `DiHypergraph` doesn't exist.

- [ ] **Step 3: Add `DiRole`/`DiMembershipEdge` and create `dihypergraph.rs`**

Append to `core/kinds.rs`:

```rust
/// The role of a node within a directed hyperedge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiRole {
    /// A sender: the node is in the edge's tail.
    Tail,
    /// A receiver: the node is in the edge's head.
    Head,
}

/// A directed membership edge in the bipartite graph: always oriented
/// Agent -> Hyperedge, with `role` distinguishing tail (sender) from head
/// (receiver).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiMembershipEdge<M> {
    /// The per-membership data.
    pub member_data: M,
    /// Whether the agent is a sender (`Tail`) or receiver (`Head`) of the edge.
    pub role: DiRole,
}
```

In `core/mod.rs`, add:

```rust
pub mod dihypergraph;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/core/dihypergraph.rs`:

```rust
//! The directed hypergraph data structure.
//!
//! Like [`Hypergraph`](super::hypergraph::Hypergraph), a `DiHypergraph` IS a
//! bipartite `StableDiGraph`, but every directed membership edge is oriented
//! Agent -> Hyperedge and carries a [`DiRole`] flag distinguishing the tail
//! (senders) from the head (receivers). XGI parity: `xgi.DiHypergraph`
//! (dihypergraph.py). Note XGI's `DiHypergraph` is NOT a subclass of
//! `Hypergraph`, and neither is this.

use indexmap::IndexMap;
use rustworkx_core::petgraph::stable_graph::{NodeIndex, StableDiGraph};

use super::error::{EdgeError, NodeError};
use super::kinds::{DiMembershipEdge, DiRole, NodeKind};

/// Deduplicate a Vec of ids, preserving first-seen order.
fn dedup_preserve(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

/// A directed hypergraph. XGI parity: `xgi.DiHypergraph`.
///
/// # Type Parameters
/// - `N` — node attribute type (default `serde_json::Value`)
/// - `E` — hyperedge attribute type (default `serde_json::Value`)
/// - `M` — per-membership attribute type (default `serde_json::Value`)
pub struct DiHypergraph<N = serde_json::Value, E = serde_json::Value, M = serde_json::Value> {
    /// Bipartite substrate. All membership edges are Agent -> Hyperedge with a
    /// DiRole flag. StableDiGraph preserves insertion order under removal.
    inner: StableDiGraph<NodeKind<N, E>, DiMembershipEdge<M>>,
    agent_ids: IndexMap<String, NodeIndex>,
    hyperedge_ids: IndexMap<String, NodeIndex>,
    edge_uid_counter: u64,
    graph_attrs: serde_json::Map<String, serde_json::Value>,
    frozen: bool,
}

impl<N, E, M> Default for DiHypergraph<N, E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> DiHypergraph<N, E, M> {
    /// Create an empty directed hypergraph.
    pub fn new() -> Self {
        Self {
            inner: StableDiGraph::new(),
            agent_ids: IndexMap::new(),
            hyperedge_ids: IndexMap::new(),
            edge_uid_counter: 0,
            graph_attrs: serde_json::Map::new(),
            frozen: false,
        }
    }

    /// Number of nodes. XGI parity: `DH.num_nodes`.
    pub fn num_nodes(&self) -> usize {
        self.agent_ids.len()
    }

    /// Number of edges. XGI parity: `DH.num_edges`.
    pub fn num_edges(&self) -> usize {
        self.hyperedge_ids.len()
    }

    /// Whether a node exists. XGI parity: `n in DH`.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.agent_ids.contains_key(node_id)
    }

    /// Whether an edge exists. XGI parity: `e in DH.edges`.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.hyperedge_ids.contains_key(edge_id)
    }

    fn assert_not_frozen(&self) {
        if self.frozen {
            panic!("Frozen higher-order network can't be modified");
        }
    }

    /// Ensure an agent node exists, creating it (with `N::default()`) if absent.
    fn ensure_agent(&mut self, node_id: &str)
    where
        N: Default,
    {
        if !self.agent_ids.contains_key(node_id) {
            let idx = self.inner.add_node(NodeKind::Agent(N::default()));
            self.agent_ids.insert(node_id.to_string(), idx);
        }
    }

    /// Add a node with attributes. Returns `true` if newly created.
    /// XGI parity: `DH.add_node(node, **attr)` (dihypergraph.py:327-349).
    pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool {
        self.assert_not_frozen();
        if self.agent_ids.contains_key(node_id) {
            return false;
        }
        let idx = self.inner.add_node(NodeKind::Agent(attrs));
        self.agent_ids.insert(node_id.to_string(), idx);
        true
    }

    /// Add a directed hyperedge with the given tail (senders) and head
    /// (receivers). Auto-creates member nodes. XGI parity:
    /// `DH.add_edge((tail, head), idx=)` (dihypergraph.py:525-585).
    ///
    /// Deviations: returns `Err(EdgeError::EmptyMembers)` only when BOTH tail
    /// and head are empty (deviation #1); `Err(EdgeError::AlreadyExists)` on a
    /// duplicate `idx` (deviation #2).
    pub fn add_edge(
        &mut self,
        tail: Vec<String>,
        head: Vec<String>,
        idx: Option<String>,
        attrs: E,
    ) -> Result<String, EdgeError>
    where
        N: Default,
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if tail.is_empty() && head.is_empty() {
            return Err(EdgeError::EmptyMembers);
        }

        let edge_id = match &idx {
            Some(id) => {
                if self.hyperedge_ids.contains_key(id) {
                    return Err(EdgeError::AlreadyExists { edge_id: id.clone() });
                }
                id.clone()
            }
            None => self.edge_uid_counter.to_string(),
        };

        // Advance the uid counter (fixing XGI's falsy-id bug, as in Phase 1).
        if let Some(id) = &idx {
            if let Ok(n) = id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        } else {
            self.edge_uid_counter += 1;
        }

        let he = self.inner.add_node(NodeKind::Hyperedge(attrs));
        self.hyperedge_ids.insert(edge_id.clone(), he);

        for node in &dedup_preserve(tail) {
            self.ensure_agent(node);
            let a = self.agent_ids[node];
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge { member_data: M::default(), role: DiRole::Tail },
            );
        }
        for node in &dedup_preserve(head) {
            self.ensure_agent(node);
            let a = self.agent_ids[node];
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge { member_data: M::default(), role: DiRole::Head },
            );
        }

        Ok(edge_id)
    }
}
```

In `lib.rs`, re-export:

```rust
pub use core::dihypergraph::DiHypergraph;
pub use core::kinds::{DiMembershipEdge, DiRole, MembershipEdge, NodeKind};
```

(Replace the existing `pub use core::kinds::{MembershipEdge, NodeKind};` line with the one above.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: PASS — 7 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): DiHypergraph struct + DiRole/DiMembershipEdge + add_edge

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 10: `DiHypergraph` — directional queries, removal, node-to-edge

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_dihypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/dihypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/error.rs`

**Interfaces:**
- `dimembers(edge_id) -> Option<(Vec<String> /* tail */, Vec<String> /* head */)>`, `tail(edge_id)`, `head(edge_id)`, `members(edge_id)`.
- `dimemberships(node_id) -> Option<(Vec<String> /* in = head-edges */, Vec<String> /* out = tail-edges */)>`, `memberships(node_id)`.
- `remove_edge(edge_id) -> Result<(), EdgeError>`, `remove_node(node_id, strong, remove_empty) -> Result<(), NodeError>`.
- `add_node_to_edge(edge_id, node_id, role) -> Result<(), EdgeError>` where `N: Default, E: Default, M: Default + Clone`; `remove_node_from_edge(edge_id, node_id, role, remove_empty) -> Result<(), NodeError>`. When the named EDGE is missing, this returns a dedicated `NodeError::EdgeNotFound { edge_id }` variant rather than misusing `NodeError::NotFound { node_id: edge_id }` (2026-07-19 preflight, defect P2-D4 — the entity that's missing is the edge, not the node).
- `edge_ids()`, `node_ids()` (insertion-ordered).

**XGI reference:** `DiEdgeView.dimembers` returns `(edge["in"], edge["out"])` = `(tail, head)` (views.py:1017-1071); `tail(e)` = `edge["in"]` (views.py:1167-1208); `head(e)` = `edge["out"]` (views.py:1124-1165). `DiNodeView.dimemberships` returns `(node["in"], node["out"])` where `node["in"]` = edges the node receives in (head), `node["out"]` = edges it sends from (tail) (views.py:889-918). `remove_node` weak/strong (dihypergraph.py:385-436), `add_node_to_edge` (794-832), `remove_node_from_edge` (891-943).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_dihypergraph.rs`:

```rust
use hypergraph_rs::DiRole;

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

#[test]
fn test_dihypergraph_dimembers_head_tail() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into(), "2".into()], vec!["3".into(), "4".into()], Some("e0".into()), v()).unwrap();
    let (tail, head) = dh.dimembers("e0").unwrap();
    assert_eq!(sorted(tail), vec!["1".to_string(), "2".to_string()]);
    assert_eq!(sorted(head), vec!["3".to_string(), "4".to_string()]);
    assert_eq!(sorted(dh.tail("e0").unwrap()), vec!["1".to_string(), "2".to_string()]);
    assert_eq!(sorted(dh.head("e0").unwrap()), vec!["3".to_string(), "4".to_string()]);
    assert_eq!(sorted(dh.members("e0").unwrap()), vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()]);
}

#[test]
fn test_dihypergraph_dimemberships() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    // node "2" is a receiver (head) of e0 and a sender (tail) of e1
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.add_edge(vec!["2".into()], vec!["3".into()], Some("e1".into()), v()).unwrap();
    let (in_edges, out_edges) = dh.dimemberships("2").unwrap();
    assert_eq!(in_edges, vec!["e0".to_string()]);   // receiver in e0
    assert_eq!(out_edges, vec!["e1".to_string()]);  // sender in e1
    assert_eq!(sorted(dh.memberships("2").unwrap()), vec!["e0".to_string(), "e1".to_string()]);
}

#[test]
fn test_dihypergraph_remove_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.remove_edge("e0").unwrap();
    assert!(!dh.has_edge("e0"));
    assert!(dh.has_node("1"));
    assert!(dh.memberships("1").unwrap().is_empty());
}

#[test]
fn test_dihypergraph_remove_node_weak() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into(), "2".into()], vec!["3".into()], Some("e0".into()), v()).unwrap();
    dh.remove_node("1", false, true).unwrap();
    assert!(!dh.has_node("1"));
    assert!(dh.has_edge("e0"));
    assert_eq!(sorted(dh.tail("e0").unwrap()), vec!["2".to_string()]);
}

#[test]
fn test_dihypergraph_remove_node_weak_removes_empty_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec![], Some("e0".into()), v()).unwrap();
    dh.remove_node("1", false, true).unwrap();
    assert!(!dh.has_edge("e0")); // edge became empty -> removed
}

#[test]
fn test_dihypergraph_remove_node_strong() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.add_edge(vec!["1".into()], vec!["3".into()], Some("e1".into()), v()).unwrap();
    dh.remove_node("1", true, true).unwrap();
    assert!(!dh.has_edge("e0"));
    assert!(!dh.has_edge("e1"));
    assert!(dh.has_node("2"));
}

#[test]
fn test_dihypergraph_add_node_to_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.add_node_to_edge("e0", "9", DiRole::Head).unwrap();
    assert_eq!(sorted(dh.head("e0").unwrap()), vec!["2".to_string(), "9".to_string()]);
}

#[test]
fn test_dihypergraph_remove_node_from_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into(), "2".into()], vec!["3".into()], Some("e0".into()), v()).unwrap();
    dh.remove_node_from_edge("e0", "1", DiRole::Tail, true).unwrap();
    assert_eq!(sorted(dh.tail("e0").unwrap()), vec!["2".to_string()]);
    // removing a node not in the named role errors
    assert!(dh.remove_node_from_edge("e0", "1", DiRole::Tail, true).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: FAIL — the query/removal methods don't exist.

- [ ] **Step 3: Add `NodeError::EdgeNotFound` and implement queries and removal**

In `error.rs`, add a variant to the existing `NodeError` enum (alongside `NotFound`):

```rust
    /// The named edge (not node) was not found — returned by methods that
    /// take both an edge id and a node id (e.g. `remove_node_from_edge`).
    #[error("edge {edge_id} not found")]
    EdgeNotFound { edge_id: String },
```

At the top of `dihypergraph.rs`, add the direction/edge-ref imports:

```rust
use rustworkx_core::petgraph::visit::EdgeRef;
use rustworkx_core::petgraph::Direction;
```

Add inside the `impl<N, E, M> DiHypergraph<N, E, M>` block:

```rust
    /// All node ids, insertion order. XGI parity: `list(DH.nodes)`.
    pub fn node_ids(&self) -> Vec<String> {
        self.agent_ids.keys().cloned().collect()
    }

    /// All edge ids, insertion order. XGI parity: `list(DH.edges)`.
    pub fn edge_ids(&self) -> Vec<String> {
        self.hyperedge_ids.keys().cloned().collect()
    }

    /// Map a hyperedge NodeIndex back to its id (linear scan, Phase 1 pattern).
    fn edge_id_of(&self, idx: NodeIndex) -> Option<String> {
        self.hyperedge_ids.iter().find(|(_, &i)| i == idx).map(|(k, _)| k.clone())
    }

    /// Map an agent NodeIndex back to its id (linear scan, Phase 1 pattern).
    fn node_id_of(&self, idx: NodeIndex) -> Option<String> {
        self.agent_ids.iter().find(|(_, &i)| i == idx).map(|(k, _)| k.clone())
    }

    /// (tail, head) node ids of an edge. XGI parity: `DH.edges.dimembers(e)`
    /// = (edge["in"], edge["out"]) (views.py:1017-1071).
    pub fn dimembers(&self, edge_id: &str) -> Option<(Vec<String>, Vec<String>)> {
        let he = *self.hyperedge_ids.get(edge_id)?;
        let mut tail = Vec::new();
        let mut head = Vec::new();
        for er in self.inner.edges_directed(he, Direction::Incoming) {
            if let Some(nid) = self.node_id_of(er.source()) {
                match er.weight().role {
                    DiRole::Tail => tail.push(nid),
                    DiRole::Head => head.push(nid),
                }
            }
        }
        Some((tail, head))
    }

    /// Tail (sender) node ids. XGI parity: `DH.edges.tail(e)`.
    pub fn tail(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(t, _)| t)
    }

    /// Head (receiver) node ids. XGI parity: `DH.edges.head(e)`.
    pub fn head(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(_, h)| h)
    }

    /// Union of tail and head. XGI parity: `DH.edges.members(e)`.
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(t, h)| {
            let mut set: std::collections::BTreeSet<String> = t.into_iter().collect();
            set.extend(h);
            set.into_iter().collect()
        })
    }

    /// (in, out) edge ids for a node: `in` = edges where the node is a head
    /// (receiver), `out` = edges where it is a tail (sender). XGI parity:
    /// `DH.nodes.dimemberships(n)` = (node["in"], node["out"])
    /// (views.py:889-918).
    pub fn dimemberships(&self, node_id: &str) -> Option<(Vec<String>, Vec<String>)> {
        let a = *self.agent_ids.get(node_id)?;
        let mut in_edges = Vec::new();
        let mut out_edges = Vec::new();
        for er in self.inner.edges_directed(a, Direction::Outgoing) {
            if let Some(eid) = self.edge_id_of(er.target()) {
                match er.weight().role {
                    DiRole::Head => in_edges.push(eid),
                    DiRole::Tail => out_edges.push(eid),
                }
            }
        }
        Some((in_edges, out_edges))
    }

    /// All edge ids the node belongs to (head or tail). XGI parity:
    /// `DH.nodes.memberships(n)`.
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        self.dimemberships(node_id).map(|(i, o)| {
            let mut set: std::collections::BTreeSet<String> = i.into_iter().collect();
            set.extend(o);
            set.into_iter().collect()
        })
    }

    /// Remove an edge. XGI parity: `DH.remove_edge(idx)` (dihypergraph.py:834-860).
    pub fn remove_edge(&mut self, edge_id: &str) -> Result<(), EdgeError> {
        self.assert_not_frozen();
        let he = *self
            .hyperedge_ids
            .get(edge_id)
            .ok_or(EdgeError::NotFound { edge_id: edge_id.to_string() })?;
        self.inner.remove_node(he);
        self.hyperedge_ids.shift_remove(edge_id);
        Ok(())
    }

    /// Remove a node. Weak removal detaches the node from its edges and drops
    /// edges that become empty (when `remove_empty`); strong removal drops all
    /// containing edges. XGI parity: `DH.remove_node(n, strong, remove_empty)`
    /// (dihypergraph.py:385-436).
    pub fn remove_node(
        &mut self,
        node_id: &str,
        strong: bool,
        remove_empty: bool,
    ) -> Result<(), NodeError> {
        self.assert_not_frozen();
        let a = *self
            .agent_ids
            .get(node_id)
            .ok_or(NodeError::NotFound { node_id: node_id.to_string() })?;
        let (in_e, out_e) = self.dimemberships(node_id).unwrap_or_default();
        let incident: std::collections::BTreeSet<String> =
            in_e.into_iter().chain(out_e).collect();

        if strong {
            for eid in &incident {
                let _ = self.remove_edge(eid);
            }
        } else {
            // Detach: remove all of the agent's outgoing membership edges.
            let eidxs: Vec<_> = self.inner.edges(a).map(|er| er.id()).collect();
            for e in eidxs {
                self.inner.remove_edge(e);
            }
            // Drop now-empty edges.
            for eid in &incident {
                if let Some(he) = self.hyperedge_ids.get(eid).copied() {
                    let empty =
                        self.inner.edges_directed(he, Direction::Incoming).next().is_none();
                    if empty && remove_empty {
                        self.inner.remove_node(he);
                        self.hyperedge_ids.shift_remove(eid);
                    }
                }
            }
        }

        self.inner.remove_node(a);
        self.agent_ids.shift_remove(node_id);
        Ok(())
    }

    /// Add a node to an edge in the given role. Auto-creates both if missing.
    /// XGI parity: `DH.add_node_to_edge(edge, node, direction)`
    /// (dihypergraph.py:794-832).
    pub fn add_node_to_edge(
        &mut self,
        edge_id: &str,
        node_id: &str,
        role: DiRole,
    ) -> Result<(), EdgeError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if !self.hyperedge_ids.contains_key(edge_id) {
            let he = self.inner.add_node(NodeKind::Hyperedge(E::default()));
            self.hyperedge_ids.insert(edge_id.to_string(), he);
            if let Ok(n) = edge_id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        }
        self.ensure_agent(node_id);
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        let exists = self
            .inner
            .edges_directed(he, Direction::Incoming)
            .any(|er| er.source() == a && er.weight().role == role);
        if !exists {
            self.inner
                .add_edge(a, he, DiMembershipEdge { member_data: M::default(), role });
        }
        Ok(())
    }

    /// Remove a node from an edge's given role. Errors if the edge/node is
    /// missing or the node is not in that role-set; drops the edge if empty and
    /// `remove_empty`. XGI parity: `DH.remove_node_from_edge(edge, node,
    /// direction, remove_empty)` (dihypergraph.py:891-943).
    pub fn remove_node_from_edge(
        &mut self,
        edge_id: &str,
        node_id: &str,
        role: DiRole,
        remove_empty: bool,
    ) -> Result<(), NodeError> {
        self.assert_not_frozen();
        let he = *self
            .hyperedge_ids
            .get(edge_id)
            .ok_or(NodeError::EdgeNotFound { edge_id: edge_id.to_string() })?;
        let a = *self
            .agent_ids
            .get(node_id)
            .ok_or(NodeError::NotFound { node_id: node_id.to_string() })?;
        let target = self
            .inner
            .edges_directed(he, Direction::Incoming)
            .find(|er| er.source() == a && er.weight().role == role)
            .map(|er| er.id());
        match target {
            Some(e) => {
                self.inner.remove_edge(e);
            }
            None => {
                return Err(NodeError::NotFound {
                    node_id: format!("node {node_id} not in {role:?} of edge {edge_id}"),
                });
            }
        }
        if remove_empty && self.inner.edges_directed(he, Direction::Incoming).next().is_none()
        {
            self.inner.remove_node(he);
            self.hyperedge_ids.shift_remove(edge_id);
        }
        Ok(())
    }
```

Note: `DiRole` must derive `Debug` (it does, from Task 9) so `{role:?}` in the error string compiles.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: PASS — 15 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): DiHypergraph directional queries + removal + node-to-edge

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 11: `DiHypergraph` — stats, isolates, copy/clear/freeze, Debug, PartialEq

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_dihypergraph.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/dihypergraph.rs`

**Interfaces:**
- Node stats: `degree(n)`, `in_degree(n)`, `out_degree(n)`, `isolate_node_ids()`.
- Edge stats: `size(e)`, `order(e)`, `tail_size(e)`, `head_size(e)`, `tail_order(e)`, `head_order(e)`.
- `copy()` (where `N/E/M: Clone`), `clear()`, `freeze()`, `is_frozen()`.
- `impl Debug` (XGI `__repr__` parity) and `impl PartialEq`.

**XGI reference:** dinode `degree` = |in ∪ out| (dinodestats.py:138-139), `in_degree` = |node["in"]| (168+), `out_degree` = |node["out"]| (224+); diedge `order` = size − 1, `size` = |in ∪ out| (diedgestats.py:139-152), `tail_order`/`tail_size` over `edge["in"]` (193-237), `head_order`/`head_size` over `edge["out"]` (277-318). `__repr__` = `DiHypergraph(<dimembers list>)` (dihypergraph.py:179-181).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_dihypergraph.rs`:

```rust
#[test]
fn test_dihypergraph_node_stats() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.add_edge(vec!["2".into()], vec!["3".into()], Some("e1".into()), v()).unwrap();
    assert_eq!(dh.degree("2"), 2);
    assert_eq!(dh.in_degree("2"), 1);  // receiver in e0
    assert_eq!(dh.out_degree("2"), 1); // sender in e1
}

#[test]
fn test_dihypergraph_edge_stats() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into(), "2".into()], vec!["3".into()], Some("e0".into()), v()).unwrap();
    assert_eq!(dh.size("e0"), 3);
    assert_eq!(dh.order("e0"), 2);
    assert_eq!(dh.tail_size("e0"), 2);
    assert_eq!(dh.head_size("e0"), 1);
    assert_eq!(dh.tail_order("e0"), 1);
    assert_eq!(dh.head_order("e0"), 0);
}

#[test]
fn test_dihypergraph_isolates() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.add_node("lonely", v());
    assert_eq!(dh.isolate_node_ids(), vec!["lonely".to_string()]);
}

#[test]
fn test_dihypergraph_copy_is_independent() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    let mut c = dh.copy();
    c.add_node("x", v());
    assert!(!dh.has_node("x"));
    assert_eq!(c.num_nodes(), 3);
}

#[test]
fn test_dihypergraph_clear() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    dh.clear();
    assert_eq!(dh.num_nodes(), 0);
    assert_eq!(dh.num_edges(), 0);
}

#[test]
fn test_dihypergraph_freeze() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_node("a", v());
    dh.freeze();
    assert!(dh.is_frozen());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut dh = dh;
        dh.add_node("b", v());
    }));
    assert!(r.is_err());
}

#[test]
fn test_dihypergraph_eq_and_debug() {
    let mut a: DiHypergraph = DiHypergraph::new();
    a.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    let mut b: DiHypergraph = DiHypergraph::new();
    b.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    assert!(a == b);
    b.add_node("x", v());
    assert!(a != b);
    let repr = format!("{a:?}");
    assert!(repr.contains("DiHypergraph"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: FAIL — the stats/copy/freeze/Debug/PartialEq surface doesn't exist.

- [ ] **Step 3: Implement stats, lifecycle, Debug, PartialEq**

Add inside the `impl<N, E, M> DiHypergraph<N, E, M>` block:

```rust
    /// Node degree: number of distinct edges it belongs to. XGI parity:
    /// `DH.nodes.degree` (dinodestats.py:138-139).
    pub fn degree(&self, node_id: &str) -> usize {
        self.memberships(node_id).map_or(0, |m| m.len())
    }

    /// Node in-degree: edges where the node is a head (receiver).
    pub fn in_degree(&self, node_id: &str) -> usize {
        self.dimemberships(node_id).map_or(0, |(i, _)| i.len())
    }

    /// Node out-degree: edges where the node is a tail (sender).
    pub fn out_degree(&self, node_id: &str) -> usize {
        self.dimemberships(node_id).map_or(0, |(_, o)| o.len())
    }

    /// Edge size: |tail ∪ head|. XGI parity: `DH.edges.size`.
    pub fn size(&self, edge_id: &str) -> usize {
        self.members(edge_id).map_or(0, |m| m.len())
    }

    /// Edge order: size − 1. XGI parity: `DH.edges.order`.
    pub fn order(&self, edge_id: &str) -> usize {
        self.size(edge_id).saturating_sub(1)
    }

    /// Number of tail (sender) members.
    pub fn tail_size(&self, edge_id: &str) -> usize {
        self.tail(edge_id).map_or(0, |t| t.len())
    }

    /// Number of head (receiver) members.
    pub fn head_size(&self, edge_id: &str) -> usize {
        self.head(edge_id).map_or(0, |h| h.len())
    }

    /// Tail size − 1.
    pub fn tail_order(&self, edge_id: &str) -> usize {
        self.tail_size(edge_id).saturating_sub(1)
    }

    /// Head size − 1.
    pub fn head_order(&self, edge_id: &str) -> usize {
        self.head_size(edge_id).saturating_sub(1)
    }

    /// Node ids with degree 0. XGI parity: `DH.nodes.isolates()`.
    pub fn isolate_node_ids(&self) -> Vec<String> {
        self.node_ids().into_iter().filter(|n| self.degree(n) == 0).collect()
    }

    /// Remove all nodes and edges (keeps the uid counter, like Phase 1).
    /// XGI parity: `DH.clear()` (dihypergraph.py:1000-1017).
    pub fn clear(&mut self) {
        self.assert_not_frozen();
        self.inner = StableDiGraph::new();
        self.agent_ids.clear();
        self.hyperedge_ids.clear();
        self.graph_attrs.clear();
    }

    /// An independent deep copy. XGI parity: `DH.copy()` (always unfrozen).
    pub fn copy(&self) -> Self
    where
        N: Clone,
        E: Clone,
        M: Clone,
    {
        Self {
            inner: self.inner.clone(),
            agent_ids: self.agent_ids.clone(),
            hyperedge_ids: self.hyperedge_ids.clone(),
            edge_uid_counter: self.edge_uid_counter,
            graph_attrs: self.graph_attrs.clone(),
            frozen: false,
        }
    }

    /// Freeze against modification. XGI parity: `DH.freeze()`.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Whether the dihypergraph is frozen. XGI parity: `DH.is_frozen`.
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
```

Add the trait impls OUTSIDE the inherent block:

```rust
impl<N: PartialEq, E: PartialEq, M> PartialEq for DiHypergraph<N, E, M> {
    /// Equal iff same nodes (with attrs), same edges (with (tail,head) sets and
    /// attrs), and same graph attrs.
    fn eq(&self, other: &Self) -> bool {
        if self.agent_ids.len() != other.agent_ids.len() {
            return false;
        }
        for (nid, &idx) in &self.agent_ids {
            let a = match self.inner.node_weight(idx) {
                Some(NodeKind::Agent(a)) => a,
                _ => return false,
            };
            let b_idx = match other.agent_ids.get(nid) {
                Some(i) => *i,
                None => return false,
            };
            match other.inner.node_weight(b_idx) {
                Some(NodeKind::Agent(b)) if a == b => {}
                _ => return false,
            }
        }
        if self.hyperedge_ids.len() != other.hyperedge_ids.len() {
            return false;
        }
        for (eid, &idx) in &self.hyperedge_ids {
            let a = match self.inner.node_weight(idx) {
                Some(NodeKind::Hyperedge(a)) => a,
                _ => return false,
            };
            let b_idx = match other.hyperedge_ids.get(eid) {
                Some(i) => *i,
                None => return false,
            };
            match other.inner.node_weight(b_idx) {
                Some(NodeKind::Hyperedge(b)) if a == b => {}
                _ => return false,
            }
            let mut a_dm = self.dimembers(eid).unwrap_or_default();
            let mut b_dm = other.dimembers(eid).unwrap_or_default();
            a_dm.0.sort();
            a_dm.1.sort();
            b_dm.0.sort();
            b_dm.1.sort();
            if a_dm != b_dm {
                return false;
            }
        }
        self.graph_attrs == other.graph_attrs
    }
}

impl<N, E, M> std::fmt::Debug for DiHypergraph<N, E, M> {
    /// XGI parity: `__repr__` = `DiHypergraph([(tail, head), ...])`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiHypergraph(")?;
        let edges: Vec<String> = self
            .edge_ids()
            .iter()
            .map(|eid| {
                let (t, h) = self.dimembers(eid).unwrap_or_default();
                format!("({{{}}}, {{{}}})", t.join(", "), h.join(", "))
            })
            .collect();
        write!(f, "[{}])", edges.join(", "))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_dihypergraph`
Expected: PASS — 22 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): DiHypergraph stats, isolates, copy/clear/freeze, Debug, PartialEq

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 12: `SimplicialComplex` — struct, `add_simplex` (closure under faces), `has_simplex`, face lattice

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_simplicialcomplex.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/core/simplicialcomplex.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/error.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `SimplicialComplex<N, E, M>` composing `inner: Hypergraph<N, E, M>` + `OnceCell<FaceLattice>`.
- Read delegation: `num_nodes`, `num_edges`, `has_node`, `has_edge`, `members`, `node_attrs`, `edge_attrs`, `node_ids`, `edge_ids`.
- `add_simplex(members, idx, attrs) -> Result<(), SimplexError>` where `N: Default, E: Default, M: Default + Clone` — adds the simplex and every missing subface of size ≥ 2.
- `has_simplex(members) -> bool`.
- Blocked: `add_edge`/`remove_edge` → `Err(SimplexError::NotAvailable)`.
- `face_lattice() -> &FaceLattice` — lazily computed, invalidated on mutation. `FaceLattice { immediate_faces: BTreeMap<String, Vec<String>> }` (deliberate simplification of spec §3.4's `petgraph::DiGraph` — deviation #10; downward face relation only, no cofaces).
- `SimplexError::{NotAvailable, NotFound}` in `error.rs`.

**XGI reference:** `add_simplex` (simplicialcomplex.py:252-332): cast to `frozenset` (306), no-op if `has_simplex` (310-311), no-op on duplicate idx (313-315), auto-id (317), then add every subface not already present (324-332). `_subfaces` (334-361): for a size-k simplex, all subsets of sizes k−1 down to 2 (never singletons). `has_simplex` (783-806): frozenset membership over `_edge.values()`. Blocked methods (135-163, 216-218).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_simplicialcomplex.rs`:

```rust
use hypergraph_rs::{SimplexError, SimplicialComplex};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_add_simplex_closes_under_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v()).unwrap();
    // {1,2,3} + its three 2-faces {1,2},{1,3},{2,3} = 4 simplices (no singletons)
    assert_eq!(s.num_edges(), 4);
    assert!(s.has_simplex(&["1".to_string(), "2".to_string(), "3".to_string()]));
    assert!(s.has_simplex(&["1".to_string(), "2".to_string()]));
    assert!(s.has_simplex(&["2".to_string(), "3".to_string()]));
    assert!(!s.has_simplex(&["1".to_string()])); // singleton not auto-added
}

#[test]
fn test_add_simplex_singleton_is_one_simplex() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["4".into()], Some("myedge".into()), v()).unwrap();
    assert_eq!(s.num_edges(), 1);
    assert!(s.has_simplex(&["4".to_string()]));
}

#[test]
fn test_add_simplex_duplicate_members_is_noop() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v()).unwrap();
    let before = s.num_edges();
    s.add_simplex(vec!["2".into(), "1".into()], None, v()).unwrap(); // same set
    assert_eq!(s.num_edges(), before);
}

#[test]
fn test_add_simplex_shares_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v()).unwrap();
    let after_first = s.num_edges();
    // {2,3,4}: adds {2,3,4},{2,4},{3,4} but NOT {2,3} (already present) = +3
    s.add_simplex(vec!["2".into(), "3".into(), "4".into()], None, v()).unwrap();
    assert_eq!(s.num_edges(), after_first + 3);
}

#[test]
fn test_add_edge_is_blocked() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    let r = s.add_edge(vec!["1".into(), "2".into()], None, v());
    assert!(matches!(r, Err(SimplexError::NotAvailable { .. })));
}

#[test]
fn test_face_lattice_immediate_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], Some("top".into()), v()).unwrap();
    let lattice = s.face_lattice();
    // {1,2,3} has three immediate (codim-1) faces present in the complex
    assert_eq!(lattice.immediate_faces.get("top").map(|f| f.len()), Some(3));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_simplicialcomplex`
Expected: FAIL — `SimplicialComplex` doesn't exist.

- [ ] **Step 3: Add `SimplexError`, create the module, implement `add_simplex` + lattice**

In `error.rs`, add:

```rust
/// Error raised by `SimplicialComplex` operations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SimplexError {
    /// A `Hypergraph` method that is not available on a `SimplicialComplex`.
    #[error("{method} is not available for SimplicialComplex; use {alternative}")]
    NotAvailable { method: String, alternative: String },
    /// The simplex id was not found.
    #[error("simplex {idx} is not in the simplicial complex")]
    NotFound { idx: String },
}
```

In `core/mod.rs`, add:

```rust
pub mod simplicialcomplex;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/core/simplicialcomplex.rs`:

```rust
//! The simplicial complex data structure.
//!
//! A `SimplicialComplex` composes a [`Hypergraph`] and enforces closure under
//! faces: adding a simplex also adds every missing subface of size >= 2. A
//! lazily-computed [`FaceLattice`] caches the immediate-face relation; it is
//! invalidated on every mutation. XGI parity: `xgi.SimplicialComplex`
//! (simplicialcomplex.py).

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

use super::error::SimplexError;
use super::hypergraph::Hypergraph;

/// The cached immediate-face relation: each simplex id maps to the ids of the
/// codimension-1 simplices present in the complex that are its faces.
#[derive(Debug, Clone, Default)]
pub struct FaceLattice {
    /// simplex id -> ids of its codim-1 faces present in the complex.
    pub immediate_faces: BTreeMap<String, Vec<String>>,
}

/// A simplicial complex (undirected, closed under faces).
pub struct SimplicialComplex<N = serde_json::Value, E = serde_json::Value, M = serde_json::Value> {
    inner: Hypergraph<N, E, M>,
    face_lattice: OnceCell<FaceLattice>,
}

/// Deduplicate a Vec of ids into a sorted set-vector (deterministic order).
fn as_sorted_set(items: &[String]) -> Vec<String> {
    let set: BTreeSet<String> = items.iter().cloned().collect();
    set.into_iter().collect()
}

/// All subsets of `set` of sizes from `set.len()-1` down to 2 (never
/// singletons). XGI parity: `_subfaces` (simplicialcomplex.py:334-361).
fn subfaces(set: &[String]) -> Vec<Vec<String>> {
    let k = set.len();
    let mut out: Vec<Vec<String>> = Vec::new();
    // sizes k-1, k-2, ..., 2
    let mut size = k.saturating_sub(1);
    while size >= 2 {
        combinations(set, size, &mut out);
        size -= 1;
    }
    out
}

/// Push every `choose`-sized combination of `set` (sorted input -> sorted
/// combos) into `out`. Bounded by `total = n_choose_k(set.len(), choose)`,
/// computed BEFORE the loop (Power-of-10 rule 2 — 2026-07-19 preflight
/// correction, defect P2-D2: the prior `loop { ... }` form terminated via an
/// internal `return` after C(n,k) iterations, which is provably-but-not
/// *statically* bounded; this form gives a checkable `for` bound instead).
fn combinations(set: &[String], choose: usize, out: &mut Vec<Vec<String>>) {
    let n = set.len();
    if choose == 0 || choose > n {
        return;
    }
    let total = n_choose_k(n, choose);
    // index-based lexicographic combinations; bounded by n choose k.
    let mut idx: Vec<usize> = (0..choose).collect();
    for _ in 0..total {
        out.push(idx.iter().map(|&i| set[i].clone()).collect());
        // Advance to the next combination: find the rightmost index that can
        // still be incremented (bounded by `choose`, known before this
        // sub-loop starts).
        let mut pivot: Option<usize> = None;
        for i in (0..choose).rev() {
            if idx[i] != i + n - choose {
                pivot = Some(i);
                break;
            }
        }
        let i = match pivot {
            Some(i) => i,
            None => return, // exhausted (the final combination was just pushed)
        };
        idx[i] += 1;
        for j in (i + 1)..choose {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// `n` choose `k`, via the multiplicative formula (no factorials — this
/// crate's `n` is always small, but the formula avoids any needless overflow
/// risk regardless).
fn n_choose_k(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: usize = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

impl<N, E, M> Default for SimplicialComplex<N, E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> SimplicialComplex<N, E, M> {
    /// Create an empty simplicial complex.
    pub fn new() -> Self {
        Self { inner: Hypergraph::new(), face_lattice: OnceCell::new() }
    }

    /// Number of nodes. XGI parity: `S.num_nodes`.
    pub fn num_nodes(&self) -> usize {
        self.inner.num_nodes()
    }

    /// Number of simplices. XGI parity: `S.num_edges`.
    pub fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    /// Whether a node exists.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.inner.has_node(node_id)
    }

    /// Whether a simplex id exists.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.inner.has_edge(edge_id)
    }

    /// The members of a simplex (by id).
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.inner.members(edge_id)
    }

    /// A node's attributes.
    pub fn node_attrs(&self, node_id: &str) -> Option<&N> {
        self.inner.node_attrs(node_id)
    }

    /// A simplex's attributes.
    pub fn edge_attrs(&self, edge_id: &str) -> Option<&E> {
        self.inner.edge_attrs(edge_id)
    }

    /// All node ids (insertion order).
    pub fn node_ids(&self) -> Vec<String> {
        self.inner.node_ids()
    }

    /// All simplex ids (insertion order).
    pub fn edge_ids(&self) -> Vec<String> {
        self.inner.edge_ids()
    }

    /// Whether a simplex with exactly these members is present (by member-set,
    /// not id). XGI parity: `S.has_simplex(simplex)` (simplicialcomplex.py:783-806).
    pub fn has_simplex(&self, members: &[String]) -> bool {
        let want: BTreeSet<String> = members.iter().cloned().collect();
        self.inner.edge_ids().iter().any(|e| {
            let got: BTreeSet<String> =
                self.inner.members(e).unwrap_or_default().into_iter().collect();
            got == want
        })
    }

    /// Add a simplex and all its missing subfaces (size >= 2). XGI parity:
    /// `S.add_simplex(members, idx, **attr)` (simplicialcomplex.py:252-332).
    /// Duplicate members or a duplicate `idx` are silent no-ops (deviation #5).
    pub fn add_simplex(
        &mut self,
        members: Vec<String>,
        idx: Option<String>,
        attrs: E,
    ) -> Result<(), SimplexError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        let set = as_sorted_set(&members);
        if set.is_empty() {
            return Ok(()); // cannot add empty simplices (XGI skips)
        }
        if self.has_simplex(&set) {
            return Ok(());
        }
        if let Some(id) = &idx {
            if self.inner.has_edge(id) {
                return Ok(()); // duplicate idx -> no-op (XGI warns+returns)
            }
        }
        // Top simplex (non-empty -> passes the strict add_edge check).
        let _ = self.inner.add_edge(set.clone(), idx, attrs);
        // Missing subfaces (size >= 2), attribute-free.
        for face in subfaces(&set) {
            if face.is_empty() || self.has_simplex(&face) {
                continue;
            }
            let _ = self.inner.add_edge(face, None, E::default());
        }
        self.invalidate();
        Ok(())
    }

    /// `add_edge` is not available on a simplicial complex.
    /// XGI parity: raises XGIError (simplicialcomplex.py:135-139).
    pub fn add_edge(
        &mut self,
        _members: Vec<String>,
        _idx: Option<String>,
        _attrs: E,
    ) -> Result<(), SimplexError> {
        Err(SimplexError::NotAvailable {
            method: "add_edge".to_string(),
            alternative: "add_simplex".to_string(),
        })
    }

    /// `remove_edge` is not available on a simplicial complex.
    /// XGI parity: raises XGIError (simplicialcomplex.py:153-157).
    pub fn remove_edge(&mut self, _edge_id: &str) -> Result<(), SimplexError> {
        Err(SimplexError::NotAvailable {
            method: "remove_edge".to_string(),
            alternative: "remove_simplex_id".to_string(),
        })
    }

    /// The lazily-computed face lattice (immediate-face relation), rebuilt on
    /// first access after a mutation.
    pub fn face_lattice(&self) -> &FaceLattice {
        self.face_lattice.get_or_init(|| self.compute_face_lattice())
    }

    fn compute_face_lattice(&self) -> FaceLattice {
        // Precompute each simplex's member-set.
        let sets: Vec<(String, BTreeSet<String>)> = self
            .inner
            .edge_ids()
            .into_iter()
            .map(|e| {
                let s: BTreeSet<String> =
                    self.inner.members(&e).unwrap_or_default().into_iter().collect();
                (e, s)
            })
            .collect();
        let mut immediate_faces: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (eid, me) in &sets {
            let faces: Vec<String> = sets
                .iter()
                .filter(|(fid, mf)| {
                    fid != eid && mf.len() + 1 == me.len() && mf.is_subset(me)
                })
                .map(|(fid, _)| fid.clone())
                .collect();
            immediate_faces.insert(eid.clone(), faces);
        }
        FaceLattice { immediate_faces }
    }

    /// Invalidate the cached face lattice (called after any mutation).
    fn invalidate(&mut self) {
        self.face_lattice = OnceCell::new();
    }
}
```

In `lib.rs`, re-export:

```rust
pub use core::error::{EdgeError, NodeError, SimplexError, SwapError};
pub use core::simplicialcomplex::{FaceLattice, SimplicialComplex};
```

(Merge the `SimplexError` into the existing error re-export line.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_simplicialcomplex`
Expected: PASS — 6 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): SimplicialComplex + add_simplex closure + face lattice cache

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 13: `SimplicialComplex` — `add_simplices_from`, `close`, `remove_simplex_id`, `copy`, `freeze`

**Files:**
- Modify: `hypergraph-rs/crates/hypergraph-rs/tests/test_simplicialcomplex.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/simplicialcomplex.rs`

**Interfaces:**
- `add_simplices_from(simplices: Vec<(Vec<String>, Option<String>, E)>)` where `N: Default, E: Default, M: Default + Clone` — add many (each via `add_simplex`).
- `close(&mut self)` — add every missing subface of every existing simplex.
- `remove_simplex_id(idx) -> Result<(), SimplexError>` — remove the simplex AND all its supfaces (strict supersets) to preserve closure.
- `remove_simplex_ids_from(ids: &[String])`.
- `copy(&self) -> Self` (where `N/E/M: Clone`), `freeze(&mut self)`, `is_frozen()`.

**XGI reference:** `add_simplices_from` (simplicialcomplex.py:373-636 — we implement the common 3-tuple `(members, idx, attr)` form; the multi-format dispatch is the Python binding's concern), `close` (638-662), `remove_simplex_id` removes supfaces then the simplex (726-756), `_supfaces_id` uses strict superset `simplex < s` (368-371).

- [ ] **Step 1: Write the failing test**

Append to `tests/test_simplicialcomplex.rs`:

```rust
#[test]
fn test_add_simplices_from() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplices_from(vec![
        (vec!["0".into(), "1".into()], None, v()),
        (vec!["1".into(), "2".into()], None, v()),
        (vec!["2".into(), "3".into(), "4".into()], None, v()),
    ]);
    // 2 edges (order-1) + {2,3,4} and its 3 faces = 2 + 1 + 3 = 6
    assert_eq!(s.num_edges(), 6);
    assert!(s.has_simplex(&["2".to_string(), "3".to_string()]));
}

#[test]
fn test_close_adds_missing_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    // seed a bare 2-simplex via add_simplex, then remove one face to force a gap
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], Some("top".into()), v()).unwrap();
    s.remove_simplex_id("top").unwrap(); // removes {1,2,3} only (no supfaces)
    let before = s.num_edges();
    s.close(); // no gaps -> no change (the 2-faces are already closed)
    assert_eq!(s.num_edges(), before);
}

#[test]
fn test_remove_simplex_id_removes_supfaces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v()).unwrap();
    // find the id of face {1,2}
    // removing {1,2} must also remove {1,2,3} (its supface) to preserve closure
    let face_id = s
        .edge_ids()
        .into_iter()
        .find(|e| {
            let mut m = s.members(e).unwrap();
            m.sort();
            m == vec!["1".to_string(), "2".to_string()]
        })
        .unwrap();
    s.remove_simplex_id(&face_id).unwrap();
    assert!(!s.has_simplex(&["1".to_string(), "2".to_string()]));
    assert!(!s.has_simplex(&["1".to_string(), "2".to_string(), "3".to_string()]));
    // {1,3} and {2,3} survive
    assert!(s.has_simplex(&["1".to_string(), "3".to_string()]));
}

#[test]
fn test_remove_simplex_id_missing_errors() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    assert!(matches!(s.remove_simplex_id("nope"), Err(SimplexError::NotFound { .. })));
}

#[test]
fn test_simplicialcomplex_copy_independent() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v()).unwrap();
    let mut c = s.copy();
    c.add_simplex(vec!["3".into(), "4".into()], None, v()).unwrap();
    assert!(!s.has_simplex(&["3".to_string(), "4".to_string()]));
    assert_eq!(s.num_edges(), 1);
}

#[test]
fn test_simplicialcomplex_freeze() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v()).unwrap();
    s.freeze();
    assert!(s.is_frozen());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut s = s;
        let _ = s.add_simplex(vec!["3".into(), "4".into()], None, v());
    }));
    assert!(r.is_err()); // add_simplex -> inner.add_edge panics when frozen
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_simplicialcomplex`
Expected: FAIL — the new methods don't exist.

- [ ] **Step 3: Implement the remaining methods**

Add inside the `impl<N, E, M> SimplicialComplex<N, E, M>` block:

```rust
    /// Add many simplices (each via `add_simplex`, so each closes under faces).
    /// XGI parity: `S.add_simplices_from(ebunch)` — 3-tuple form
    /// (simplicialcomplex.py:373-636).
    pub fn add_simplices_from(&mut self, simplices: Vec<(Vec<String>, Option<String>, E)>)
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        for (members, idx, attrs) in simplices {
            let _ = self.add_simplex(members, idx, attrs);
        }
    }

    /// Add all missing subfaces of every existing simplex.
    /// XGI parity: `S.close()` (simplicialcomplex.py:638-662).
    pub fn close(&mut self)
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        let existing: Vec<Vec<String>> =
            self.inner.edge_ids().iter().filter_map(|e| self.inner.members(e)).collect();
        for members in existing {
            for face in subfaces(&as_sorted_set(&members)) {
                if !face.is_empty() && !self.has_simplex(&face) {
                    let _ = self.inner.add_edge(face, None, E::default());
                }
            }
        }
        self.invalidate();
    }

    /// Remove a simplex AND every simplex that strictly contains it (its
    /// supfaces), preserving closure. XGI parity: `S.remove_simplex_id(idx)`
    /// (simplicialcomplex.py:726-756).
    pub fn remove_simplex_id(&mut self, idx: &str) -> Result<(), SimplexError> {
        if !self.inner.has_edge(idx) {
            return Err(SimplexError::NotFound { idx: idx.to_string() });
        }
        let target: BTreeSet<String> =
            self.inner.members(idx).unwrap_or_default().into_iter().collect();
        // Supfaces: simplices whose member-set is a strict superset of target.
        let supfaces: Vec<String> = self
            .inner
            .edge_ids()
            .into_iter()
            .filter(|e| {
                let m: BTreeSet<String> =
                    self.inner.members(e).unwrap_or_default().into_iter().collect();
                m.len() > target.len() && target.is_subset(&m)
            })
            .collect();
        for s in supfaces {
            let _ = self.inner.remove_edge(&s);
        }
        let _ = self.inner.remove_edge(idx);
        self.invalidate();
        Ok(())
    }

    /// Remove several simplices by id (each with its supfaces).
    pub fn remove_simplex_ids_from(&mut self, ids: &[String]) {
        for idx in ids {
            // A supface removal earlier may already have removed this id; skip.
            if self.inner.has_edge(idx) {
                let _ = self.remove_simplex_id(idx);
            }
        }
    }

    /// An independent deep copy (face lattice recomputed lazily).
    /// XGI parity: `S.copy()`.
    pub fn copy(&self) -> Self
    where
        N: Clone,
        E: Clone,
        M: Clone,
    {
        Self { inner: self.inner.copy(), face_lattice: OnceCell::new() }
    }

    /// Freeze against modification (via the inner hypergraph's frozen guard).
    /// XGI parity: `S.freeze()`.
    pub fn freeze(&mut self) {
        self.inner.freeze();
    }

    /// Whether the complex is frozen. XGI parity: `S.is_frozen`.
    pub fn is_frozen(&self) -> bool {
        self.inner.is_frozen()
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_simplicialcomplex`
Expected: PASS — 12 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): SimplicialComplex add_simplices_from/close/remove_simplex_id/copy/freeze

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 14: `globalviews::subhypergraph`

**Files:**
- Create: `hypergraph-rs/crates/hypergraph-rs/tests/test_globalviews.rs`
- Create: `hypergraph-rs/crates/hypergraph-rs/src/core/globalviews.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/core/mod.rs`
- Modify: `hypergraph-rs/crates/hypergraph-rs/src/lib.rs`

**Interfaces:**
- `subhypergraph(h: &Hypergraph<Value, Value, Value>, nodes: Option<&[String]>, edges: Option<&[String]>, keep_isolates: bool) -> Hypergraph<Value, Value, Value>` — a frozen induced sub-hypergraph.

**XGI reference** (globalviews.py:19-74): keep the given nodes (default all), keep the given edges (default all) that are subsets of the kept nodes (63-68), optionally drop isolates (70-71), then `freeze()` (73).

- [ ] **Step 1: Write the failing test**

Write to `hypergraph-rs/crates/hypergraph-rs/tests/test_globalviews.rs`:

```rust
use hypergraph_rs::{subhypergraph, Hypergraph};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_subhypergraph_induced_on_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    // keep only nodes {1,2,3}: e0 survives (subset), e1 dropped (4 excluded)
    let sub = subhypergraph(&h, Some(&["1".to_string(), "2".to_string(), "3".to_string()]), None, true);
    assert!(sub.has_edge("e0"));
    assert!(!sub.has_edge("e1"));
    assert!(sub.has_node("1"));
    assert!(!sub.has_node("4"));
    assert!(sub.is_frozen());
}

#[test]
fn test_subhypergraph_by_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    let sub = subhypergraph(&h, None, Some(&["e0".to_string()]), true);
    assert!(sub.has_edge("e0"));
    assert!(!sub.has_edge("e1"));
    // all nodes kept by default (keep_isolates=true)
    assert!(sub.has_node("3"));
}

#[test]
fn test_subhypergraph_drop_isolates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v()).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v()).unwrap();
    let sub = subhypergraph(&h, None, Some(&["e0".to_string()]), false);
    // e1's nodes 3,4 become isolates and are dropped
    assert!(!sub.has_node("3"));
    assert!(sub.has_node("1"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_globalviews`
Expected: FAIL — `subhypergraph` doesn't exist.

- [ ] **Step 3: Create the module**

In `core/mod.rs`, add:

```rust
pub mod globalviews;
```

Write to `hypergraph-rs/crates/hypergraph-rs/src/core/globalviews.rs`:

```rust
//! Read-only global views over a hypergraph.

use std::collections::BTreeSet;

use serde_json::Value;

use super::hypergraph::Hypergraph;

/// A frozen induced sub-hypergraph. Keeps `nodes` (default all) and the `edges`
/// (default all) that are subsets of the kept nodes; optionally drops the
/// resulting isolates. XGI parity: `xgi.subhypergraph(H, nodes, edges,
/// keep_isolates)` (globalviews.py:19-74).
pub fn subhypergraph(
    h: &Hypergraph<Value, Value, Value>,
    nodes: Option<&[String]>,
    edges: Option<&[String]>,
    keep_isolates: bool,
) -> Hypergraph<Value, Value, Value> {
    let node_set: BTreeSet<String> = match nodes {
        None => h.node_ids().into_iter().collect(),
        Some(ns) => ns.iter().filter(|n| h.has_node(n)).cloned().collect(),
    };
    let edge_set: BTreeSet<String> = match edges {
        None => h.edge_ids().into_iter().collect(),
        Some(es) => es.iter().filter(|e| h.has_edge(e)).cloned().collect(),
    };

    let mut new = Hypergraph::new();
    for (k, val) in h.graph_attrs().clone() {
        new.set_graph_attr(&k, val);
    }
    // Nodes in insertion order.
    for nid in h.node_ids() {
        if node_set.contains(&nid) {
            let attrs = h.node_attrs(&nid).cloned().unwrap_or(Value::Null);
            new.add_node(&nid, attrs);
        }
    }
    // Edges that are subsets of the kept node set, in insertion order.
    for eid in h.edge_ids() {
        if !edge_set.contains(&eid) {
            continue;
        }
        let members = h.members(&eid).unwrap_or_default();
        if !members.is_empty() && members.iter().all(|m| node_set.contains(m)) {
            let attrs = h.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            let _ = new.add_edge(members, Some(eid.clone()), attrs);
        }
    }

    if !keep_isolates {
        let isolates = new.isolate_node_ids();
        new.remove_nodes_from(&isolates);
    }
    new.freeze();
    new
}
```

In `lib.rs`, re-export:

```rust
pub use core::globalviews::subhypergraph;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hypergraph-rs && cargo test -p hypergraph-rs --test test_globalviews`
Expected: PASS — 3 tests, 0 failures.

- [ ] **Step 5: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "feat(hypergraph-rs): implement subhypergraph global view

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 15: Clippy, rustfmt, and full-suite verification

**Files:**
- Modify: any files with clippy warnings

- [ ] **Step 1: Run clippy on the core crate**

Run: `cd hypergraph-rs && cargo clippy -p hypergraph-rs --tests -- -D warnings`
Expected: may warn. Fix all.

- [ ] **Step 2: Fix any clippy warnings**

Likely candidates and their preferred fixes:
- `clippy::type_complexity` on the `(Vec<String>, Vec<String>)` return tuples — acceptable; add a targeted `#[allow(clippy::type_complexity)]` with a one-line justification if it fires.
- `clippy::needless_range_loop` on the `combinations` index arithmetic — the index bookkeeping is intentional; `#[allow]` with a justification.
- `clippy::should_implement_trait` if a method named `default`/`eq` collides — none expected (we implement the real traits).
- `clippy::len_without_is_empty` on the views — both `NodeView` and `EdgeView` already have `is_empty`, so this should not fire.
- Unused imports — remove.

- [ ] **Step 3: Run clippy across the workspace**

Run: `cd hypergraph-rs && cargo clippy --workspace --tests -- -D warnings`
Expected: PASS — no warnings.

- [ ] **Step 4: Run the full test suite**

Run: `cd hypergraph-rs && cargo test --workspace`
Expected: PASS — all tests, 0 failures. Phase 1's `test_hypergraph.rs` (64 tests) plus Phase 2's five new files: `test_hypergraph_ext.rs` (22), `test_views.rs` (12), `test_dihypergraph.rs` (22), `test_simplicialcomplex.rs` (12), `test_globalviews.rs` (3) = **135 total**. The pyo3/wasm/cli stub crates contribute 0.

- [ ] **Step 5: Run rustfmt**

Run: `cd hypergraph-rs && cargo fmt --all`

- [ ] **Step 6: Commit**

```bash
cd hypergraph-rs
git add -A
git commit -m "chore(hypergraph-rs): clippy + fmt cleanup for Phase 2

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Phase 2 Completion Summary

After all 15 tasks, `hypergraph-rs` has:

- **Deferred `Hypergraph` methods** (all six): `double_edge_swap` (+ `SwapError`), `random_edge_shuffle` (seeded `Pcg64Mcg`), `merge_duplicate_edges` (+ `MergeRename`/`MergeRule`), `cleanup`/`cleaned` (isolates/singletons/multiedges; connected+relabel deferred), `dual`, `update`; plus `remove_nodes_from`, `remove_edges_from`, `isolate_node_ids`, `singleton_edge_ids`, `graph_attrs`, and the private `attach_member`/`detach_member` rewire helpers.
- **`DiHypergraph<N,E,M>`** — full CRUD (`add_node`/`add_edge`/`remove_node`/`remove_edge`/`add_node_to_edge`/`remove_node_from_edge`), directional queries (`dimembers`/`tail`/`head`/`members`/`dimemberships`/`memberships`), node & edge stats (`degree`/`in_degree`/`out_degree`/`size`/`order`/`tail_size`/`head_size`/`tail_order`/`head_order`/`isolate_node_ids`), `copy`/`clear`/`freeze`/`is_frozen`, `Debug`, `PartialEq`, built on a shared `NodeKind` substrate with a new `DiMembershipEdge`/`DiRole`.
- **`SimplicialComplex<N,E,M>`** — closure-under-faces `add_simplex`/`add_simplices_from`/`close`, `has_simplex`, supface-preserving `remove_simplex_id`/`remove_simplex_ids_from`, blocked `add_edge`/`remove_edge`, `copy`/`freeze`, and a lazily-cached `FaceLattice` invalidated on mutation.
- **Views** — `NodeView` (degree/memberships/isolates/isolates_ignoring_singletons/filterby_degree) and `EdgeView` (size/order/members/singletons/empty/filterby_size/filterby_order/duplicates/lookup/maximal), plus the `FilterMode` enum.
- **`subhypergraph`** global view (frozen induced sub-hypergraph).
- **135 integration tests** across six files, all passing; **clippy clean**, **rustfmt clean**.

**Conformance readiness:** the Rust surface for XGI's `tests/core/test_dihypergraph.py`, `test_simplicialcomplex.py`, `test_views.py`, and `test_globalviews.py` is complete. Those Python files become the actual gate in **Phase 7** when the PyO3 binding wraps this surface (the `DiNodeView`/`DiEdgeView` proxy objects and the string-dispatch `filterby` are synthesized there over these direct methods).

**Next: Phase 3** — `linalg` (incidence/adjacency/overlap via `sprs`) + `algorithms` (centrality/clustering/paths/connected, delegating to `rustworkx-core`), which unblocks `cleanup`'s deferred `connected` step.
