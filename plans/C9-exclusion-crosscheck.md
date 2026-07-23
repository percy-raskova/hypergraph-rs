# C9 — Exclusion Cross-Check: does hypergraph-rs cover everything babylon actually calls?

**Status:** gate artifact for babylon issue **#282** (drop-in `xgi` → `hypergraph_rs` swap).
**Date:** 2026-07-22. **Conformance baseline:** 311 passing / 0 failing ported oracle tests @ `9fb2710`.
**Method:** static. Every `H.*` expression in babylon's seven `xgi`-importing files was enumerated verbatim, then matched against `tests/ported/**` and `tests/conftest_exclusions.py`. No pytest run was performed for this document; every COVERED verdict below cites a *specific, named, non-excluded* oracle test that the last conformance run reports as passing.

**Why this document exists:** a drop-in swap fails *silently* far more easily than it fails loudly. The failure mode we are guarding against is not `AttributeError` — that surfaces on the first call. It is a method that exists, returns a plausible value, and returns it in a different **order** or under a different **ID identity** than xgi did, so `community_overlap_matrix` quietly produces a permuted matrix and babylon's solidarity numbers drift by an amount nobody can attribute. §3 is the part of this document that matters most.

---

## 1. The used surface

Seven babylon files import `xgi`. Three of them (`engine/bifurcation_monitor.py`, `domain/bifurcation/analysis.py`, `domain/bifurcation/axis.py`) only carry `H: xgi.Hypergraph` as a type-annotated pass-through parameter and make **zero** `H.*` calls — they are swap-neutral by inspection. All real usage lives in `engine/systems/community.py`, `engine/graph_wrappers.py`, `domain/bifurcation/bridges.py`, and `domain/bifurcation/consciousness.py`.

The complete surface is **11 distinct expressions**. Nothing else. No `remove_node`, no `remove_edge`, no `SimplicialComplex`, no `DiHypergraph`, no direct access to the private `_node` / `_edge` / `_net_attr` dicts.

## 2. Cross-check table

| # | xgi expression | babylon call sites | Verdict | Evidence |
|---|---|---|---|---|
| 1 | `xgi.Hypergraph()` | `community.py:74`; `graph_wrappers.py:94` | **COVERED** | `tests/ported/core/test_hypergraph.py::test_hypergraph_attrs`, `::test_add_edge` construct it bare. Not in `FILE_EXCLUSIONS` or `TEST_EXCLUSIONS`. |
| 2 | `H.add_node(agent_id)` | `community.py:82`; `graph_wrappers.py:118` | **COVERED** | `core/test_hypergraph.py::test_add_node_attr` (with and without kwargs). Not excluded. |
| 3 | `agent_id in H.nodes` / `not in H.nodes` (undirected `Hypergraph`) | `community.py:81,128`; `graph_wrappers.py:117,149`; `consciousness.py:85` | **UNTESTED** | No ported test asserts this exact expression on an *undirected* `Hypergraph`. What IS pinned: `x in H` (bare `Hypergraph.__contains__`, `core/test_hypergraph.py::test_contains` / `::test_add_node_attr`, impl `hypergraph.rs:433`) and `x in H.nodes` on **DiHypergraph** only (`core/test_dihypergraph.py:162`). `NodeView.__contains__` (`views.rs:121`) delegates to the same `PyIdMap::get_key()` primitive as the tested `in H` path, so residual risk is low — but the expression babylon relies on at five sites is not pinned by name. **Action A1 below.** |
| 4 | `H.add_edge(members, idx=…, **attrs)` | `community.py:93-108` (11 float/str kwargs); `graph_wrappers.py:119` | **COVERED** | `core/test_hypergraph.py::test_add_edge` (`idx=` int), `::test_add_edge_with_id` (`idx="myedge"`, str), `::test_add_edge_with_attr` (kwargs + `H.edges[0]` readback). All three pin the `idx=` spelling (not `id=`) for **both** int and str ids. Not excluded. |
| 5 | `H.nodes.memberships(agent_id)` | `community.py:130-131`; `graph_wrappers.py:151-152`; `consciousness.py:89` | **COVERED** | `core/test_views.py::test_memberships_read_only`. Not excluded. |
| 6 | `edge_id in H.edges` / `not in H.edges` | `community.py:156,162`; `graph_wrappers.py:135` | **COVERED** | `core/test_hypergraph.py::test_add_edge` (`0 in H.edges`), `::test_add_edge_with_id` (`"myedge" in H.edges`), `core/test_views.py::test_singletons` (`1 not in H.edges`). Both int and str id forms, both polarities. Not excluded. |
| 7 | `H.edges.members(edge_id)` | `community.py:157,163,173`; `graph_wrappers.py:137`; `bridges.py:114` | **COVERED** | `core/test_hypergraph.py::{test_add_edge, test_add_edge_with_id, test_add_edge_with_attr, test_lshift}`; `core/test_views.py::{test_members_read_only, test_edge_order}`. Not excluded. |
| 8 | `for edge_id in H.edges:` / `set(H.edges)` | `community.py:167`; `bridges.py:101`; `graph_wrappers.py:124` | **COVERED** | `core/test_globalviews.py:46` (`set(H.edges) == set(new_H.edges)`) exercises `EdgeView.__iter__` on an undirected `Hypergraph`. Not excluded. Note this pins *membership* of the iteration, not its order — see §3. |
| 9 | `H.edges[edge_id]` (attrs getitem) | `community.py:169` (`.get("category")`) | **COVERED** | `core/test_hypergraph.py::test_add_edge_with_attr` asserts `H.edges[0] == {"color": "red", "place": "peru"}`. Not excluded. |
| 10 | `H.num_edges` | `graph_wrappers.py:157` | **COVERED** | Asserted dozens of times across `core/test_hypergraph.py` (incl. `::test_lshift`). Not excluded. |
| 11 | `xgi.incidence_matrix(H, sparse=False, index=True)` | `community.py:197-201` | **COVERED** | `tests/ported/linalg/test_matrix.py::test_incidence_matrix:59` calls `xgi.incidence_matrix(H1, index=True, sparse=False)` — call shape matches babylon's verbatim — and asserts the `(matrix, rowdict, coldict)` tuple, the dense path, and both index dicts. Implemented at `crates/hypergraph-rs-python/src/linalg.rs:189-251`, exported via `python/hypergraph_rs/linalg/__init__.py` `_BOUND` and re-exported top-level. Excluded tests in that file are only `test_fix_649`, `test_boundary_matrix`, `test_empty`, `test_adjacency_tensor` — **not** this one. |

**Score: 10 COVERED / 1 UNTESTED / 0 EXCLUDED.**

### 2.1 Two "known leads" that turned out to be false alarms

Both are worth recording so nobody re-raises them:

- **`convert/test_incidence.py` is whole-file excluded** (`conftest_exclusions.py:12`, "deferred: to_incidence_matrix / from_incidence_matrix CONVERT dispatchers"). This is **not** the symbol babylon calls. `xgi.convert.to_incidence_matrix` / `from_incidence_matrix` are separate bound names, still `make_deferred(...)` stubs at `python/hypergraph_rs/convert/__init__.py:20-21`. `xgi.incidence_matrix` — the LINALG function at `community.py:197` — is fully implemented, bound, and tested. Babylon calls neither convert dispatcher.
- **The `SimplicialComplex` `remove_node` / memberships-primitive gap** (`conftest_exclusions.py:142-148`) never touches babylon. A grep of all seven xgi-importing files for `.remove_node(`, `.remove_edge(`, `SimplicialComplex`, `DiHypergraph` returns **zero** matches. The related `_net_attr` / `_edge` / `_node` live-reference-vs-snapshot binding gap (`algorithms/test_properties.py::test_equal`, `utils/test_utilities.py::test_iddict`, both excluded) is likewise irrelevant — babylon never reads those private dicts.

---

## 3. Ordering and ID-type analysis (the silent-corruption surface)

This section exists because `community.py:197-207` inverts a `rowdict` into a node index and then computes `I @ I.T`. If the two backends emitted rows in different orders **and** babylon assumed a fixed order, the overlap matrix would be silently permuted — every value present, every value in the wrong cell.

**Ordering.** `hypergraph_rs`'s `node_ids()` / `edge_ids()` return `agent_ids.keys()` / `hyperedge_ids.keys()` from `IndexMap<String, NodeIndex>` fields documented as "Insertion-ordered bimap" (`crates/hypergraph-rs/src/core/hypergraph.rs:31-41`, accessors at `:292-300`). That is architecturally the same contract as xgi's dict-backed `H._node` / `H._edge`. `xgi.incidence_matrix`'s `rowdict` / `coldict` are built straight off those same ordered vectors (`crates/hypergraph-rs-python/src/linalg.rs:199-201`).

So the orders should agree — but babylon does not *depend* on them agreeing. `community_overlap_matrix` inverts **the same call's** returned `rowdict` into `node_index` (`community.py:203-204`) before indexing into `I @ I.T` (`community.py:206`). The matrix and the index map come from one call and are therefore self-consistent by construction, under either backend, regardless of absolute row order. **No permutation risk exists at babylon's actual call site.**

The condition that would create one: a babylon golden fixture that pins exact row/column order against a real-xgi baseline. None of the seven files scanned contains such a fixture. If one is ever added, this analysis must be redone — insertion-order *parity* is architecturally plausible but is not asserted by any conformance test, and nothing stops the two backends diverging on, say, the order in which `add_edge` auto-creates previously-unseen member nodes.

**ID types.** Babylon's ids are uniformly Python `str`. `CommunityType` (`src/babylon/models/enums/community.py:12`) is a `StrEnum`, so `idx=comm_type.value` is a `str`; every agent id (`membership.agent_id`) is a `str`. `PyIdMap::canonical()` (`crates/hypergraph-rs-python/src/pyid.rs:75-90`) takes the `PyString` fast path for every id babylon ever mints. There is no int/str mixed-identity surface in babylon's usage, so the classic xgi footgun (`H.add_edge(..., idx=0)` then `H.edges["0"]`) cannot fire here. The oracle suite nonetheless pins both branches (`test_add_edge` int, `test_add_edge_with_id` str), which is why item 4 is COVERED rather than "COVERED for str only".

**Weighted incidence.** `linalg.rs:15-19` documents that the `weight` callable is applied binding-side rather than inside the core. Babylon never passes `weight=` (`community.py:197-201` passes only `sparse` and `index`), so this deviation is moot today. Flagged only so that a future babylon change that starts passing `weight=` re-triggers this check.

---

## 4. Verdict

**SWAP-SAFE WITH CAVEATS.** Ten of babylon's eleven xgi expressions are pinned by named, passing, non-excluded oracle tests, and the one gap is a syntax-level gap over a primitive that *is* tested through an adjacent path. The exclusion registry contains nothing that intersects babylon's surface: the two exclusions that looked threatening (convert-dispatcher incidence, SimplicialComplex node removal) are provably different symbols on code paths babylon never enters. The ordering analysis finds no silent-permutation risk because babylon's one order-sensitive computation is self-consistent by construction.

There is no **blocker**. There are two things that must land **before** the swap merges, because both are cheap and both are the kind of thing that makes a later regression un-diagnosable:

**A1 — Write the missing containment oracle test (required before swap).**
Add to `tests/ported/core/test_hypergraph.py` (or a clearly-named local supplement if the ported file must stay a verbatim mirror) a test asserting, on an **undirected** `Hypergraph`:

```python
H = xgi.Hypergraph()
H.add_node("C001")
assert "C001" in H.nodes
assert "C002" not in H.nodes
H.add_edge(["C002", "C003"], idx="settler")
assert "C002" in H.nodes          # auto-created member is visible in the NodeView
assert "C999" not in H.nodes
```

It must run green against **both** real `xgi` and `hypergraph_rs` — that dual run is the whole point; a test that only runs against the new backend proves parity with nothing. This covers the exact five babylon sites (`community.py:81,128`; `graph_wrappers.py:117,149`; `consciousness.py:85`), including the auto-created-member case that `graph_wrappers.py:117` depends on.

**A2 — Fix the doc-rot in `tests/DEFERRED_SYMBOLS.md:116` (required before using it as a gate artifact).**
That line lists `incidence_matrix` as an unbuilt / `NotImplementedError` symbol. It is factually stale: `linalg.rs:189-251` implements it, `python/hypergraph_rs/linalg/__init__.py`'s `_BOUND` list exports it, and `linalg/test_matrix.py::test_incidence_matrix` passes against it. The file's mtime (2026-07-22 17:55) predates `linalg.rs`'s (18:31). Anyone auditing the swap against `DEFERRED_SYMBOLS.md` alone would wrongly conclude that `community_overlap_matrix` is unsupported and block a swap that is in fact fine. Regenerate or hand-correct that line. The authoritative gate artifacts are `conftest_exclusions.py` plus the passing suite — not this doc.

**Standing caveats (no action, record only).**
- Insertion-order parity between the backends is architecturally sound but is not asserted by any test. It is safe *today* only because babylon never assumes an absolute order. Adding any order-pinned golden to babylon re-opens §3.
- If babylon ever starts passing `weight=` to `incidence_matrix`, re-verify the binding-side weight application at `linalg.rs:15-19`.
- The three pass-through files (`bifurcation_monitor.py`, `analysis.py`, `axis.py`) are swap-neutral **as long as they stay pass-through**. If any of them grows a direct `H.*` call, this cross-check is stale.

**Re-run trigger:** this document is valid for babylon's xgi surface as of 2026-07-22. Any new `H.*` expression in any of the seven files, or any addition to `FILE_EXCLUSIONS` / `TEST_EXCLUSIONS`, invalidates it.
