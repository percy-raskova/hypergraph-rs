# XGI Compat Layer — Plan A: Honest Register Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all six T0 carry-over defects and ship `compat/divergences.toml` with 34
measured entries plus `compat/partial_symbols.toml`, so the manifest states what this port
actually does instead of what its plumbing implies.

**Architecture:** Authored TOML is the source of truth; `tools/gen_manifest.py` reads it and
introspects the built extension; `manifest.json` is generated, committed, and diffed in CI.
This plan adds no runtime code — it corrects generators, guards, and data. The
`hypergraph_rs.compat` package and `check()` are Plans B and C.

**Tech Stack:** Python 3.13 (repo `.venv`), `tomllib` (stdlib), pytest, mise tasks, maturin.

**Spec:** `docs/superpowers/specs/2026-07-30-xgi-compat-layer-t1-design.md`
**Measurement record:** `docs/superpowers/specs/evidence/2026-07-30-divergence-register-measurement.md`
(the ready-to-paste TOML for Task 6 lives there, already corrected) and
`docs/superpowers/specs/evidence/2026-07-30-partial-symbols-sweep.md` (Task 7).

## Global Constraints

- **Never publish.** No PyPI/cargo/npm publish, no `gh release`, no artifact upload. Build,
  pack-dry-run and smoke only.
- **Cargo single-flight.** One build at a time. Never run concurrent `cargo`/`maturin`/`pytest`.
- **Never touch babylon's `.venv`** or any other project's files. This repo's `.venv` only.
- **`tests/ported/` oracle files are immutable.** A file under `tests/ported/` that has an
  upstream counterpart at `<xgi>/tests/<same rel path>` may not be modified. Repo-authored
  files there (no upstream counterpart) are editable.
- **Power-of-10:** every loop statically bounded, explicit types, no bare `except`, no
  generic catch-alls, functions under ~100 lines.
- **Commits:** `git commit --no-verify`, conventional messages.
- **The taxonomy is closed.** Nine classes: `deferred-symbol`, `core-gap`, `binding-gap`,
  `divergence`, `float-parity`, `upstream-private`, `env`, `upstream-bug`, `out-of-scope`.
  No new class may be introduced by this plan.
- **Guard rule (non-negotiable).** Every guard added here must pin the PROPERTY, not a proxy.
  For each test you write, name the mutation it must catch, perform that mutation, watch the
  test fail, then revert. A guard not proven by mutation is not done.
- **Gate command:** `mise run py:gate` (Python), `mise run rust:check` (Rust). Both must be
  green before any commit.
- **The gate discovers tests; it does not list them.** Owner ruling, 2026-07-31, after
  Task 1's review found that every test file this plan creates would otherwise run in no
  gate at all: `py:gate`'s meta-test invocation is
  `pytest tests/ --ignore=tests/ported -q`, never an explicit file list. A hand-maintained
  list is itself a proxy — forgetting an entry is silent, and that is exactly the failure
  this constraint exists to prevent. **Do not pin the meta-suite's pass count anywhere**;
  it rises as tasks add guards. The count that must stay fixed is the ported suite's
  `316 passed / 1 skipped / 77 xfailed`.

---

### Task 1: One symbol-status predicate

**Why:** Two copies of this logic exist and disagree. `audit_symbols.py:19` uses
`getattr(obj, "__module__", "").endswith("_deferred")` against a `getattr(hr, name, None)`
lookup, so it misclassifies any module whose name merely ends in `_deferred`, and silently
reports an unbound `__all__` name as `bound`. `gen_manifest.py:48-75` already repaired both
bugs. Both are fail-open in an artifact whose entire purpose is an honest claim.

**Files:**
- Create: `tools/symbol_status.py`
- Create: `tests/test_symbol_status.py`
- Modify: `tools/audit_symbols.py` (replace lines 16-20's classification loop)
- Modify: `tools/gen_manifest.py` (`_symbol_status`, lines 48-75, delegates)

**Interfaces:**
- Produces: `DEFERRED_MODULE: str`, `SymbolStatusError(RuntimeError)`,
  `symbol_status(namespace: ModuleType, names: Sequence[str]) -> dict[str, str]` returning
  values in `{"bound", "deferred"}`. Task 7 extends the value set with `"bound-partial"`.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_symbol_status.py
"""The deferred-stub predicate must pin the module identity, not a name suffix."""

import pathlib
import sys
import types

import pytest

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from symbol_status import DEFERRED_MODULE, SymbolStatusError, symbol_status


def _ns(name: str, **attrs: object) -> types.ModuleType:
    mod = types.ModuleType(name)
    for key, value in attrs.items():
        setattr(mod, key, value)
    return mod


def _obj_from(module_name: str) -> type:
    cls = type("Probe", (), {})
    cls.__module__ = module_name
    return cls


def test_real_stub_is_deferred() -> None:
    ns = _ns("fake", T=_obj_from(DEFERRED_MODULE))
    assert symbol_status(ns, ["T"]) == {"T": "deferred"}


def test_module_merely_ending_in_deferred_is_bound() -> None:
    """The endswith() predicate answers 'deferred' here. That is the bug."""
    ns = _ns("fake", T=_obj_from("hypergraph_rs.legacy_not_deferred"))
    assert symbol_status(ns, ["T"]) == {"T": "bound"}


def test_ordinary_symbol_is_bound() -> None:
    ns = _ns("fake", T=_obj_from("hypergraph_rs._hypergraph_rs_core"))
    assert symbol_status(ns, ["T"]) == {"T": "bound"}


def test_declared_but_unbound_name_raises() -> None:
    """getattr(ns, name, None) would silently class this 'bound'."""
    ns = _ns("fake")
    with pytest.raises(SymbolStatusError, match="ghost"):
        symbol_status(ns, ["ghost"])


def test_object_without_module_attribute_is_bound() -> None:
    ns = _ns("fake", T=object())
    assert symbol_status(ns, ["T"]) == {"T": "bound"}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_symbol_status.py -q`
Expected: collection error — `ModuleNotFoundError: No module named 'symbol_status'`.

- [ ] **Step 3: Write the module**

```python
# tools/symbol_status.py
"""The one predicate deciding whether a public symbol is a deferred stub.

Two copies of this logic existed and disagreed. `gen_manifest.py` compared the
module string exactly and raised on an unbound name; `audit_symbols.py` used
`__module__.endswith("_deferred")` against a `getattr(..., None)` lookup, so it
mis-classified any module whose name merely ends in `_deferred` and silently
reported an unbound `__all__` name as `bound`. Both directions are fail-open in
an artifact whose whole purpose is an honest capability claim, and the drift
check cannot catch either: regeneration reproduces the same wrong answer.

NOTE ON THE AXIS: `bound` means "not routed through the `_deferred` stub
factory" — NOT "fully implemented". Some bound symbols raise
`NotImplementedError` from Rust for particular arguments; those are recorded
separately in `compat/partial_symbols.toml`.
"""

from collections.abc import Sequence
from types import ModuleType

DEFERRED_MODULE = "hypergraph_rs._deferred"


class SymbolStatusError(RuntimeError):
    """A namespace declares a name it does not bind; no honest status exists."""


def symbol_status(namespace: ModuleType, names: Sequence[str]) -> dict[str, str]:
    """Classify each name as `bound` or `deferred`, or raise.

    Fails loudly rather than defaulting: an unbound name would otherwise be
    classed `bound`, over-claiming capability.
    """
    out: dict[str, str] = {}
    # Bounded: one pass over the caller's finite name sequence.
    for name in names:
        if not hasattr(namespace, name):
            raise SymbolStatusError(
                f"{namespace.__name__} declares {name!r} but does not bind it; "
                "a status here would over-claim capability"
            )
        obj = getattr(namespace, name)
        module = getattr(obj, "__module__", "") or ""
        out[name] = "deferred" if module == DEFERRED_MODULE else "bound"
    return out
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `.venv/bin/python -m pytest tests/test_symbol_status.py -q`
Expected: `5 passed`.

- [ ] **Step 5: Make both tools use it**

In `tools/gen_manifest.py`, delete the `_DEFERRED_MODULE` constant (line 25) and replace the
body of `_symbol_status` (lines 48-75) with a delegation, keeping `ManifestError` as the
type callers see:

```python
from symbol_status import SymbolStatusError, symbol_status  # noqa: E402


def _symbol_status() -> dict[str, str]:
    """`bound` vs `deferred` for every name in the package's `__all__`.

    See tools/symbol_status.py for the axis definition and why this fails loudly.
    """
    import hypergraph_rs

    try:
        return symbol_status(hypergraph_rs, hypergraph_rs.__all__)
    except SymbolStatusError as exc:
        raise ManifestError(str(exc)) from exc
```

In `tools/audit_symbols.py`, replace lines 16-20 with:

```python
    from symbol_status import symbol_status

    status = symbol_status(hr, sorted(want))
    deferred = [n for n, s in status.items() if s == "deferred"]
    bound = [n for n, s in status.items() if s == "bound"]
```

and add the `sys.path` insert `audit_symbols.py` currently lacks, immediately after its
imports:

```python
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
```

- [ ] **Step 6: Prove the manifest is unchanged**

Run:
```bash
cp python/hypergraph_rs/compat/manifest.json /tmp/manifest.before.json
mise run py:manifest
diff /tmp/manifest.before.json python/hypergraph_rs/compat/manifest.json && echo "IDENTICAL"
```
Expected: `IDENTICAL`. A refactor that changes the manifest is a behaviour change, not a
refactor — if it differs, stop and report which symbols moved and why.

- [ ] **Step 7: Prove the guard by mutation**

Reintroduce the bug in `tools/symbol_status.py`:
`out[name] = "deferred" if module.endswith("_deferred") else "bound"`.
Run `.venv/bin/python -m pytest tests/test_symbol_status.py -q`.
Expected: `test_module_merely_ending_in_deferred_is_bound` FAILS. Then revert.

Second mutation: replace the `hasattr`-then-raise with `obj = getattr(namespace, name, None)`.
Expected: `test_declared_but_unbound_name_raises` FAILS. Then revert.

Record both results in your report. A guard not proven by mutation is not done.

- [ ] **Step 7b: Wire the gate to discover tests**

A guard no gate runs protects nothing. `.mise.toml`'s `py:gate` currently names four test
files explicitly, so `tests/test_symbol_status.py` — and every test file later tasks add —
would never execute in CI. Replace the explicit list in the `py:gate` `run` block:

```
$PY -m pytest tests/ --ignore=tests/ported -q
```

leaving the `tools/audit_symbols.py` line before it and the `tests/ported` line after it
unchanged. Confirm the new invocation collects strictly more than the old one:

```bash
.venv/bin/python -m pytest tests/ --ignore=tests/ported -q --collect-only 2>&1 | tail -3
```

Note the collected count in your report. Do not pin it in any test — it rises as later tasks
add guards.

- [ ] **Step 8: Run the full gate and commit**

Run: `mise run py:gate`
Expected: EXIT=0. The ported suite must still report `316 passed, 1 skipped, 77 xfailed`;
the meta-suite count is now higher than before and must not be pinned.

```bash
git add tools/symbol_status.py tools/audit_symbols.py tools/gen_manifest.py tests/test_symbol_status.py .mise.toml
git commit --no-verify -m "fix(compat): one symbol-status predicate, fail-closed

audit_symbols.py classified stubs with __module__.endswith('_deferred') against
a getattr(..., None) lookup: it mis-read any module merely ending in '_deferred'
and silently reported an unbound __all__ name as bound. gen_manifest.py had
already repaired both. Extracts the repaired predicate to tools/symbol_status.py
and points both tools at it; the manifest is byte-identical after the change.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Pin the manifest to the installed xgi

**Why:** `XGI_TARGET = "0.10.2"` is recorded in the manifest but never checked against
`xgi.__version__`. Measured: `rg -n "XGI_TARGET|__version__" tools/ tests/ python/` returns
five hits, none of which reads real xgi's version. The gate would score against any installed
xgi and silently stop meaning "conformance to 0.10.2" — the score would still be produced,
and still look authoritative.

**Files:**
- Modify: `tools/gen_manifest.py` (add `_require_xgi_target`, call it in `build()`)
- Modify: `tools/audit_symbols.py` (call it before comparing surfaces)
- Create: `tests/test_xgi_version_pin.py`

**Interfaces:**
- Consumes: `ManifestError` from Task 1's unchanged `gen_manifest`.
- Produces: `gen_manifest.require_xgi_target() -> None`, raising `ManifestError`.

**Deliberately NOT in `hypergraph_rs/__init__.py`:** putting the check there would impose a
real-xgi runtime dependency on every downstream consumer of the wheel. The check belongs at
generation and audit time, where xgi is already a hard requirement.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_xgi_version_pin.py
"""A conformance score against the wrong xgi is not a conformance score."""

import pathlib
import sys

import pytest
import xgi

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

import gen_manifest


def test_matching_version_passes() -> None:
    assert xgi.__version__ == gen_manifest.XGI_TARGET
    gen_manifest.require_xgi_target()


def test_mismatched_version_raises_naming_both(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(xgi, "__version__", "9.9.9", raising=False)
    with pytest.raises(gen_manifest.ManifestError) as excinfo:
        gen_manifest.require_xgi_target()
    message = str(excinfo.value)
    assert "9.9.9" in message
    assert gen_manifest.XGI_TARGET in message


def test_build_refuses_on_mismatch(monkeypatch: pytest.MonkeyPatch) -> None:
    """The check must gate build(), not merely exist."""
    monkeypatch.setattr(xgi, "__version__", "9.9.9", raising=False)
    with pytest.raises(gen_manifest.ManifestError):
        gen_manifest.build()
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_xgi_version_pin.py -q`
Expected: FAIL — `AttributeError: module 'gen_manifest' has no attribute 'require_xgi_target'`.

- [ ] **Step 3: Implement the check**

Add to `tools/gen_manifest.py`, after `_load_toml`:

```python
def require_xgi_target() -> None:
    """Refuse to proceed against an xgi that is not the pinned target.

    Without this the whole gate keeps producing a score, and that score
    silently stops meaning "conformance to 0.10.2" — the failure mode is a
    number that still looks authoritative.
    """
    try:
        import xgi
    except ModuleNotFoundError as exc:
        raise ManifestError(
            f"real xgi must be installed (pip install xgi=={XGI_TARGET}): {exc}"
        ) from exc
    actual = getattr(xgi, "__version__", "unknown")
    if actual != XGI_TARGET:
        raise ManifestError(
            f"installed xgi is {actual}, but this port targets {XGI_TARGET}; "
            "a conformance score measured against a different xgi is not a "
            "conformance score. Install the target or update XGI_TARGET."
        )
```

In `build()`, call it as the first statement, and simplify the now-redundant
`ModuleNotFoundError` arm of the `_dotted_paths("xgi")` call to a plain call:

```python
def build() -> dict:
    require_xgi_target()
    import hypergraph_rs
    ...
    theirs = _dotted_paths("xgi")
```

In `tools/audit_symbols.py`, call it immediately after importing xgi:

```python
    from gen_manifest import require_xgi_target

    require_xgi_target()
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `.venv/bin/python -m pytest tests/test_xgi_version_pin.py -q`
Expected: `3 passed`.

- [ ] **Step 5: Prove the guard by mutation**

Weaken the raise to a warning:
```python
    if actual != XGI_TARGET:
        print(f"warning: xgi {actual} != {XGI_TARGET}")
```
Run the test. Expected: `test_mismatched_version_raises_naming_both` and
`test_build_refuses_on_mismatch` both FAIL. Revert.

Second mutation: keep the raise in `require_xgi_target` but delete its call from `build()`.
Expected: `test_build_refuses_on_mismatch` FAILS — this is the mutation that proves the check
is *wired*, not merely present. Revert.

- [ ] **Step 6: Run the full gate and commit**

Run: `mise run py:gate`
Expected: EXIT=0.

```bash
git add tools/gen_manifest.py tools/audit_symbols.py tests/test_xgi_version_pin.py
git commit --no-verify -m "fix(compat): refuse to generate a manifest against the wrong xgi

XGI_TARGET was recorded in the manifest but never compared to xgi.__version__,
so the gate would score against any installed xgi and keep reporting a number
that had silently stopped meaning conformance to 0.10.2. Gates build() and the
symbol audit; deliberately NOT in hypergraph_rs.__init__, which would impose a
real-xgi runtime dependency on consumers.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: The oracle guard must iterate upstream, not us

**Why:** `tests/test_oracle_integrity.py:55` iterates `PORTED.rglob("test_*.py")`. A deleted
ported file never enters the loop, so deleting the oracle leaves the guard green. The guard
pins "the files we have match upstream" when the property is "we have upstream's files, and
they match". This is the fifth instance of the proxy-guard pattern in this repo.

**Files:**
- Modify: `tests/test_oracle_integrity.py`
- Create: `tests/test_oracle_integrity_unit.py`

**Interfaces:**
- Produces: `_compare_tree(upstream: pathlib.Path, ported: pathlib.Path) -> tuple[list[str], list[str]]`
  returning `(missing, drifted)`, both sorted lists of relative path strings. Task 4 extends
  its inputs.

- [ ] **Step 1: Measure the legitimately-unported set FIRST**

Some upstream files may never have been ported on purpose. An unconditional
`assert missing == []` would fail immediately and tell you nothing. Measure before asserting:

```bash
.venv/bin/python - <<'PY'
import pathlib, xgi
up = pathlib.Path(xgi.__file__).resolve().parent.parent / "tests"
ported = pathlib.Path("tests/ported")
missing = sorted(
    str(p.relative_to(up)) for p in up.rglob("test_*.py")
    if not (ported / p.relative_to(up)).exists()
)
print(f"upstream files: {len(list(up.rglob('test_*.py')))}")
print(f"ported files with a counterpart: {len(list(ported.rglob('test_*.py')))}")
print(f"MISSING ({len(missing)}):")
for m in missing:
    print("  ", m)
PY
```

Record the exact output in your report. Every entry becomes an `UNPORTED` allowlist row with
a reason, sourced from `compat/exclusions.toml` or `tests/EXCLUSIONS.md` where one exists. If
you cannot justify an entry from an existing record, say so — do not invent a reason.

- [ ] **Step 2: Write the failing unit test**

```python
# tests/test_oracle_integrity_unit.py
"""The oracle comparison must notice a MISSING file, not only a drifted one."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from test_oracle_integrity import _compare_tree


def test_missing_ported_file_is_reported(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    (upstream / "sub").mkdir(parents=True)
    ported.mkdir()
    (upstream / "sub" / "test_x.py").write_text("import xgi\nassert True\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == ["sub/test_x.py"]
    assert drifted == []


def test_drifted_file_is_reported(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    upstream.mkdir()
    ported.mkdir()
    (upstream / "test_y.py").write_text("import xgi\nassert 1 == 1\n")
    (ported / "test_y.py").write_text("import hypergraph_rs as xgi\nassert 1 == 2\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == []
    assert drifted == ["test_y.py"]


def test_import_swap_alone_is_not_drift(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    upstream.mkdir()
    ported.mkdir()
    (upstream / "test_z.py").write_text("import xgi\nfrom xgi import Hypergraph\n")
    (ported / "test_z.py").write_text(
        "import hypergraph_rs as xgi\nfrom hypergraph_rs import Hypergraph\n"
    )

    missing, drifted = _compare_tree(upstream, ported)

    assert (missing, drifted) == ([], [])
```

- [ ] **Step 3: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_oracle_integrity_unit.py -q`
Expected: `ImportError: cannot import name '_compare_tree'`.

- [ ] **Step 4: Rewrite the guard**

Replace `test_ported_oracle_files_are_verbatim_upstream` (lines 48-66) with:

```python
# Upstream test files with no ported counterpart, each with the reason it was
# never ported. Measured in Task 3 Step 1 — do NOT add an entry to silence a
# failure. A file appearing here that you cannot justify from
# compat/exclusions.toml or tests/EXCLUSIONS.md is a deleted oracle, not an
# exemption.
UNPORTED: dict[str, str] = {
    # filled in from Step 1's measured output
}


def _compare_tree(
    upstream: pathlib.Path, ported: pathlib.Path
) -> tuple[list[str], list[str]]:
    """Return (missing, drifted) relative paths, comparing UPSTREAM's file list.

    Iterating upstream rather than our own tree is the whole point: a deleted
    ported file must be a failure, and it cannot be one if the loop never
    visits it.
    """
    missing: list[str] = []
    drifted: list[str] = []
    # Bounded: one pass over upstream's finite file list.
    for path in sorted(upstream.rglob("test_*.py")):
        rel = str(path.relative_to(upstream))
        ours = ported / rel
        if not ours.exists():
            missing.append(rel)
            continue
        if _normalise(ours.read_bytes()) != _normalise(path.read_bytes()):
            drifted.append(rel)
    return missing, drifted


def test_ported_oracle_files_are_verbatim_upstream() -> None:
    if not UPSTREAM.exists():
        raise AssertionError(
            f"upstream oracle not found at {UPSTREAM}; install xgi==0.10.2"
        )
    missing, drifted = _compare_tree(UPSTREAM, PORTED)
    unexplained = [m for m in missing if m not in UNPORTED]
    assert unexplained == [], (
        "upstream oracle files have no ported counterpart and no recorded "
        "reason — a deleted oracle silently removes the evidence behind every "
        "conformance claim that rested on it:\n  " + "\n  ".join(unexplained)
    )
    assert drifted == [], (
        "ported oracle files differ from upstream XGI 0.10.2 beyond the import "
        "swap — every conformance claim rests on these being verbatim:\n  "
        + "\n  ".join(drifted)
    )
    stale = sorted(set(UNPORTED) - set(missing))
    assert stale == [], (
        "UNPORTED lists files that ARE now ported; remove them so the "
        "allowlist cannot hide a future deletion:\n  " + "\n  ".join(stale)
    )
```

Fill `UNPORTED` from Step 1's measured output.

- [ ] **Step 5: Run both test files**

Run: `.venv/bin/python -m pytest tests/test_oracle_integrity.py tests/test_oracle_integrity_unit.py -q`
Expected: `4 passed`.

- [ ] **Step 6: Prove the guard by mutation — the one that matters**

```bash
git mv tests/ported/algorithms/test_clustering.py /tmp/test_clustering.py.hold
.venv/bin/python -m pytest tests/test_oracle_integrity.py -q ; echo "EXIT=$?"
git mv /tmp/test_clustering.py.hold tests/ported/algorithms/test_clustering.py
git status --porcelain   # MUST be empty
```
Expected: the run FAILS naming `algorithms/test_clustering.py`. Before this task it passed.
If `git status` is not empty after restoring, stop and fix before continuing.

Second mutation: revert `_compare_tree` to iterate `ported.rglob(...)`. Expected:
`test_missing_ported_file_is_reported` FAILS. Revert.

- [ ] **Step 7: Run the full gate and commit**

Run: `mise run py:gate`
Expected: EXIT=0.

```bash
git add tests/test_oracle_integrity.py tests/test_oracle_integrity_unit.py
git commit --no-verify -m "fix(conformance): oracle guard iterates upstream, so deletion fails

The guard walked our own ported tree, so a deleted oracle file never entered
the loop and the suite stayed green with the evidence gone. It pinned 'the
files we have match upstream' when the property is 'we have upstream's files,
and they match'. Now walks upstream's list, with an explicit UNPORTED allowlist
that is itself checked for staleness. Verified by deleting a ported file and
watching the guard fail.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Cover the oracle's shared `conftest.py`

**Why:** `tests/conftest.py` is 391 lines: roughly 341 verbatim upstream lines (about 30
fixtures every ported test depends on) plus the Task-15 conformance glue starting at line
344. It sits one level above `tests/ported/`, and `test_*.py` cannot match `conftest.py`, so
the guard structurally never opens it. Every ported test's inputs have zero byte-fidelity
coverage: silently changing `edgelist1()` would alter what the oracle actually asserts while
the suite stayed green.

**Approach — sentinel, not relocation.** The upstream content is a *prefix* of our file. A
sentinel comment marks where upstream ends and repo-authored code begins; the guard compares
everything above the sentinel against upstream's whole file. This is surgical: no fixture
moves, no import indirection, and the repo-authored section stays exactly where pytest
already finds it.

**Files:**
- Modify: `tests/conftest.py` (insert the sentinel; no other change)
- Modify: `tests/test_oracle_integrity.py` (add the conftest check)
- Modify: `tests/test_oracle_integrity_unit.py` (add its unit test)

- [ ] **Step 1: Verify the prefix really is verbatim upstream**

Do not assume it. Measure:

```bash
.venv/bin/python - <<'PY'
import pathlib, xgi, sys
sys.path.insert(0, "tests")
from test_oracle_integrity import _normalise
up = (pathlib.Path(xgi.__file__).resolve().parent.parent / "tests" / "conftest.py")
ours = pathlib.Path("tests/conftest.py")
u, o = up.read_bytes(), ours.read_bytes()
print("upstream lines:", u.count(b"\n"), "ours:", o.count(b"\n"))
n_up = _normalise(u)
prefix = _normalise(o)[:len(n_up)]
print("PREFIX MATCHES UPSTREAM:", prefix == n_up)
if prefix != n_up:
    import difflib
    a = n_up.decode().splitlines()
    b = prefix.decode().splitlines()
    print("\n".join(list(difflib.unified_diff(a, b, "upstream", "ours-prefix"))[:60]))
PY
```

If the prefix does NOT match, stop and report the diff. Do not edit the fixtures to force a
match — an existing divergence in the oracle's fixtures is a finding, not a chore.

- [ ] **Step 2: Write the failing unit test**

Append to `tests/test_oracle_integrity_unit.py`:

```python
from test_oracle_integrity import SENTINEL, _upstream_prefix


def test_prefix_split_returns_everything_above_the_sentinel() -> None:
    body = b"import xgi\nFIXTURE = 1\n" + SENTINEL + b"\nREPO = 2\n"
    assert _upstream_prefix(body) == b"import xgi\nFIXTURE = 1\n"


def test_missing_sentinel_raises() -> None:
    import pytest

    with pytest.raises(AssertionError, match="sentinel"):
        _upstream_prefix(b"import xgi\nno sentinel here\n")


def test_duplicated_sentinel_raises() -> None:
    import pytest

    body = b"a\n" + SENTINEL + b"\nb\n" + SENTINEL + b"\nc\n"
    with pytest.raises(AssertionError, match="exactly once"):
        _upstream_prefix(body)
```

- [ ] **Step 3: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_oracle_integrity_unit.py -q`
Expected: `ImportError: cannot import name 'SENTINEL'`.

- [ ] **Step 4: Add the sentinel to `tests/conftest.py`**

Immediately before the existing line `# --- Task 15: conformance exclusion glue ---`
(currently line 344), insert exactly:

```python
# === END VERBATIM XGI 0.10.2 conftest — repo-authored code below this line ===
# tests/test_oracle_integrity.py byte-compares everything ABOVE this marker
# against upstream's conftest.py. Do not move, reword, or duplicate it, and do
# not add repo-authored code above it.
```

- [ ] **Step 5: Implement the check**

Add to `tests/test_oracle_integrity.py`:

```python
SENTINEL = b"# === END VERBATIM XGI 0.10.2 conftest — repo-authored code below this line ==="


def _upstream_prefix(raw: bytes) -> bytes:
    """Everything above the sentinel, which must appear exactly once."""
    parts = raw.split(SENTINEL)
    assert len(parts) != 1, (
        "tests/conftest.py has lost its sentinel; without it the ~30 upstream "
        "fixtures every ported test depends on have no byte-fidelity coverage"
    )
    assert len(parts) == 2, (
        f"the sentinel must appear exactly once, found {len(parts) - 1}"
    )
    return parts[0]


def test_shared_conftest_prefix_is_verbatim_upstream() -> None:
    upstream = UPSTREAM / "conftest.py"
    if not upstream.exists():
        raise AssertionError(f"upstream conftest not found at {upstream}")
    ours = (PORTED.parent / "conftest.py").read_bytes()
    assert _normalise(_upstream_prefix(ours)) == _normalise(upstream.read_bytes()), (
        "tests/conftest.py's upstream section differs from XGI 0.10.2's. Every "
        "ported test's inputs come from these fixtures; a change here silently "
        "alters what the oracle asserts."
    )
```

- [ ] **Step 6: Run the tests**

Run: `.venv/bin/python -m pytest tests/test_oracle_integrity.py tests/test_oracle_integrity_unit.py -q`
Expected: all pass.

- [ ] **Step 7: Prove the guard by mutation — the one that matters**

Change one fixture's return value in the verbatim section of `tests/conftest.py` (for
example, alter one edge in `edgelist1()`), then:

Run: `.venv/bin/python -m pytest tests/test_oracle_integrity.py -q`
Expected: FAILS. Before this task it passed silently. Revert with
`git checkout -- tests/conftest.py`, then re-add the sentinel and confirm
`git diff --stat` shows only the sentinel insertion.

- [ ] **Step 8: Run the full gate and commit**

Run: `mise run py:gate`
Expected: EXIT=0.

```bash
git add tests/conftest.py tests/test_oracle_integrity.py tests/test_oracle_integrity_unit.py
git commit --no-verify -m "fix(conformance): byte-cover the oracle's shared conftest fixtures

tests/conftest.py holds ~341 verbatim upstream lines — about 30 fixtures every
ported test's inputs come from — and the oracle guard structurally never opened
it, because it globs test_*.py one directory down. A sentinel now marks where
upstream ends and repo-authored code begins, and the guard byte-compares the
prefix. Verified by altering a fixture's return value and watching the guard
fail where it previously stayed green.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Wire `gate_report.py` into the gate

**Why:** `tools/gate_report.py` computes the honest score with its class breakdown and
cross-checks xfail counts against the manifest — and nothing invokes it. Measured:
`rg -n gate_report .mise.toml .github/ tests/ tools/` returns hits only inside the file
itself. A tool that never runs cannot catch anything.

**Counts must be exact, not parsed.** Scraping pytest's human-readable summary line is
itself a proxy: the wording, order and pluralisation are presentation, not contract. A
`pytest_sessionfinish` hook reads the real counts from the terminal reporter's own stats.

**Files:**
- Modify: `tests/conftest.py` (repo-authored section, below the Task 4 sentinel)
- Modify: `tools/gate_report.py` (accept `--from-stats`)
- Modify: `.mise.toml` (`py:gate`)
- Create: `tests/test_gate_wiring.py`
- Modify: `.gitignore` (add `.gate-stats.json` if absent)

- [ ] **Step 1: Write the failing test**

```python
# tests/test_gate_wiring.py
"""A reporting tool nothing invokes cannot catch anything."""

import pathlib
import tomllib

ROOT = pathlib.Path(__file__).parent.parent


def _gate_run() -> str:
    with (ROOT / ".mise.toml").open("rb") as fh:
        config = tomllib.load(fh)
    return config["tasks"]["py:gate"]["run"]


def test_gate_invokes_gate_report() -> None:
    assert "tools/gate_report.py" in _gate_run(), (
        "gate_report.py computes the honest score and cross-checks xfail counts "
        "against the manifest; orphaned, it can never disagree with anything"
    )


def test_gate_report_reads_exact_stats_not_a_parsed_summary() -> None:
    run = _gate_run()
    assert "--from-stats" in run, (
        "counts must come from the reporter's own stats, not from scraping "
        "pytest's human-readable summary line"
    )


def test_gate_still_runs_the_ported_suite_last() -> None:
    run = _gate_run()
    assert "tests/ported" in run
    assert run.index("tests/ported") < run.index("tools/gate_report.py"), (
        "the report must be produced from the ported run, so it comes after it"
    )


def test_gate_discovers_meta_tests_rather_than_listing_them() -> None:
    """Task 1's review found six guards that would have run in no gate at all,
    because py:gate enumerated four test files by hand. A list is a proxy for
    'the tests run' — forgetting an entry is silent."""
    run = _gate_run()
    assert "tests/ --ignore=tests/ported" in run, (
        "py:gate must DISCOVER tests/, not enumerate files"
    )
    listed = re.findall(r"tests/test_\w+\.py", run)
    assert listed == [], (
        f"py:gate names individual test files ({listed}); any guard not on that "
        "list runs nowhere, which is how test_symbol_status.py was orphaned at birth"
    )
```

Add `import re` to that file's imports.

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_gate_wiring.py -q`
Expected: 3 failures.

- [ ] **Step 3: Emit exact stats from the ported run**

Append to `tests/conftest.py`, below the Task 4 sentinel:

```python
def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    """Write the run's exact outcome counts for tools/gate_report.py.

    Read from the terminal reporter's own stats rather than scraping the
    printed summary line: that line's wording and order are presentation, and
    pinning it would be one more guard on a proxy.
    """
    reporter = session.config.pluginmanager.get_plugin("terminalreporter")
    if reporter is None:  # -p no:terminal, or a non-terminal runner
        return
    counts = {key: len(value) for key, value in reporter.stats.items() if key}
    target = pathlib.Path(session.config.rootpath) / ".gate-stats.json"
    target.write_text(json.dumps(counts, indent=2, sort_keys=True) + "\n")
```

Add `import json` and `import pathlib` to the repo-authored section if not already imported
there. Do **not** add imports above the sentinel.

- [ ] **Step 4: Teach `gate_report.py` to read them**

Replace `tools/gate_report.py`'s `__main__` block (lines 58-61) with:

```python
def _from_stats(path: pathlib.Path) -> tuple[int, int, int]:
    """Exact counts from the ported run, written by pytest_sessionfinish."""
    if not path.exists():
        raise SystemExit(
            f"{path} not found — run the ported suite first; the report is "
            "derived from a real run, never from assumed counts"
        )
    stats = json.loads(path.read_text())
    return (
        int(stats.get("passed", 0)),
        int(stats.get("skipped", 0)),
        int(stats.get("xfailed", 0)),
    )


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--from-stats":
        raise SystemExit(main(*_from_stats(pathlib.Path(sys.argv[2]))))
    if len(sys.argv) == 4:
        raise SystemExit(
            main(int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]))
        )
    raise SystemExit(
        "usage: gate_report.py <passed> <skipped> <xfailed>\n"
        "   or: gate_report.py --from-stats <path>"
    )
```

- [ ] **Step 5: Wire it into `py:gate`**

In `.mise.toml`, append to the `py:gate` `run` block, after the ported-suite line:

```
$PY tools/gate_report.py --from-stats .gate-stats.json
```

Keep the report ephemeral: add `.gate-stats.json` to `.gitignore` (`compat-report.json` is
already there). Do **not** add a pass-rate threshold — a hardcoded floor is a new stale
proxy, and the drift check plus the xfail cross-check already fail loudly on disagreement.

- [ ] **Step 6: Run the gate and read the report**

Run: `mise run py:gate`
Expected: EXIT=0, and the report prints with `pass_rate` around `0.6423`, `invisible: 98`,
`denominator: 492`, and a `gaps_by_class` breakdown. If `gate_report` raises the
"inconsistent inputs" error, the registry and the suite genuinely disagree — investigate
rather than adjusting the numbers.

- [ ] **Step 7: Prove the wiring by mutation**

Delete the `gate_report.py` line from `.mise.toml`'s `py:gate`. Run
`.venv/bin/python -m pytest tests/test_gate_wiring.py -q`. Expected:
`test_gate_invokes_gate_report` FAILS. Revert.

- [ ] **Step 8: Commit**

```bash
git add tests/conftest.py tests/test_gate_wiring.py tools/gate_report.py .mise.toml .gitignore
git commit --no-verify -m "fix(conformance): wire gate_report into py:gate with exact counts

gate_report.py cross-checks the suite's xfail count against the manifest's gap
count and computes the honest denominator — and nothing invoked it, so it could
never disagree with anything. Counts now come from the terminal reporter's own
stats via pytest_sessionfinish rather than from scraping the printed summary
line, whose wording is presentation rather than contract. Report stays a local
ephemeral artifact: nothing uploaded, nothing committed, no pass-rate floor.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Populate `compat/divergences.toml` with 34 measured entries

**Why:** The manifest ships `divergences: []` with a note admitting it is unpopulated. The
register was measured against live xgi 0.10.2 in the spec's evidence documents: 34 live,
Python-visible divergences, of which the old D1–D17 supplies 14. Shipping the 14 alone would
omit D33 — `H.edges[e][k] = v` silently discarded — and read as complete.

**Files:**
- Modify: `compat/divergences.toml`
- Create: `tests/test_divergence_register.py`
- Modify: `tests/ported/test_gate_summary.py` (repo-authored — extract the closed taxonomy)
- Create: `tools/taxonomy.py`

**Interfaces:**
- Produces: `tools/taxonomy.py` exporting `CLASSES: frozenset[str]` — the nine-class closed
  set, single source of truth for both registries.

**Source of the entries:** `docs/superpowers/specs/evidence/2026-07-30-divergence-register-measurement.md`,
section "Ready-to-paste TOML". Copy it **verbatim** — it is committed, corrected, and is the
measurement record. Do not retype the entries; do not re-derive them.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_divergence_register.py
"""The shipped divergence register must be complete, unique, and in-taxonomy."""

import pathlib
import sys
import tomllib

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from taxonomy import CLASSES

ROOT = pathlib.Path(__file__).parent.parent
FIELDS = {"id", "api", "ours", "xgi", "class", "reason"}


def _register() -> list[dict]:
    with (ROOT / "compat" / "divergences.toml").open("rb") as fh:
        return tomllib.load(fh).get("divergence", [])


def test_register_is_populated() -> None:
    """34 measured in the evidence document, plus D38 (F7, subclassability),
    which was found during the method-layer sweep after that document froze."""
    assert len(_register()) == 35, (
        "the measured live, Python-visible register is 35 entries; a short "
        "register reads as complete when it is not"
    )


def test_every_class_is_in_the_closed_taxonomy() -> None:
    offenders = sorted(
        {e["class"] for e in _register() if e["class"] not in CLASSES}
    )
    assert offenders == [], f"off-taxonomy classes: {offenders}"


def test_ids_are_unique() -> None:
    ids = [e["id"] for e in _register()]
    assert len(ids) == len(set(ids)), "duplicate divergence ids"


def test_no_entry_has_unknown_or_missing_fields() -> None:
    for entry in _register():
        extra = set(entry) - FIELDS
        assert extra == set(), f"{entry.get('id')}: unknown fields {sorted(extra)}"
        absent = FIELDS - set(entry)
        assert absent == set(), f"{entry.get('id')}: missing fields {sorted(absent)}"


def test_d34_names_the_default_path_not_in_place_false() -> None:
    """in_place=False is the one cleanup path that AGREES; the first draft
    inverted this and it was caught only by re-measurement."""
    entry = next(e for e in _register() if e["id"] == "D34")
    assert "in_place=False" not in entry["api"]


def test_dead_register_entries_are_absent() -> None:
    """D1, D6, D8, D10, D15, D16 are resolved, Rust-only, or never register
    entries. Shipping them would claim differences that do not exist."""
    ids = {e["id"] for e in _register()}
    assert ids.isdisjoint({"D1", "D6", "D8", "D10", "D15", "D16"})
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_divergence_register.py -q`
Expected: `ModuleNotFoundError: No module named 'taxonomy'`.

- [ ] **Step 3: Extract the closed taxonomy**

```python
# tools/taxonomy.py
"""The closed exclusion/divergence taxonomy — one definition, two registries.

The set was previously inline in tests/ported/test_gate_summary.py. Both
compat/exclusions.toml and compat/divergences.toml are validated against it, so
the two files cannot drift into private vocabularies.
"""

CLASSES: frozenset[str] = frozenset(
    {
        "deferred-symbol",
        "core-gap",
        "binding-gap",
        "divergence",
        "float-parity",
        "upstream-private",
        "env",
        "upstream-bug",
        "out-of-scope",
    }
)
```

In `tests/ported/test_gate_summary.py`, replace the inline literal set at line 13 with an
import of `CLASSES`, leaving the assertions unchanged. (This file is repo-authored — no
upstream counterpart — so it is editable under the Global Constraints.)

- [ ] **Step 4: Populate the register**

Copy the entire fenced TOML block from the evidence document's "Ready-to-paste TOML" section
into `compat/divergences.toml`, appended below the existing schema-comment header. Keep the
header — it documents the six fields — and delete only its final line,
`# Intentionally empty in T0.`

Then apply the one addition the evidence document does not yet carry, from spec F7:

```toml
[[divergence]]
id = "D38"
api = "class MyGraph(hypergraph_rs.Hypergraph) — subclassing any core class"
ours = "TypeError: type 'hypergraph_rs.Hypergraph' is not an acceptable base type; Py_TPFLAGS_BASETYPE is False."
xgi = "Subclasses normally; Py_TPFLAGS_BASETYPE is True."
class = "binding-gap"
reason = "PyO3 classes are not subclassable unless declared #[pyclass(subclass)]. Import-fatal at class-definition time, so a consumer that subclasses cannot even load. Found during the method-layer sweep, absent from the 34-entry register."
```

The Step 1 test already expects 35 for this reason. If your copy of the evidence block yields
any count other than 34 before adding D38, stop and report the discrepancy rather than
adjusting the test to match what you pasted.

- [ ] **Step 5: Verify the register parses and matches reality**

```bash
.venv/bin/python - <<'PY'
import pathlib, tomllib, collections
with pathlib.Path("compat/divergences.toml").open("rb") as fh:
    reg = tomllib.load(fh)["divergence"]
print("entries:", len(reg))
print("by class:", dict(collections.Counter(e["class"] for e in reg)))
print("ids:", ", ".join(e["id"] for e in reg))
PY
```

Record the output. Then spot-check three entries against the live runtime — pick D33, D34 and
one you choose — confirming the `ours` and `xgi` text still describes what the libraries do:

```bash
.venv/bin/python -c "
import xgi, hypergraph_rs as hrs
for n, lib in (('xgi', xgi), ('hrs', hrs)):
    H = lib.Hypergraph(); H.add_edge(['a','b'], idx='e1')
    H.edges['e1']['color'] = 'red'
    print(n, 'D33:', dict(H.edges['e1']))
    G = lib.Hypergraph(); G.add_edge(['a','b'])
    print(n, 'D34:', type(G.cleanup()).__name__)
"
```
Expected: `xgi D33: {'color': 'red'}` / `hrs D33: {}`; `xgi D34: Hypergraph` /
`hrs D34: NoneType`.

- [ ] **Step 6: Run the tests**

Run: `.venv/bin/python -m pytest tests/test_divergence_register.py -q`
Expected: `6 passed`.

- [ ] **Step 7: Prove the guards by mutation**

Add a temporary entry with `class = "made-up"`. Expected:
`test_every_class_is_in_the_closed_taxonomy` FAILS. Remove it.

Restore D34's `api` to `"Hypergraph.cleanup(..., in_place=False)"`. Expected:
`test_d34_names_the_default_path_not_in_place_false` FAILS. Revert.

- [ ] **Step 8: Commit**

```bash
git add compat/divergences.toml tools/taxonomy.py tests/test_divergence_register.py tests/ported/test_gate_summary.py
git commit --no-verify -m "feat(compat): ship the measured divergence register

35 live, Python-visible divergences, measured against real xgi 0.10.2 rather
than transcribed from D1-D17 — which supplies only 14 of them, and includes
seven entries (D1, D6, D8, D10, D15, D16 and half of D1-ripple) that are
resolved, Rust-only, or were never register entries at all. Shipping those
would claim differences that do not exist; omitting the 20 unnumbered ones
would omit D33, where H.edges[e][k]=v is silently discarded.

Extracts the nine-class taxonomy to tools/taxonomy.py so exclusions.toml and
divergences.toml validate against one closed set.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: `bound-partial` — symbols that are bound and still raise

**Why:** `manifest["symbols"]` reports `bound` for symbols that raise `NotImplementedError`
for ordinary arguments. Three sites are confirmed: `degree_assortativity(H, exact=False)`
(xgi returns `-0.5195…`), `D.nodes.in_degree(order=1)`, and `H.edges.attrs('color')()`. Two
are untested by the ported suite. A compat checker reporting these as usable would be lying,
which is exactly what Plan C will do unless the data exists.

**Files:**
- Create: `compat/partial_symbols.toml`
- Modify: `tools/gen_manifest.py` (load it; promote status; expose `partial_symbols`)
- Create: `tests/test_partial_symbols.py`

**Interfaces:**
- Consumes: `symbol_status()` from Task 1.
- Produces: manifest key `partial_symbols: list[dict]` and symbol status value
  `"bound-partial"`.

**Source:** `docs/superpowers/specs/evidence/2026-07-30-partial-symbols-sweep.md`. Entries
marked there as **simulated** rather than observed (the `drawing.*` finding) must be
**excluded** from the shipped TOML — record them in your report as pending owner input, not
as fact.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_partial_symbols.py
"""`bound` must not hide a symbol that raises for ordinary arguments."""

import json
import pathlib
import sys
import tomllib

import pytest

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
FIELDS = {"symbol", "site", "trigger", "reason"}


def _authored() -> list[dict]:
    with (ROOT / "compat" / "partial_symbols.toml").open("rb") as fh:
        return tomllib.load(fh).get("partial_symbol", [])


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def test_register_is_non_empty_and_well_formed() -> None:
    entries = _authored()
    assert entries, "three partial sites are confirmed; an empty file hides them"
    for entry in entries:
        assert set(entry) == FIELDS, f"{entry.get('symbol')}: fields {sorted(entry)}"


def test_manifest_promotes_them_to_bound_partial() -> None:
    manifest = _manifest()
    for entry in _authored():
        symbol = entry["symbol"]
        assert manifest["symbols"][symbol] == "bound-partial", (
            f"{symbol} is recorded partial but the manifest still calls it "
            f"{manifest['symbols'][symbol]!r}"
        )


def test_no_symbol_is_both_partial_and_deferred() -> None:
    manifest = _manifest()
    partial = {e["symbol"] for e in _authored()}
    deferred = {n for n, s in manifest["symbols"].items() if s == "deferred"}
    assert partial.isdisjoint(deferred)


def test_manifest_exposes_the_detail() -> None:
    assert _manifest()["partial_symbols"] == _authored()


def test_degree_assortativity_exact_false_still_raises() -> None:
    """A direct pin: when this stops raising, the entry must be deleted."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([["a", "b"], ["b", "c"], ["a", "c"]])
    with pytest.raises(NotImplementedError):
        hrs.degree_assortativity(H, exact=False)
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_partial_symbols.py -q`
Expected: `FileNotFoundError` for `compat/partial_symbols.toml`.

- [ ] **Step 3: Author the register**

```toml
# compat/partial_symbols.toml
#
# Symbols the manifest calls `bound` — meaning "not routed through the _deferred
# stub factory" — that nonetheless raise NotImplementedError for ordinary
# arguments. Without this file a compat checker reports them as usable.
#
# Schema, one table per site:
#
#   [[partial_symbol]]
#   symbol  = "degree_assortativity"   # the public name in __all__
#   site    = "crates/.../algorithms.rs:163"
#   trigger = "exact=False"            # the argument shape that raises
#   reason  = "..."                    # what xgi does instead
#
# An entry here promotes the symbol to `bound-partial` in manifest.json. When a
# gap is CLOSED, delete its entry — tests/test_partial_symbols.py pins the
# raising behaviour, so a fix that leaves the entry breaks the build.
```

Then add one `[[partial_symbol]]` table per **verified** entry from the sweep document,
copying its `site`, `trigger` and `reason` verbatim. Verify each one raises before you write
it:

```bash
.venv/bin/python -c "
import hypergraph_rs as hrs, xgi
H = hrs.Hypergraph(); H.add_edges_from([['a','b'],['b','c'],['a','c']])
X = xgi.Hypergraph(); X.add_edges_from([['a','b'],['b','c'],['a','c']])
for label, fn in (('degree_assortativity(exact=False)', lambda L, G: L.degree_assortativity(G, exact=False)),):
    for name, lib, graph in (('xgi', xgi, X), ('hrs', hrs, H)):
        try: print(name, label, '->', fn(lib, graph))
        except NotImplementedError as e: print(name, label, '-> NotImplementedError:', e)
"
```

- [ ] **Step 4: Teach the generator**

In `tools/gen_manifest.py`, add the loader and promotion:

```python
def _partial_symbols() -> list[dict]:
    return _load_toml("partial_symbols.toml").get("partial_symbol", [])


def _promote_partials(status: dict[str, str], partials: list[dict]) -> dict[str, str]:
    """Mark bound-but-raising symbols `bound-partial`.

    A symbol recorded partial that the package reports `deferred` is a
    contradiction — one of the two records is stale — so it raises rather than
    silently picking a winner.
    """
    out = dict(status)
    # Bounded: one pass over the authored register.
    for entry in partials:
        symbol = entry["symbol"]
        if symbol not in out:
            raise ManifestError(
                f"partial_symbols.toml names {symbol!r}, which is not in "
                "hypergraph_rs.__all__"
            )
        if out[symbol] == "deferred":
            raise ManifestError(
                f"{symbol!r} is recorded partial but is a deferred stub; "
                "delete the partial entry or the stub"
            )
        out[symbol] = "bound-partial"
    return out
```

In `build()`, replace `"symbols": _symbol_status(),` with:

```python
    partials = _partial_symbols()
    ...
        "symbols": _promote_partials(_symbol_status(), partials),
        "partial_symbols": partials,
```

and extend `symbols_status` to name the third value:

```python
        "symbols_status": (
            "`bound` means 'not routed through the _deferred stub factory', NOT "
            "'fully implemented'. `bound-partial` means the symbol is bound and "
            "still raises NotImplementedError for the argument shapes recorded "
            "in `partial_symbols`. Read `symbols` together with `exclusions`, "
            "`partial_symbols` and `divergences`."
        ),
```

- [ ] **Step 5: Regenerate and run**

Run: `mise run py:manifest && .venv/bin/python -m pytest tests/test_partial_symbols.py -q`
Expected: all pass.

- [ ] **Step 6: Prove the guards by mutation**

Add a `[[partial_symbol]]` entry naming a symbol that is a deferred stub. Run
`mise run py:manifest`. Expected: `ManifestError` naming the contradiction. Remove it.

Add an entry and do **not** regenerate. Run
`.venv/bin/python -m pytest tests/test_partial_symbols.py tests/test_manifest_drift.py -q`.
Expected: `test_manifest_promotes_them_to_bound_partial` and the drift check both FAIL.
Remove it and regenerate.

- [ ] **Step 7: Commit**

```bash
git add compat/partial_symbols.toml tools/gen_manifest.py tests/test_partial_symbols.py python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "feat(compat): bound-partial status for symbols that still raise

manifest['symbols'] reported 'bound' for symbols raising NotImplementedError on
ordinary arguments — degree_assortativity(exact=False), in_degree(order=...),
edges.attrs(k)() — two of which no ported test exercises. 'bound' has always
meant 'not a stub', never 'implemented', and a compat checker built on it would
report those call sites as usable. Adds compat/partial_symbols.toml, promotes
the named symbols to bound-partial, and pins the raising behaviour so closing a
gap without deleting its entry breaks the build.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: Regenerate, retire the placeholder status, close the gate

**Why:** `manifest.json` still carries `divergences_status: "unpopulated-T0…"`, which is now
false. This task makes the shipped artifact tell the truth and proves the whole plan green
together.

**Files:**
- Modify: `tools/gen_manifest.py` (`divergences_status`, `schema_version`)
- Modify: `python/hypergraph_rs/compat/manifest.json` (regenerated)
- Modify: `tests/test_manifest_drift.py` (add the status assertion)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_manifest_drift.py`:

```python
def test_divergences_status_no_longer_claims_unpopulated() -> None:
    manifest = json.loads(DEST.read_text())
    assert manifest["divergences"], "the register is populated; the manifest must say so"
    assert "unpopulated-T0" not in manifest["divergences_status"], (
        "the placeholder status outlived the placeholder — a shipped artifact "
        "describing itself as empty when it is not is worse than an empty one"
    )


def test_manifest_divergences_match_the_authored_register() -> None:
    """The manifest must carry the register verbatim, not a stale copy."""
    manifest = json.loads(DEST.read_text())
    with (ROOT / "compat" / "divergences.toml").open("rb") as fh:
        authored = tomllib.load(fh)["divergence"]
    assert manifest["divergences"] == authored
```

Add `import tomllib` to the file's imports if absent, and confirm `ROOT` is already defined
there — if it is not, derive it as `pathlib.Path(__file__).parent.parent`.

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_manifest_drift.py -q`
Expected: FAIL on `unpopulated-T0`.

- [ ] **Step 3: Update the generator**

In `tools/gen_manifest.py`, bump `"schema_version": 1` to `2` and replace
`divergences_status`:

```python
        "divergences_status": (
            "Populated from compat/divergences.toml, measured against real xgi "
            f"{XGI_TARGET} rather than transcribed from the D1-D17 register, "
            "which covers under half of the live Python-visible surface. Each "
            "entry is a difference that is DELIBERATE and OBSERVABLE; deferred "
            "symbols and missing namespaces are gaps, recorded separately. "
            "A symbol absent from this list is NOT thereby verified equivalent."
        ),
```

That final sentence is load-bearing: it is the difference between "we found no divergence"
and "there is none", and Plan C's `COVERED` rule depends on the distinction.

- [ ] **Step 4: Regenerate and verify**

Run:
```bash
mise run py:manifest
.venv/bin/python -c "
import json, pathlib
m = json.loads(pathlib.Path('python/hypergraph_rs/compat/manifest.json').read_text())
print('schema_version:', m['schema_version'])
print('divergences:', len(m['divergences']))
print('partial_symbols:', len(m['partial_symbols']))
import collections
print('symbol statuses:', dict(collections.Counter(m['symbols'].values())))
print('status:', m['divergences_status'][:80], '...')
"
```
Record the output in your report.

- [ ] **Step 5: Run both full gates**

Run: `mise run py:gate`
Expected: EXIT=0. The **ported-suite** counts must be unchanged from T0
(`316 passed, 1 skipped, 77 xfailed`) — this plan touches no ported test. If they moved,
stop and explain why before continuing. The meta-suite count is higher than T0's, because
Task 1 wired `py:gate` to discover `tests/` and every task since has added guards; report
the number you observe but do not treat any particular value as expected.

Run: `mise run rust:check`
Expected: EXIT=0, 501 tests. Nothing here touches Rust; this is a regression check.

- [ ] **Step 6: Verify the wheel still carries the manifest**

```bash
.venv/bin/python -m pytest tests/test_manifest_drift.py tests/test_divergence_register.py tests/test_partial_symbols.py tests/test_symbol_status.py tests/test_xgi_version_pin.py tests/test_oracle_integrity.py tests/test_oracle_integrity_unit.py tests/test_gate_wiring.py -q
```
Expected: all pass together. Running them as one session catches import-order coupling that
per-file runs hide.

- [ ] **Step 7: Commit**

```bash
git add tools/gen_manifest.py python/hypergraph_rs/compat/manifest.json tests/test_manifest_drift.py
git commit --no-verify -m "feat(compat): manifest v2 — populated register, honest status

Retires the divergences_status placeholder, which claimed the register was
unpopulated after it had been populated. The replacement states the one thing
Plan C's COVERED rule depends on: a symbol absent from the divergence list is
NOT thereby verified equivalent. Silence is not evidence.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Done when

- `mise run py:gate` EXIT=0 with the ported counts unchanged at 316/1/77.
- `mise run rust:check` EXIT=0 with 501 tests.
- `manifest.json` reports `schema_version: 2`, a 35-entry `divergences` list, a non-empty
  `partial_symbols` list, and `bound-partial` among its symbol statuses.
- Every guard added here has been proven by the named mutation, with the result recorded.
- Deleting a ported oracle file fails the suite. Altering a shared fixture fails the suite.
  Both passed silently before this plan.

## Deliberately not in this plan

The `types` layer, `check()`, the 48 submodule shims, `AttrProxy`, `DivergenceWarning` at
divergent sites, and the babylon swap. Plans B and C.
