# XGI Honest Gate (T0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the XGI conformance gate reproducible and make its number mean what it says, so the parity work that follows can be measured at all.

**Architecture:** The exclusion registry moves from a prefix-matched Python dict to an exact-matched TOML file with a closed class taxonomy. Anything that *runs and fails* becomes a strict xfail rather than a skip, so a gap that gets fixed breaks the build instead of rotting silently. A generator introspects the built extension and emits `manifest.json`, which becomes the single artifact pytest, CI, and (in T1) the shipped `xgi.compat` all read.

**Tech Stack:** Python 3.13, pytest, maturin/PyO3, Rust 1.91.1, TOML (`tomllib`, stdlib), GitHub Actions, mise.

**Source spec:** `docs/superpowers/specs/2026-07-30-xgi-drop-in-parity-design.md` (commit `8ba11b1`). Findings referenced as F1–F6 are that document's §1.1.

**Scope:** This plan is T0 only (spec §8). T1 (loud gaps: shims, `AttrProxy`, `DivergenceWarning`, `xgi.compat`) and T2 (cheap tier) get their own plans. T1 cannot be measured before T0 lands.

## Global Constraints

- **No publishing, ever.** No PyPI, cargo, npm, twine, `gh release`, or `wasm-pack publish`. Build, `--dry-run`, and smoke only. Publishing is an owner decision (`plans/EXECUTION-STATE.md`).
- **Never touch babylon's `.venv`.** This repo's own `.venv` holds the read-only xgi 0.10.2 oracle.
- **Cargo single-flight.** One build at a time; never run concurrent `cargo`/`pytest` jobs.
- **Determinism (III.7).** No `HashMap` in any public iteration path; insertion order via `IndexMap`.
- **Power-of-10 rule 2.** Every loop needs a statically provable bound.
- **Commits:** plain `git commit --no-verify`, conventional messages. Commit at each task end.
- **Do not modify any file under `tests/ported/` that has an upstream counterpart** in `.venv/lib/python3.13/site-packages/tests/`. Those are verbatim copies of XGI 0.10.2's vendored suite modulo imports (spec F6) and their integrity is the oracle's whole value. All reconciliation with them happens in the registry, never in the test text.

  **Sixteen files under `tests/ported/` are repo-authored and ARE editable** — they have no upstream counterpart and are not part of the oracle:

  ```
  test_generators_smoke.py     test_algorithms_smoke.py    test_linalg_smoke.py
  test_simplicial_smoke.py     test_dihypergraph_smoke.py  test_stubs_present.py
  test_exception_hierarchy.py  test_views_smoke.py         test_stats_smoke.py
  test_pyhypergraph_smoke.py   test_property_differential.py
  test_package_imports.py      test_gate_summary.py        test_convert_utils_smoke.py
  drawing/test_scene_graph.py  drawing/test_mpl_optional.py
  ```

  Verify membership before editing anything in that tree:
  `ls .venv/lib/python3.13/site-packages/tests/<relpath>` — if it exists, the file is oracle and is off-limits.
- **Python 3.13**, pytest invoked as `.venv/bin/python -m pytest`.

---

## File Structure

**Create:**

| Path | Responsibility |
|---|---|
| `tests/test_registry_integrity.py` | Guards the registry itself: exact-match, no orphans, no phantom entries. Lives outside `tests/ported/` so the exclusion glue never touches it. |
| `compat/exclusions.toml` | Authored registry. Exact keys, one `class` per entry from the closed taxonomy. |
| `compat/divergences.toml` | Authored D-register. **T0 creates it with the schema and a header comment only**; T1 populates it from `tests/conformance/main.rs`. |
| `tools/gen_manifest.py` | Introspects the built extension + reads the TOMLs → `manifest.json`. |
| `python/hypergraph_rs/compat/manifest.json` | Generated artifact. Placed under the package now so T1's `xgi.compat` needs no move. |
| `tests/test_manifest_drift.py` | Regenerates the manifest and asserts it matches the committed copy. |

**Modify:**

| Path | Change |
|---|---|
| `tests/conftest.py:344-359` | Exclusion glue: exact match, class-qualified entries, skip-vs-xfail by class. |
| `tests/conftest_exclusions.py` | Becomes a thin reader over `compat/exclusions.toml`. Keeps its module-level `FILE_EXCLUSIONS`/`TEST_EXCLUSIONS` names so nothing else breaks. |
| `tests/ported/drawing/test_scene_graph.py:19` | Replace the hash-order assertion with an order-independent one. |
| `.github/workflows/ci.yml:72-84` | Add hash-seed matrix, manifest drift step, and honest-score report. |
| `.mise.toml` | Add `py:manifest` and `py:gate` tasks so CI runs what devs run. |

---

## The class taxonomy

Every exclusion entry carries exactly one `class`. This drives the skip-vs-xfail decision in Task 5 and the score breakdown in Task 8.

| Class | Meaning | Disposition |
|---|---|---|
| `deferred-symbol` | Calls a `make_deferred` stub | `xfail(strict=True)` |
| `core-gap` | Needs Rust core work that does not exist | `xfail(strict=True)` |
| `binding-gap` | Fixable in the PyO3 layer | `xfail(strict=True)` |
| `divergence` | Deliberate, D-numbered behavior difference | `xfail(strict=True)` |
| `float-parity` | ULP / trajectory divergence | `xfail(strict=True)` |
| `upstream-private` | Test touches XGI internals not part of the port | `xfail(strict=True)` |
| `env` | Cannot run here: network, pandas, matplotlib, import error | `skip` |
| `upstream-bug` | Upstream's own inline skip | `skip` |
| `out-of-scope` | Runs, but its result says nothing about the port | `skip` |

**Rule:** `env`, `upstream-bug`, and `out-of-scope` skip. Everything else xfails strictly.

> **`out-of-scope` added during Task 4 adjudication.** `test_import_time.py` calls
> `subprocess.run([sys.executable, "-X", "importtime", "-c", "import xgi"])`. The
> import target sits inside a *string literal*, so the port's mechanical
> `import xgi` → `import hypergraph_rs as xgi` swap never reached it, and the file
> has an upstream counterpart (it is a verbatim XGI port). The test therefore
> measures **real xgi's** import time — and real xgi is installed as the
> differential oracle, so it would likely pass while measuring the wrong library
> entirely.
>
> That is neither `env` (nothing blocks it from running) nor `upstream-bug`
> (upstream's test is fine; the porting swap simply cannot reach into a string).
> It must not be `xfail(strict)`: it would XPASS and break the build. Forcing it
> into `env` would overstate environmental blockage and bury the more interesting
> fact. Disposition is `skip` because running it is meaningless, not because it
> is impossible.

---

### Task 1: Registry integrity tests (RED)

Proves F1 with executing tests before touching anything.

**Files:**
- Create: `tests/test_registry_integrity.py`

**Interfaces:**
- Consumes: `tests/conftest_exclusions.py`'s `FILE_EXCLUSIONS: dict[str, str]` and `TEST_EXCLUSIONS: dict[str, list[tuple[str, str]]]`.
- Produces: `_test_names(path) -> list[str]` and `_qualified_test_names(path) -> list[str]`, reused by Tasks 2 and 6.

- [ ] **Step 1: Write the failing tests**

```python
# tests/test_registry_integrity.py
"""Integrity tests for the conformance exclusion registry (spec F1).

These guard the REGISTRY, not the port. They deliberately live outside
tests/ported/ so the exclusion glue in conftest.py never rewrites them.
"""

import ast
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS  # noqa: E402

PORTED = pathlib.Path(__file__).parent / "ported"


def _test_names(path: pathlib.Path) -> list[str]:
    """Every bare test function/method name defined in `path`, via AST."""
    tree = ast.parse(path.read_text())
    out: list[str] = []
    # Bounded: one pass over the module's AST nodes.
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                out.append(node.name)
    return out


def _qualified_test_names(path: pathlib.Path) -> list[str]:
    """Test names as the registry must key them: `Class::method` or `func`."""
    tree = ast.parse(path.read_text())
    out: list[str] = []
    # Bounded: module body, then one class body per class.
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                out.append(node.name)
        elif isinstance(node, ast.ClassDef):
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if sub.name.startswith("test_"):
                        out.append(f"{node.name}::{sub.name}")
    return out


def test_no_exclusion_entry_claims_a_test_it_does_not_name():
    """F1: `startswith` matching lets one entry sweep up its prefix-siblings."""
    offenders: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            continue
        names = _test_names(path)
        owned = {entry for entry, _ in entries}
        for entry, _ in entries:
            for name in names:
                if name != entry and name.startswith(entry) and name not in owned:
                    offenders.append(f"{rel}::{name}  <- swept up by entry {entry!r}")
    assert offenders == [], (
        "registry entries claim tests they do not name:\n  " + "\n  ".join(offenders)
    )


def test_every_exclusion_entry_names_a_real_test():
    """A phantom entry is a silent no-op; the D5 note records a past instance."""
    phantom: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            phantom.append(f"{rel}  (file does not exist)")
            continue
        known = set(_test_names(path)) | set(_qualified_test_names(path))
        for entry, _ in entries:
            if entry not in known:
                phantom.append(f"{rel}::{entry}")
    assert phantom == [], (
        "registry names tests that do not exist:\n  " + "\n  ".join(phantom)
    )


def test_class_scoped_tests_are_qualified_in_the_registry():
    """A bare entry matching a method in 2+ classes is ambiguous (test_spectral.py)."""
    ambiguous: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            continue
        qualified = _qualified_test_names(path)
        for entry, _ in entries:
            if "::" in entry:
                continue
            hits = [q for q in qualified if q.endswith(f"::{entry}")]
            if hits:
                ambiguous.append(f"{rel}::{entry} is class-scoped; matches {hits}")
    assert ambiguous == [], (
        "bare registry entries target class-scoped tests:\n  " + "\n  ".join(ambiguous)
    )


def test_every_file_exclusion_names_a_real_file():
    missing = [rel for rel in FILE_EXCLUSIONS if not (PORTED / rel).exists()]
    assert missing == [], f"FILE_EXCLUSIONS names files that do not exist: {missing}"
```

- [ ] **Step 2: Run to verify the expected failures**

Run: `.venv/bin/python -m pytest tests/test_registry_integrity.py -v`

Expected: `test_no_exclusion_entry_claims_a_test_it_does_not_name` **FAILS**, listing exactly these four offenders:

```
readwrite/test_hif.py::test_write_hif_compressed  <- swept up by entry 'test_write_hif'
linalg/test_matrix.py::test_empty_order  <- swept up by entry 'test_empty'
core/test_simplicialcomplex.py::test_add_simplices_from_iterable_of_members  <- swept up by entry 'test_add_simplices_from'
core/test_simplicialcomplex.py::test_add_simplices_from_wrong_format  <- swept up by entry 'test_add_simplices_from'
```

`test_class_scoped_tests_are_qualified_in_the_registry` **FAILS**, naming `communities/test_spectral.py` entries. The other two are expected to pass — they guard against regressions rather than exposing current defects. If either fails, record the output; it is a finding beyond F1 and must be reported before continuing.

- [ ] **Step 3: Commit the red tests**

```bash
git add tests/test_registry_integrity.py
git commit --no-verify -m "test(conformance): registry integrity tests — RED, proves F1

Four tests guarding the exclusion registry itself. Two fail today:
prefix-matched entries claim four tests they do not name, and
test_spectral.py's class-scoped entries are keyed by bare method name.
Fixed in the next task."
```

---

### Task 2: Exact-match glue and the four orphan entries

**Files:**
- Modify: `tests/conftest.py:344-359`
- Modify: `tests/conftest_exclusions.py` (add 4 entries, qualify the spectral entries)

**Interfaces:**
- Consumes: `_test_names`/`_qualified_test_names` from Task 1.
- Produces: `entry_matches_name(name, entry) -> bool` in `conftest_exclusions.py` — the single matching implementation, imported by both `conftest.py` and `tests/test_registry_integrity.py`. And `_entry_matches(item, entry) -> bool` in `conftest.py`, a thin pytest-item adapter over it, reused by Task 5.

> **Plan amendment (owner ruling, after Task 1 review).** Task 1's
> `test_no_exclusion_entry_claims_a_test_it_does_not_name` asserts a property of
> the registry's *text* — no test name may extend an entry's prefix. That is the
> wrong target: F1 lives in the **matcher**, not the registry. Once exact matching
> lands, a prefix relationship is harmless, but the textual test forbids it
> forever — so Task 5's deletion of the four orphan entries would turn it red
> again, invisibly (Task 5 runs only `pytest tests/ported`, which never collects
> the integrity file). This task therefore **replaces** that test, and
> `test_class_scoped_tests_are_qualified_in_the_registry`, with one test that
> asserts the real property through the matcher. See Step 5.

- [ ] **Step 1: Add the shared matcher to `tests/conftest_exclusions.py`**

Append to `tests/conftest_exclusions.py`. This is the single implementation of "does this collected test match this registry entry" — `conftest.py` and the integrity tests both delegate to it, so the rule cannot drift between enforcement and verification.

```python
def entry_matches_name(name: str, entry: str) -> bool:
    """Does collected-test `name` match registry `entry`?

    Both are either `"test_func"` or `"ClassName::test_method"`. Matching is
    EXACT, with one tolerance: pytest appends a parametrization suffix
    (`test_foo[1-2]`), and `[` cannot occur in a Python identifier, so
    `name.startswith(entry + "[")` cannot reach a sibling test the way a bare
    `startswith` does. A bare entry never matches a class-scoped test and vice
    versa, so an entry can never claim a test it does not name (spec F1).
    """
    if ("::" in entry) != ("::" in name):
        return False
    if "::" in entry:
        entry_cls, entry_fn = entry.split("::", 1)
        name_cls, name_fn = name.split("::", 1)
        if entry_cls != name_cls:
            return False
    else:
        entry_fn, name_fn = entry, name
    return name_fn == entry_fn or name_fn.startswith(entry_fn + "[")
```

- [ ] **Step 2: Delegate from `tests/conftest.py`**

Replace lines 354-359 (the `pytest_collection_modifyitems` body) with:

```python
def _entry_matches(item, entry: str) -> bool:
    """Pytest-item adapter over `entry_matches_name` (the single matcher).

    Derives the item's registry-shaped name — `Class::method` for a class-scoped
    test, the bare name otherwise — then defers. All matching semantics live in
    `conftest_exclusions.entry_matches_name` so enforcement here and verification
    in tests/test_registry_integrity.py can never drift apart.
    """
    cls = getattr(item, "cls", None)
    name = f"{cls.__name__}::{item.name}" if cls is not None else item.name
    return entry_matches_name(name, entry)


def pytest_collection_modifyitems(config, items):
    for item in items:
        rel = str(item.fspath).split("/ported/", 1)[-1]
        for name, reason in TEST_EXCLUSIONS.get(rel, []):
            if _entry_matches(item, name):
                item.add_marker(
                    pytest.mark.skip(reason=f"[conformance-exclusion] {reason}")
                )
```

Also update the import at `tests/conftest.py:346` so the delegate resolves:

```python
from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS, entry_matches_name
```

- [ ] **Step 3: Discover the exact class-qualified names to write**

Run:

```bash
.venv/bin/python -c "
import ast, pathlib
p = pathlib.Path('tests/ported/communities/test_spectral.py')
t = ast.parse(p.read_text())
for n in t.body:
    if isinstance(n, ast.ClassDef):
        for s in n.body:
            if isinstance(s, ast.FunctionDef) and s.name.startswith('test_'):
                print(f'{n.name}::{s.name}')
"
```

Record the output. Every `communities/test_spectral.py` entry in `TEST_EXCLUSIONS` must be rewritten to the matching `Class::method` string from this list. The `_kmeans` reason applies **only** to the `TestKMeans` entries — the `TestSpectralClustering` method of the same name carries its own inline skip and must not be given the `_kmeans` reason (this is the reason-mismatch the review found).

- [ ] **Step 4: Add the four orphan entries with their true reasons**

In `tests/conftest_exclusions.py`, add to the existing lists:

```python
    "readwrite/test_hif.py": [
        # ... existing entries unchanged ...
        ("test_write_hif_compressed",
         "env: gzip-compressed HIF write/read of a plain Hypergraph is IMPLEMENTED and passes; "
         "this entry exists only to record that it was previously skipped by the prefix-match "
         "bug (spec F1). REMOVE this entry in Task 5 — it must become a passing test, not an "
         "xfail."),
    ],
    "linalg/test_matrix.py": [
        # ... existing entries unchanged ...
        ("test_empty_order",
         "env: passes; previously swept up by the `test_empty` prefix entry (spec F1). "
         "REMOVE in Task 5."),
    ],
    "core/test_simplicialcomplex.py": [
        # ... existing entries unchanged ...
        ("test_add_simplices_from_iterable_of_members",
         "env: passes; previously swept up by the `test_add_simplices_from` prefix entry "
         "(spec F1). REMOVE in Task 5."),
        ("test_add_simplices_from_wrong_format",
         "env: passes; previously swept up by the `test_add_simplices_from` prefix entry "
         "(spec F1). REMOVE in Task 5."),
    ],
```

These four entries are **deliberately temporary**. They keep the registry honest about what is currently skipped while Task 1's tests go green; Task 5 deletes them so the tests run and count as passes. This two-step exists so that exactly one behavior changes per task.

- [ ] **Step 5: Retarget the integrity tests at the matcher**

In `tests/test_registry_integrity.py`, **delete** both `test_no_exclusion_entry_claims_a_test_it_does_not_name` and `test_class_scoped_tests_are_qualified_in_the_registry`, and add the two below. Keep `test_every_exclusion_entry_names_a_real_test` and `test_every_file_exclusion_names_a_real_file` unchanged. Add `entry_matches_name` to the existing import from `conftest_exclusions`.

The first is a direct unit test of the F1 defect — it is what fails if anyone reintroduces prefix matching. The second replaces both deleted tests: 0 hits catches an unqualified class-scoped entry (what the old test 3 caught), 2+ hits catches an entry claiming siblings (what the old test 1 caught).

```python
def test_entry_matches_name_rejects_prefix_siblings() -> None:
    """The F1 regression guard. A bare `startswith` fails every `not` below."""
    assert entry_matches_name("test_write_hif", "test_write_hif")
    assert not entry_matches_name("test_write_hif_compressed", "test_write_hif")
    assert not entry_matches_name("test_empty_order", "test_empty")
    # pytest's parametrization suffix must still match; `[` cannot occur in an
    # identifier, so this tolerance cannot reach a sibling test.
    assert entry_matches_name("test_foo[1-2]", "test_foo")
    # Class scoping is symmetric: neither shape matches the other.
    assert entry_matches_name("TestA::test_x", "TestA::test_x")
    assert not entry_matches_name("TestB::test_x", "TestA::test_x")
    assert not entry_matches_name("TestA::test_x", "test_x")
    assert not entry_matches_name("test_x", "TestA::test_x")


def test_each_entry_matches_exactly_one_test() -> None:
    """Every registry entry must claim exactly the one test it names.

    Supersedes the earlier prefix-text assertions. Those asked whether the
    registry's TEXT contained prefix relationships, which exact matching renders
    harmless — and which Task 5 deliberately restores. This asks the real
    question, through the matcher itself, so it survives Task 5 unchanged.
    """
    problems: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            problems.append(f"{rel}  (file does not exist)")
            continue
        names = set(_qualified_test_names(path))
        # Bounded: entries for this file, times names in this file.
        for entry, _ in entries:
            hits = sorted(n for n in names if entry_matches_name(n, entry))
            if len(hits) != 1:
                problems.append(f"{rel}::{entry} matches {len(hits)}: {hits}")
    assert problems == [], (
        "registry entries that do not claim exactly one test:\n  "
        + "\n  ".join(problems)
    )
```

- [ ] **Step 5b: Extend the gate-summary whitelist**

`tests/ported/test_gate_summary.py::test_no_unexcused_exclusions` (repo-authored, editable per Global Constraints) asserts every exclusion reason contains one of a hand-maintained keyword list. That list predates this plan and does not know the class taxonomy, so the four `env:` reasons added in Step 4 fail it.

Add the eight taxonomy class names to its `sanctioned` tuple, leaving the existing keywords in place:

```python
    sanctioned = ("pandas", "network", "decorator", "image", "private", "deferred",
                  "trajectory", "parity", "colormap", "Trie", "pickle", "scene-graph",
                  "import-time", "fetch",   # "fetch" = OQ-2 disk-fixture reason (f)
                  # Task 4's class taxonomy — this whitelist is superseded by
                  # taxonomy validation in Task 4 Step 4b, which deletes it.
                  "env", "core-gap", "binding-gap", "divergence",
                  "float-parity", "upstream-private", "deferred-symbol",
                  "upstream-bug")
```

- [ ] **Step 6: Run the integrity tests**

Run: `.venv/bin/python -m pytest tests/test_registry_integrity.py -v`
Expected: all four PASS (two carried over from Task 1, two new).

If `test_each_entry_matches_exactly_one_test` reports an entry matching 0 tests, a class-scoped entry was not qualified in Step 3/4 — fix the entry, not the test.

- [ ] **Step 7: Run the ported suite and confirm the count is unchanged**

Run: `.venv/bin/python -m pytest tests/ported -q`
Expected: `311 passed, 83 skipped` — identical to before. Exact matching plus four explicit entries reproduces the old prefix behavior exactly. Any other number means the matcher changed semantics and must be investigated before committing.

- [ ] **Step 8: Commit**

```bash
git add tests/conftest.py tests/conftest_exclusions.py tests/test_registry_integrity.py \
        tests/ported/test_gate_summary.py
git commit --no-verify -m "fix(conformance): exact-match exclusion entries, close F1

Replaces item.name.startswith(name) with exact matching that tolerates
pytest parametrization and requires class-scoped entries to be written
Class::method. Adds explicit entries for the four tests the prefix match
was silently claiming, and corrects the reason-mismatch on
test_spectral.py's class-scoped entries.

Suite count unchanged at 311/83 — this task changes which entry claims
which test, not what runs."
```

---

### Task 3: Scene-graph determinism and the hash-seed CI leg

Closes F2. The general fix is the CI leg; the test edit is the specific one.

**Files:**
- Modify: `tests/ported/drawing/test_scene_graph.py:16-19`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace the hash-order assertion**

In `tests/ported/drawing/test_scene_graph.py`, replace lines 16-19 with:

```python
    # `add_edge` passes members through a Python `set` before storing, faithfully
    # reproducing real XGI (`members = set(members)` in hypergraph.py; mirrored at
    # crates/hypergraph-rs-python/src/hypergraph.rs:1291-1294). Member ORDER is
    # therefore CPython hash-set iteration order, which varies per process with
    # PYTHONHASHSEED. Asserting a specific order pins one sample of XGI's own
    # nondeterminism and fails ~11 runs in 12 (spec F2). The membership SET is
    # the actual contract.
    assert set(scene["hyperedges"][0]["member_ids"]) == {"a", "b", "c"}
    assert len(scene["hyperedges"][0]["member_ids"]) == 3  # no duplicates
```

- [ ] **Step 2: Verify it is now seed-independent**

Run:

```bash
for i in 1 2 3 4 5 6 7 8; do
  PYTHONHASHSEED=$i .venv/bin/python -m pytest \
    tests/ported/drawing/test_scene_graph.py -q 2>&1 | tail -1
done
```

Expected: eight identical `4 passed` lines. Before this change the same loop produced a mix of pass and fail.

- [ ] **Step 3: Add the hash-seed matrix to CI**

In `.github/workflows/ci.yml`, replace the `xgi-conformance` job's `strategy`-less definition (lines 72-84) so the job runs under several seeds:

```yaml
  xgi-conformance:
    name: "Python bindings — XGI conformance gate (hashseed ${{ matrix.hashseed }})"
    runs-on: ubuntu-latest
    timeout-minutes: 25
    strategy:
      fail-fast: false
      matrix:
        # A test that pins CPython hash-set iteration order passes under one
        # seed and fails under others (spec F2). Several fixed seeds plus one
        # random draw catch that class of defect; `0` disables randomization
        # entirely, which is the reproducible baseline.
        hashseed: ["0", "1", "42", "random"]
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-python@v5
        with: { python-version: "3.13" }
      - run: pip install maturin pytest -r tests/requirements-dev.txt
      - run: pip install "xgi==0.10.2"                 # test-only differential oracle
      - run: maturin develop
      - run: python tools/audit_symbols.py             # regenerate + assert the deferred ledger
      - run: pytest tests/ported -q
        env:
          PYTHONHASHSEED: ${{ matrix.hashseed }}
```

- [ ] **Step 4: Run the full suite under two seeds locally**

Run:

```bash
PYTHONHASHSEED=0 .venv/bin/python -m pytest tests/ported -q 2>&1 | tail -1
PYTHONHASHSEED=12345 .venv/bin/python -m pytest tests/ported -q 2>&1 | tail -1
```

Expected: both report `311 passed, 83 skipped`. If any *other* test proves seed-dependent, stop and report it — that is a new finding of the same class as F2 and needs its own entry, not a silent fix.

- [ ] **Step 5: Commit**

```bash
git add tests/ported/drawing/test_scene_graph.py .github/workflows/ci.yml
git commit --no-verify -m "fix(conformance): make scene-graph test seed-independent; add hashseed matrix

test_render_produces_valid_scene_graph asserted member_ids == ['c','a','b'],
one sample of CPython hash-set iteration order. Measured 1 pass in 12 runs,
yet it sat in the passing column — the previously reported green gate was a
lucky roll (spec F2). The binding is faithful here; it deliberately mirrors
XGI's `members = set(members)`. The membership SET is the real contract.

The general fix is the CI matrix: the conformance gate now runs under
PYTHONHASHSEED 0/1/42/random, so any test pinning hash order fails loudly
instead of intermittently.

Sole exception to the do-not-touch-tests/ported rule, justified because the
assertion pins nondeterminism rather than XGI behavior."
```

---

### Task 4: Move the registry to TOML with the class taxonomy

**Files:**
- Create: `compat/exclusions.toml`
- Create: `compat/divergences.toml` (schema + header only)
- Create: `tools/migrate_exclusions.py`
- Modify: `tests/conftest_exclusions.py` (becomes a reader)

**Interfaces:**
- Produces: `compat/exclusions.toml` with the schema below; `conftest_exclusions.py` keeps exporting `FILE_EXCLUSIONS` and `TEST_EXCLUSIONS` with unchanged types, plus a new `ENTRY_CLASSES: dict[tuple[str, str], str]` keyed by `(relpath, entry)`.

- [ ] **Step 1: Write the migration tool**

```python
# tools/migrate_exclusions.py
"""One-shot: convert tests/conftest_exclusions.py to compat/exclusions.toml.

Classification is deterministic from the existing reason text — the reasons
were written with consistent vocabulary. Every entry the rules cannot classify
is emitted with class="UNCLASSIFIED" and must be resolved by hand before the
TOML is committed; the tool prints them at the end and exits non-zero.
"""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tests"))

from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS  # noqa: E402

# Ordered most-specific first; the first matching rule wins.
RULES: list[tuple[str, str]] = [
    # Task 2's four temporary entries open with "env:". Without this rule they
    # migrate as UNCLASSIFIED, and a hand-resolution slip would flip them from
    # `skip` to strict `xfail` under Task 5 — where they would XPASS, since
    # those four tests actually pass. (Task 2 review, minor.)
    ("env:", "env"),
    ("network:", "env"),
    ("network reader", "env"),
    ("pandas", "env"),
    ("matplotlib", "env"),
    ("float non-determinism", "float-parity"),
    ("float-trajectory", "float-parity"),
    ("not achievable", "float-parity"),
    ("core gap", "core-gap"),
    ("core-gap", "core-gap"),
    ("no core primitive", "core-gap"),
    ("core deviation", "divergence"),
    ("documented deviation", "divergence"),
    ("representational deviation", "divergence"),
    ("binding gap", "binding-gap"),
    ("binding-gap", "binding-gap"),
    ("private", "upstream-private"),
    ("not part of the port", "upstream-private"),
    ("decorator machinery", "binding-gap"),
    ("deferred:", "deferred-symbol"),
    ("Inconsistent seeding", "upstream-bug"),
]


def classify(reason: str) -> str:
    # Bounded: one pass over the fixed RULES table.
    for needle, cls in RULES:
        if needle.lower() in reason.lower():
            return cls
    return "UNCLASSIFIED"


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")


def main() -> int:
    out: list[str] = [
        "# Conformance exclusion registry (spec §4 C2).",
        "# Keys are EXACT test names: `test_name` or `ClassName::test_name`.",
        "# Every entry carries one `class` from the closed taxonomy:",
        "#   deferred-symbol | core-gap | binding-gap | divergence",
        "#   float-parity | upstream-private | env | upstream-bug",
        "# `env` and `upstream-bug` produce skip; everything else xfail(strict).",
        "",
    ]
    unclassified: list[str] = []

    for rel, reason in FILE_EXCLUSIONS.items():
        cls = classify(reason)
        if cls == "UNCLASSIFIED":
            unclassified.append(f"FILE {rel}")
        out += [
            "[[file_exclusion]]",
            f'path = "{esc(rel)}"',
            f'class = "{cls}"',
            f'reason = "{esc(reason)}"',
            "",
        ]

    for rel, entries in TEST_EXCLUSIONS.items():
        for entry, reason in entries:
            cls = classify(reason)
            if cls == "UNCLASSIFIED":
                unclassified.append(f"TEST {rel}::{entry}")
            out += [
                "[[test_exclusion]]",
                f'path = "{esc(rel)}"',
                f'test = "{esc(entry)}"',
                f'class = "{cls}"',
                f'reason = "{esc(reason)}"',
                "",
            ]

    dest = pathlib.Path("compat/exclusions.toml")
    dest.parent.mkdir(exist_ok=True)
    dest.write_text("\n".join(out))
    print(f"wrote {dest}")

    if unclassified:
        print("\nUNCLASSIFIED — resolve by hand before committing:")
        # Bounded: one pass over the collected list.
        for u in unclassified:
            print("  ", u)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Run it and resolve every UNCLASSIFIED entry by hand**

Run: `.venv/bin/python tools/migrate_exclusions.py`

If it exits non-zero, open `compat/exclusions.toml` and set each `class = "UNCLASSIFIED"` to the correct value using the taxonomy table above, reading that entry's `reason` to decide. Re-run until it exits 0 **without** re-running the generator over your hand edits — once resolved, the TOML is the source of truth and `tools/migrate_exclusions.py` is never run again.

- [ ] **Step 3: Create the divergence register stub**

```toml
# compat/divergences.toml
#
# The D-register (D1-D17), promoted from tests/conformance/main.rs so it can be
# shipped and queried rather than living only in Rust test code (spec §4 C1).
#
# T0 creates this file with its schema fixed so `manifest.json` never changes
# shape later. T1 populates it and wires DivergenceWarning to these entries.
#
# Schema, one table per divergence:
#
#   [[divergence]]
#   id     = "D3"                       # the register number
#   api    = "Hypergraph.add_edge"      # the affected public surface
#   ours   = "..."                      # what hypergraph-rs does
#   xgi    = "..."                      # what XGI 0.10.2 does
#   class  = "divergence"               # taxonomy class
#   reason = "..."                      # why the difference exists
#
# Intentionally empty in T0.
```

- [ ] **Step 4: Turn `conftest_exclusions.py` into a reader**

Replace the entire file with:

```python
"""Reader over compat/exclusions.toml — the single authored registry (spec §4 C2).

Re-exports FILE_EXCLUSIONS / TEST_EXCLUSIONS with their original types so
existing importers keep working, and adds ENTRY_CLASSES for the skip-vs-xfail
decision in conftest.py.
"""

import pathlib
import tomllib

_TOML = pathlib.Path(__file__).parent.parent / "compat" / "exclusions.toml"

with _TOML.open("rb") as fh:
    _DATA = tomllib.load(fh)

FILE_EXCLUSIONS: dict[str, str] = {
    e["path"]: e["reason"] for e in _DATA.get("file_exclusion", [])
}

FILE_CLASSES: dict[str, str] = {
    e["path"]: e["class"] for e in _DATA.get("file_exclusion", [])
}

TEST_EXCLUSIONS: dict[str, list[tuple[str, str]]] = {}
ENTRY_CLASSES: dict[tuple[str, str], str] = {}

# Bounded: one pass over the parsed TOML array.
for _e in _DATA.get("test_exclusion", []):
    TEST_EXCLUSIONS.setdefault(_e["path"], []).append((_e["test"], _e["reason"]))
    ENTRY_CLASSES[(_e["path"], _e["test"])] = _e["class"]
```

**Carry `entry_matches_name` across unchanged.** Task 2 added it to this file, and both `conftest.py` and `tests/test_registry_integrity.py` import it. Copy the function verbatim into the rewritten file — do not reimplement or alter it. Verify with `rg -n 'def entry_matches_name' tests/conftest_exclusions.py` after the rewrite.

**Add the non-emptiness guard** (Task 1 review, deferred minor — due here). With the registry now parsed from TOML rather than a Python literal, an empty or mis-keyed parse becomes a realistic regression, and it would make three integrity tests pass vacuously. Append to `tests/test_registry_integrity.py`:

```python
def test_registry_is_not_empty() -> None:
    """A TOML parse that silently yields nothing would make the integrity
    tests above pass vacuously (Task 1 review, deferred minor)."""
    assert FILE_EXCLUSIONS, "FILE_EXCLUSIONS parsed empty from compat/exclusions.toml"
    assert TEST_EXCLUSIONS, "TEST_EXCLUSIONS parsed empty from compat/exclusions.toml"
    assert ENTRY_CLASSES, "ENTRY_CLASSES parsed empty from compat/exclusions.toml"
```

Add `ENTRY_CLASSES` to that file's import from `conftest_exclusions`.

- [ ] **Step 4b: Replace the gate-summary keyword whitelist with taxonomy validation**

`tests/ported/test_gate_summary.py::test_no_unexcused_exclusions` is a *fourth* uncoordinated source of truth about the registry — a hand-maintained keyword list that has to be extended by hand every time a reason is worded differently (Task 2 Step 5b had to do exactly that). That is the DRY defect this whole tier exists to remove. Now that every entry carries a `class` from a closed taxonomy, the check becomes exact.

Replace the body of `test_no_unexcused_exclusions` with:

```python
def test_no_unexcused_exclusions():
    """Every exclusion must carry a class from the closed taxonomy.

    Supersedes the previous hand-maintained keyword whitelist, which validated
    reason PROSE by substring and needed editing whenever a reason was worded
    differently. The taxonomy is exact and is the same field that drives the
    skip-vs-xfail decision in conftest.py, so this check and the runtime
    behaviour cannot disagree.
    """
    from conftest_exclusions import ENTRY_CLASSES, FILE_CLASSES

    valid = {
        "deferred-symbol", "core-gap", "binding-gap", "divergence",
        "float-parity", "upstream-private", "env", "upstream-bug",
        "out-of-scope",
    }
    bad = []
    # Bounded: one pass over each registry mapping.
    for path, cls in FILE_CLASSES.items():
        if cls not in valid:
            bad.append(f"{path}: class `{cls}` not in taxonomy")
    for (path, name), cls in ENTRY_CLASSES.items():
        if cls not in valid:
            bad.append(f"{path}::{name}: class `{cls}` not in taxonomy")
    assert bad == [], "exclusions with an invalid class:\n  " + "\n  ".join(bad)
```

This is where `FILE_CLASSES` — exported in Step 4 and unused until now — earns its place.

- [ ] **Step 5: Verify nothing moved**

Run: `.venv/bin/python -m pytest tests/ported tests/test_registry_integrity.py -q`
Expected: `315 passed, 83 skipped` (311 ported + 4 integrity tests).

- [ ] **Step 6: Commit**

```bash
git add compat/ tools/migrate_exclusions.py tests/conftest_exclusions.py
git commit --no-verify -m "refactor(conformance): registry to TOML with a closed class taxonomy

Moves the hand-maintained dicts into compat/exclusions.toml, one class per
entry from the eight-value taxonomy. The class is what lets a gap be
described rather than counted, and it drives the skip-vs-xfail decision in
the next task. conftest_exclusions.py becomes a thin reader so existing
importers are unaffected. Adds compat/divergences.toml with its schema fixed
but empty, so manifest.json never changes shape when T1 populates it."
```

---

### Task 5: Convert running failures to strict xfail

The core of T0. `env` and `upstream-bug` keep skipping; everything else must run.

**Files:**
- Modify: `tests/conftest.py` (`pytest_collection_modifyitems`)
- Modify: `compat/exclusions.toml` (delete the four temporary entries from Task 2)

- [ ] **Step 1: Delete the four temporary entries**

Remove the four `[[test_exclusion]]` tables added in Task 2 Step 3 — they are marked `REMOVE in Task 5` in their reason text: `test_write_hif_compressed`, `test_empty_order`, `test_add_simplices_from_iterable_of_members`, `test_add_simplices_from_wrong_format`.

- [ ] **Step 2: Apply the class-driven marker**

Replace `pytest_collection_modifyitems` in `tests/conftest.py` with:

```python
# Classes whose tests genuinely cannot run. Everything else runs and is
# expected to fail, so it gets a STRICT xfail: if the gap is ever closed the
# test XPASSes and the build breaks, forcing the registry to be updated. That
# is what makes the register self-correcting rather than a document that rots
# into over-reporting (spec §5.1).
_SKIP_CLASSES = frozenset({"env", "upstream-bug", "out-of-scope"})


def pytest_collection_modifyitems(config, items):
    for item in items:
        rel = str(item.fspath).split("/ported/", 1)[-1]
        for name, reason in TEST_EXCLUSIONS.get(rel, []):
            if not _entry_matches(item, name):
                continue
            cls = ENTRY_CLASSES.get((rel, name), "deferred-symbol")
            tag = f"[conformance-exclusion:{cls}] {reason}"
            if cls in _SKIP_CLASSES:
                item.add_marker(pytest.mark.skip(reason=tag))
            else:
                item.add_marker(pytest.mark.xfail(reason=tag, strict=True))
```

Update the import at `tests/conftest.py:346` to pull in the new name:

```python
from conftest_exclusions import (
    ENTRY_CLASSES,
    FILE_EXCLUSIONS,
    TEST_EXCLUSIONS,
    entry_matches_name,
)
```

`entry_matches_name` must stay in this import — `_entry_matches` delegates to it and dropping it raises `NameError` at collection.

- [ ] **Step 3: Run and read the outcome carefully**

Run: `.venv/bin/python -m pytest tests/ported -q -rX`

Expected shape: roughly `315 passed, ~11 skipped, ~72 xfailed`, **0 failed and 0 xpassed**. The four Task-2 entries now pass, raising passes from 311 to 315.

**Any XPASS is a real finding, not a nuisance.** It means the registry claims a gap that no longer exists. For each one: delete its entry from `compat/exclusions.toml` and re-run. Do not weaken `strict=True` to make XPASSes disappear — that reintroduces exactly the decay this task removes.

**Any FAILED is a test whose class was wrong.** A test marked `env` that actually runs and fails needs its class corrected, not a skip restored.

- [ ] **Step 4: Confirm strictness actually bites**

Temporarily add a bogus entry to `compat/exclusions.toml`:

```toml
[[test_exclusion]]
path = "core/test_globalviews.py"
test = "test_subhypergraph"
class = "core-gap"
reason = "TEMPORARY strictness probe — delete immediately after verifying."
```

Run: `.venv/bin/python -m pytest tests/ported/core/test_globalviews.py -q`
Expected: **FAILS** with `[XPASS(strict)]`. This proves a stale entry breaks the build.

Now delete the probe entry and re-run; expected clean. (`test_subhypergraph` is confirmed present at `tests/ported/core/test_globalviews.py:7` and currently passes — that file has no exclusions at all.)

- [ ] **Step 5: Commit**

```bash
git add tests/conftest.py compat/exclusions.toml
git commit --no-verify -m "feat(conformance): strict xfail for gaps that run; skip only for gaps that cannot

skip is the wrong primitive for a gap whose test executes: a fixed gap stays
green forever and the registry decays into over-reporting, which is exactly
how the four F1 orphans arose. Anything not classed env/upstream-bug now
carries xfail(strict=True), so closing a gap XPASSes and breaks the build
until the entry is removed.

Deletes the four temporary Task-2 entries; those tests now run and pass,
taking the ported suite from 311 to 315 passing."
```

---

### Task 6: Honest denominator for uncollectable files

`FILE_EXCLUSIONS` drops 100 tests before collection, so they appear in neither column (F3). Counting them by AST restores the denominator.

**Files:**
- Create: `tests/test_denominator.py`
- Modify: `tools/gen_manifest.py` is not yet created; this task writes the counting helper it will reuse.
- Create: `tools/count_tests.py`

**Interfaces:**
- Produces: `count_tests_in_file(path) -> int` and `file_exclusion_test_counts() -> dict[str, int]`, both consumed by Task 7's generator.

- [ ] **Step 1: Write the counter**

> **SUPERSEDED — see shipped code.** This `ast.walk`-over-`FunctionDef` counter double-counts:
> Python is last-definition-wins, but `ast.walk` visits every `FunctionDef` node regardless of
> redefinition, so a module that defines the same test name twice (`stats/test_core_stats_functions.py`
> does this twice over) is overcounted by 2 — 52 instead of the 50 pytest actually collects. The
> `if path.exists() else 0` fallback in `file_exclusion_test_counts` also silently reports a
> deleted/renamed registry path as zero tests hidden, rather than erroring, quietly shrinking the
> invisible-test headline count. Fixed in commits `380305c` (dedupe to unique `(class, name)`
> identities, not AST nodes — the true invisible total is 98, not 100) and `1670a49` (raise
> `FileNotFoundError` instead of defaulting to 0). Shipped version: `tools/count_tests.py`.

```python
# tools/count_tests.py
"""AST test counting for files pytest never collects (spec §4.1, F3).

A whole-file exclusion removes its tests from BOTH the pass and skip counts,
so the reported denominator silently shrinks by exactly the tests we can least
account for. pytest cannot count them — some of these files cannot even import
— so they are counted statically instead.
"""

import ast
import pathlib

PORTED = pathlib.Path(__file__).parent.parent / "tests" / "ported"


def count_tests_in_file(path: pathlib.Path) -> int:
    """Number of test functions/methods defined in `path`, via AST."""
    tree = ast.parse(path.read_text())
    n = 0
    # Bounded: one pass over the module's AST nodes.
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                n += 1
    return n


def file_exclusion_test_counts() -> dict[str, int]:
    """`{relpath: test count}` for every whole-file exclusion."""
    import sys

    sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tests"))
    from conftest_exclusions import FILE_EXCLUSIONS

    out: dict[str, int] = {}
    # Bounded: one pass over the registry's file entries.
    for rel in FILE_EXCLUSIONS:
        path = PORTED / rel
        out[rel] = count_tests_in_file(path) if path.exists() else 0
    return out


if __name__ == "__main__":
    counts = file_exclusion_test_counts()
    # Bounded: one pass over the computed counts.
    for rel, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"{n:4d}  {rel}")
    print(f"{sum(counts.values()):4d}  TOTAL INVISIBLE")
```

- [ ] **Step 2: Write the test that pins the number**

> **SUPERSEDED — see shipped code.** `test_whole_file_exclusions_hide_a_known_number_of_tests`
> below asserts only `sum(counts.values()) == 100` — a total-only pin. Two files can drift by
> equal and opposite amounts (one overcounted, one undercounted) and the sum still matches,
> which is the exact silent-drift failure mode this tier exists to remove. Fixed in commit
> `380305c`: the shipped version pins every file's count individually via an `EXPECTED_INVISIBLE`
> dict, plus a separate test for the sum (98, not 100 — see the Step 1 supersession above for why
> the total changed). Shipped version: `tests/test_denominator.py`.

```python
# tests/test_denominator.py
"""The honest denominator includes tests no pytest run can see (spec F3)."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from count_tests import file_exclusion_test_counts  # noqa: E402


def test_whole_file_exclusions_hide_a_known_number_of_tests():
    """Pins F3's measurement so a new file exclusion cannot silently shrink
    the denominator. Update this number deliberately, with the reason, when a
    file exclusion is added or removed."""
    counts = file_exclusion_test_counts()
    assert sum(counts.values()) == 100, (
        "invisible-test count changed; update the expected value ONLY with a "
        f"recorded reason. Per-file: {counts}"
    )


def test_no_file_exclusion_is_silently_empty():
    """A file exclusion counting zero tests is either a wrong path or a file
    that no longer has tests — both are registry bugs."""
    zero = [rel for rel, n in file_exclusion_test_counts().items() if n == 0]
    assert zero == [], f"file exclusions that match no tests: {zero}"
```

- [ ] **Step 3: Run**

Run: `.venv/bin/python -m pytest tests/test_denominator.py -v`
Expected: both PASS. `tools/count_tests.py` run directly should print the 13 files and `100 TOTAL INVISIBLE`.

- [ ] **Step 4: Commit**

```bash
git add tools/count_tests.py tests/test_denominator.py
git commit --no-verify -m "feat(conformance): AST-count tests hidden by whole-file exclusions

13 FILE_EXCLUSIONS entries drop their files before collection, so their 100
tests appear in neither the pass nor the skip column and the reported
denominator silently shrinks by exactly the tests least accounted for
(spec F3). pytest cannot count them — some cannot import — so they are
counted statically. The count is pinned by a test so adding a file exclusion
is a deliberate, reviewed act."
```

---

### Task 7: The manifest generator

**Files:**
- Create: `tools/gen_manifest.py`
- Create: `python/hypergraph_rs/compat/manifest.json` (generated)

**Interfaces:**
- Consumes: `count_tests.file_exclusion_test_counts()`, `compat/exclusions.toml`, `compat/divergences.toml`, the built `hypergraph_rs` extension.
- Produces: `manifest.json` with the schema below. T1's `xgi.compat` reads exactly this file and no other.

- [ ] **Step 1: Write the generator**

```python
# tools/gen_manifest.py
"""Emit python/hypergraph_rs/compat/manifest.json — the single source of truth
(spec §3).

Derived by introspection: which symbols are bound vs deferred, which of real
xgi's dotted paths resolve. Authored: the exclusion registry and the divergence
register. The manifest carries NO test-run outcome — a static capability claim
belongs in the wheel, a run outcome belongs in CI (spec §5.2).
"""

import importlib
import json
import pathlib
import pkgutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from count_tests import file_exclusion_test_counts  # noqa: E402

DEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
XGI_TARGET = "0.10.2"


def _load_toml(name: str) -> dict:
    path = ROOT / "compat" / name
    with path.open("rb") as fh:
        return tomllib.load(fh)


def _dotted_paths(pkg_name: str) -> set[str]:
    """Every dotted submodule path under `pkg_name`, relative to it."""
    mod = importlib.import_module(pkg_name)
    out: set[str] = set()
    # Bounded: pkgutil walks a finite package tree.
    for _, name, _ in pkgutil.walk_packages(mod.__path__, pkg_name + "."):
        out.add(name.replace(pkg_name + ".", "", 1))
    return out


def _symbol_status() -> dict[str, str]:
    """`bound` vs `deferred` for every name in the package's __all__."""
    import hypergraph_rs

    out: dict[str, str] = {}
    # Bounded: one pass over __all__.
    for name in hypergraph_rs.__all__:
        obj = getattr(hypergraph_rs, name, None)
        mod = getattr(obj, "__module__", "") or ""
        out[name] = "deferred" if mod.endswith("_deferred") else "bound"
    return out


def build() -> dict:
    import hypergraph_rs

    exclusions = _load_toml("exclusions.toml")
    divergences = _load_toml("divergences.toml")

    ours = _dotted_paths("hypergraph_rs")
    try:
        theirs = _dotted_paths("xgi")
    except ModuleNotFoundError as exc:  # explicit: xgi is a hard requirement here
        raise SystemExit(
            "real xgi must be installed to generate the manifest "
            f"(pip install xgi=={XGI_TARGET}): {exc}"
        ) from exc

    return {
        "schema_version": 1,
        "hypergraph_rs_version": getattr(hypergraph_rs, "__version__", "unknown"),
        "xgi_target": XGI_TARGET,
        "symbols": _symbol_status(),
        "namespaces": {
            "expected": sorted(theirs),
            "resolved": sorted(theirs & ours),
            "missing": sorted(theirs - ours),
        },
        "divergences": divergences.get("divergence", []),
        "exclusions": {
            "files": exclusions.get("file_exclusion", []),
            "tests": exclusions.get("test_exclusion", []),
            "invisible_test_counts": file_exclusion_test_counts(),
        },
    }


def main() -> int:
    DEST.parent.mkdir(parents=True, exist_ok=True)
    DEST.write_text(json.dumps(build(), indent=2, sort_keys=True) + "\n")
    print(f"wrote {DEST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Generate and inspect**

Run: `.venv/bin/python tools/gen_manifest.py`

Then:

```bash
.venv/bin/python -c "
import json, pathlib
m = json.loads(pathlib.Path('python/hypergraph_rs/compat/manifest.json').read_text())
s = m['symbols']
print('bound   :', sum(1 for v in s.values() if v == 'bound'))
print('deferred:', sum(1 for v in s.values() if v == 'deferred'))
print('ns missing:', len(m['namespaces']['missing']))
print('invisible :', sum(m['exclusions']['invisible_test_counts'].values()))
"
```

Expected: `bound: 93`, `deferred: 71`, `ns missing: 48`, `invisible: 98`.

> `invisible` is **98**, not the 100 an earlier draft of this line said. Task 6's
> review found that counting `def test_` occurrences overstated
> `stats/test_core_stats_functions.py` by 2 — it defines two test names twice
> each, and Python is last-definition-wins, so `pytest --collect-only` collects
> 50 there, not 52.

Any deviation means introspection disagrees with `tests/DEFERRED_SYMBOLS.md` — investigate before continuing; that disagreement is itself the class of defect this plan exists to eliminate.

- [ ] **Step 3: Commit**

```bash
git add tools/gen_manifest.py python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "feat(compat): manifest generator — one artifact, four consumers

Introspects the built extension for bound-vs-deferred, walks real xgi for
expected dotted paths, reads the authored TOMLs, and emits manifest.json.
Verified against the existing ledger: 93 bound / 71 deferred / 48 missing
namespaces / 98 invisible tests.

Carries no score: a static capability claim belongs in the wheel, a run
outcome belongs in CI. Placed under the package so T1's xgi.compat needs
no move."
```

---

### Task 8: Drift check, honest score, and CI wiring

**Files:**
- Create: `tests/test_manifest_drift.py`
- Create: `tools/gate_report.py`
- Modify: `.github/workflows/ci.yml`
- Modify: `.mise.toml`

- [ ] **Step 1: Write the drift test**

```python
# tests/test_manifest_drift.py
"""The committed manifest must match what the current tree generates (spec §3).

This is what makes drift impossible: an exclusion whose gap was fixed, a symbol
that became bound, or a namespace that started resolving all change the
generated manifest, and the build fails until the committed copy is refreshed.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from gen_manifest import DEST, build  # noqa: E402


def test_committed_manifest_matches_generated():
    committed = json.loads(DEST.read_text())
    generated = build()
    assert committed == generated, (
        "manifest.json is stale — regenerate with `mise run py:manifest` and "
        "commit the result. If the diff is a symbol that became bound or an "
        "exclusion that no longer applies, update compat/exclusions.toml too."
    )
```

- [ ] **Step 2: Write the honest-score reporter**

```python
# tools/gate_report.py
"""Emit compat-report.json: the honest score with its class breakdown.

Reads a pytest JSON-free summary by re-deriving counts from the manifest and a
pytest run's exit summary passed on argv. Kept deliberately simple: the score
is a CI artifact, never shipped.

Usage:  python tools/gate_report.py <passed> <skipped> <xfailed>
"""

import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from gen_manifest import DEST  # noqa: E402


def main(passed: int, skipped: int, xfailed: int) -> int:
    manifest = json.loads(DEST.read_text())
    invisible = sum(manifest["exclusions"]["invisible_test_counts"].values())

    by_class: collections.Counter = collections.Counter()
    # Bounded: one pass over the registry's test entries.
    for entry in manifest["exclusions"]["tests"]:
        by_class[entry["class"]] += 1

    denominator = passed + skipped + xfailed + invisible
    report = {
        "passed": passed,
        "skipped": skipped,
        "xfailed": xfailed,
        "invisible": invisible,
        "denominator": denominator,
        "pass_rate": round(passed / denominator, 4) if denominator else 0.0,
        "gaps_by_class": dict(by_class),
    }
    out = ROOT / "compat-report.json"
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("usage: gate_report.py <passed> <skipped> <xfailed>")
    raise SystemExit(main(int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])))
```

- [ ] **Step 3: Add the mise tasks**

Append to `.mise.toml`:

```toml
[tasks."py:manifest"]
description = "Regenerate python/hypergraph_rs/compat/manifest.json from the built extension + authored TOMLs. Requires real xgi 0.10.2 installed (differential oracle)."
run = ".venv/bin/python tools/gen_manifest.py"

[tasks."py:gate"]
description = "The XGI conformance gate as CI runs it: symbol audit, manifest drift, registry integrity, then the ported suite. Sequential — heavy builds single-flight."
run = """
set -e
.venv/bin/python tools/audit_symbols.py
.venv/bin/python -m pytest tests/test_manifest_drift.py tests/test_registry_integrity.py tests/test_denominator.py -q
.venv/bin/python -m pytest tests/ported -q
"""
```

- [ ] **Step 4: Wire CI**

In `.github/workflows/ci.yml`, in the `xgi-conformance` job, insert between the `audit_symbols.py` step and the `pytest tests/ported` step:

```yaml
      - name: "Manifest drift + registry integrity (identical to local — mise run py:gate)"
        run: pytest tests/test_manifest_drift.py tests/test_registry_integrity.py tests/test_denominator.py -q
```

- [ ] **Step 4b: Pin the class → marker mapping**

Task 5's review found the skip half of `pytest_collection_modifyitems` is **dead at runtime and unpinned**: after Task 5 no *test* entry carries `env`, `upstream-bug`, or `out-of-scope` (the only skip-class entry is a file exclusion, whose class is never consulted). Inverting the branch condition would turn all 77 xfails into skips and nothing in the suite would fail. The reviewer verified the branch by hand — but hand-verification is exactly what this tier replaces with executable guards.

Add to `tests/test_registry_integrity.py`:

> **SUPERSEDED — see shipped code.** This guard asserts the CONTENTS of `_SKIP_CLASSES` — a
> constant, not the behaviour that consumes it. Its docstring claims "This is the guard for
> that [dead skip branch]," but inverting the branch condition it is supposed to guard
> (`conftest.py`'s `if cls in _SKIP_CLASSES` → `not in`) does not change this assertion at all:
> mutation testing confirmed all 77 strict xfails silently become skips and the whole gate stays
> green. Fixed in commit `5b3bfdf`: `test_skip_and_xfail_markers_land_on_the_right_classes` now
> drives `pytest_collection_modifyitems` against synthetic registry entries and asserts which
> marker (`skip` vs strict `xfail`) actually lands, with `test_skip_classes_are_exactly_the_
> unrunnable_ones` kept alongside it as the narrower constant-drift pin (renamed/added/removed
> class). Shipped version: `tests/test_registry_integrity.py`.

```python
def test_skip_classes_are_exactly_the_unrunnable_ones() -> None:
    """Pins which classes skip and which xfail.

    The skip branch in conftest.py has no live coverage — no test entry
    currently carries a skip class — so inverting the condition would silently
    turn all 77 strict xfails into skips, restoring the hiding behaviour this
    tier removed. This is the guard for that.
    """
    import conftest

    assert conftest._SKIP_CLASSES == frozenset({"env", "upstream-bug", "out-of-scope"})

    valid = {
        "deferred-symbol", "core-gap", "binding-gap", "divergence",
        "float-parity", "upstream-private", "env", "upstream-bug",
        "out-of-scope",
    }
    xfail_classes = valid - conftest._SKIP_CLASSES
    assert xfail_classes == {
        "deferred-symbol", "core-gap", "binding-gap",
        "divergence", "float-parity", "upstream-private",
    }
```

- [ ] **Step 5: Run the whole gate locally**

Run: `mise run py:gate`

Expected: symbol audit clean, the three meta-test files pass, ported suite reports **`316 passed, 1 skipped, 77 xfailed, 0 failed, 0 xpassed`** (= 394 collected).

> **These are measured actuals as of Task 5, not estimates.** An earlier draft of
> this line said `315 passed, ~11 skipped, ~72 xfailed`; that predated Task 4's
> reclassification of 8 `env` entries and the deletion of a stale entry. The
> `~11 skipped` figure in particular is now wrong by an order of magnitude: after
> Task 5 there is exactly **one** skip in the whole ported suite, and it is
> upstream's own inline `@pytest.mark.skip` at `communities/test_spectral.py:57`,
> not a registry entry. Every one of the 77 registry gaps executes.
>
> If your run disagrees with these numbers, report the difference — do not adjust
> anything to reach them.

- [ ] **Step 6: Verify the drift check actually bites**

```bash
.venv/bin/python -c "
import json, pathlib
p = pathlib.Path('python/hypergraph_rs/compat/manifest.json')
m = json.loads(p.read_text()); m['xgi_target'] = 'TAMPERED'
p.write_text(json.dumps(m, indent=2, sort_keys=True) + '\n')"
.venv/bin/python -m pytest tests/test_manifest_drift.py -q
```

Expected: **FAILS**. Then restore with `mise run py:manifest` and confirm it passes.

- [ ] **Step 7: Commit**

```bash
git add tests/test_manifest_drift.py tools/gate_report.py .mise.toml .github/workflows/ci.yml
git commit --no-verify -m "feat(conformance): manifest drift check, honest score report, CI wiring

CI regenerates the manifest and fails on any diff, so a fixed gap, a newly
bound symbol, or a newly resolving namespace breaks the build until the
registry is updated — the registry can no longer quietly over-report.

gate_report.py emits the score with its class breakdown against the honest
denominator (visible + invisible), so the number carries its own explanation
instead of being opaque. The report is a CI artifact and is never shipped.

mise run py:gate is what CI runs, per the one-invocation-source-of-truth rule."
```

---

## Definition of done

- [ ] `mise run py:gate` is green from a clean checkout.
- [ ] The ported suite is green under `PYTHONHASHSEED` 0, 1, 42, and random.
- [ ] Zero `XPASS`, zero `FAILED` in the ported suite.
- [ ] `compat/exclusions.toml` has no `UNCLASSIFIED` entries.
- [ ] `compat-report.json` reports a denominator of **492** (394 collected + 98 invisible) and a pass count of **316**.
- [ ] Tampering with `manifest.json` fails `tests/test_manifest_drift.py`.
- [ ] Adding a bogus exclusion for a passing test fails the suite with `XPASS(strict)`.
- [ ] No file under `tests/ported/` is modified except `test_scene_graph.py`.

## Deliberately NOT in T0

Each has a recorded reason; none is an oversight.

- **48 submodule shims** — T1. Requires the `drawing.draw` module/function collision handling from spec §4 C5.
- **`AttrProxy`** — T1. Rust work, bounded at 2 affected tests.
- **`DivergenceWarning`** and populating `divergences.toml` — T1.
- **`xgi.compat` API and `check()`** — T1, built last.
- **The ~12 cheap-tier symbols** — T2.
- **Converting `FILE_EXCLUSIONS` to per-test entries** — deferred. Task 6 makes the denominator honest by counting, which is what F3 actually requires; converting the entries additionally requires the shims (some files cannot import), so it belongs after T1.
- **Fixing raster test executability** — out of scope. `mod raster` is gated behind `cells3d`, absent from the python crate's default features, so `cargo test --workspace` compiles the raster test module out; an explicit `--features cells3d` run fails to link because pyo3's `extension-module` omits libpython. Real, unrelated to XGI parity, needs its own change.
