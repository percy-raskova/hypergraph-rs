# Phase 7 — Python bindings & the XGI conformance gate

> **For agentic workers:** implement task-by-task (superpowers:subagent-driven-development or
> executing-plans). Every `cargo`/`maturin`/`pytest`/`pip` command is for **execution time** — do not
> run any while authoring. Steps use `- [ ]` checkboxes for tracking.

**Repo:** `/home/user/projects/game/babylon/hypergraph-rs` (own git repo; a subrepo of Babylon).
**Runs strictly after** Phases 0–6 (all Rust core APIs those plans define are available; nothing else).
**Spec:** `docs/superpowers/specs/2026-07-18-hypergraph-rs-design.md` §4, §6, §9, §10.3 (Phase 7).

**Goal.** Bind the Rust core to Python via PyO3 0.29 + maturin; expose XGI 0.10.2's **164-symbol**
public surface under package name `hypergraph_rs` so Babylon's `import hypergraph_rs as xgi` swap is
1-for-1; ship `.pyi` stubs and the matplotlib drawing shim (behind the `mpl` extra); and **pass XGI's
own test suite as the conformance oracle** — copy the 51 oracle test files into `tests/ported/`, swap
the import, and drive every non-excluded assertion to green (core first, then module-by-module).

This phase adds **no new Rust core logic** except one sanctioned additive method — every other task is a
*binding* (PyO3 wrapper), a *shim* (Python-side behavioral translation), a *ported test*, or *packaging*.
The single sanctioned core addition is **Task 0** (`add_empty_edge`, additive + non-breaking, returning
the assigned edge id — orchestrator ruling on OQ-1, 2026-07-19; the existing strict `add_edge` is
untouched).

**Non-negotiable environment facts (verified 2026-07-19; do not contradict):**
- rustc/cargo 1.91.1 (MSRV 1.85), **maturin 1.14.1** (pipx, on PATH), python3.13. pyo3 pinned **0.29**
  (workspace). Binding crate `crates/hypergraph-rs-python`, `[lib] name = "_hypergraph_rs_core"`,
  `crate-type = ["cdylib"]`.
- This phase creates the **subrepo's OWN venv** (`python3.13 -m venv .venv` at the workspace root).
  **NEVER touch Babylon's `.venv`** (`/home/user/projects/game/babylon/.venv`).
- Oracle corpus (real xgi-org/xgi @ v0.10.2, 51 `test_*.py` + `conftest.py`):
  `/tmp/claude-1000/-home-user-projects-game-babylon--claude-worktrees-docs-hypergraph-rs-design/a9bdcfa3-d5c2-4825-afe9-6cfbcdc5df12/scratchpad/xgi-0.10.2/tests/`
  (the spec §9.1 `.pre-commit-cache` path is **stale** — use this corpus).
- Installed xgi package (symbol-inventory source of truth):
  `/home/user/projects/game/babylon/.venv/lib/python3.13/site-packages/xgi/`.
- **HARD CONSTRAINT — NO PUBLISHING.** No `maturin publish`, `twine`, PyPI upload, `cargo publish`,
  `gh release`. Build wheels to `dist/` and smoke-test only. Publishing is an owner decision.
- **No `unsafe`.** All loops statically bounded (Power-of-10 rule 2). PyO3 macros are safe Rust.
- Every behavioral claim about XGI is cited to the oracle corpus (`tests/…`) or installed package
  (`xgi/…`). Deviations declared explicitly.
- Commit after each task. Trailer: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- Python conformance tests live at the **workspace-root** `tests/` (`conftest.py` + `ported/`) per
  spec §9.1 — they are pytest files, **not** cargo targets. Rust integration tests stay under
  `crates/<crate>/tests/`.

---

## §A. Orientation — the three facts that drive the design

The conformance gate (spec §9.1): *"XGI's test files pass against the Rust port with
`import hypergraph_rs as xgi` — the only change."* Three verified facts govern the whole phase.

### Fact 1 — XGI node/edge IDs are arbitrary hashables, preserved by type on readback.

The Rust core keys everything by `String` (`crates/hypergraph-rs/src/core/hypergraph.rs:31-36`:
`agent_ids: IndexMap<String, NodeIndex>`). But XGI's tests assert **integer** identity:
`assert list(H1.nodes) == [1, 2, 3, 4, 5, 6, 8, 7]` (`tests/core/test_hypergraph.py:79`), set
membership over ints (`tests/core/test_hypergraph.py:60-66`), and int-keyed attr dicts
`H.nodes[1] == {"color": "red", …}` (`tests/core/test_hypergraph.py:87`). The binding therefore needs
a **PyId translation layer** — a per-hypergraph, insertion-ordered bimap between the original Python
object and the `String` key the core uses — so `H.nodes` yields the *original ints/strings/tuples*,
not their stringified forms. This is Task 2, the crux of the phase.

### Fact 2 — tests do deep submodule imports that must resolve as real modules.

Verified across the corpus (`grep -rh "from xgi" tests/`):
`from xgi.exception import IDNotFound, XGIError`, `from xgi.utils import dual_dict`,
`from xgi.utils import IDDict`, `from xgi.readwrite.json import read_json, write_json`,
`from xgi.utils.utilities import crest_r`, `from xgi.utils.trie import Trie`,
`from xgi.algorithms.assortativity import _choose_degrees`,
`from xgi.drawing.draw import _CCW_sort, _draw_arg_to_arr, _interp_draw_arg`. The package skeleton
(Task 1) must expose `hypergraph_rs.exception`, `hypergraph_rs.utils`, `hypergraph_rs.utils.utilities`,
`hypergraph_rs.readwrite.json`, etc. as importable modules. Imports of **private helpers no phase
built** (`_choose_degrees`, `crest_r`, `Trie`, the `_draw_*` internals) put those test cases on the
**exclusion list** (they fail at collection or reference an unbuilt symbol).

### Fact 3 — the Rust core built a subset of XGI's 164 symbols.

Phases 0–6 explicitly deferred a large set (tensor centralities, Hodge/boundary matrices, simplicial
Kuramoto, most `convert` dispatchers, flag-complex generators, network readers, several `utils`). The
binding binds **what exists** and emits a machine-checked **Deferred Symbol Ledger** (Task 3) — the
honest "symbols deferred because no phase built them" list the brief requires. Tests exercising only
deferred symbols are excluded with a cited reason.

### The 164-symbol inventory (extracted 2026-07-19 by `ast.literal_eval` of each `__all__`)

| submodule | `__all__` | | submodule | `__all__` |
|---|---|---|---|---|
| `core` | 4 | | `generators` | 25 |
| `algorithms` | 36 | | `linalg` | 11 |
| `communities` | 1 | | `readwrite` | 15 |
| `convert` | 27 | | `stats` | 6 |
| `drawing` | 20 | | `utils` | 15 |
| `dynamics` | 4 | | **total** | **164** |

`xgi.__init__.__all__` is the concatenation of these (`xgi/__init__.py:28-40`). The binding does
**not** need all 164 green: it needs every **non-excluded test** green plus a documented ledger.

### Expected Deferred Symbol Ledger (from Phase 0–6 summaries; reconciled mechanically by Task 3)

No Phase 0–6 plan built these, so the binding cannot expose them. Task 3's audit script regenerates
this list from the actual compiled surface — the table is the *expected* result, never hand-trusted:

- **algorithms (6):** `h_eigenvector_centrality`, `z_eigenvector_centrality`,
  `uniform_h_eigenvector_centrality`, `node_edge_centrality`, `line_vector_centrality`,
  `largest_connected_hypergraph` (Phase 3 summary "Deferred").
- **linalg (3):** `boundary_matrix`, `hodge_laplacian`, `adjacency_tensor` (Phase 3 "Deferred").
- **dynamics (2):** `simulate_simplicial_kuramoto`, `compute_simplicial_order_parameter` (Phase 6
  "Deferred"). *Spec correction:* §9.2/§10.3 label dynamics "SIR/SIS epidemics"; the actual xgi 0.10.2
  `dynamics/` is **synchronization/Kuramoto** (verified: `xgi/dynamics/__init__.py` exports only the 4
  kuramoto symbols; the only dynamics test is `tests/dynamics/test_synchronization.py`). Phase 6 built
  Kuramoto. The plan follows the code, not the stale label.
- **convert (~14):** `to_hif_dict`/`from_hif_dict`, `to_encapsulation_dag`, `empirical_subsets_filter`,
  `to_line_graph`, `from_simplex_dict`, `from_max_simplices`, `k_skeleton`, `from_incidence_matrix`,
  `to_incidence_matrix`, `to_hypergraph`, `to_dihypergraph`, `to_simplicial_complex`,
  `from_bipartite_pandas_dataframe`, `to_bipartite_pandas_dataframe` (Phase 6 "Deferred"; pandas pair
  never ported per spec §4.4).
- **generators (~11):** `empty_simplicial_complex`, `empty_dihypergraph`, `node_swap`,
  `shuffle_hyperedges`, `flag_complex`, `flag_complex_d2`, `random_flag_complex`,
  `random_flag_complex_d2`, `dcsbm_hypergraph`, `uniform_HSBM`, `uniform_HPPM` (not in Phase 4's built
  list).
- **readwrite (~4):** `load_xgi_data`, `download_xgi_data` (network, gated off),
  `read_hif_collection`, `write_hif_collection` (Phase 4 built undirected HIF only).
- **stats (6, shimmed not deferred):** `nodestat_func`, `edgestat_func`, `dinodestat_func`,
  `diedgestat_func`, `dispatch_stat`, `dispatch_many_stats` are XGI's decorator/dispatch *machinery*
  (spec §4.4: "replaced with idiomatic Rust patterns"). The binding **synthesizes** the observable
  `H.nodes.degree.asdict()` surface over Phase 4's `Stat<T>` rather than re-exporting the decorators.
  `test_core_stats_functions.py` (which tests the decorators directly) is a documented exclusion.
- **utils (~10):** `binomial_sequence`, `find_triangles`, `geometric`, `hist`, `pairwise_incidence`,
  `request_json_from_url`, `request_json_from_url_cached`, `ttsv1`, `ttsv2`, `update_uid_counter`
  (Phase 6 built only `dual_dict`/`powerset`/`subfaces`/`get_network_type`/`convert_labels_to_integers`).

**Per-symbol policy:** Task 3 decides **omit** vs **stub-raise** per deferred symbol — *omit* if no
test imports it at module scope; *stub-raise* (`NotImplementedError("deferred: …")`) if a non-excluded
test module imports it at top level (so the module imports and only the specific test fails/skips, not
the whole file).

---

## §B. File structure (Phase 7 additions)

```
hypergraph-rs/
├── pyproject.toml                               # NEW (Task 1) — maturin backend, mpl extra
├── dist/                                         # NEW — maturin wheel output (gitignored)
├── tools/
│   ├── audit_symbols.py                         # NEW (Task 3) — Deferred Symbol Ledger generator
│   └── port_tests.sh                            # NEW (Task 15) — one-shot oracle import-swap copier
├── crates/hypergraph-rs-python/
│   ├── Cargo.toml                               # MODIFY (Task 1) — deps: numpy, feature-gated core
│   ├── src/
│   │   ├── lib.rs                               # MODIFY — module registration (grows every task)
│   │   ├── pyid.rs                              # NEW (Task 2) — PyId layer + json bridge
│   │   ├── errors.rs                            # NEW (Task 3) — XGIError/IDNotFound mapping
│   │   ├── hypergraph.rs                        # NEW (Task 4) — PyHypergraph
│   │   ├── views.rs                             # NEW (Task 5) — PyNodeView/PyEdgeView
│   │   ├── dihypergraph.rs                      # NEW (Task 6)
│   │   ├── simplicial.rs                        # NEW (Task 7) — PySimplicialComplex + subhypergraph
│   │   ├── linalg.rs                            # NEW (Task 8) — numpy/scipy returns
│   │   ├── algorithms.rs                        # NEW (Task 9)
│   │   ├── generators.rs                        # NEW (Task 10)
│   │   ├── stats.rs                             # NEW (Task 11) — Stat proxy objects
│   │   ├── convert.rs / readwrite.rs / utils.rs / dynamics.rs / communities.rs  # NEW (Task 12)
│   │   └── layout.rs                            # NEW (Task 13) — layout + render scene-graph
│   └── hypergraph_rs/                            # NEW — pure-Python package (Task 1+)
│       ├── __init__.py                          # mirrors xgi/__init__.py — 164 names
│       ├── _deferred.py                         # make_deferred() factory
│       ├── exception.py                         # XGIException/XGIError/IDNotFound/frozen (Task 3)
│       ├── py.typed  /  _hypergraph_rs_core.pyi # stubs (Task 14)
│       ├── algorithms/assortativity.py          # deep-import shim: _choose_degrees stub (Task 1, D4)
│       ├── utils/trie.py                         # deep-import shim: Trie stub (Task 1, D4)
│       ├── core/ algorithms/ communities/ convert/ dynamics/ generators/ linalg/ stats/  # __init__.py each
│       ├── drawing/__init__.py                  # matplotlib shim (Task 13)
│       ├── readwrite/{__init__.py, json.py}     # json.py = deep-import target
│       └── utils/{__init__.py, utilities.py}    # utilities.py = deep-import target; IDDict, crest_r
├── tests/                                        # workspace-root Python conformance (spec §9.1)
│   ├── conftest.py                              # adapted oracle conftest + exclusion glue (Task 15)
│   ├── conftest_exclusions.py                   # FILE_EXCLUSIONS / TEST_EXCLUSIONS registry (Task 15)
│   ├── requirements-dev.txt                     # pytest numpy scipy networkx pandas matplotlib
│   ├── EXCLUSIONS.md                            # triage protocol (Task 15)
│   ├── DEFERRED_SYMBOLS.md                      # machine-generated (Task 3)
│   └── ported/                                  # the 51 oracle files, import-swapped (Task 15)
│       ├── core/ algorithms/ communities/ convert/ drawing/ dynamics/ generators/
│       ├── linalg/ readwrite/ stats/ utils/
│       └── (data/ copied if present — OQ-2)
├── crates/hypergraph-rs/src/core/hypergraph.rs      # MODIFY (Task 0) — additive add_empty_edge
└── crates/hypergraph-rs/tests/
    ├── test_hypergraph_ext.rs                        # MODIFY (Task 0) — failing test for add_empty_edge
    └── (property-differential lives in Python — see Task 19)
```

## §C. Global constraints

- **Bounded loops:** every loop over nodes/edges/members iterates a collection whose length is read
  before the loop. No `while true`. The PyId collision loop is bounded (`bump > 1_000_000` break).
- **Explicit error handling:** every core `Result<_, EdgeError|NodeError|SwapError>` maps to a concrete
  Python exception via Task 3 helpers — never a bare `?` surfacing a Rust `Debug` string.
- **Type safety:** `#[pyclass]`/`#[pymethods]` with explicit signatures; `.pyi` stubs keep the surface
  MyPy-strict-clean (spec §6.5).
- **Determinism (III.7 parity):** all iteration is insertion-ordered (core `IndexMap` + `StableDiGraph`;
  the PyId bimap is itself an `IndexMap`). Task 19's property-differential test is the backstop.
- **Deviations shimmed, not re-implemented in the core.** The four baked-in core deviations
  (duplicate `idx`→`Err`, empty members→`Err`, `Vec` not set, missing-id→typed error) are translated
  back to XGI behavior **in the binding** (Task 4 owns all four). No core changes (except OQ-1).
- **serde_json bridge:** Python attrs cross as `serde_json::Value` via `py_any_to_json`/`json_to_py`
  (Task 2). Non-JSON-serializable Python objects **as attribute values** (not IDs) raise `TypeError`
  — a declared deviation (XGI stores arbitrary Python objects as attrs; we require JSON-representable).

---

## Task 0 — core: `add_empty_edge` (the one sanctioned additive core method, TDD)

**Why first:** the empty-members deviation shim (Task 4) needs the core to *hold* an empty hyperedge,
but Phase 0–1's `add_edge` deliberately returns `Err(EdgeError::EmptyMembers)` for empty input
(verified: `crates/hypergraph-rs/src/core/hypergraph.rs:142-144`). **Orchestrator ruling (2026-07-19):
OQ-1 APPROVED — add exactly ONE additive, non-breaking core method, `add_empty_edge`, that *returns
the assigned edge id* (so the binding needs no separate `next_auto_edge_id`, per review D9).** This is
the single core change in Phase 7; it adds no new logic to any existing method (existing `add_edge`
behavior is byte-identical). TDD it here — failing Rust test first, in `crates/hypergraph-rs/tests/`.

**Files:** `crates/hypergraph-rs/src/core/hypergraph.rs` (new method); `crates/hypergraph-rs/tests/test_hypergraph_ext.rs` (append a failing test).

- [ ] **Step 1 — failing Rust test** appended to `crates/hypergraph-rs/tests/test_hypergraph_ext.rs`:

```rust
#[test]
fn add_empty_edge_creates_memberless_hyperedge_and_returns_id() {
    use hypergraph_rs::Hypergraph;
    use serde_json::Value;
    let mut h: Hypergraph<Value, Value, Value> = Hypergraph::new();
    // Auto-id path: returns the assigned id ("0"), edge exists, has zero members.
    let id = h.add_empty_edge(None, Value::Null);
    assert_eq!(id, "0");
    assert!(h.has_edge(&id));
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.num_nodes(), 0);                        // no members => no agent nodes
    assert_eq!(h.edge_members(&id).map(|m| m.len()), Some(0));
    // Explicit-id path: returned id echoes the request; counter still advances past numeric ids.
    let id2 = h.add_empty_edge(Some("e7".to_string()), Value::Null);
    assert_eq!(id2, "e7");
    assert!(h.has_edge("e7"));
    // Auto-id after an explicit non-numeric id continues the uid counter (parity with add_edge).
    let id3 = h.add_empty_edge(None, Value::Null);
    assert_eq!(id3, "1");
}
```

> `edge_members` is the existing core accessor (Phase 1); if its name differs, adjust the assert to the
> real accessor at execution — the load-bearing asserts are *id returned* + *edge exists* + *zero members*.

- [ ] **Step 2 — the additive method** in `core/hypergraph.rs` (mirrors `add_edge`'s id/counter logic,
  minus the `EmptyMembers` guard and the member loop; **non-breaking — `add_edge` is untouched**):

```rust
/// Add a hyperedge with NO members (an empty edge). Returns the assigned edge id.
///
/// XGI 0.10.2 permits empty edges (`H.add_edge([])`); the strict core `add_edge` rejects them with
/// `EdgeError::EmptyMembers` by design. This additive method is the sanctioned binding-support hook
/// (Phase 7 OQ-1) — it duplicates only `add_edge`'s id-assignment/counter policy, adds a lone
/// `Hyperedge` node with no membership edges, and returns the id (so callers need no separate
/// auto-id peek).
pub fn add_empty_edge(&mut self, idx: Option<String>, attrs: E) -> String {
    self.assert_not_frozen();
    let edge_id = match &idx {
        Some(id) => id.clone(),
        None => self.edge_uid_counter.to_string(),
    };
    if let Some(id) = &idx {
        if let Ok(n) = id.parse::<u64>() {
            if n >= self.edge_uid_counter {
                self.edge_uid_counter = n + 1;
            }
        }
    } else {
        self.edge_uid_counter += 1;
    }
    let he_idx = self.inner.add_node(NodeKind::Hyperedge(attrs));
    self.hyperedge_ids.insert(edge_id.clone(), he_idx);
    edge_id
}
```

> **Determinism unchanged:** the method touches only `edge_uid_counter` and inserts one node into the
> existing `StableDiGraph` + `hyperedge_ids` `IndexMap`, so tick-hash parity (III.7) is preserved. It
> does NOT overwrite an existing id — the binding checks `edge_ids.get_key(...)` (Task 4 shim) *before*
> calling this, so the duplicate-idx case never reaches here.

- [ ] **Run & confirm (execution time):** `cargo test -p hypergraph-rs add_empty_edge` → 1 passed;
  `cargo test -p hypergraph-rs` still fully green (additive, non-breaking).

**Commit:** `feat(core): add_empty_edge — additive binding-support method for XGI empty-edge parity`

---

## Task 1 — maturin `pyproject.toml`, package skeleton, subrepo venv, import smoke test

**Why first:** nothing binds until the package imports. Establishes the maturin build, the
`hypergraph_rs/` tree mirroring `xgi/__init__.py`, and the deep-import module stubs. TDD: the failing
test is "package imports and mirrors XGI's 164-name surface; deep imports resolve".

**Files:** `pyproject.toml`; `hypergraph_rs/**` package skeleton; modify
`crates/hypergraph-rs-python/{Cargo.toml,src/lib.rs}`; `tests/ported/test_package_imports.py`.

- [ ] **Step 1 — `hypergraph-rs/pyproject.toml`:**

```toml
[build-system]
requires = ["maturin>=1.14,<2.0"]
build-backend = "maturin"

[project]
name = "hypergraph-rs"
# Version tracks XGI API parity, NOT the XGI package version (spec §6.1: "0.10.2-xgi-parity").
version = "0.1.0"
description = "A Rust port of XGI (xgi-org/xgi) 0.10.2 as a rustworkx-core plugin, with PyO3 bindings."
requires-python = ">=3.11"
license = { text = "BSD-3-Clause" }
authors = [{ name = "Babylon Project" }]
readme = "README.md"
# Core install: NO matplotlib, NO pandas, NO networkx. The stack the core surface needs is
# numpy (arrays out of linalg) + scipy (sparse matrices).
dependencies = ["numpy>=1.24", "scipy>=1.10"]

[project.optional-dependencies]
mpl = ["matplotlib>=3.7"]   # spec §6.3: the matplotlib drawing shim is opt-in.

[tool.maturin]
python-source = "crates/hypergraph-rs-python"
# The last segment MUST equal the `[lib] name` + `#[pymodule] fn` (both pinned `_hypergraph_rs_core`),
# else CPython looks for PyInit__core and import fails. So the extension is `hypergraph_rs._hypergraph_rs_core`.
module-name = "hypergraph_rs._hypergraph_rs_core"
manifest-path = "crates/hypergraph-rs-python/Cargo.toml"
features = ["pyo3/extension-module"]
```

- [ ] **Step 2 — `crates/hypergraph-rs-python/Cargo.toml`** (add deps + enable every built module):

```toml
[package]
name = "hypergraph-rs-python"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "PyO3 bindings for hypergraph-rs"

[lib]
name = "_hypergraph_rs_core"
crate-type = ["cdylib"]

[dependencies]
# Turn on every built module (spec §10.1 `all` minus `network` — no network reads in the gate).
hypergraph-rs = { path = "../hypergraph-rs", features = [
    "algorithms", "generators", "stats", "readwrite", "layout", "viz",
    "dynamics", "communities", "convert",
] }
pyo3 = { version = "0.29", features = ["extension-module"] }
numpy = "0.29"                 # rust-numpy versions track pyo3 1:1 — 0.29 pairs with pyo3 0.29 (NOT 0.23)
serde_json = "1"
rustworkx-core = { workspace = true }
```

> Do **not** add a direct `petgraph` dep — reach petgraph only via `rustworkx_core::petgraph`
> (workspace rule; a direct pin resolves a second, type-incompatible copy — verified 2026-07-18,
> spec §10.1 comment).

- [ ] **Step 3 — `crates/hypergraph-rs-python/src/lib.rs`** (registration hub; each task uncomments
  its `mod` + registrar):

```rust
//! PyO3 bindings for hypergraph-rs — the `_core` extension module.
//!
//! The pure-Python `hypergraph_rs/` package (mirroring XGI's `__init__.py`) imports from this
//! extension. Deliberate core deviations (spec §3.1) are shimmed back to XGI behavior HERE.

use pyo3::prelude::*;

mod pyid;
mod errors;
// mod hypergraph;   // Task 4
// mod views;        // Task 5
// mod dihypergraph; // Task 6
// mod simplicial;   // Task 7
// mod linalg;       // Task 8
// mod algorithms;   // Task 9
// mod generators;   // Task 10
// mod stats;        // Task 11
// mod convert; mod readwrite; mod utils; mod dynamics; mod communities; // Task 12
// mod layout;       // Task 13

#[pymodule]
fn _hypergraph_rs_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register(m)?;
    // hypergraph::register(m)?;   // …each task uncomments its registrar…
    Ok(())
}
```

- [ ] **Step 4 — `hypergraph_rs/__init__.py`** mirroring `xgi/__init__.py:1-40` (verified layout):

```python
"""hypergraph_rs — a Rust port of XGI 0.10.2, import-compatible as ``import hypergraph_rs as xgi``.

The public surface mirrors ``xgi/__init__.py`` (164 symbols). Symbols no Rust phase implemented raise
``NotImplementedError`` on call (see ``DEFERRED_SYMBOLS.md``); they still *import* so any test module
referencing them at top level collects cleanly.
"""
from . import (  # noqa: F401
    utils, core, exception, algorithms, communities, convert, drawing,
    dynamics, generators, linalg, readwrite, stats,
)
from .utils import *        # noqa: F401,F403
from .core import *         # noqa: F401,F403
from .algorithms import *   # noqa: F401,F403
from .communities import *  # noqa: F401,F403
from .convert import *      # noqa: F401,F403
from .drawing import *      # noqa: F401,F403
from .dynamics import *     # noqa: F401,F403
from .generators import *   # noqa: F401,F403
from .linalg import *       # noqa: F401,F403
from .readwrite import *    # noqa: F401,F403
from .stats import *        # noqa: F401,F403

__version__ = "0.10.2-xgi-parity"   # PORT of XGI 0.10.2, not XGI itself (spec §6.1)

__all__ = (
    core.__all__ + algorithms.__all__ + communities.__all__ + convert.__all__
    + drawing.__all__ + dynamics.__all__ + generators.__all__ + linalg.__all__
    + readwrite.__all__ + stats.__all__ + utils.__all__
)
```

- [ ] **Step 5 — the shared deferred factory** `hypergraph_rs/_deferred.py`:

```python
"""Factory for symbols no Rust phase implemented (see DEFERRED_SYMBOLS.md).
They import cleanly (so test modules collect) but raise on call."""

def make_deferred(name):
    def _deferred(*args, **kwargs):
        raise NotImplementedError(
            f"deferred: no hypergraph-rs phase implemented `{name}`; see DEFERRED_SYMBOLS.md"
        )
    _deferred.__name__ = name
    _deferred.__qualname__ = name
    return _deferred
```

- [ ] **Step 6 — every submodule `__init__.py`** declares the **exact XGI `__all__`** (so
  `set(hypergraph_rs.__all__) == set(xgi.__all__)`) and pulls bound symbols from `.._hypergraph_rs_core` under a
  `try/except ImportError` fallback to `make_deferred`, keeping `import hypergraph_rs` green at *every*
  task. Representative — `hypergraph_rs/linalg/__init__.py` (11 names; 8 bound in Task 8, 3 deferred):

```python
"""Matrices (mirrors xgi/linalg/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ["adjacency_matrix", "incidence_matrix", "intersection_profile", "clique_motif_matrix",
          "degree_matrix", "laplacian", "multiorder_laplacian", "normalized_hypergraph_laplacian"]
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)   # until Task 8 binds it

# Phase 3 "Deferred": Hodge/boundary/tensor
boundary_matrix = make_deferred("boundary_matrix")
hodge_laplacian = make_deferred("hodge_laplacian")
adjacency_tensor = make_deferred("adjacency_tensor")

__all__ = ["boundary_matrix", "hodge_laplacian", "adjacency_matrix", "incidence_matrix",
           "intersection_profile", "clique_motif_matrix", "degree_matrix", "adjacency_tensor",
           "laplacian", "multiorder_laplacian", "normalized_hypergraph_laplacian"]
```

  Write all 11 submodule `__init__.py` files this way (full `__all__` from §A inventory). As each
  binding task lands, it deletes that symbol's `try/except` fallback. Also create the deep-import
  targets (Fact 2):
  - `hypergraph_rs/exception.py` — placeholder re-exporting the four names (full text: Task 3).
  - `hypergraph_rs/utils/__init__.py` — exports `dual_dict`, `IDDict`, `powerset`, `subfaces`,
    `get_network_type`, `convert_labels_to_integers` (bound Task 12) + the ~10 deferred stubs.
    `IDDict` is pure-Python: `class IDDict(dict): pass` (verified target of `from xgi.utils import IDDict`).
  - `hypergraph_rs/utils/utilities.py` — re-exports from `hypergraph_rs.utils`; `crest_r = make_deferred("crest_r")`.
  - `hypergraph_rs/readwrite/__init__.py` + `hypergraph_rs/readwrite/json.py` — the latter re-exports
    `read_json`/`write_json` (target of `from xgi.readwrite.json import read_json, write_json`,
    `tests/readwrite/test_json.py`).
  - **`hypergraph_rs/algorithms/assortativity.py` (D4 — mandatory).** `algorithms/test_assortativity.py:5`
    does a MODULE-LEVEL `from hypergraph_rs.algorithms.assortativity import _choose_degrees` after the
    swap; without this module the whole file dies at *collection* (a `pytest_collection_modifyitems`
    skip cannot save it), taking the wanted-green `test_dynamical_assortativity`/`test_degree_assortativity`
    with it. So expose the private helper as a stub:
    ```python
    from .._deferred import make_deferred
    _choose_degrees = make_deferred("_choose_degrees")  # private helper; not part of the port
    ```
  - **`hypergraph_rs/utils/trie.py` (D4 — mandatory).** `algorithms/test_simpliciality.py:4` does a
    MODULE-LEVEL `from hypergraph_rs.utils.trie import Trie`; without this module the whole file dies at
    collection, taking the wanted-green simpliciality-function tests (`test_edit_simpliciality`, …) with
    it. Expose a stub `Trie` (the direct-`Trie()` tests are then TEST_EXCLUSIONS-skipped in Task 15, D5):
    ```python
    from .._deferred import make_deferred
    Trie = make_deferred("Trie")  # utils.trie.Trie not ported (Phase 6); its direct tests are excluded
    ```
  - `hypergraph_rs/py.typed` (empty).

- [ ] **Step 7 — failing test** `tests/ported/test_package_imports.py`:

```python
def test_package_imports():
    import hypergraph_rs as xgi
    assert xgi.__version__ == "0.10.2-xgi-parity"

def test_all_has_164_symbols():
    import hypergraph_rs as xgi
    assert len(set(xgi.__all__)) == 164

def test_deep_import_targets_resolve():
    from hypergraph_rs.exception import IDNotFound, XGIError  # noqa: F401
    from hypergraph_rs.utils import dual_dict, IDDict  # noqa: F401
    from hypergraph_rs.readwrite.json import read_json, write_json  # noqa: F401
```

- [ ] **Run & confirm (execution time):**
```bash
cd /home/user/projects/game/babylon/hypergraph-rs
python3.13 -m venv .venv && . .venv/bin/activate
pip install maturin pytest numpy scipy
maturin develop
pytest tests/ported/test_package_imports.py -q
```
Expected: 3 passed (`__all__` is exactly 164; deep imports resolve).

**Commit:** `feat(python): maturin pyproject + hypergraph_rs package skeleton mirroring xgi __init__`

---

## Task 2 — the PyId translation layer + serde_json bridge (the crux)

**Why:** the core keys by `String`; XGI preserves the original Python object on readback (Fact 1). This
task builds that bijection and the attribute serde bridge; no XGI method yet. Its behavioral proof is
Task 4's `test_node_ids_preserve_python_type` (integer IDs round-trip as ints) — recorded as an explicit
cross-task dependency.

**Files:** create `crates/hypergraph-rs-python/src/pyid.rs` (`mod pyid;` already in `lib.rs`).

- [ ] **Step 1 — `PyKey` (hashable canonical identity):**

```rust
//! Translation between arbitrary hashable Python IDs and the core's `String` keys.
//!
//! XGI node/edge IDs are any hashable Python object, preserved by identity on readback
//! (`tests/core/test_hypergraph.py:79`). The core keys by `String` (`core/hypergraph.rs:31`).
//! `PyIdMap` is the per-hypergraph bijection that lets the binding accept and return the
//! original Python objects while the core stays String-keyed.

use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyInt, PyString, PyTuple, PyList, PyDict};
use serde_json::Value;
use std::hash::{Hash, Hasher};

/// A hashable, `Eq` canonical identity for a Python hashable object.
/// Compared by Python hash AND repr — distinguishes `1` from `"1"` from `(1,)`.
#[derive(Clone)]
pub struct PyKey { pub obj: Py<PyAny>, hash: isize, repr: String }

impl PyKey {
    pub fn new(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        // hash() raises TypeError for unhashable objects — XGI's `[1,2,3] not in H`
        // TypeError path (tests/core/test_hypergraph.py:70).
        let hash = obj.hash()?;
        let repr = obj.repr()?.to_string();
        Ok(Self { obj: obj.clone().unbind(), hash, repr })
    }
}
impl PartialEq for PyKey {
    fn eq(&self, o: &Self) -> bool { self.hash == o.hash && self.repr == o.repr }
}
impl Eq for PyKey {}
impl Hash for PyKey {
    fn hash<H: Hasher>(&self, s: &mut H) { self.hash.hash(s); self.repr.hash(s); }
}
```

- [ ] **Step 2 — `PyIdMap` (the bijection, insertion-ordered):**

```rust
#[derive(Default)]
pub struct PyIdMap {
    to_key: IndexMap<PyKey, String>,
    from_key: IndexMap<String, Py<PyAny>>,
    counter: u64,
}

impl PyIdMap {
    pub fn new() -> Self { Self::default() }

    fn canonical(&mut self, obj: &Bound<'_, PyAny>) -> String {
        if let Ok(s) = obj.downcast::<PyString>() { return s.to_string_lossy().into_owned(); }
        if obj.is_instance_of::<PyBool>() { return obj.str().map(|s| s.to_string()).unwrap_or_default(); }
        if obj.is_instance_of::<PyInt>() {
            if let Ok(v) = obj.extract::<i64>() { return v.to_string(); }
        }
        let n = self.counter; self.counter += 1; format!("__id{n}")
    }

    /// Intern a Python ID → (existing or new) core key. Collision-safe, bounded.
    pub fn intern(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<String> {
        let key = PyKey::new(obj)?;
        if let Some(k) = self.to_key.get(&key) { return Ok(k.clone()); }
        let mut core = self.canonical(obj);
        let mut bump = 0u64;
        while self.from_key.contains_key(&core) {
            bump += 1;
            core = format!("{core}\u{1}{bump}");   // U+0001 never appears in real ids
            if bump > 1_000_000 { break; }         // bounded (Power-of-10 rule 2)
        }
        self.to_key.insert(key, core.clone());
        self.from_key.insert(core.clone(), obj.clone().unbind());
        Ok(core)
    }

    /// Look up an existing core key WITHOUT interning (read-only queries / membership).
    pub fn get_key(&self, obj: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        Ok(self.to_key.get(&PyKey::new(obj)?).cloned())
    }

    pub fn get_obj<'py>(&self, py: Python<'py>, core: &str) -> Option<Bound<'py, PyAny>> {
        self.from_key.get(core).map(|o| o.bind(py).clone())
    }

    pub fn forget(&mut self, core: &str) {
        if let Some(obj) = self.from_key.shift_remove(core) {
            Python::with_gil(|py| {
                if let Ok(k) = PyKey::new(obj.bind(py)) { self.to_key.shift_remove(&k); }
            });
        }
    }

    /// Deep-copy under the GIL (Py<PyAny> clones are refcount bumps) — used by PyHypergraph::copy.
    pub fn clone_via_py(&self, py: Python<'_>) -> Self {
        Self {
            to_key: self.to_key.iter()
                .map(|(k, v)| (PyKey { obj: k.obj.clone_ref(py), hash: k.hash_raw(), repr: k.repr_raw() }, v.clone()))
                .collect(),
            from_key: self.from_key.iter().map(|(k, v)| (k.clone(), v.clone_ref(py))).collect(),
            counter: self.counter,
        }
    }
}
```

> Add tiny `hash_raw()`/`repr_raw()` accessors to `PyKey` (or make its fields `pub`) so `clone_via_py`
> rebuilds keys without a GIL `repr()` call.

- [ ] **Step 3 — serde_json ⇄ Python attribute bridge:**

```rust
/// Python ATTRIBUTE VALUE → serde_json::Value. Non-serializable values raise TypeError
/// (declared deviation: XGI stores arbitrary Python objects as attrs; we require JSON-representable).
pub fn py_any_to_json(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() { return Ok(Value::Null); }
    if let Ok(b) = obj.downcast::<PyBool>() { return Ok(Value::Bool(b.is_true())); }
    if let Ok(i) = obj.extract::<i64>() { return Ok(Value::from(i)); }
    if let Ok(f) = obj.extract::<f64>() { return Ok(Value::from(f)); }
    if let Ok(s) = obj.downcast::<PyString>() { return Ok(Value::from(s.to_string_lossy().into_owned())); }
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut out = Vec::with_capacity(list.len());
        for it in list.iter() { out.push(py_any_to_json(&it)?); }
        return Ok(Value::Array(out));
    }
    if let Ok(tup) = obj.downcast::<PyTuple>() {
        let mut out = Vec::with_capacity(tup.len());
        for it in tup.iter() { out.push(py_any_to_json(&it)?); }
        return Ok(Value::Array(out));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() { map.insert(k.str()?.to_string(), py_any_to_json(&v)?); }
        return Ok(Value::Object(map));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "attribute value of type {} is not JSON-serializable (hypergraph-rs deviation)",
        obj.get_type().name()?)))
}

/// serde_json::Value → Python object.
pub fn json_to_py<'py>(py: Python<'py>, v: &Value) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        Value::Null => py.None().into_bound(py),
        Value::Bool(b) => b.into_pyobject(py)?.to_owned().into_any(),
        Value::Number(n) if n.is_i64() => n.as_i64().unwrap().into_pyobject(py)?.into_any(),
        Value::Number(n) => n.as_f64().unwrap().into_pyobject(py)?.into_any(),
        Value::String(s) => s.into_pyobject(py)?.into_any(),
        Value::Array(a) => {
            let list = PyList::empty(py);
            for it in a { list.append(json_to_py(py, it)?)?; }
            list.into_any()
        }
        Value::Object(m) => {
            let dict = PyDict::new(py);
            for (k, val) in m { dict.set_item(k, json_to_py(py, val)?)?; }
            dict.into_any()
        }
    })
}
```

- [ ] **Step 4 — failing Rust unit test** (pure-Rust invariant; behavioral proof deferred to Task 4):

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn collision_separator_is_bounded_control_char() {
        // The U+0001 collision loop is bounded by 1_000_000 (Power-of-10 rule 2).
        // The behavioral bijection (int ids round-trip as ints) is proved in Task 4's
        // test_node_ids_preserve_python_type. Documented cross-reference.
        assert_eq!('\u{1}' as u32, 1);
    }
}
```

- [ ] **Run & confirm:** `cargo test -p hypergraph-rs-python` → 1 passed.

**Commit:** `feat(python): PyId translation layer (Python hashable IDs ⇄ core String keys) + json bridge`

---

## Task 3 — exception hierarchy, error mapping, and the Deferred Symbol Ledger audit

**Why:** the oracle imports `from xgi.exception import IDNotFound, XGIError`
(`tests/core/test_hypergraph.py:7`) and asserts `pytest.raises(XGIError)` / `pytest.raises(IDNotFound)`
widely. `IDNotFound` subclasses **both `XGIException` and `KeyError`** (`xgi/exception.py:31`). This
task also generates the honest Deferred Symbol Ledger.

**Files:** `crates/hypergraph-rs-python/src/errors.rs`; finalize `hypergraph_rs/exception.py`;
`tools/audit_symbols.py`; `tests/DEFERRED_SYMBOLS.md` (generated).

- [ ] **Step 1 — `hypergraph_rs/exception.py`** (verbatim parity with `xgi/exception.py:1-46`):

```python
"""XGI exception hierarchy (verbatim parity with xgi/exception.py).

XGIException
├── XGIError        — domain-specific errors (e.g. modifying a frozen network)
└── IDNotFound      — also inherits KeyError; raised when a node/edge ID is missing
"""


class XGIException(Exception):
    """Base class for exceptions in XGI."""


class XGIError(XGIException):
    """Exception for a serious error in XGI."""


class IDNotFound(XGIException, KeyError):
    """Raised when a node or edge is not in the hypergraph."""


def frozen(*args, **kwargs):
    """Raises XGIError when trying to modify a frozen network."""
    raise XGIError("Frozen higher-order network can't be modified")
```

- [ ] **Step 2 — `errors.rs`** (import the Python exception classes so raised errors match what tests
  catch):

```rust
//! Bridge core `EdgeError`/`NodeError`/`SwapError` → XGI-compatible Python exceptions.
//!
//! Tests catch `xgi.exception.XGIError` and `xgi.exception.IDNotFound`
//! (`tests/core/test_hypergraph.py:7`). `IDNotFound` subclasses KeyError (`xgi/exception.py:31`),
//! so it also satisfies `pytest.raises(KeyError)`.

use pyo3::prelude::*;

// pyo3 0.29 has NO `import_exception_bound!` (verified in the cached crate source — only
// `import_exception!` exists, and it already yields Bound-compatible types with `::new_err(...)`).
pyo3::import_exception!(hypergraph_rs.exception, XGIError);
pyo3::import_exception!(hypergraph_rs.exception, IDNotFound);
pyo3::import_exception!(hypergraph_rs.exception, XGIException);

pub fn id_not_found(id: &str) -> PyErr { IDNotFound::new_err(format!("{id} is not in the network")) }
pub fn xgi_error(msg: impl Into<String>) -> PyErr { XGIError::new_err(msg.into()) }

/// Registrar (nothing to add to `_core`; the classes live in pure Python). Kept for uniformity.
pub fn register(_m: &Bound<'_, PyModule>) -> PyResult<()> { Ok(()) }
```

> **Deviation shims mapped here** (spec §3.1): duplicate `idx` (`EdgeError::AlreadyExists`) → Task 4
> `warnings.warn(UserWarning)` + no-op; empty members (`EdgeError::EmptyMembers`) → Task 4 silent empty
> edge; missing-id lookups → `id_not_found()` (`IDNotFound`, subclasses `KeyError`);
> `remove_node_from_edge` illegal case → `xgi_error()` (`XGIError`).

- [ ] **Step 3 — the audit script** `tools/audit_symbols.py`:

```python
#!/usr/bin/env python3
"""Compare hypergraph_rs's public surface against XGI 0.10.2's 164 symbols; emit the ledger.

Fails if __all__ != XGI's 164. Classifies each symbol as bound or deferred (a stub whose
__module__ endswith '_deferred'). Writes tests/DEFERRED_SYMBOLS.md.
"""
import importlib

def main():
    xgi = importlib.import_module(
        "xgi", package=None)  # the installed oracle; used only to read the canonical 164 names
    hr = importlib.import_module("hypergraph_rs")
    want, have = set(xgi.__all__), set(hr.__all__)
    assert have == want, f"surface mismatch: missing={want-have} extra={have-want}"
    deferred, bound = [], []
    for name in sorted(want):
        obj = getattr(hr, name, None)
        is_stub = getattr(obj, "__module__", "").endswith("_deferred")
        (deferred if is_stub else bound).append(name)
    with open("tests/DEFERRED_SYMBOLS.md", "w") as f:
        f.write(f"# Deferred symbols ({len(deferred)}/164 not built by any phase)\n\n")
        for n in deferred:
            f.write(f"- `{n}` — no Phase 0-6 plan built it; calling raises NotImplementedError.\n")
        f.write(f"\n# Bound symbols ({len(bound)}/164)\n\n")
        for n in bound: f.write(f"- `{n}`\n")
    print(f"bound={len(bound)} deferred={len(deferred)} total={len(bound)+len(deferred)}")

if __name__ == "__main__":
    main()
```

- [ ] **Step 4 — failing test** `tests/ported/test_exception_hierarchy.py`:

```python
def test_id_not_found_is_keyerror():
    from hypergraph_rs.exception import IDNotFound, XGIException
    assert issubclass(IDNotFound, KeyError)
    assert issubclass(IDNotFound, XGIException)

def test_surface_matches_xgi_164():
    import hypergraph_rs as xgi
    assert len(set(xgi.__all__)) == 164
```

- [ ] **Run & confirm:**
```bash
pip install "xgi==0.10.2"         # D8: the audit reads XGI's canonical 164 names; xgi must be present
maturin develop && pytest tests/ported/test_exception_hierarchy.py -q
python tools/audit_symbols.py     # writes tests/DEFERRED_SYMBOLS.md; bound+deferred == 164
```
Expected: 2 passed; ledger written.

> **D8:** Task 1 installs only `maturin pytest numpy scipy`, so `xgi` is absent until now. `audit_symbols.py`
> imports `xgi` to read the canonical 164 names, so it would raise `ModuleNotFoundError('xgi')` if run
> earlier. Install `xgi==0.10.2` into the **subrepo venv** (never Babylon's) before the audit runs here;
> it is also the test-only differential oracle Task 19 relies on.

**Commit:** `feat(python): XGI exception hierarchy + error mapping + deferred-symbol audit`

---

## Task 4 — `PyHypergraph`: constructors, CRUD, the four deviation shims

**Why:** `Hypergraph` is P0 (~80 tests, `tests/core/test_hypergraph.py`). Wraps Phase 0–2's
`Hypergraph<Value,Value,Value>` with the PyId layer, the serde bridge, and the four deviation shims.
Proves Task 2 (integer IDs round-trip as ints).

**Files:** `crates/hypergraph-rs-python/src/hypergraph.rs`; uncomment `mod hypergraph;` +
`hypergraph::register(m)?;`.

**Design.** `PyHypergraph { inner: CoreHg, node_ids: PyIdMap, edge_ids: PyIdMap, net_attr }`. Every
Python-facing method translates IDs through the maps. XGI's constructor `Hypergraph(incoming_data=None,
**attr)` accepts list-of-iterables, dict, another Hypergraph/SC, numpy incidence, or pandas DataFrame
(`tests/core/test_hypergraph.py:12-16`). The binding supports **list, dict, Hypergraph/SC copy**
natively; **numpy incidence** (needs `from_incidence_matrix`, deferred) and **pandas** (spec §4.4)
raise `XGIError`/are excluded — called out in `test_constructor`'s exclusion.

- [ ] **Step 1 — struct + constructor dispatch:**

```rust
//! PyO3 binding for the core `Hypergraph`. Deviation shims (spec §3.1) live here.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use hypergraph_rs::Hypergraph as CoreHg;
use hypergraph_rs::core::error::EdgeError;
use serde_json::Value;
use crate::pyid::{PyIdMap, py_any_to_json, json_to_py};
use crate::errors::{id_not_found, xgi_error};

#[pyclass(name = "Hypergraph", module = "hypergraph_rs")]
pub struct PyHypergraph {
    pub inner: CoreHg<Value, Value, Value>,
    pub node_ids: PyIdMap,
    pub edge_ids: PyIdMap,
    pub net_attr: serde_json::Map<String, Value>,
}

#[pymethods]
impl PyHypergraph {
    #[new]
    #[pyo3(signature = (incoming_data=None, **attr))]
    fn new(py: Python<'_>, incoming_data: Option<Bound<'_, PyAny>>,
           attr: Option<Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut hg = Self { inner: CoreHg::new(), node_ids: PyIdMap::new(),
                            edge_ids: PyIdMap::new(), net_attr: serde_json::Map::new() };
        if let Some(a) = attr { for (k, v) in a.iter() {
            hg.net_attr.insert(k.str()?.to_string(), py_any_to_json(&v)?);
        }}
        if let Some(data) = incoming_data { hg.ingest(py, &data)?; }
        Ok(hg)
    }

    /// `H["name"]` — network-level attr; missing → XGIError (tests/core/test_hypergraph.py:55-56).
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.net_attr.get(key) {
            Some(v) => Ok(json_to_py(py, v)?.unbind()),
            None => Err(xgi_error(format!("{key} is not a network attribute"))),
        }
    }

    #[getter] fn num_nodes(&self) -> usize { self.inner.num_nodes() }
    #[getter] fn num_edges(&self) -> usize { self.inner.num_edges() }

    #[getter]
    fn _net_attr(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.net_attr { d.set_item(k, json_to_py(py, v)?)?; }
        Ok(d.unbind())
    }
}

impl PyHypergraph {
    fn ingest(&mut self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(list) = data.downcast::<PyList>() {
            for edge in list.iter() { self.add_edge_impl(py, &edge, None, None)?; }
            return Ok(());
        }
        if let Ok(dict) = data.downcast::<PyDict>() {
            for (idx, members) in dict.iter() { self.add_edge_impl(py, &members, Some(&idx), None)?; }
            return Ok(());
        }
        if let Ok(other) = data.extract::<PyRef<PyHypergraph>>() {
            self.inner = other.inner.copy();
            self.node_ids = other.node_ids.clone_via_py(py);
            self.edge_ids = other.edge_ids.clone_via_py(py);
            return Ok(());
        }
        // numpy/pandas/int → XGIError (matches pytest.raises(XGIError): xgi.Hypergraph(1),
        // tests/core/test_hypergraph.py:41). numpy-incidence + pandas constructors are documented
        // exclusions (no phase built from_incidence_matrix / the pandas slice).
        Err(xgi_error("could not interpret data as a hypergraph"))
    }
}
```

- [ ] **Step 2 — `add_edge` with all four deviation shims** (the heart of the phase):

```rust
#[pymethods]
impl PyHypergraph {
    /// `H.add_edge(members, idx=None, **attr)`. Shims: duplicate idx → warn+no-op;
    /// empty members → silent empty edge.
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_edge(&mut self, py: Python<'_>, members: Bound<'_, PyAny>,
                idx: Option<Bound<'_, PyAny>>, attr: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        self.add_edge_impl(py, &members, idx.as_ref(), attr.as_ref())
    }
}

impl PyHypergraph {
    fn add_edge_impl(&mut self, py: Python<'_>, members: &Bound<'_, PyAny>,
                     idx: Option<&Bound<'_, PyAny>>, attr: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let mut core_members = Vec::new();
        for m in members.try_iter()? { core_members.push(self.node_ids.intern(&m?)?); }

        let mut e_attrs = serde_json::Map::new();
        if let Some(a) = attr { for (k, v) in a.iter() {
            e_attrs.insert(k.str()?.to_string(), py_any_to_json(&v)?);
        }}

        let requested_core = match idx {
            Some(o) => {
                if self.edge_ids.get_key(o)?.is_some() {
                    // DEVIATION SHIM 1: duplicate idx → warn + no-op (XGI add_edge).
                    let warnings = py.import("warnings")?;
                    warnings.call_method1("warn",
                        (format!("edge {} already exists, ignoring", o.repr()?),
                         py.get_type::<pyo3::exceptions::PyUserWarning>()))?;
                    return Ok(());
                }
                Some(self.edge_ids.intern(o)?)
            }
            None => None,
        };

        match self.inner.add_edge(core_members, requested_core.clone(), Value::Object(e_attrs.clone())) {
            Ok(core_id) => {
                if requested_core.is_none() {
                    // Auto-id path: XGI auto-ids are ints 0,1,2,… — register the Python int.
                    let obj = core_id.parse::<i64>().ok()
                        .and_then(|n| n.into_pyobject(py).ok()).map(|b| b.into_any())
                        .unwrap_or_else(|| core_id.clone().into_pyobject(py).unwrap().into_any());
                    self.edge_ids.intern(&obj)?;
                }
                Ok(())
            }
            // DEVIATION SHIM 2: empty members → silent empty edge (core rejects; we allow).
            // Task 0's `add_empty_edge(idx, attrs) -> String` RETURNS the assigned id, so there is no
            // separate `next_auto_edge_id` call (review D9 / OQ-1 ruling).
            Err(EdgeError::EmptyMembers) => {
                let core_id = self.inner.add_empty_edge(requested_core, Value::Object(e_attrs));
                if idx.is_none() {
                    let obj = core_id.parse::<i64>().ok()
                        .and_then(|n| n.into_pyobject(py).ok()).map(|b| b.into_any())
                        .unwrap_or_else(|| core_id.clone().into_pyobject(py).unwrap().into_any());
                    self.edge_ids.intern(&obj)?;
                }
                Ok(())
            }
            Err(EdgeError::AlreadyExists { .. }) => Ok(()),   // shimmed above; unreachable
            Err(e) => Err(xgi_error(format!("{e:?}"))),
        }
    }
}
```

> **OQ-1 (RESOLVED — see Task 0):** the empty-edge shim calls the single sanctioned additive core
> method `add_empty_edge(idx, attrs) -> String`, landed with a failing-first Rust test in **Task 0**.
> Per the orchestrator ruling (2026-07-19) and review D9, it is **one** method (not two) — it *returns*
> the assigned id, so no separate `next_auto_edge_id` is needed. It is additive and non-breaking (the
> strict `add_edge` is untouched), so no empty-edge tests need excluding on this account.

- [ ] **Step 3 — node CRUD + membership queries** (`add_node`, `add_nodes_from`, `remove_node`,
  `remove_edge`, `has_node`, `__contains__`, `memberships`, `members`, `H.degree`). Representative:

```rust
#[pymethods]
impl PyHypergraph {
    /// `n in H` — unhashable objects return False, not error (XGI test_contains,
    /// tests/core/test_hypergraph.py:70).
    fn __contains__(&self, n: &Bound<'_, PyAny>) -> bool {
        matches!(self.node_ids.get_key(n), Ok(Some(_)))
    }

    #[pyo3(signature = (n, strong=false))]
    fn remove_node(&mut self, n: &Bound<'_, PyAny>, strong: bool) -> PyResult<()> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        self.inner.remove_node(&core, strong).map_err(|_| id_not_found(&core))?;
        self.node_ids.forget(&core);
        Ok(())
    }
    // add_node / add_nodes_from / remove_edge / has_node / memberships / members …
    // each intern-on-write, translate-on-read; verify the exact core remove_node signature
    // (weak+strong is a single bool per Phase 0-1 summary) at execution.
}
```

- [ ] **Step 4 — `H.nodes` / `H.edges` getters** return Task 5's proxies. For Task 4, stub them to a
  minimal id iterator so `set(H.nodes)`/`list(H.nodes)` work (the constructor test needs `set(H.nodes)`);
  Task 5 replaces the stubs with full views.

- [ ] **Step 5 — failing test** `tests/ported/test_pyhypergraph_smoke.py`:

```python
def test_node_ids_preserve_python_type():   # THE Task-2 behavioral proof
    import hypergraph_rs as xgi
    H = xgi.Hypergraph()
    H.add_edge([1, 2, 3], idx="e1")
    assert list(H.nodes) == [1, 2, 3]        # ints, not "1","2","3"
    assert all(isinstance(n, int) for n in H.nodes)
    assert H.num_nodes == 3 and H.num_edges == 1

def test_duplicate_idx_warns_and_noops():
    import warnings, hypergraph_rs as xgi
    H = xgi.Hypergraph(); H.add_edge([1, 2], idx="e1")
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always"); H.add_edge([3, 4], idx="e1")
        assert any(issubclass(x.category, UserWarning) for x in w)
    assert H.num_edges == 1                    # no-op

def test_missing_node_raises_idnotfound():
    import pytest, hypergraph_rs as xgi
    from hypergraph_rs.exception import IDNotFound
    H = xgi.Hypergraph()
    with pytest.raises(IDNotFound): H.remove_node(99)
```

- [ ] **Run & confirm:** `maturin develop && pytest tests/ported/test_pyhypergraph_smoke.py -q` → 3 passed.

**Commit:** `feat(python): PyHypergraph — constructors, CRUD, four XGI deviation shims`

---

## Task 5 — `PyNodeView` / `PyEdgeView` + the `Stat` getter

**Why:** `tests/core/test_views.py` (~30). XGI's `H.nodes` is a `NodeView` with `__iter__`/`__len__`/
`__contains__`/`__getitem__`/`.memberships()`/`.degree`/`.filterby(...)`; `H.edges.members(e)` returns
a **set** (deviation: core returns `Vec` → convert; `tests/core/test_hypergraph.py:83`). Wraps Phase
2's `NodeView`/`EdgeView` + `FilterMode`.

**Files:** `crates/hypergraph-rs-python/src/views.rs`; register.

- [ ] **Step 1 — the view proxies** hold a `Py<PyHypergraph>` and re-borrow under the GIL per call
  (standard PyO3 back-reference pattern; a `#[pyclass]` cannot hold a Rust borrow):

```rust
//! PyNodeView / PyEdgeView proxy objects.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet, PyList};
use crate::hypergraph::PyHypergraph;

#[pyclass(name = "NodeView", module = "hypergraph_rs")]
pub struct PyNodeView { pub hg: Py<PyHypergraph> }

#[pymethods]
impl PyNodeView {
    fn __len__(&self, py: Python<'_>) -> usize { self.hg.borrow(py).inner.num_nodes() }

    fn __iter__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let hg = slf.hg.borrow(py);
        let items: Vec<Py<PyAny>> = hg.inner.node_ids()
            .filter_map(|c| hg.node_ids.get_obj(py, c).map(|o| o.unbind())).collect();
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> bool {
        self.hg.borrow(py).node_ids.get_key(n).ok().flatten().is_some()
    }

    /// `H.nodes[n]` → attribute dict (tests/core/test_hypergraph.py:87).
    fn __getitem__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let core = hg.node_ids.get_key(n)?.ok_or_else(|| crate::errors::id_not_found("node"))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = hg.inner.node_attrs(&core).cloned() {
            for (k, v) in m { d.set_item(k, crate::pyid::json_to_py(py, &v)?)?; }
        }
        Ok(d.unbind())
    }

    /// `H.nodes.memberships(n)` → a SET of edge ids (deviation: core Vec → set).
    fn memberships(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let core = hg.node_ids.get_key(n)?.ok_or_else(|| crate::errors::id_not_found("node"))?;
        let set = PySet::empty(py)?;
        for e in hg.inner.memberships(&core).unwrap_or_default() {
            if let Some(o) = hg.edge_ids.get_obj(py, &e) { set.add(o)?; }
        }
        Ok(set.unbind())
    }

    #[getter]
    fn degree(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        crate::stats::PyNodeStat::degree(self.hg.clone_ref(py))   // Task 11 owns PyNodeStat
    }
}
```

  `PyEdgeView` mirrors this with `.members(e)` returning a **set**, `.order`/`.size` stats, and Phase 2's
  `singletons/empty/filterby_size/filterby_order/duplicates/maximal`. Full text follows the same pattern.
  Add `use serde_json::Value;`.

- [ ] **Step 2 — wire `H.nodes`/`H.edges` getters in `hypergraph.rs`** (replace Task 4 stubs):

```rust
#[pymethods]
impl PyHypergraph {
    #[getter] fn nodes(slf: Py<Self>) -> crate::views::PyNodeView { crate::views::PyNodeView { hg: slf } }
    #[getter] fn edges(slf: Py<Self>) -> crate::views::PyEdgeView { crate::views::PyEdgeView { hg: slf } }
}
```

- [ ] **Step 3 — failing test** `tests/ported/test_views_smoke.py`:

```python
def test_edge_members_is_a_set():
    import hypergraph_rs as xgi
    H = xgi.Hypergraph([[1, 2, 3], [4]])
    assert H.edges.members(0) == {1, 2, 3}      # a set, ints preserved
    assert isinstance(H.edges.members(0), set)

def test_node_degree_asdict():
    import hypergraph_rs as xgi
    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])   # edgelist1
    assert H.nodes.degree.asdict() == {1:1,2:1,3:1,4:1,5:1,6:2,7:1,8:1}
```

> Values cited from `tests/stats/test_nodestats.py:10-14` (edgelist1 degrees).

- [ ] **Run & confirm:** `pytest tests/ported/test_views_smoke.py -q` → 2 passed (after Task 11's Stat).

**Commit:** `feat(python): PyNodeView/PyEdgeView proxy objects + Stat getter wiring`

---

## Task 6 — `PyDiHypergraph` + directed views

**Why:** `tests/core/test_dihypergraph.py` (~50, P0). Wraps Phase 2's `DiHypergraph`. Directed
`add_edge` takes a `(tail_set, head_set)` pair (`conftest.py:diedgelist1` = `({1,2,3},{4})`,
`dihypergraph1` = `{"e1": [{"a","b"}, {"c"}], …}`). Same PyId + deviation-shim pattern + `DiNodeView`/
`DiEdgeView` (synthesized per Phase 2 summary note).

**Files:** `crates/hypergraph-rs-python/src/dihypergraph.rs`; register `DiHypergraph`.

- [ ] **Step 1 — struct + directed add_edge:**

```rust
#[pymethods]
impl PyDiHypergraph {
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_edge(&mut self, py: Python<'_>, members: Bound<'_, PyAny>,
                idx: Option<Bound<'_, PyAny>>, attr: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        let pair: (Bound<'_, PyAny>, Bound<'_, PyAny>) = members.extract()
            .map_err(|_| crate::errors::xgi_error("directed edge must be a (tail, head) pair"))?;
        let mut tail = Vec::new();
        for m in pair.0.try_iter()? { tail.push(self.node_ids.intern(&m?)?); }
        let mut head = Vec::new();
        for m in pair.1.try_iter()? { head.push(self.node_ids.intern(&m?)?); }
        // edge-attr serde, duplicate-idx warn shim (as Task 4), core add_edge(tail, head, idx, attrs)
        let _ = (py, idx, attr, tail, head);
        Ok(())
    }
    // dimembers/tail/head/dimemberships/degree/in_degree/out_degree — wrap Phase 2 methods
}
```

- [ ] **Step 2 — `DiNodeView`/`DiEdgeView`** mirror Task 5; `.dimembers(e)` returns a `(set, set)`
  tuple, `.tail(e)`/`.head(e)` return sets. Full text in the patch.

- [ ] **Step 3 — failing test** `tests/ported/test_dihypergraph_smoke.py`:

```python
def test_directed_members_roundtrip():
    import hypergraph_rs as xgi
    H = xgi.DiHypergraph([({1, 2, 3}, {4}), ({5, 6}, {6, 7, 8})])   # diedgelist1
    assert H.edges.dimembers(0) == ({1, 2, 3}, {4})
    assert H.num_nodes == 8
```

> Cited from `conftest.py:diedgelist1` + the DiHypergraph API (Phase 2 summary).

- [ ] **Run & confirm:** `pytest tests/ported/test_dihypergraph_smoke.py -q` → passed.

**Commit:** `feat(python): PyDiHypergraph + directed views`

---

## Task 7 — `PySimplicialComplex` + `subhypergraph`

**Why:** `tests/core/test_simplicialcomplex.py` (~40, P1) and `test_globalviews.py` (~15). Wraps
Phase 2's `SimplicialComplex` (closure-under-faces `add_simplex`/`add_simplices_from`/`close`,
blocked `add_edge`) and `globalviews::subhypergraph`.

**Files:** `crates/hypergraph-rs-python/src/simplicial.rs`; register `SimplicialComplex`, `subhypergraph`.

- [ ] **Step 1 — `PySimplicialComplex`** — own `#[pyclass]` with an overriding `add_edge` that raises
  (XGI `SimplicialComplex.add_edge` raises `XGIError`), plus `add_simplex`/`add_simplices_from`/`close`:

```rust
#[pymethods]
impl PySimplicialComplex {
    fn add_edge(&self, _a: Bound<'_, PyAny>) -> PyResult<()> {
        Err(crate::errors::xgi_error(
            "Cannot add_edge to a SimplicialComplex; use add_simplex/add_simplices_from"))
    }
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_simplex(&mut self, py: Python<'_>, members: Bound<'_, PyAny>,
                   idx: Option<Bound<'_, PyAny>>, attr: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        // intern members, call inner.add_simplex (closes under faces), register minted face ids
        let _ = (py, members, idx, attr); Ok(())
    }
    // add_simplices_from / close / has_simplex …
}
```

- [ ] **Step 2 — `subhypergraph`** as a module `#[pyfunction]`:

```rust
#[pyfunction]
#[pyo3(signature = (h, nodes=None, edges=None))]
fn subhypergraph(py: Python<'_>, h: PyRef<PyHypergraph>,
                 nodes: Option<Bound<'_, PyAny>>, edges: Option<Bound<'_, PyAny>>) -> PyResult<PyHypergraph> {
    // translate selections to core keys, call globalviews::subhypergraph, rebuild bimaps
    let _ = (py, h, nodes, edges); todo!("wrap globalviews::subhypergraph")
}
```

- [ ] **Step 3 — failing test** `tests/ported/test_simplicial_smoke.py`:

```python
def test_add_edge_blocked_on_sc():
    import pytest, hypergraph_rs as xgi
    from hypergraph_rs.exception import XGIError
    S = xgi.SimplicialComplex()
    with pytest.raises(XGIError): S.add_edge([1, 2, 3])

def test_closure_under_faces():
    import hypergraph_rs as xgi
    S = xgi.SimplicialComplex(); S.add_simplices_from([[1, 2, 3]])
    assert {1, 2} in [set(S.edges.members(e)) for e in S.edges]   # closure adds all faces
```

- [ ] **Run & confirm:** `pytest tests/ported/test_simplicial_smoke.py -q` → passed.

**Commit:** `feat(python): PySimplicialComplex + subhypergraph global view`

---

## Task 8 — `linalg` bindings (numpy / scipy returns)

**Why:** `tests/linalg/test_matrix.py` (P1) + `tests/readwrite/test_incidence_matrix.py` (Babylon uses
this). Phase 3 built 8 matrix functions. XGI returns numpy (dense) or scipy sparse; when `index=True`,
`incidence_matrix` returns `(I, rowdict, coldict)` (`tests/linalg/test_matrix.py`).

**Files:** `crates/hypergraph-rs-python/src/linalg.rs`; register the 8 functions.

- [ ] **Step 1 — dense + sparse + the index tuple:**

```rust
use numpy::{IntoPyArray, PyArrayMethods};

#[pyfunction]
#[pyo3(signature = (h, order=None, sparse=true, index=false, weight=None))]
fn incidence_matrix(py: Python<'_>, h: PyRef<PyHypergraph>, order: Option<usize>,
                    sparse: bool, index: bool, weight: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let _ = (order, weight);
    let matrix: Py<PyAny> = if sparse {
        to_scipy_csr(py, &h.inner.incidence_matrix_sparse())?
    } else {
        // D11: do NOT pass a core ndarray straight to `into_pyarray` — rust-numpy 0.29 bundles its own
        // ndarray version, which may differ from the core crate's ndarray (the scirs2-numpy fork exists
        // precisely to bridge an ndarray-0.17 gap). A version-mismatched Array2 fails to typecheck.
        // Cross via a flat Vec<f64> + shape, then reshape inside numpy — no core ndarray type crosses
        // the boundary. (Verify with `cargo tree -i ndarray` at execution.)
        let (rows, cols, data) = h.inner.incidence_matrix_dense_vec();   // (nrows, ncols, row-major Vec<f64>)
        data.into_pyarray(py).reshape((rows, cols))?.into_any().unbind()
    };
    wrap_index(py, matrix, &h, index)   // returns (matrix, rowdict, coldict) when index else matrix
}
```

> **D11:** if Phase 3 exposes only a typed `Array2` (`incidence_matrix()`), add a thin
> `incidence_matrix_dense_vec()` accessor (or `.into_raw_vec()` at the call site) so the crossing value
> is a plain `Vec<f64>` + shape. Same treatment for any other dense-matrix return in this task.

- [ ] **Step 2 — scipy CSR helper** (build `scipy.sparse.csr_matrix((data, indices, indptr), shape=…)`):

```rust
fn to_scipy_csr(py: Python<'_>, m: &sprs::CsMat<f64>) -> PyResult<Py<PyAny>> {
    let sp = py.import("scipy.sparse")?;
    let data = m.data().to_vec().into_pyarray(py);
    let indices: Vec<i32> = m.indices().iter().map(|&i| i as i32).collect();
    let indptr: Vec<i32> = m.indptr().as_slice().unwrap_or(&[]).iter().map(|&i| i as i32).collect();
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("shape", (m.rows(), m.cols()))?;
    let csr = sp.getattr("csr_matrix")?.call(
        ((data, indices.into_pyarray(py), indptr.into_pyarray(py)),), Some(&kwargs))?;
    Ok(csr.into_any().unbind())
}
```

> **Determinism:** rows = node insertion order, cols = edge insertion order (Phase 3 summary,
> "row/col-ordered by node/edge insertion order (III.7)"). The rowdict/coldict map matrix index →
> original Python id via the bimaps.

  Bind all 8: `incidence_matrix`, `adjacency_matrix`, `clique_motif_matrix`, `intersection_profile`,
  `degree_matrix`, `laplacian`, `multiorder_laplacian`, `normalized_hypergraph_laplacian`. Deferred
  stubs (already in Task 1): `boundary_matrix`, `hodge_laplacian`, `adjacency_tensor`.

- [ ] **Step 3 — failing test** `tests/ported/test_linalg_smoke.py`:

```python
def test_incidence_matrix_dense_shape():
    import numpy as np, hypergraph_rs as xgi
    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])   # edgelist1: 8 nodes, 4 edges
    I = xgi.incidence_matrix(H, sparse=False)
    assert isinstance(I, np.ndarray) and I.shape == (8, 4)
```

- [ ] **Run & confirm:** `pytest tests/ported/test_linalg_smoke.py -q` → passed.

**Commit:** `feat(python): linalg bindings (numpy dense + scipy sparse returns)`

---

## Task 9 — `algorithms` bindings

**Why:** `tests/algorithms/` (centrality, clustering, shortest_path, connected, assortativity,
properties, simpliciality). Bind Phase 3's non-tensor subset; deferred tensor centralities +
`largest_connected_hypergraph` are `make_deferred` stubs (Task 1). Results that are node-keyed dicts
return original Python ids via the bimaps.

**Files:** `crates/hypergraph-rs-python/src/algorithms.rs`; register.

- [ ] **Step 1 — representative bindings:**

```rust
#[pyfunction] fn is_connected(h: PyRef<PyHypergraph>) -> bool { h.inner.is_connected() }

#[pyfunction]
fn clustering_coefficient(py: Python<'_>, h: PyRef<PyHypergraph>) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    for (core, val) in h.inner.clustering_coefficient() {
        if let Some(o) = h.node_ids.get_obj(py, &core) { d.set_item(o, val)?; }
    }
    Ok(d.unbind())
}
```

  **Bind (register all Phase-3-built):** `dynamical_assortativity`, `degree_assortativity`,
  `clique_eigenvector_centrality`, `katz_centrality`, `clustering_coefficient`,
  `local_clustering_coefficient`, `two_node_clustering_coefficient`, `is_connected`,
  `connected_components`, `largest_connected_component`, `number_connected_components`,
  `node_connected_component`, `equal`, `num_edges_order`, `max_edge_order`, `is_uniform`,
  `is_possible_order`, `edge_neighborhood`, `degree_counts`, `degree_histogram`, `unique_edge_sizes`,
  `density`, `incidence_density`, `single_source_shortest_path_length`, `shortest_path_length`,
  `edit_simpliciality`, `simplicial_edit_distance`, `face_edit_simpliciality`,
  `mean_face_edit_distance`, `simplicial_fraction`.
  **Deferred stubs:** `h_eigenvector_centrality`, `z_eigenvector_centrality`,
  `uniform_h_eigenvector_centrality`, `node_edge_centrality`, `line_vector_centrality`,
  `largest_connected_hypergraph`.

> **Verify `equal` at execution** — Phase 3 named `properties` but not `equal` explicitly. If the core
> lacks it, it becomes a deferred stub; Task 3's audit makes this honest automatically.

- [ ] **Step 2 — failing test** `tests/ported/test_algorithms_smoke.py`:

```python
def test_is_connected_and_components():
    import hypergraph_rs as xgi
    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])   # 3 components
    assert xgi.is_connected(H) is False
    assert xgi.number_connected_components(H) == 3
```

- [ ] **Run & confirm:** `pytest tests/ported/test_algorithms_smoke.py -q` → passed.

**Commit:** `feat(python): algorithms bindings (non-tensor subset) + deferred stubs`

---

## Task 10 — `generators` bindings (seeded, integer node ids)

**Why:** `tests/generators/`. XGI generators take `seed=` and produce **integer** node ids
(`xgi.random_hypergraph(10, [0.2], seed=1)`, `tests/drawing/test_layout.py:6`). Bind each returning a
fresh `PyHypergraph`/`PySimplicialComplex`. §9.4 splitting happens at the *test* level (Task 17).

**Files:** `crates/hypergraph-rs-python/src/generators.rs`; register.

- [ ] **Step 1 — representative + the int-id builder:**

```rust
#[pyfunction]
#[pyo3(signature = (n, ps, order=None, seed=None))]
fn random_hypergraph(py: Python<'_>, n: usize, ps: Vec<f64>, order: Option<usize>,
                     seed: Option<u64>) -> PyResult<PyHypergraph> {
    let core = hypergraph_rs::generators::random_hypergraph(n, &ps, order, seed);
    Ok(PyHypergraph::from_core_with_int_ids(py, core))   // node keys "0".."n-1" → Python ints
}
```

  `from_core_with_int_ids` (add to `hypergraph.rs`) builds a `PyHypergraph` from a core whose String
  node-keys are `"0".."n-1"`, populating `node_ids`/`edge_ids` with the corresponding Python **ints**.
  **Bind:** `empty_hypergraph`, `trivial_hypergraph`, `complete_hypergraph`, `complement`,
  `ring_lattice`, `star_clique`, `sunflower`, `fast_random_hypergraph`, `random_hypergraph`,
  `chung_lu_hypergraph`, `watts_strogatz_hypergraph`, `uniform_erdos_renyi_hypergraph`,
  `uniform_hypergraph_configuration_model`, `random_simplicial_complex`. **Deferred stubs:**
  `empty_simplicial_complex`, `empty_dihypergraph`, `node_swap`, `shuffle_hyperedges`, `flag_complex`,
  `flag_complex_d2`, `random_flag_complex`, `random_flag_complex_d2`, `dcsbm_hypergraph`, `uniform_HSBM`,
  `uniform_HPPM`.

- [ ] **Step 2 — failing test** `tests/ported/test_generators_smoke.py`:

```python
def test_random_hypergraph_structural():
    import hypergraph_rs as xgi
    H = xgi.random_hypergraph(10, [0.2], seed=1)
    assert H.num_nodes == 10                  # structural (§9.4 option a)
    assert all(isinstance(n, int) for n in H.nodes)
```

- [ ] **Run & confirm:** `pytest tests/ported/test_generators_smoke.py -q` → passed.

**Commit:** `feat(python): generators bindings (seeded, integer node ids) + deferred stubs`

---

## Task 11 — `stats`: the `Stat` proxy objects (`H.nodes.degree.asdict()`, `H.degree()`)

**Why:** `tests/stats/`. The observable surface is `H.nodes.degree.asdict()`/`.aslist()`,
`H.degree(order=None)`, `H.nodes.filterby("degree", 2)`, `H.edges.order.asdict()`. XGI's 6 stats
`__all__` are the decorator machinery → `make_deferred` stubs (Task 1); the binding synthesizes the
`Stat` objects over Phase 4's `Stat<T>`. `test_core_stats_functions.py` (tests the decorators) is a
documented exclusion.

**Files:** `crates/hypergraph-rs-python/src/stats.rs`; register `PyNodeStat`, `PyEdgeStat`.

- [ ] **Step 1 — `PyNodeStat`:**

```rust
#[pyclass(name = "NodeStat", module = "hypergraph_rs")]
pub struct PyNodeStat { hg: Py<PyHypergraph>, kind: NodeStatKind }
#[derive(Clone, Copy)] enum NodeStatKind { Degree }

impl PyNodeStat {
    pub fn degree(hg: Py<PyHypergraph>) -> Self { Self { hg, kind: NodeStatKind::Degree } }
    fn values(&self, py: Python<'_>) -> Vec<(String, i64)> {
        let hg = self.hg.borrow(py);
        match self.kind {
            NodeStatKind::Degree => hg.inner.node_ids()
                .map(|c| (c.to_string(), hg.inner.node_degree(c) as i64)).collect(),
        }
    }
}

#[pymethods]
impl PyNodeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.values(py) {
            d.set_item(hg.node_ids.get_obj(py, &core).unwrap(), v)?;   // degrees are ints in XGI
        }
        Ok(d.unbind())
    }
    fn aslist(&self, py: Python<'_>) -> Vec<i64> { self.values(py).into_iter().map(|(_, v)| v).collect() }
}
```

  `PyEdgeStat` mirrors with `Order`/`Size` kinds. Add `H.degree(order=None)` to `hypergraph.rs`
  (delegates to the same Phase-4 order-filtered degree; used directly at `tests/stats/test_nodestats.py:12`).

- [ ] **Step 2 — failing test** `tests/ported/test_stats_smoke.py`:

```python
def test_degree_stat_dict_and_list():
    import hypergraph_rs as xgi
    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])
    assert H.degree() == {1:1,2:1,3:1,4:1,5:1,6:2,7:1,8:1}
    assert H.nodes.degree.asdict() == H.degree()
```

- [ ] **Run & confirm:** `pytest tests/ported/test_stats_smoke.py -q` → passed.

**Commit:** `feat(python): stats — Stat proxy objects (asdict/aslist/filterby) + H.degree`

---

## Task 12 — `convert` / `readwrite` / `utils` / `dynamics` / `communities` bindings

**Why:** the remaining module tests. Each is a thin `#[pyfunction]` layer over Phase 4/6. One task,
five sub-steps; bind what phases built, stub the rest.

- [ ] **12a `readwrite`** (Phase 4): `read_json`/`write_json` (+ re-exported in `readwrite/json.py`),
  `read_edgelist`/`write_edgelist`/`parse_edgelist`, bipartite edgelist trio, `read_incidence_matrix`/
  `write_incidence_matrix`, `read_hif`/`write_hif`, `load_bigg_data` (**network reader — bind it, but
  its test file is FILE-EXCLUDED per D7: `test_bigg_data.py` is `@webtest`/`@slow` and hits the BiGG
  server; not asserted green in the gate**). File I/O via path
  strings. Deferred stubs: `load_xgi_data`, `download_xgi_data`, `read_hif_collection`,
  `write_hif_collection`. Representative:

```rust
#[pyfunction]
#[pyo3(signature = (h, path))]
fn write_json(h: PyRef<PyHypergraph>, path: &str) -> PyResult<()> {
    let json = h.inner.to_hypergraph_dict_json();
    std::fs::write(path, json).map_err(|e| crate::errors::xgi_error(e.to_string()))
}
```

- [ ] **12b `convert`** (Phase 6): `to_hyperedge_list`/`from_hyperedge_list`, `to_hyperedge_dict`/
  `from_hyperedge_dict`, bipartite edgelist pair, `to_bipartite_graph`/`from_bipartite_graph`
  (**networkx** on the Python side — build `nx.Graph`/`nx.DiGraph`), `to_graph` (1-skeleton → `nx.Graph`),
  `to_hypergraph_dict`/`from_hypergraph_dict`, `cut_to_order`. Deferred stubs: the ~14 §A convert
  symbols. `to_graph`:

```rust
#[pyfunction]
fn to_graph(py: Python<'_>, h: PyRef<PyHypergraph>) -> PyResult<Py<PyAny>> {
    let nx = py.import("networkx")?;
    let g = nx.getattr("Graph")?.call0()?;
    for (a, b) in h.inner.skeleton_edges() {   // Phase 3 skeleton()
        let (oa, ob) = (h.node_ids.get_obj(py, &a).unwrap(), h.node_ids.get_obj(py, &b).unwrap());
        g.call_method1("add_edge", (oa, ob))?;
    }
    Ok(g.into_any().unbind())
}
```

- [ ] **12c `utils`** (Phase 6): `dual_dict`, `powerset`, `subfaces`, `get_network_type`,
  `convert_labels_to_integers`. `IDDict` is the pure-Python `class IDDict(dict)` (Task 1). Deferred
  stubs: the ~10 utils helpers + `crest_r` + `Trie`.

- [ ] **12d `dynamics`** (Phase 6, **Kuramoto**): `simulate_kuramoto`,
  `compute_kuramoto_order_parameter` (numpy arrays in/out). Deferred stubs:
  `simulate_simplicial_kuramoto`, `compute_simplicial_order_parameter`. **§9.4 / OQ-3 (ruling: keep the
  split):** `tests/dynamics/test_synchronization.py` asserts a hard-coded `output` array to ~1e-8. That
  is **not a legitimate cross-language contract** — the integrator applies libm transcendentals
  (`sin`/`exp`), which per the project float policy (CLAUDE.md; spec §9.4) do **not** reproduce
  bit-for-bit across implementations (unlike basic IEEE-754 ops). So the exact-trajectory asserts are
  excluded (OQ-3) while the structural shape/`times` asserts are kept.

- [ ] **12e `communities`** (Phase 6): `spectral_clustering`.

- [ ] **Failing test** `tests/ported/test_convert_utils_smoke.py`:

```python
def test_dual_dict():
    from hypergraph_rs.utils import dual_dict
    assert dual_dict({0: [1, 2, 3], 1: [4]}) == {1: [0], 2: [0], 3: [0], 4: [1]}

def test_to_graph_is_nx():
    import networkx as nx, hypergraph_rs as xgi
    G = xgi.to_graph(xgi.Hypergraph([[1, 2, 3]]))
    assert isinstance(G, nx.Graph) and G.number_of_nodes() == 3
```

> `dual_dict` semantics cited from `xgi/utils/utilities.py` (`from xgi.utils import dual_dict`,
> `tests/core/test_hypergraph.py:8`).

- [ ] **Run & confirm:** `pytest tests/ported/test_convert_utils_smoke.py -q` → passed.

**Commit:** `feat(python): convert/readwrite/utils/dynamics/communities bindings + deferred stubs`

---

## Task 13 — `layout` + `render()` scene-graph + the matplotlib drawing shim (`mpl` extra)

**Why:** `tests/drawing/test_layout.py` (P1) needs the layout functions; `test_draw.py`/
`test_draw_utils.py` are **replaced** by scene-graph tests (spec §9.3). Binds Phase 5's `layout`/`viz`
and writes the matplotlib shim (spec §6.3), gated so the core install never imports matplotlib.

**Files:** `crates/hypergraph-rs-python/src/layout.rs`; `hypergraph_rs/drawing/__init__.py`.

- [ ] **Step 1 — layout functions** return `dict[id → (x, y)]` keyed by original ids. XGI names
  (`xgi/drawing/__init__.py`): `random_layout`, `pairwise_spring_layout`, `barycenter_spring_layout`,
  `weighted_barycenter_spring_layout`, `circular_layout`, `spiral_layout`,
  `barycenter_kamada_kawai_layout`, `bipartite_spring_layout`, `pca_transform`,
  `edge_positions_from_barycenters`. Map to Phase 5's `spring_layout`/`hyperedge_aware_layout`/
  `bipartite_layout`/`radial_layout`; ones with no Phase-5 analogue are `make_deferred` stubs (verify
  at execution). `random_layout`:

```rust
#[pyfunction]
#[pyo3(signature = (h, seed=None))]
fn random_layout(py: Python<'_>, h: PyRef<PyHypergraph>, seed: Option<u64>) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    for (core, (x, y)) in hypergraph_rs::layout::random_positions(&h.inner, seed.unwrap_or(0)) {
        d.set_item(h.node_ids.get_obj(py, &core).unwrap(), (x, y))?;
    }
    Ok(d.unbind())
}
```

> **§9.4 for layout:** `test_layout.py` asserts *seed reproducibility* (`np.allclose(pos2, pos3)` same
> seed → equal; different seed → not equal, `tests/drawing/test_layout.py:11-13`) — structural and
> achievable with `rand_pcg`. Exact numpy-coordinate parity is not asserted there, so `test_layout.py`
> ports as-is (Phase 5 summary confirms).

- [ ] **Step 2 — `H.render(strategy)`** on `PyHypergraph` returns the scene-graph dict (Phase 5 `render`):

```rust
#[pymethods]
impl PyHypergraph {
    #[pyo3(signature = (strategy=None))]
    fn render(&self, py: Python<'_>, strategy: Option<&str>) -> PyResult<Py<PyDict>> {
        let scene = self.inner.render_to_json_value(strategy.unwrap_or("hyperedge_aware"));
        Ok(crate::pyid::json_to_py(py, &scene)?.downcast_into::<PyDict>()?.unbind())
    }
}
```

- [ ] **Step 3 — the matplotlib shim** `hypergraph_rs/drawing/__init__.py` (spec §6.3; the 20 drawing
  `__all__` names — layout ones from `.._hypergraph_rs_core`, `draw*` are mpl shims):

```python
"""Matplotlib-based drawing for backward compat with XGI (spec §6.3).

OPTIONAL FEATURE: requires ``pip install 'hypergraph-rs[mpl]'``.
Primary viz pathway is the scene-graph via ``H.render()`` (matplotlib-free).
"""
from .._deferred import make_deferred
from .._hypergraph_rs_core import (
    random_layout, pairwise_spring_layout, barycenter_spring_layout,
    weighted_barycenter_spring_layout, circular_layout, spiral_layout,
    barycenter_kamada_kawai_layout, bipartite_spring_layout, pca_transform,
    edge_positions_from_barycenters,
)

try:
    import matplotlib.pyplot as plt  # noqa: F401
    HAS_MPL = True
except ImportError:
    HAS_MPL = False


def _require_mpl(fn):
    def wrapper(*args, **kwargs):
        if not HAS_MPL:
            raise ImportError(
                "matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. "
                "For matplotlib-free rendering, use `H.render()` for a scene-graph.")
        return fn(*args, **kwargs)
    wrapper.__name__ = fn.__name__
    return wrapper


@_require_mpl
def draw(H, pos=None, ax=None, **kwargs):
    """Draw a Hypergraph with matplotlib (XGI ``xgi.draw`` parity). Layout is computed in Rust
    (deterministic); matplotlib is a pure renderer over the scene-graph."""
    import matplotlib.pyplot as plt
    scene = H.render("hyperedge_aware")
    ax = ax or plt.subplots()[1]
    for he in scene["hyperedges"]:
        pts = he.get("hull") or []
        if pts:
            ax.fill([p[0] for p in pts], [p[1] for p in pts], alpha=0.2)
    for node in scene["nodes"]:
        ax.plot(node["x"], node["y"], "o")
    return ax


draw_nodes = _require_mpl(lambda *a, **k: draw(*a, **k))
draw_hyperedges = _require_mpl(lambda *a, **k: draw(*a, **k))
draw_simplices = make_deferred("draw_simplices")
draw_node_labels = make_deferred("draw_node_labels")
draw_hyperedge_labels = make_deferred("draw_hyperedge_labels")
draw_multilayer = make_deferred("draw_multilayer")
draw_bipartite = make_deferred("draw_bipartite")
draw_undirected_dyads = make_deferred("draw_undirected_dyads")
draw_directed_dyads = make_deferred("draw_directed_dyads")

__all__ = [
    "random_layout", "pairwise_spring_layout", "barycenter_spring_layout",
    "weighted_barycenter_spring_layout", "pca_transform", "circular_layout", "spiral_layout",
    "barycenter_kamada_kawai_layout", "bipartite_spring_layout", "edge_positions_from_barycenters",
    "draw", "draw_nodes", "draw_hyperedges", "draw_simplices", "draw_node_labels",
    "draw_hyperedge_labels", "draw_multilayer", "draw_bipartite", "draw_undirected_dyads",
    "draw_directed_dyads",
]
```

- [ ] **Step 4 — the scene-graph replacement test** (spec §9.3) `tests/ported/drawing/test_scene_graph.py`:

```python
def test_render_produces_valid_scene_graph():
    import hypergraph_rs as xgi
    H = xgi.Hypergraph(); H.add_edge(["a", "b", "c"], idx="e1")
    scene = H.render("hyperedge_aware")
    assert len(scene["nodes"]) == 3
    assert len(scene["hyperedges"]) == 1
    assert scene["hyperedges"][0]["member_ids"] == ["a", "b", "c"]
    assert scene["bounding_box"][2] > scene["bounding_box"][0]   # max_x > min_x
```

> Verify the exact scene-graph JSON keys (`member_ids`, `bounding_box`, `hull`, `nodes[].x`) against
> Phase 5's `SceneGraph` serde output at execution and adjust asserts to the real schema — the spec
> snippet is illustrative.

- [ ] **Step 5 — mpl-off test** `tests/ported/drawing/test_mpl_optional.py`:

```python
def test_draw_requires_mpl_extra():
    import hypergraph_rs as xgi, hypergraph_rs.drawing as d
    if not d.HAS_MPL:
        import pytest
        with pytest.raises(ImportError, match="mpl"):
            xgi.draw(xgi.Hypergraph([[1, 2]]))
```

- [ ] **Run & confirm:**
```bash
maturin develop && pytest tests/ported/drawing/test_scene_graph.py tests/ported/drawing/test_mpl_optional.py -q
```
Expected: passed (core install, no matplotlib).

**Commit:** `feat(python): layout bindings + H.render scene-graph + matplotlib shim (mpl extra)`

---

## Task 14 — `.pyi` type stubs (MyPy-strict surface)

**Why:** spec §6.5 — the swap removes Babylon's `type: ignore[import-untyped]` because the binding
ships complete stubs. maturin does not auto-generate `.pyi` from PyO3 (that needs `pyo3-stub-gen`, not
in the pin set); hand-write them, MyPy-strict-clean.

**Files:** `hypergraph_rs/_hypergraph_rs_core.pyi` (+ `py.typed` from Task 1). The stub filename MUST
match the extension module's last segment (`_hypergraph_rs_core`, D3), else MyPy won't associate it.

- [ ] **Step 1 — `_hypergraph_rs_core.pyi` head** (extend to every `#[pyclass]`/`#[pyfunction]`):

```python
from typing import Any, Hashable, Iterable, Iterator

class Hypergraph:
    def __init__(self, incoming_data: Any = ..., **attr: Any) -> None: ...
    @property
    def num_nodes(self) -> int: ...
    @property
    def num_edges(self) -> int: ...
    @property
    def nodes(self) -> NodeView: ...
    @property
    def edges(self) -> EdgeView: ...
    def add_node(self, node: Hashable, **attr: Any) -> None: ...
    def add_edge(self, members: Iterable[Hashable], idx: Hashable | None = ..., **attr: Any) -> None: ...
    def remove_node(self, n: Hashable, strong: bool = ...) -> None: ...
    def __contains__(self, n: object) -> bool: ...
    def __getitem__(self, key: str) -> Any: ...
    def degree(self, order: int | None = ...) -> dict[Hashable, int]: ...
    def render(self, strategy: str | None = ...) -> dict[str, Any]: ...

class NodeView:
    def __iter__(self) -> Iterator[Hashable]: ...
    def __len__(self) -> int: ...
    def __contains__(self, n: object) -> bool: ...
    def __getitem__(self, n: Hashable) -> dict[str, Any]: ...
    def memberships(self, n: Hashable) -> set[Hashable]: ...
    @property
    def degree(self) -> NodeStat: ...

class NodeStat:
    def asdict(self) -> dict[Hashable, int]: ...
    def aslist(self) -> list[int]: ...

# … EdgeView, EdgeStat, DiHypergraph, SimplicialComplex, subhypergraph,
#    and every #[pyfunction] (linalg/algorithms/generators/convert/readwrite/utils/dynamics/layout) …
def incidence_matrix(h: Hypergraph, order: int | None = ..., sparse: bool = ...,
                     index: bool = ..., weight: Any = ...) -> Any: ...
def random_hypergraph(n: int, ps: list[float], order: int | None = ..., seed: int | None = ...) -> Hypergraph: ...
```

- [ ] **Step 2 — failing test** `tests/ported/test_stubs_present.py`:

```python
def test_pyi_shipped():
    import importlib.util, os
    root = os.path.dirname(importlib.util.find_spec("hypergraph_rs").origin)
    assert os.path.exists(os.path.join(root, "py.typed"))
    assert os.path.exists(os.path.join(root, "_hypergraph_rs_core.pyi"))
```

- [ ] **Run & confirm:** `pytest tests/ported/test_stubs_present.py -q` → passed. (A `mypy --strict`
  check over a tiny consumer snippet is added to CI in Task 19.)

**Commit:** `feat(python): hand-written .pyi stubs for the public surface (mypy-strict)`

---

## Task 15 — the conformance harness: copy oracle tests, adapt conftest, exclusion mechanism, triage protocol

**Why:** operationalizes the gate. (a) copies all 51 oracle files into `tests/ported/` with the import
swap, (b) adapts the conftest, (c) builds the exclusion mechanism, (d) writes the triage protocol.
**No test is expected green yet** — Tasks 16–18 green them. This task's gate: "collection succeeds;
every excluded file is excluded for a written reason; every non-excluded file at least *collects*".

**Files:** `tools/port_tests.sh`, `tests/ported/**`, `tests/conftest.py`,
`tests/conftest_exclusions.py`, `tests/requirements-dev.txt`, `tests/EXCLUSIONS.md`.

- [ ] **Step 1 — the import-swap copier** `tools/port_tests.sh` (run once; output committed):

```bash
#!/usr/bin/env bash
# Port XGI's oracle tests into tests/ported/ with the import swap (spec §9.1).
set -euo pipefail
ORACLE="/tmp/claude-1000/-home-user-projects-game-babylon--claude-worktrees-docs-hypergraph-rs-design/a9bdcfa3-d5c2-4825-afe9-6cfbcdc5df12/scratchpad/xgi-0.10.2/tests"
DEST="tests/ported"
mkdir -p "$DEST"
# D10: copy only test_*.py — NOT the oracle conftest.py. The single hand-authored conftest lives at
# the workspace-root tests/ (Step 2) with the exclusion glue; a copied ported/conftest.py would
# duplicate every fixture at a deeper level AND lack the exclusion glue. Fixtures propagate downward.
find "$ORACLE" -name 'test_*.py' -print0 | while IFS= read -r -d '' f; do   # bounded: fixed file set
  rel="${f#"$ORACLE"/}"
  mkdir -p "$DEST/$(dirname "$rel")"
  sed -E 's/^import xgi$/import hypergraph_rs as xgi/; s/from xgi(\.|[[:space:]])/from hypergraph_rs\1/' \
      "$f" > "$DEST/$rel"
done
# OQ-2 (orchestrator ruling: FETCH). The corpus clone lacks tests/readwrite/data/; fetch it from the
# xgi v0.10.2 git tag so the disk-fixture readwrite tests can run. Sparse checkout (no full clone):
if [ ! -d "$ORACLE/readwrite/data" ]; then
  TMP="$(mktemp -d)"
  git -C "$TMP" init -q
  git -C "$TMP" remote add origin https://github.com/xgi-org/xgi.git
  git -C "$TMP" config core.sparseCheckout true
  echo "tests/readwrite/data/*" > "$TMP/.git/info/sparse-checkout"
  # Non-fatal: if there is no network at execution, fall through to per-file exclusion (below).
  if git -C "$TMP" fetch -q --depth 1 origin refs/tags/v0.10.2 && git -C "$TMP" checkout -q FETCH_HEAD; then
    mkdir -p "$ORACLE/readwrite"
    cp -r "$TMP/tests/readwrite/data" "$ORACLE/readwrite/data"
  else
    echo "WARN: could not fetch readwrite/data from v0.10.2 (OQ-2) — disk-fixture readwrite tests fall to exclusion"
  fi
  rm -rf "$TMP"
fi
[ -d "$ORACLE/readwrite/data" ] && mkdir -p "$DEST/readwrite" && cp -r "$ORACLE/readwrite/data" "$DEST/readwrite/data" || \
  echo "WARN: no readwrite/data available — inline-string/tempfile readwrite tests still pass; disk-fixture ones excluded"
```

> **OQ-2 (RESOLVED — FETCH, orchestrator ruling 2026-07-19):** the corpus clone at
> `…/scratchpad/xgi-0.10.2/tests/` has **no `readwrite/data/` directory** (verified 2026-07-19:
> `find tests -type d` lists no data dir). The copier above fetches `tests/readwrite/data/` from the
> **xgi v0.10.2 git tag** via a shallow sparse checkout (no publishing, read-only fetch). **Only if
> the fetch is impossible at execution** (no network) do the disk-fixture readwrite tests move to the
> exclusion ledger, citing "OQ-2 fetch unavailable"; inline-string + tempfile cases (e.g.
> `test_read_incidence_matrix`) stay green either way. Task 17 resolves per-file.

- [ ] **Step 2 — the adapted conftest.** Copy the oracle conftest with the same import swap (it imports
  `networkx`, `numpy`, `pandas`, `pytest`, `xgi` at top level and defines all fixtures —
  `edgelist1..10`, `dict5`, `incidence5`, `dataframe5`, `bipartite_graph1..4`, `hyperwithattrs`,
  `dihypergraph1`, `simplicialcomplex1`, etc., all cited from the oracle conftest). Because it imports
  `pandas`/`networkx` unconditionally, **those must be in the subrepo venv** for any test to collect —
  install them as test-only deps (they are XGI test dependencies, not runtime deps of `hypergraph-rs`).
  `tests/requirements-dev.txt`:

```
pytest
numpy
scipy
networkx
pandas
matplotlib
```

  Add the exclusion glue below the fixtures in `tests/conftest.py`:

```python
import pytest
from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS

def pytest_ignore_collect(collection_path, config):
    rel = str(collection_path).split("/ported/", 1)[-1]
    return rel in FILE_EXCLUSIONS

def pytest_collection_modifyitems(config, items):
    for item in items:
        rel = str(item.fspath).split("/ported/", 1)[-1]
        for name, reason in TEST_EXCLUSIONS.get(rel, []):
            if item.name.startswith(name):
                item.add_marker(pytest.mark.skip(reason=f"[conformance-exclusion] {reason}"))
```

- [ ] **Step 3 — the exclusion registry** `tests/conftest_exclusions.py`:

```python
"""Central exclusion registry for the XGI conformance gate. Every entry cites a reason.

FILE_EXCLUSIONS — whole files skipped at collection (won't import, or all-deferred).
TEST_EXCLUSIONS — individual tests skipped (a file mostly passes but for specific cases).
"""

FILE_EXCLUSIONS = {
    "convert/test_pandas.py": "pandas integration not ported (spec §4.4).",
    "readwrite/test_xgi_data.py": "network reader load_xgi_data/download_xgi_data gated off (spec §4.3).",
    "readwrite/test_bigg_data.py":
        "network: load_bigg_data is @pytest.mark.webtest/@slow and hits the BiGG server; its skipif "
        "(sys.version_info != (3,12) and not Linux) does NOT skip on the target Linux/py3.13 box (D7).",
    "stats/test_core_stats_functions.py":
        "tests the nodestat_func/dispatch decorator machinery, replaced by idiomatic Rust (spec §4.4).",
    "drawing/test_draw.py":
        "matplotlib @mpl_image_compare pixel tests; replaced by scene-graph tests (spec §9.3).",
    "drawing/test_draw_utils.py":
        "imports private _CCW_sort/_draw_arg_to_arr/_interp_draw_arg (no phase built them); "
        "replaced by scene-graph tests (spec §9.3).",
    "test_import_time.py": "asserts xgi package import-time budget; not a semantic conformance test.",
}

TEST_EXCLUSIONS = {
    "algorithms/test_centrality.py": [
        ("test_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_z_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        # D6: uniform_h_eigenvector_centrality is a DEFERRED symbol (§A); its test exercises the
        # NotImplementedError stub and MUST be skipped or it fails the gate.
        ("test_uniform_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_node_edge_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_line_vector_centrality", "deferred: tensor centrality (Phase 3)."),
        # (test_clique_eigenvector_centrality + test_katz_centrality stay green — those are bound.)
    ],
    "algorithms/test_assortativity.py": [
        # D5: real function name is `test_choose_degrees` (single underscore, oracle line 81) — the
        # earlier `test__choose_degrees` (double underscore) never matched, so the skip never fired.
        ("test_choose_degrees", "imports private _choose_degrees; helper not part of the port."),
    ],
    "algorithms/test_simpliciality.py": [
        # D5: Trie is imported at module scope (needs the utils/trie.py shim from Task 1/D4 to collect);
        # the tests that construct `Trie()` directly exercise the unported data structure → skip them.
        # The bound simpliciality-FUNCTION tests (test_edit_simpliciality, …) stay green.
        ("test_is_simplex", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_potential_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_missing_subfaces", "deferred: utils.trie.Trie not ported (Phase 6)."),
    ],
    # D5: utils/test_utilities.py had PHANTOM entries — `crest_r` lives only in the (file-excluded)
    # drawing/test_draw.py and `Trie` only in test_simpliciality.py; neither test exists in
    # test_utilities.py (its tests are test_iddict/test_dual_dict/test_powerset/test_subfaces/…),
    # so both entries were misdirected no-ops and are deleted.
    "dynamics/test_synchronization.py": [
        ("test_simulate_kuramoto",
         "exact numpy float-trajectory parity not achievable (§9.4); structural shape asserts kept."),
        ("test_simulate_simplicial_kuramoto", "deferred: simplicial Kuramoto (Phase 6)."),
    ],
}
```

- [ ] **Step 4 — the triage protocol** `tests/EXCLUSIONS.md`:

```markdown
# XGI conformance triage protocol

When a ported test is red, classify it into exactly ONE bucket and act:

1. BINDING BUG — the Rust core is correct but the PyO3 layer mistranslates (wrong ID type,
   set-vs-list, missing kwarg, wrong exception class). → Fix the binding (src/*.rs). This is the
   default assumption; ~all reds should land here.

2. CORE BUG / GAP — the core computes a wrong value or lacks a built-but-needed method the binding
   assumed. → STOP. Do not paper over it in the binding. If it is a genuine XGI-semantics divergence in
   a *built* function, fix it in the core crate and regenerate that phase's Rust tests. The one
   pre-sanctioned additive method (`add_empty_edge`) is already landed in **Task 0**; any *other* core
   gap discovered here is NOT pre-approved — escalate rather than silently add core methods.

3. DOCUMENTED EXCLUSION — the test exercises a DEFERRED symbol, a pandas/network/matplotlib-image path,
   a private XGI internal, or an RNG/float-exact assertion unachievable with rand_pcg (§9.4). → Add it
   to FILE_EXCLUSIONS/TEST_EXCLUSIONS with a one-line reason. An exclusion needs one of these reasons:
     (a) deferred symbol (cite DEFERRED_SYMBOLS.md),
     (b) pandas (spec §4.4),
     (c) network (xgi-data spec §4.3; bigg-data @webtest, D7),
     (d) mpl image-compare (spec §9.3),
     (e) RNG/float-trajectory non-determinism (§9.4),
     (f) OQ-2 fetch of tests/readwrite/data/ unavailable at execution (disk-fixture readwrite tests only).
   Never exclude to dodge a fixable binding bug.

TARGET: every non-excluded test passes. The ledger is append-only during Tasks 16–18 and reviewed at
Task 19 — any exclusion lacking an (a)–(e) reason is a triage failure.
```

- [ ] **Run & confirm:**
```bash
bash tools/port_tests.sh
pip install -r tests/requirements-dev.txt
maturin develop
pytest tests/ported --collect-only -q     # must succeed (no collection errors)
```
Expected: collection succeeds; excluded files ignored; non-excluded files collect.

**Commit:** `test(conformance): port 51 XGI oracle files + conftest + exclusion registry + triage protocol`

---

## Task 16 — Green the P0 core tests

**Why:** the P0 tier is the substance of the gate: `core/test_hypergraph.py` (~80),
`test_dihypergraph.py` (~50), `test_views.py` (~30), `test_globalviews.py` (~15),
`test_simplicialcomplex.py` (~40). Iterate: run one file, triage every red per Task 15, fix bindings
(or file core gaps, or add cited exclusions), repeat until green.

**Files:** iterative edits to `hypergraph.rs`/`views.rs`/`dihypergraph.rs`/`simplicial.rs`/`errors.rs`;
append `conftest_exclusions.py` as needed (with reasons).

- [ ] **Method (bounded loop — 5 files, each converges):**
  1. `pytest tests/ported/core/test_hypergraph.py -q`; triage each failure.
  2. Common expected fixes, each cited:
     - `H << H2` (lshift union), `test_lshift` (`test_hypergraph.py:71`): add `__lshift__`
       (union into a copy; XGI `Hypergraph.__lshift__`). Verify Phase 2 `update` covers the semantics;
       if not, synthesize in the binding.
     - `copy(H)`/`deepcopy(H)` (`from copy import copy, deepcopy`, `test_hypergraph.py:3`): implement
       `__copy__`/`__deepcopy__` over core `copy()` + `clone_via_py`.
     - pickle (`import pickle`, `test_hypergraph.py:1`): **OQ-4 — IMPLEMENT (orchestrator ruling
       2026-07-19), not exclude.** XGI supports pickle, so the binding must too. Implementation path:
       give `PyHypergraph` a `__reduce__` returning `(reconstruct_callable, (state_dict,))`, where
       `state_dict` is the binding's own dict conversion — the hyperedge-dict `{edge_id: [members...]}`
       plus node/edge/net attrs — built with the **original Python ids** (the PyKey/PyIdMap layer
       preserves node-id *types*, so ints round-trip as ints through pickle, not as `"1"`). The
       reconstruct callable is a module-level `#[pyfunction]` that rebuilds a fresh `PyHypergraph` from
       `state_dict` via the same ingest path. (`__getstate__`/`__setstate__` are an acceptable
       equivalent if `__reduce__` proves awkward under PyO3.) **Only if a genuine technical wall emerges
       at execution** does `test_pickle` become a cited exclusion — the escape hatch is last resort, not
       the plan.
     - `H.cleanup()`/`H.merge_duplicate_edges()` (Phase 2 built these with the string-arg→enum
       deviation): the binding accepts XGI string kwargs (`rename="tuple"`, `merge_rule="union"`) and
       maps to `MergeRename`/`MergeRule`; `cleanup` cases touching the deferred `connected`/`relabel`
       steps are excluded (cite Phase 2 deferral).
  3. Repeat for `test_dihypergraph.py`, `test_views.py`, `test_globalviews.py`,
     `test_simplicialcomplex.py`.

- [ ] **Run & confirm:** `pytest tests/ported/core -q` → all non-excluded core tests pass; the ledger
  names each skip with a reason.

**Commit:** `test(conformance): P0 core tests green (hypergraph/dihypergraph/views/globalviews/sc)`

---

## Task 17 — Green algorithms + linalg + generators + stats + convert/utils/dynamics/communities/readwrite

**Why:** the P1–P3 module tiers. Same triage loop, module by module, in dependency order.

**Files:** iterative edits to module binding files + exclusion appends.

- [ ] **Order and per-module notes (each cited):**
  1. **`linalg/test_matrix.py`** — verify `(matrix, rowdict, coldict)` index tuples + dense/sparse
     returns; tensor/Hodge tests excluded (deferred).
  2. **`algorithms/*`** — `test_connected`, `test_properties`, `test_clustering`, `test_shortest_path`,
     `test_assortativity` green; `test_centrality` minus tensor cases; `test_simpliciality` green.
  3. **`generators/*`** — apply the **§9.4 split**: structural-property tests pass as-is; exact-membership
     tests (assuming numpy's RNG sequence) are converted to structural assertions OR marked parity-based
     with a header note (option (a) default, option (b) for a small golden set). Document each converted
     test in the file header. Flag-complex/SBM generator tests excluded (deferred).
  4. **`stats/test_nodestats.py` / `test_edgestats.py` / `test_dinodestats.py` / `test_diedgestats.py`**
     — green over the `Stat` proxy; stats the core lacks (e.g. `average_neighbor_degree` — verify) →
     those specific tests excluded (deferred) or the stat added if Phase 4 built it.
  5. **`readwrite/*`** — `test_json`, `test_incidence_matrix` (inline-string cases), `test_edgelist`,
     `test_bipartite_edgelist`, `test_hif`. Disk-fixture cases resolved by OQ-2 (fetch `data/`; see
     Task 15). `test_xgi_data` AND `test_bigg_data` are both FILE-EXCLUDED (network — D7); do not assert
     `test_bigg_data` green offline.
  6. **`convert/*`** — built round-trips green (hyperedge list/dict, bipartite, `to_graph`,
     hypergraph_dict, `cut_to_order`); dispatcher/HIF-dict/encapsulation/line-graph/k-skeleton/pandas
     tests excluded (deferred, cite §A ledger).
  7. **`dynamics/test_synchronization.py`** — shape/`times` asserts kept via a structural variant; exact
     trajectory asserts excluded (§9.4, OQ-3).
  8. **`communities/test_spectral.py`** — spectral clustering structural assertions (cluster sizes /
     valid partition), not exact eigenvector values (§9.4).
  9. **`utils/test_utilities.py`** — `dual_dict`/`powerset`/`subfaces`/`get_network_type`/
     `convert_labels_to_integers` green; `crest_r`/`Trie`/deferred-helper cases excluded.

- [ ] **Run & confirm:**
```bash
pytest tests/ported/linalg tests/ported/algorithms tests/ported/generators tests/ported/stats \
       tests/ported/readwrite tests/ported/convert tests/ported/dynamics tests/ported/communities \
       tests/ported/utils -q
```
Expected: all non-excluded module tests pass; every skip carries a cited reason.

**Commit:** `test(conformance): algorithms/linalg/generators/stats/convert/rw/dynamics/communities/utils green`

---

## Task 18 — Green the drawing/layout tier (scene-graph replacements)

**Why:** `drawing/test_layout.py` (P1, portable — seed-reproducibility asserts) + the scene-graph
replacement tests (spec §9.3). Confirm the `mpl` extra path.

**Files:** `tests/ported/drawing/test_layout.py` (green), the Task 13 scene-graph/mpl-off tests,
exclusion entries for the two mpl-image files.

- [ ] **Run & confirm:**
```bash
pytest tests/ported/drawing -q                       # core (no mpl): layout + scene-graph
pip install 'matplotlib>=3.7'                         # opt into the mpl extra locally
pytest tests/ported/drawing -q                        # the mpl-gated draw smoke now passes
```
Expected: layout + scene-graph pass with and without matplotlib; the two image-compare files stay
excluded (§9.3).

**Commit:** `test(conformance): drawing tier — layout green + scene-graph replacements (mpl extra verified)`

---

## Task 19 — property-differential oracle, CI wiring, release-build smoke, final gate

**Why:** the ADR052-pattern backstop (spec §9.5) catches drift the fixed suite misses; CI wires the
gate; a `maturin build --release` smoke proves the wheel builds (NO publish). Closes the phase.

**Files:** `tests/ported/test_property_differential.py`; extend `hypergraph-rs/.github/workflows/ci.yml`;
`tests/ported/test_gate_summary.py`.

- [ ] **Step 1 — the property-differential test.** Spec §9.5 places it as a Rust proptest running real
  XGI via an embedded interpreter. **Correction:** embedding a Python interpreter in a cargo test
  (`pyo3` `auto-initialize`) is fragile; the cleaner home is **Python** — compare `hypergraph_rs` vs
  real `xgi` (installed as a test-only oracle) over random op sequences. `tests/ported/test_property_differential.py`:

```python
"""ADR052-pattern differential test: random op sequences behave identically in hypergraph_rs
and real xgi (installed as a test-only oracle)."""
import random
import pytest

xgi_real = pytest.importorskip("xgi")   # skip if the oracle isn't installed
import hypergraph_rs as hr


def _apply(H, ops):
    for _, payload in ops:
        try:
            H.add_edge(payload)
        except Exception:
            pass
    return H


# PyKey stress (orchestrator ruling): exercise the id-translation layer over MANY hashable id TYPES,
# not just small ints — this is the load-bearing verification that ints/strings/tuples/floats/bools all
# round-trip by identity through hypergraph_rs exactly as through real xgi.
ID_POOL = [1, 2, 3, 0, -5, "a", "b", "node_x", (1, 2), (3, 4), 3.5, True, "1", 10, 99]


@pytest.mark.parametrize("seed", range(50))     # bounded: fixed 50 sequences (widened from 25)
def test_differential_membership_and_order(seed):
    rng = random.Random(seed)
    ops = [("add_edge", [ID_POOL[i] for i in rng.sample(range(len(ID_POOL)), rng.randint(1, 5))])
           for _ in range(rng.randint(1, 8))]
    a = _apply(hr.Hypergraph(), list(ops))
    b = _apply(xgi_real.Hypergraph(), list(ops))
    assert list(a.nodes) == list(b.nodes)         # insertion order (III.7) AND id-type identity
    assert [type(n) for n in a.nodes] == [type(n) for n in b.nodes]   # e.g. 1 stays int, "1" stays str
    assert a.num_nodes == b.num_nodes and a.num_edges == b.num_edges
    for e in b.edges:
        assert a.edges.members(e) == b.edges.members(e)
```

> Mirrors ADR052's `test_graph_iteration_order.py` differential pattern (the approach that proved
> BabylonGraph's order contract). `importorskip`-guarded so CI without the oracle still runs the rest.
> The mixed `ID_POOL` (incl. the `1` vs `"1"` distinction the `PyKey` hash+repr equality must separate)
> is the primary safety net for the PyId layer — the orchestrator flagged PyKey as the highest-risk
> component, so this differential test is deliberately the load-bearing check, not the unit test.

- [ ] **Step 2 — CI** (`hypergraph-rs/.github/workflows/ci.yml`, extend the §9.7 skeleton — build+test
  only, no publish):

```yaml
  xgi-conformance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: { python-version: "3.13" }
      - run: pip install maturin pytest -r tests/requirements-dev.txt
      - run: pip install "xgi==0.10.2"                 # test-only differential oracle
      - run: maturin develop
      - run: python tools/audit_symbols.py             # regenerate + assert the deferred ledger
      - run: pytest tests/ported -q                    # the 51 ported files, adapted + excluded

  wheel-build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pip install maturin
      - run: maturin build --release --out dist        # build wheel to dist/, do NOT upload
      - run: python -c "import glob; assert glob.glob('dist/*.whl'), 'no wheel built'"
```

> **No `maturin publish`, no `twine`, no PyPI upload** — publishing is an owner decision (hard
> constraint). `dist/` is gitignored.

- [ ] **Step 3 — the final gate test** `tests/ported/test_gate_summary.py`:

```python
def test_no_unexcused_exclusions():
    """Every exclusion must carry one of the sanctioned reasons (triage protocol)."""
    from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS
    sanctioned = ("pandas", "network", "decorator", "image", "private", "deferred",
                  "trajectory", "parity", "colormap", "Trie", "pickle", "scene-graph",
                  "import-time", "fetch")   # "fetch" = OQ-2 disk-fixture reason (f)
    def ok(r): return any(s in r for s in sanctioned)
    for path, reason in FILE_EXCLUSIONS.items():
        assert ok(reason), f"{path}: unexcused exclusion `{reason}`"
    for path, entries in TEST_EXCLUSIONS.items():
        for name, reason in entries:
            assert ok(reason), f"{path}::{name}: unexcused exclusion `{reason}`"
```

- [ ] **Run & confirm (the whole gate):**
```bash
cd /home/user/projects/game/babylon/hypergraph-rs && . .venv/bin/activate
pip install "xgi==0.10.2" -r tests/requirements-dev.txt
maturin develop
python tools/audit_symbols.py
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
pytest tests/ported -q                 # THE conformance gate
maturin build --release --out dist     # wheel smoke (no publish)
```
Expected: every non-excluded ported test passes; the differential test agrees with real xgi; the
deferred ledger is regenerated; a wheel is produced in `dist/`. **The conformance gate is met.**

**Commit:** `test(conformance): property-differential oracle + CI wiring + release wheel smoke — GATE MET`

---

## Phase 7 Completion Summary

After all 20 tasks (**Task 0** — the one additive core method `add_empty_edge` — plus **Tasks 1–19**,
all bindings/shims/ported-tests/packaging), `hypergraph-rs` has a complete PyO3 binding and the
conformance gate is green:

- **`pyproject.toml`** (maturin backend) with the `mpl` optional extra; the core install pulls only
  numpy + scipy (no matplotlib/pandas/networkx).
- **`hypergraph_rs/` package** mirroring `xgi/__init__.py` exactly — `set(__all__)` is the same 164
  symbols; deep-import modules (`exception`, `utils`, `utils.utilities`, `readwrite.json`) resolve.
- **The PyId translation layer** — arbitrary hashable Python IDs ⇄ core `String` keys, type-preserving
  and insertion-ordered (III.7). This is what makes `list(H.nodes) == [1, 2, 3]` (ints) hold.
- **The four deviation shims** owned by the binding (duplicate idx → warn+no-op, empty members → silent
  edge, Vec → set, missing id → `IDNotFound`/`XGIError`), leaving the strict Rust core untouched.
- **Bindings** for every symbol Phases 0–6 built: core Hypergraph/DiHypergraph/SimplicialComplex +
  views + Stat proxies, linalg (numpy/scipy), algorithms (non-tensor), generators (seeded), stats,
  convert/readwrite/utils/dynamics/communities, layout + `render()` scene-graph, and the matplotlib
  drawing shim.
- **`.pyi` stubs** making the surface MyPy-strict-clean (the Babylon `type: ignore` removal, spec §6.5).
- **The Deferred Symbol Ledger** (`DEFERRED_SYMBOLS.md`, **machine-generated by
  `tools/audit_symbols.py`** — the count is emitted by the audit, never hand-estimated; the plan's own
  §A deferred lists sum to ~55–60, so the earlier "~40" figure was an undercount, D12) — the honest
  list of the symbols no phase built, each a stub raising `NotImplementedError`.
- **The conformance gate:** all 51 oracle test files ported to `tests/ported/` with the
  `import hypergraph_rs as xgi` swap; every non-excluded test passes; every exclusion carries one of
  one of the sanctioned reasons (a–f: deferred / pandas / network / mpl-image / private-internal /
  RNG-float-parity / OQ-2-fetch-unavailable), gate-checked by `test_gate_summary.py`;
  the ADR052-pattern property-differential test agrees with real xgi.
- **No publishing** — wheels build to `dist/`, `maturin develop` for local install; PyPI upload is an
  owner decision.

**Declared deviations & resolved rulings (orchestrator, 2026-07-19 — all open questions now closed):**
- **OQ-1 — RESOLVED (implement):** ONE additive, non-breaking core method `add_empty_edge(idx, attrs)
  -> String` that *returns* the assigned id (no separate `next_auto_edge_id`, per D9). Landed TDD in
  **Task 0**; the strict `add_edge` is untouched.
- **OQ-2 — RESOLVED (fetch):** `tests/readwrite/data/` is fetched from the xgi v0.10.2 git tag (shallow
  sparse checkout, read-only) by `tools/port_tests.sh`; only a genuine no-network execution demotes the
  disk-fixture readwrite tests to a cited exclusion (inline-string/tempfile cases stay green regardless).
- **OQ-3 — RESOLVED (keep the split):** `test_synchronization.py` exact-trajectory asserts (~1e-8) are
  excluded because libm transcendentals (`sin`/`exp`) are not a bit-reproducible cross-language contract
  (float policy, §9.4); structural shape/`times` asserts are kept.
- **OQ-4 — RESOLVED (implement):** pickle round-trip is implemented via `__reduce__` (reconstruct
  callable + the binding's own id-preserving dict state), so node-id *types* survive pickling; exclusion
  is last-resort only if a technical wall appears at execution (Task 16).
- **Deviation — attribute values must be JSON-representable:** XGI stores arbitrary Python objects as
  attrs; the serde bridge (Task 2) raises `TypeError` for non-JSON-serializable attr values. Declared.
- **Deviation — the four baked-in core deviations are shimmed in the binding** (Task 4): duplicate idx →
  warn+no-op, empty members → silent empty edge (via Task 0), `Vec`→`set`, missing id → `IDNotFound`.
- **Spec vs code — dynamics = Kuramoto, not SIR/SIS:** spec §9.2/§10.3 mislabel dynamics as
  "SIR/SIS epidemics"; xgi 0.10.2 `dynamics/` is Kuramoto synchronization (Phase 6 built Kuramoto). The
  plan follows the code; the stale spec label is noted, not obeyed.
- **No publishing (hard constraint):** wheels build to `dist/` (`maturin build --release`) and
  `maturin develop` installs locally; there is **no** `maturin publish` / `twine` / PyPI upload /
  `cargo publish` / `gh release` anywhere — publishing is an owner decision.

**Next: Phase 8** — WASM bindings (`wasm-pack` must be installed first: prefer `npm i -g wasm-pack`
over `cargo install`; verify `wasm-pack --version`).
