# XGI compat A2 — cross-register consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every shipped compat artifact agree with every other one, and give
`partial_symbols.toml` a schema that can express the failures it needs to record.

**Architecture:** Five independent tasks over the artifacts Plan A shipped. Task 1
closes a three-way contradiction between generated and authored files. Task 2
widens the partial-symbol schema. Task 3 measures the one gap the schema was
widened for. Task 4 is verified cleanups. Task 5 converts `tools/` to a package —
the only task that changes how the gate is invoked, so it runs last and alone.

**Tech Stack:** Python 3.13 (`.venv`, ABI tag `cp313`), pytest, TOML registers
under `compat/`, `mise` task runner, PyO3 extension (pre-built — **never rebuilt
by this plan**).

**Spec:** `docs/superpowers/specs/2026-07-31-xgi-compat-a2-cross-register-design.md`

> **Revision note, 2026-07-31.** This plan was rewritten after an adversarial
> pre-flight over its own text raised 49 findings across five lenses, of which 15
> were confirmed by independent verifiers and only 2 refuted. Six defects were
> load-bearing: a test selector that could never match, a guard file that flagged
> itself, a `.mise.toml` change that would have broken an existing gate-wiring
> guard, a shell variable consumed in a later step's shell, the wrong wheel named
> first, and two tasks that edited a generator input without regenerating. Every
> fix below is grounded in a command run against the tree at `3ab6860`.

## Global Constraints

Every task's requirements implicitly include this section.

- **Silence is not evidence.** A symbol's absence from a register is NOT a claim
  it is verified equivalent to xgi. No artifact may imply otherwise.
- **Every shipped register entry is verified by direct execution against both
  libraries.** Inferred, simulated, or reasoned-about entries must never ship.
  Task 3 is the one place where "both libraries" needs care — read its Step 4.
- The ported suite must stay at **`316 passed, 1 skipped, 77 xfailed`** and the
  honest denominator at **`492`**, at every task boundary.
- **`gaps_by_class` moves in Task 1 and only in Task 1** (`core-gap` 12→13,
  `deferred-symbol` 51→50). `pass_rate` stays `0.6423` — numerator and
  denominator are untouched. Print before/after and record both in the report.
- **`compat-report.json` is 16 lines.** Read it with `cat compat-report.json`,
  never `tail -14` — that drops the opening brace and the first key, producing
  output that is not valid JSON and hides `denominator`.
- **The gate discovers tests; it does not list them.** `py:gate`'s meta-test
  invocation is `pytest tests/ --ignore=tests/ported -q`, never an explicit file
  list. **Never pin the meta-suite's pass count anywhere.**
- **`py:gate` runs under `set -e` and its second line runs the meta suite.** Any
  step that runs `mise run py:gate` while a deliberately-failing red-phase test
  exists will abort before reaching the report. Capture baselines *before*
  writing red-phase tests.
- Files under `tests/ported/` that have an upstream XGI counterpart are
  **immutable**. Repo-authored files there are editable
  (`tests/ported/test_gate_summary.py` is repo-authored).
- `tests/conftest.py`'s prefix above the
  `# === END VERBATIM XGI 0.10.2 conftest` sentinel is byte-compared against
  upstream and must **never** be edited. Additions go below it. The repo-authored
  section below the sentinel imports `json`, `pathlib` and `pytest`, but **not
  `sys`**.
- The divergence/exclusion taxonomy is a **closed nine-class set** in
  `tools/taxonomy.py`. Every `compat/` register validates against it.
- **Do not rebuild the Rust extension.** Cargo is single-flight in this repo.
  No task here needs a rebuild.
- Python is `.venv/bin/python`. **Never run `import xgi` as a bash command** —
  that invokes ImageMagick's screenshot tool and dumps a multi-MB capture into
  the repo. Use `.venv/bin/python -c "import xgi; ..."`.
- **Never modify babylon's `.venv`.** This repo has its own.
- **No publishing of any kind** — no cargo/PyPI/npm publish, no artifact upload.
- Power-of-10: every loop statically bounded, explicit type annotations, no bare
  `except`, no generic catch-alls, functions under ~100 lines.
- Commit with `git commit --no-verify` and a conventional-commit message. Do not
  use `git -C`. Leave the tree clean; confirm with `git status --porcelain`.

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `tools/gen_manifest.py` | gains `partial_symbol_names()`; `partial_symbols_status` updated | 1, 3 |
| `tools/audit_symbols.py` | applies the promotion when emitting the ledger | 1, 5 |
| `tests/DEFERRED_SYMBOLS.md` | generated; gains a bound-partial section | 1, 3 |
| `compat/exclusions.toml` | one entry reclassified `deferred-symbol` → `core-gap` | 1 |
| `tests/test_cross_register.py` | **new** — pins agreement *between* artifacts | 1, 5 |
| `compat/partial_symbols.toml` | gains `exception` and `class`; one trigger trimmed | 2, 3, 4 |
| `tests/test_partial_symbols.py` | `FIELDS` grows; taxonomy check; `match=`; uniqueness | 2, 4, 5 |
| `pyproject.toml` | gains `[tool.pytest.ini_options] markers` | 4 |
| `tools/__init__.py` | **new** — makes `tools` importable as a package | 5 |
| `tests/test_gate_wiring.py` | its three invocation pins re-pointed at the module | 5 |
| `.mise.toml` | `$PY tools/x.py` → `$PY -m tools.x` | 5 |

---

### Task 1: One promotion, applied everywhere

**Why:** Three shipped artifacts disagree about `degree_assortativity`,
`Hypergraph` and `DiHypergraph`. `audit_symbols.py` reads `symbol_status()` raw
and never applies `_promote_partials`, so its generated ledger shows the
pre-Task-7 picture; `exclusions.toml` independently calls the symbol
`deferred-symbol` while the manifest calls it `bound-partial`.

**Files:**
- Modify: `tools/gen_manifest.py` (add `partial_symbol_names()`)
- Modify: `tools/audit_symbols.py:26-38`
- Modify: `compat/exclusions.toml:364-368` (the `[[test_exclusion]]` block whose
  `test = "test_degree_assortativity"` — locate it by that value, not by line)
- Regenerate: `tests/DEFERRED_SYMBOLS.md`
- Create: `tests/test_cross_register.py`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `gen_manifest.partial_symbol_names() -> set[str]` — the symbols named
  in `compat/partial_symbols.toml`. Task 5 changes its import path to
  `tools.gen_manifest`.

- [ ] **Step 1: Record the baseline FIRST**

This must happen before any red-phase test exists. `py:gate` runs under `set -e`
and its second line runs the meta suite, so once a deliberately-failing test is
on disk the gate aborts before printing the report.

```bash
mise run py:gate && cat compat-report.json
```
Save the JSON verbatim into your report — it is the "before" half of the
constraint that `gaps_by_class` moves only in this task. Expected now:
`core-gap` 12, `deferred-symbol` 51, `denominator` 492, `pass_rate` 0.6423.

- [ ] **Step 2: Write the failing test**

Create `tests/test_cross_register.py`. Note there is **no `sys.path` preamble** —
this file imports nothing from `tools/`, and Task 5 adds a guard that fails any
test file carrying one.

```python
# tests/test_cross_register.py
"""Artifacts that describe the same symbol must not contradict each other."""

import json
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).parent.parent
MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
LEDGER = ROOT / "tests" / "DEFERRED_SYMBOLS.md"


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def _all_exclusions() -> list[dict]:
    """Both arrays. file_exclusion carries its own `class` field and ten of its
    entries are `deferred-symbol`, so a guard reading only test_exclusion would
    miss half the file."""
    with (ROOT / "compat" / "exclusions.toml").open("rb") as fh:
        data = tomllib.load(fh)
    return data.get("test_exclusion", []) + data.get("file_exclusion", [])


def _partial_symbols() -> set[str]:
    return {n for n, s in _manifest()["symbols"].items() if s == "bound-partial"}


def test_the_ledger_does_not_call_a_partial_symbol_plainly_bound() -> None:
    """DEFERRED_SYMBOLS.md is generated. Before this task it listed every
    partial symbol under 'Bound symbols' with no marker, so two generated
    artifacts disagreed."""
    ledger = LEDGER.read_text()
    assert "# Bound symbols" in ledger, "the ledger's section headings changed"
    bound_section = ledger.split("# Bound symbols")[1]
    for symbol in sorted(_partial_symbols()):
        assert f"- `{symbol}`\n" not in bound_section, (
            f"{symbol} is bound-partial in the manifest but the ledger lists "
            f"it as plainly bound"
        )


def test_the_ledger_names_every_partial_symbol() -> None:
    ledger = LEDGER.read_text()
    for symbol in sorted(_partial_symbols()):
        assert f"- `{symbol}`" in ledger, f"{symbol} vanished from the ledger"


def test_no_exclusion_calls_a_bound_partial_symbol_deferred() -> None:
    """Name-based, deliberately: an exclusion whose test or path name embeds a
    symbol the manifest calls bound-partial must not be classed
    `deferred-symbol`. The symbol is bound — only some argument shapes raise."""
    partial = _partial_symbols()
    for excl in _all_exclusions():
        if excl["class"] != "deferred-symbol":
            continue
        subject = excl.get("test") or excl.get("path") or ""
        for symbol in partial:
            assert symbol not in subject, (
                f"{subject} is classed deferred-symbol, but the manifest calls "
                f"{symbol} bound-partial — the binding exists"
            )
```

Because `# Bound symbols` is the ledger's **last** heading after Step 5, taking
everything after it is correct and needs no second split.

- [ ] **Step 3: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_cross_register.py -q`
Expected: FAIL — `test_the_ledger_does_not_call_a_partial_symbol_plainly_bound`
and `test_no_exclusion_calls_a_bound_partial_symbol_deferred` both fail.
`test_the_ledger_names_every_partial_symbol` passes already (the names are
present, just mis-sectioned); that is expected and correct — it guards against
the fix deleting them instead of moving them.

**Do not run `mise run py:gate` from here until Step 7.** It would abort on
these failures.

- [ ] **Step 4: Add the shared derivation to gen_manifest.py**

Insert immediately after `_partial_symbols()` in `tools/gen_manifest.py`:

```python
def partial_symbol_names() -> set[str]:
    """The symbols recorded in compat/partial_symbols.toml.

    One derivation, two consumers: build() promotes them to `bound-partial`,
    and audit_symbols.py separates them out of the plain-bound ledger. When
    these drifted apart, two generated artifacts disagreed.
    """
    return {entry["symbol"] for entry in _partial_symbols()}
```

- [ ] **Step 5: Apply the promotion in audit_symbols.py**

In `tools/audit_symbols.py`, replace everything from
`    from symbol_status import symbol_status` (line 26) through the `print(...)`
call (line 38) with:

```python
    from gen_manifest import partial_symbol_names
    from symbol_status import symbol_status

    status = symbol_status(hr, sorted(want))
    partial = partial_symbol_names()
    deferred = [n for n, s in status.items() if s == "deferred"]
    bound = [n for n, s in status.items() if s == "bound" and n not in partial]
    partials = [n for n, s in status.items() if s == "bound" and n in partial]
    with open("tests/DEFERRED_SYMBOLS.md", "w") as f:
        f.write(f"# Deferred symbols ({len(deferred)}/164 not built by any phase)\n\n")
        for n in deferred:
            f.write(f"- `{n}` — no Phase 0-6 plan built it; calling raises NotImplementedError.\n")
        f.write(f"\n# Bound-partial symbols ({len(partials)}/164)\n\n")
        f.write("These are bound, but some argument shapes raise. See "
                "`compat/partial_symbols.toml` for the exact call shapes.\n\n")
        for n in partials:
            f.write(f"- `{n}`\n")
        f.write(f"\n# Bound symbols ({len(bound)}/164)\n\n")
        f.write("Bound is a BINDING fact, not a behaviour one: it means the "
                "symbol is not routed through the `_deferred` stub factory. "
                "Absence from the partial list above is not evidence a symbol "
                "never raises — it may simply be unmeasured.\n\n")
        for n in bound:
            f.write(f"- `{n}`\n")
    print(f"bound={len(bound)} partial={len(partials)} deferred={len(deferred)} "
          f"total={len(bound)+len(partials)+len(deferred)}")
```

`# Bound symbols` is emitted **last** so the test's single split captures the
whole section. Leave the existing `from gen_manifest import require_xgi_target`
at line 18 alone — Task 5 re-qualifies it.

- [ ] **Step 6: Reclassify the exclusion**

In `compat/exclusions.toml`, find the `[[test_exclusion]]` block whose
`test = "test_degree_assortativity"` (currently at lines 364-368) and change
only its class line:

```toml
class = "core-gap"
```

Leave `path`, `test` and `reason` untouched — the reason already describes a
Rust-core gap ("awaits the Phase-4 rand_pcg pin"), which is exactly why
`core-gap` is the honest class and `binding-gap` is not: the PyO3 binding is
present and `exact=True` works through it.

- [ ] **Step 7: Regenerate and run**

```bash
mise run py:manifest
.venv/bin/python tools/audit_symbols.py
.venv/bin/python -m pytest tests/test_cross_register.py -q
```
Expected: all pass.

- [ ] **Step 8: Confirm the movement is exactly the predicted one**

```bash
mise run py:gate && cat compat-report.json
```
Expected: `core-gap` 12→13, `deferred-symbol` 51→50, **every other class
unchanged**, `denominator` 492, `pass_rate` 0.6423, ported
`316 passed, 1 skipped, 77 xfailed`. Diff this against Step 1's saved JSON and
put both in your report. If any other number moved, stop and report — a second
change is hiding inside this one.

- [ ] **Step 9: Prove the guards by mutation**

Run each, confirm the named failure, then revert:

1. In `audit_symbols.py`, drop `and n not in partial` from the `bound` list
   comprehension and re-run `.venv/bin/python tools/audit_symbols.py`. Expected:
   `test_the_ledger_does_not_call_a_partial_symbol_plainly_bound` fails.
2. In `audit_symbols.py`, change the partials loop to write nothing, and
   regenerate. Expected: `test_the_ledger_names_every_partial_symbol` fails.
3. In `exclusions.toml`, set the class back to `deferred-symbol`. Expected:
   `test_no_exclusion_calls_a_bound_partial_symbol_deferred` fails.

Record the observed failure text for each in your report.

- [ ] **Step 10: Commit**

```bash
git add tools/gen_manifest.py tools/audit_symbols.py compat/exclusions.toml \
        tests/DEFERRED_SYMBOLS.md tests/test_cross_register.py \
        python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "fix(compat): apply the partial promotion in every artifact

audit_symbols.py read symbol_status() raw and never applied the promotion, so
its generated ledger showed the pre-Task-7 picture while manifest.json called
the same three symbols bound-partial. exclusions.toml independently classed
test_degree_assortativity deferred-symbol, which the binding contradicts.

One derivation now feeds both generators. gaps_by_class moves core-gap 12->13
and deferred-symbol 51->50 by design; pass_rate and the denominator are
untouched."
```

---

### Task 2: A schema that can name the failure

**Why:** `partial_symbols.toml` records only *that* a symbol raises, not *what*
it raises. `drawing.*` raises `ImportError`, not `NotImplementedError`, so the
register cannot describe it. The file is also the only `compat/` register not
validated against the closed taxonomy.

**Files:**
- Modify: `compat/partial_symbols.toml` (header prose + schema comment + all four entries)
- Modify: `tests/test_partial_symbols.py` (`FIELDS` at line 15, plus new tests)

**Interfaces:**
- Consumes: `tools/taxonomy.py`'s `CLASSES: frozenset[str]`.
- Produces: `partial_symbols.toml` entries with the field set
  `{"symbol", "class", "exception", "site", "trigger", "reason"}`. Tasks 3 and 4
  both read `entry["exception"]`.

- [ ] **Step 1: Write the failing test**

In `tests/test_partial_symbols.py`, replace line 15 with:

```python
FIELDS = {"symbol", "class", "exception", "site", "trigger", "reason"}
```

and append:

```python
def test_every_entry_carries_a_taxonomy_class() -> None:
    """The nine-class set is closed and shared with exclusions.toml; a register
    outside it cannot be joined against the others."""
    from taxonomy import CLASSES

    for entry in _authored():
        assert entry["class"] in CLASSES, (
            f"{entry['symbol']}: class {entry['class']!r} is not one of "
            f"{sorted(CLASSES)}"
        )


def test_every_entry_names_the_exception_it_raises() -> None:
    """`bound-partial` is not NotImplementedError-specific. The drawing.*
    family raises ImportError, and a register that cannot say so would send a
    consumer without the mpl extra to a symbol reported usable."""
    for entry in _authored():
        assert entry["exception"], f"{entry['symbol']}: empty exception"
        assert entry["exception"] in {e.__name__ for e in (
            NotImplementedError, ImportError, TypeError, ValueError, KeyError,
            AttributeError, RuntimeError,
        )}, f"{entry['symbol']}: {entry['exception']!r} is not a known builtin"


def test_entries_are_unique() -> None:
    """A double-pasted entry is otherwise accepted silently: the symbol map is
    idempotent and the manifest list just grows."""
    seen = [(e["symbol"], e["site"]) for e in _authored()]
    assert len(seen) == len(set(seen)), f"duplicate (symbol, site): {seen}"
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_partial_symbols.py -q`
Expected: FAIL — `test_register_is_non_empty_and_well_formed` fails on the
widened `FIELDS`, and both new field tests fail with `KeyError`.
`test_entries_are_unique` passes already; it is a regression guard, not a
red-phase test.

- [ ] **Step 3: Add both fields to all four entries**

These are **four separate two-line insertions**, not one pasteable block. In
`compat/partial_symbols.toml`, add `class` and `exception` immediately after the
`symbol` line of each `[[partial_symbol]]` block, matching the block to its
values by the `symbol` and `site` values already there:

| Block (identified by) | `class` | `exception` |
|---|---|---|
| `symbol = "degree_assortativity"` | `"core-gap"` | `"NotImplementedError"` |
| `symbol = "DiHypergraph"`, site names the `in_degree` getter | `"binding-gap"` | `"NotImplementedError"` |
| `symbol = "DiHypergraph"`, site names the `out_degree` getter | `"binding-gap"` | `"NotImplementedError"` |
| `symbol = "Hypergraph"`, site names `PyEdgeStat::__call__` | `"binding-gap"` | `"NotImplementedError"` |

So each block gains exactly:

```toml
class = "binding-gap"
exception = "NotImplementedError"
```

with `class` set per the table above.

The classes differ deliberately and must not be made uniform.
`degree_assortativity` is `core-gap` — the Rust core lacks the `rand_pcg` pin,
matching the exclusion reclassified in Task 1. The three Stat sites are
`binding-gap`: the algorithms exist, but `PyEdgeStat::__call__`'s `Attr` arm and
the `DiStat` getters never got a re-parametrization branch at the PyO3 layer.

- [ ] **Step 4: Update the whole header, not just the schema block**

In `compat/partial_symbols.toml`, the header has prose above and below the
schema comment. Both mention `NotImplementedError` specifically and would
contradict the widened schema. Replace the entire comment header — every line
from `# compat/partial_symbols.toml` down to the blank line before the first
`[[partial_symbol]]` — with:

```
# compat/partial_symbols.toml
#
# Symbols the manifest calls `bound` — meaning "not routed through the _deferred
# stub factory" — that nonetheless raise for ordinary arguments. Without this
# file a compat checker reports them as usable. The exception raised is NOT
# always NotImplementedError; each entry names its own.
#
# Schema, one table per site:
#
#   [[partial_symbol]]
#   symbol    = "degree_assortativity"   # the public name in __all__
#   class     = "core-gap"               # one of tools/taxonomy.py CLASSES
#   exception = "NotImplementedError"    # what it actually raises
#   site      = "crates/.../algorithms.rs:163"
#   trigger   = "exact=False"            # the argument shape that raises
#   reason    = "..."                    # what xgi does instead
#
# An entry here promotes the symbol to `bound-partial` in manifest.json. When a
# gap is CLOSED, delete its entry — tests/test_partial_symbols.py pins the
# raising behaviour for the sites it can reach, so a fix that leaves the entry
# breaks the build.
#
# `symbol` must be a name in `hypergraph_rs.__all__` (symbol_status() only
# classifies top-level names — see tools/symbol_status.py). Some sites raise
# inside instance methods reached through a class rather than through a
# top-level callable; for those, `symbol` names the enclosing class and
# `site`/`trigger`/`reason` carry the exact dotted call shape.
#
# A symbol's ABSENCE from this file is NOT a statement that it never raises —
# it may simply be unmeasured.
```

- [ ] **Step 5: Regenerate and run**

```bash
mise run py:manifest
.venv/bin/python -m pytest tests/test_partial_symbols.py tests/test_cross_register.py -q
```
Expected: all pass. The manifest carries `partial_symbols` verbatim, so
regenerating is required — `test_manifest_drift` compares against a fresh
`build()`.

- [ ] **Step 6: Prove the guards by mutation**

1. Set one entry's `class` to `"not-a-class"`. Expected:
   `test_every_entry_carries_a_taxonomy_class` fails naming the bad value.
2. Set one entry's `exception` to `""`. Expected:
   `test_every_entry_names_the_exception_it_raises` fails on the empty check.
3. Duplicate any whole `[[partial_symbol]]` block. Expected:
   `test_entries_are_unique` fails listing the repeated pair.

Revert each. Record the observed failure text.

- [ ] **Step 7: Run the full gate before committing**

```bash
mise run py:gate && cat compat-report.json
```
Expected: ported `316 passed, 1 skipped, 77 xfailed`; `denominator` 492;
`gaps_by_class` **identical to Task 1's post-state** — this task must not move
it. The `class` values added here live in `partial_symbols.toml`, which
`gate_report.py` does not read; confirm that by comparing the JSON.

- [ ] **Step 8: Commit**

```bash
git add compat/partial_symbols.toml tests/test_partial_symbols.py \
        python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "feat(compat): partial symbols carry a class and the exception they raise

bound-partial was NotImplementedError-specific, so the register could not
describe the drawing.* family's ImportError. Adds an exception field across all
four entries and a taxonomy-validated class, making this the last compat/
register to join the closed nine-class set. Adds a uniqueness guard."
```

---

### Task 3: Measure the drawing family

**Why:** `draw`, `draw_nodes` and `draw_hyperedges` are in `__all__` with status
`bound`. They raise `ImportError` without matplotlib, which is installed here —
so the failure has never been observed, only inferred. Under the plan's evidence
rule an inferred entry must not ship. This task observes it or reports BLOCKED.

**Files:**
- Modify: `compat/partial_symbols.toml` (up to three entries, only if observed)
- Modify: `tools/gen_manifest.py` (`partial_symbols_status`, only if measured)
- Regenerate: `tests/DEFERRED_SYMBOLS.md`
- Create: `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md`

**Interfaces:**
- Consumes: Task 2's six-field schema, including `exception`.
- Produces: nothing later tasks import. If this task reports BLOCKED, Tasks 4
  and 5 are unaffected — neither reads the drawing entries.

- [ ] **Step 1: Pick the wheel whose ABI matches**

Two wheels exist and only one is usable. `.venv` is CPython 3.13 (`cp313`);
`target/wheels/…-cp312-cp312-….whl` is a 207 KB pre-Phase-7 stub with the wrong
ABI tag, and `dist/…-cp313-cp313-….whl` is the current 1.3 MB build.

```bash
.venv/bin/python -c "import sys; print('need cp%d%d' % sys.version_info[:2])"
ls -la dist/*.whl target/wheels/*.whl 2>/dev/null
```
Select the wheel whose ABI tag matches the printed value — today that is
`dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl`. **If no matching wheel
exists, stop and report BLOCKED**, naming what you looked for. Building one is a
cargo operation and cargo is single-flight in this repo; it is not authorized by
this plan. Do not run `maturin`, `cargo build`, or `pip install -e .`.

- [ ] **Step 2: Measure, in ONE shell invocation**

Steps do not share shell state, so the scratch path must not be carried in a
variable across steps. Run this whole block as a single command and capture all
of its output:

```bash
set -e
WHEEL=dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl   # from Step 1
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
.venv/bin/python -m venv "$SCRATCH/mpl-free"
"$SCRATCH/mpl-free/bin/pip" install --quiet "$WHEEL"
"$SCRATCH/mpl-free/bin/python" -c "
import importlib.util
print('matplotlib present:', importlib.util.find_spec('matplotlib') is not None)
"
"$SCRATCH/mpl-free/bin/python" -c "
import hypergraph_rs as hrs
H = hrs.Hypergraph()
H.add_edges_from([['a','b'], ['b','c']])
for name in ('draw', 'draw_nodes', 'draw_hyperedges'):
    fn = getattr(hrs, name)
    try:
        fn(H)
    except BaseException as exc:
        print(name, '->', type(exc).__name__, ':', exc)
    else:
        print(name, '-> NO RAISE')
"
```

Expected: `matplotlib present: False`, then one line per symbol. The `trap`
removes the scratch directory when the shell exits, so no later cleanup step is
needed. If `matplotlib present: True`, the wheel pulls it in as a hard
dependency — that is itself the finding; record it and report
DONE_WITH_CONCERNS rather than inventing an entry.

- [ ] **Step 3: Establish what xgi does — and read this carefully**

The usual "run the same call against real xgi" does not work here, and writing
an entry as if it did would be exactly the fabricated evidence this plan
forbids. **xgi 0.10.2 hard-depends on matplotlib** — its metadata carries an
unconditional `Requires-Dist: matplotlib>=3.4`, so xgi cannot be installed in an
mpl-free environment at all. Our wheel makes it optional
(`Requires-Dist: matplotlib>=3.7 ; extra == 'mpl'`, `Provides-Extra: mpl`).

Confirm both facts and record the output:

```bash
.venv/bin/python -c "
import importlib.metadata as md
print('xgi:', [r for r in (md.metadata('xgi').get_all('Requires-Dist') or []) if 'matplotlib' in r])
"
.venv/bin/python -c "
import zipfile
z = zipfile.ZipFile('dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl')
name = [n for n in z.namelist() if n.endswith('METADATA')][0]
for line in z.read(name).decode().splitlines():
    if line.startswith(('Requires-Dist', 'Provides-Extra')):
        print(' ', line)
"
```

The divergence to record is therefore **the packaging asymmetry**, not a
behavioural difference at the call site: in an mpl-free environment xgi is
uninstallable, while `hypergraph_rs` imports cleanly and fails only when the
drawing symbol is called. State it that way in the entries and the evidence
document. Do not claim to have observed xgi raising anything in that
environment — you cannot, and saying so would be false.

- [ ] **Step 4: Write the evidence document**

Create `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md`
containing: the wheel path and ABI tag used, the venv construction command, the
verbatim output of Step 2, the metadata output of Step 3, and one line per
symbol stating the exception type observed. State explicitly that xgi's side is
established from packaging metadata rather than execution, and why. If any
symbol printed `NO RAISE`, say so — it means that symbol is not import-gated and
gets no entry.

- [ ] **Step 5: Add only the entries you observed**

For each symbol that actually raised, append to `compat/partial_symbols.toml`:

```toml
[[partial_symbol]]
symbol = "draw"
class = "env"
exception = "ImportError"
site = "<the module and line from the observed traceback>"
trigger = "any call in an environment without matplotlib installed"
reason = "hypergraph-rs makes matplotlib an optional `mpl` extra, so this symbol imports and raises only at call time. xgi 0.10.2 hard-depends on matplotlib (Requires-Dist: matplotlib>=3.4), so the same environment cannot host xgi at all — the divergence is in packaging, not at the call site."
```

`env` is the taxonomy class for environment-dependent gaps; confirm it is in
`tools/taxonomy.py`'s `CLASSES` before writing it. Use the **observed** site and
exception, never a predicted one. A symbol that printed `NO RAISE` gets no
entry.

**If Step 1 or Step 2 aborted, skip Steps 5 and 6 entirely** and report BLOCKED
or DONE_WITH_CONCERNS. The remaining tasks do not depend on this one.

- [ ] **Step 6: Update partial_symbols_status — only if entries were added**

In `tools/gen_manifest.py`, the string currently ends
`"Known-unmeasured: the drawing.* ImportError family."` — that sentence becomes
false once the entries exist. Replace **only that final sentence** with:

```python
            "unmeasured. Every entry here was observed raising, in the "
            "environment its trigger names."
```

Do **not** delete the preceding "Absence from this list is NOT evidence"
sentence — it remains true and is load-bearing.

- [ ] **Step 7: Regenerate everything the new entries feed**

Adding entries changes `partial_symbol_names()`, which changes both the manifest
and the generated ledger. Both must be regenerated or Task 1's cross-register
guard fails.

```bash
mise run py:manifest
.venv/bin/python tools/audit_symbols.py
.venv/bin/python -m pytest tests/test_partial_symbols.py tests/test_cross_register.py -q
mise run py:gate && cat compat-report.json
```
Expected: all pass; ported `316 passed, 1 skipped, 77 xfailed`; `denominator`
492; `gaps_by_class` unchanged from Task 1's post-state.

- [ ] **Step 8: State the limitation plainly, then commit**

Mutation: change one new entry's `exception` to `NotImplementedError` and re-run
the suite. **This is expected to find no guard.** The register cannot execute an
mpl-free import from inside the normal suite, so these entries are *not*
behaviourally pinned the way the other four are. Record that in your report as a
known limitation — do not imply otherwise.

```bash
git add compat/partial_symbols.toml tools/gen_manifest.py \
        tests/DEFERRED_SYMBOLS.md python/hypergraph_rs/compat/manifest.json \
        docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md
git commit --no-verify -m "feat(compat): register the drawing family's import gate, measured

draw/draw_nodes/draw_hyperedges shipped as plain bound while raising ImportError
without matplotlib, which the resolver's safe-verdict rule would have reported
usable. Measured in a throwaway venv built from the cp313 wheel with no
matplotlib. The divergence is a packaging asymmetry: xgi hard-depends on
matplotlib and cannot be installed in that environment at all, while ours makes
it an optional extra. Entries carry the observed exception and site; symbols
that did not raise get no entry."
```

---

### Task 4: The verified cleanups

**Why:** Three defects the Task 7 review found and one warning class, all
verified still live. None needs a decision; all were deferred only because they
were out of that task's scope.

**Files:**
- Modify: `compat/partial_symbols.toml` (the `Hypergraph` entry's `trigger`)
- Modify: `tests/test_partial_symbols.py` (three `match=` arguments, one new test)
- Modify: `pyproject.toml` (register the `slow` marker)

**Interfaces:**
- Consumes: Task 2's six-field schema — the new test reads `entry["exception"]`,
  which does not exist before Task 2.
- Produces: nothing later tasks import.

- [ ] **Step 1: Write the failing test**

Append to `tests/test_partial_symbols.py`. Note the selector: **no entry's
`site` contains the lowercase substring `attrs`** — the Hypergraph entry's site
reads `…stats.rs:471 (PyEdgeStat::__call__, EdgeStatKind::Attr arm)`, with a
capital `Attr` and no trailing `s`. Select on `PyEdgeStat::__call__`, the same
key Step 4 uses to identify the block.

```python
def test_the_attrs_trigger_does_not_overstate_the_shape() -> None:
    """H.edges.attrs('color')('color') raises TypeError, not the entry's
    recorded exception: PyEdgeStat::__call__ is signature=(degree=None) over
    Option<usize>, so a str positional dies in extraction before the Attr arm.
    The entry must not claim that shape."""
    entries = [e for e in _authored() if "PyEdgeStat::__call__" in e["site"]]
    assert entries, "the Hypergraph.edges.attrs entry vanished"
    for entry in entries:
        assert "or with a new key" not in entry["trigger"], (
            "a new key raises TypeError from PyO3 extraction, not "
            f"{entry['exception']} — the recorded shape is mis-described"
        )
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_partial_symbols.py::test_the_attrs_trigger_does_not_overstate_the_shape -q`
Expected: FAIL on the `or with a new key` assertion — **not** on `assert
entries`. If it fails on `assert entries`, the selector is wrong; stop and
report rather than editing the register to suit the test.

- [ ] **Step 3: Verify the claim before acting on it**

```bash
.venv/bin/python -c "
import hypergraph_rs as hrs
H = hrs.Hypergraph()
H.add_edges_from([({1,2},{'color':'blue'}), ({3,4,5},{'color':'purple'})])
for shape, call in (('zero-arg', lambda: H.edges.attrs('color')()),
                    ('new key',  lambda: H.edges.attrs('color')('color'))):
    try:
        call()
    except BaseException as exc:
        print(shape, '->', type(exc).__name__, ':', exc)
    else:
        print(shape, '-> NO RAISE')
"
```
Expected: `zero-arg -> NotImplementedError`, `new key -> TypeError`. Record the
verbatim output; if it differs, stop and report rather than editing the entry.

- [ ] **Step 4: Trim the trigger**

In `compat/partial_symbols.toml`, in the entry whose site names
`PyEdgeStat::__call__`, replace the `trigger` value with:

```toml
trigger = "Re-invoking the EdgeStat returned by H.edges.attrs(key) with zero arguments — H.edges.attrs('color')(). A str positional instead raises TypeError from PyO3 extraction before the Attr arm is reached. Only the Attr kind raises; Size/Order re-parametrize fine on the same __call__."
```

- [ ] **Step 5: Narrow the three behavioural pins**

The three messages, measured against the current build:

| Call | Message |
|---|---|
| `degree_assortativity(H, exact=False)` | `degree_assortativity(exact=False) (random sampling) is not ported; call with exact=True` |
| `D.nodes.in_degree()` | `this Stat does not accept parameters` |
| `H.edges.attrs('color')()` | `this Stat does not accept parameters` |

**Two of the three are byte-identical.** A per-site distinctive fragment is
therefore impossible, and pretending otherwise would produce a `match=` that
does not discriminate. Use:

- `match="call with exact=True"` for the assortativity pin — genuinely unique.
- `match="does not accept parameters"` for both Stat pins, with a comment saying
  the two sites share one message and that the pin narrows away *unrelated*
  `NotImplementedError`s rather than distinguishing the two sites from each
  other.

Re-run the measurement yourself before writing the fragments — if a message has
changed, use the observed text, not the table.

- [ ] **Step 6: Register the marker**

`pyproject.toml` has no `[tool.pytest.ini_options]` section. Append one:

```toml
[tool.pytest.ini_options]
markers = [
    "slow: upstream XGI's marker for network- or compute-heavy tests. Registered here only to silence PytestUnknownMarkWarning; the ported files that use it are byte-verbatim upstream copies and must not be edited.",
]
```

Adding this section makes pytest treat `pyproject.toml` as its config file.
Confirm in Step 7 that the ported counts are unchanged — if the rootdir or
collection changed, the ported total will move and that must not pass silently.

- [ ] **Step 7: Regenerate and run everything**

The trigger edit changes a generator input: `build()` carries `partial_symbols`
into `manifest.json` verbatim, so `test_manifest_drift` fails without a
regeneration.

```bash
mise run py:manifest
.venv/bin/python -m pytest tests/test_partial_symbols.py tests/test_cross_register.py -q
mise run py:gate && cat compat-report.json
```
Expected: all pass; the three `PytestUnknownMarkWarning` lines are gone; ported
`316 passed, 1 skipped, 77 xfailed`; `denominator` 492; `gaps_by_class`
unchanged from Task 1's post-state.

- [ ] **Step 8: Prove the match= guards by mutation**

For one of the three pins, change its `match=` to a string the real message does
not contain. Expected: that test fails with pytest's "DID NOT MATCH" style
error, proving the argument is load-bearing rather than decorative. Revert.

- [ ] **Step 9: Commit**

```bash
git add compat/partial_symbols.toml tests/test_partial_symbols.py \
        pyproject.toml python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "fix(compat): trim an overstated trigger, narrow the pins, register slow

The attrs entry claimed a new-key re-call raises its recorded exception; it
raises TypeError from PyO3 extraction before the Attr arm. The three behavioural
pins accepted any NotImplementedError; two of the three sites share one message,
so the Stat pins narrow to that shared text and the assortativity pin to its own.
pyproject.toml had no pytest config at all, so upstream's slow marker warned on
every gate run."
```

---

### Task 5: `tools/` becomes a package

**Why:** The `sys.path` preamble is duplicated in fifteen files. Ten are under
`tests/` and five are tools. This task removes the ones that can go and comments
the ones that cannot.

**The preambles are three different things.** Do not treat them uniformly:

| Kind | Files | Inserts | Fate |
|---|---|---|---|
| test → `tools/` | `test_denominator`, `test_divergence_register`, `test_manifest_drift`, `test_partial_symbols`, `test_symbol_status`, `test_xgi_version_pin`, `tests/ported/test_gate_summary` | `…/tools` | removed; imports re-qualified |
| test → `tests/` | `test_ignore_hook`, `test_oracle_integrity_unit`, `test_registry_integrity` | own directory | removed; the shared conftest insert covers them |
| tool → `tests/` | `migrate_exclusions:12`, `count_tests:45` | `…/tests` | **kept**, commented |

The two kept inserts import `tests/conftest_exclusions.py`, and `tests/` must
never become a package — it holds the byte-verbatim oracle conftest. The three
tool→tool inserts (`gen_manifest:19`, `gate_report:20`, `audit_symbols:11`)
dissolve under packaging.

This task changes the gate's command line. It runs last and alone.

**Files:**
- Create: `tools/__init__.py`
- Modify: `tools/gen_manifest.py`, `tools/gate_report.py`, `tools/audit_symbols.py`
- Modify: `tools/migrate_exclusions.py`, `tools/count_tests.py` (comments only)
- Modify: `.mise.toml` (`py:manifest`, `py:gate`)
- Modify: `tests/test_gate_wiring.py` (three assertions pin the old invocation)
- Modify: `tests/conftest.py` (one insert, **below** the sentinel)
- Modify: the ten test files that carry a preamble
- Create: `tests/test_no_duplicate_path_preamble.py`

**Interfaces:**
- Consumes: `gen_manifest.partial_symbol_names()` from Task 1 — its import path
  becomes `tools.gen_manifest`.

- [ ] **Step 1: Write the failing test**

Create `tests/test_no_duplicate_path_preamble.py`. The guard must not match
itself: it lives under `tests/` and necessarily mentions the string it searches
for. Both defences are used — the needle is assembled at runtime, and the file
exempts itself by path.

```python
# tests/test_no_duplicate_path_preamble.py
"""One place puts the repo root on sys.path for tests, not ten."""

import pathlib

ROOT = pathlib.Path(__file__).parent.parent
SELF = pathlib.Path(__file__).resolve()

# Assembled at runtime so this file's own source never contains the literal it
# searches for. Without this the guard flags itself and can never pass.
NEEDLE = "sys.path" + ".insert"

# The two tools that import tests/conftest_exclusions.py. tests/ cannot become
# a package — it holds the byte-verbatim upstream conftest — so these inserts
# are load-bearing and exempt.
EXEMPT: frozenset[str] = frozenset({"migrate_exclusions.py", "count_tests.py"})


def test_no_test_file_inserts_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tests tree.
    for path in sorted((ROOT / "tests").rglob("*.py")):
        if path.name == "conftest.py" or path.resolve() == SELF:
            continue
        if NEEDLE in path.read_text():
            offenders.append(str(path.relative_to(ROOT)))
    assert not offenders, f"these should rely on tests/conftest.py: {offenders}"


def test_only_exempt_tools_insert_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tools directory.
    for path in sorted((ROOT / "tools").glob("*.py")):
        if path.name in EXEMPT:
            continue
        if NEEDLE in path.read_text():
            offenders.append(path.name)
    assert not offenders, f"tools is a package; these need no insert: {offenders}"
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_no_duplicate_path_preamble.py -q`
Expected: FAIL — the first test lists ten test files, the second lists
`gen_manifest.py`, `gate_report.py`, `audit_symbols.py`. If either lists
`test_no_duplicate_path_preamble.py` itself, the self-exemption is broken.

- [ ] **Step 3: Create the package marker**

Create `tools/__init__.py`:

```python
"""Repo tooling: manifest generation, symbol audit, gate reporting.

A package so a tool can import a sibling without a sys.path preamble. Invoked
as `python -m tools.<name>`; see .mise.toml.
"""
```

- [ ] **Step 4: Qualify the tools→tools imports**

In each of `tools/gen_manifest.py`, `tools/gate_report.py`,
`tools/audit_symbols.py`: delete the `sys.path.insert(...)` line that points at
`tools/`, then qualify **every** sibling import. Re-derive the full list rather
than trusting this one — `rg -n "^from (count_tests|symbol_status|gen_manifest|taxonomy) import" tools/`
— but it must at minimum cover:

| File | Before | After |
|---|---|---|
| `gen_manifest.py:21` | `from count_tests import file_exclusion_test_counts` | `from tools.count_tests import …` |
| `gen_manifest.py:22` | `from symbol_status import SymbolStatusError, symbol_status` | `from tools.symbol_status import …` |
| `gate_report.py:22` | `from gen_manifest import DEST` | `from tools.gen_manifest import DEST` |
| `audit_symbols.py:18` | `from gen_manifest import require_xgi_target` | `from tools.gen_manifest import require_xgi_target` |
| `audit_symbols.py` (Task 1) | `from gen_manifest import partial_symbol_names` | `from tools.gen_manifest import …` |
| `audit_symbols.py` (Task 1) | `from symbol_status import symbol_status` | `from tools.symbol_status import …` |

`audit_symbols.py:18` is easy to miss: it sits inside `main()`, above the
imports Task 1 added.

On `# noqa: E402`: remove it only where the import is no longer preceded by a
statement. In `gen_manifest.py` the imports still sit below `ROOT = …`, so E402
still applies there and the comment must stay. Check each site rather than
deleting them all. If removing an insert leaves `sys` unused in a file, remove
that import too; if it is still used elsewhere, leave it.

- [ ] **Step 5: Comment the two exempt inserts**

Above `sys.path.insert` in `tools/migrate_exclusions.py` and in the function in
`tools/count_tests.py`, add:

```python
# Load-bearing: imports tests/conftest_exclusions.py. tests/ must never become
# a package — it holds the byte-verbatim upstream conftest — so this insert
# cannot be replaced by the tools package. tests/test_no_duplicate_path_preamble.py
# exempts these two files by name.
```

- [ ] **Step 6: Update the gate**

In `.mise.toml`, change the `py:manifest` run line:

```
run = ".venv/bin/python -m tools.gen_manifest"
```

and, in `py:gate`, change **only the two tool lines**. The full block, for
context — `set -e`, the `PY` default and the `rm -f` must all stay exactly as
they are, and the two pytest lines are unchanged:

```
set -e
PY="${PY:-.venv/bin/python}"
rm -f .gate-stats.json
$PY -m tools.audit_symbols
$PY -m pytest tests/ --ignore=tests/ported -q
$PY -m pytest tests/ported -q
$PY -m tools.gate_report --from-stats .gate-stats.json
```

- [ ] **Step 7: Re-point the gate-wiring pins**

`tests/test_gate_wiring.py` byte-pins the old invocation string in three places
and will fail the moment Step 6 lands. Re-point each at the tool rather than its
path:

- line 17: `assert "tools/gate_report.py" in _gate_run()` →
  `assert "tools.gate_report" in _gate_run()`
- line 34: `assert run.index("tests/ported") < run.index("tools/gate_report.py")` →
  `assert run.index("tests/ported") < run.index("tools.gate_report")`
- any other assertion naming `tools/audit_symbols.py` or `tools/gate_report.py`
  — re-derive with `rg -n "tools/" tests/test_gate_wiring.py` and fix each.

Keep every assertion's message text: the guard's purpose is unchanged, only the
string it matches. Do not weaken an assertion to a substring that would also
match the old form.

- [ ] **Step 8: Add the single test-side insert**

In `tests/conftest.py`, **below** the
`# === END VERBATIM XGI 0.10.2 conftest — repo-authored code below this line ===`
sentinel and its three comment lines, add:

```python
import sys

# One place, not ten. The repo root lets tests import the `tools` package; the
# tests directory lets the three tests that import sibling helpers (conftest,
# conftest_exclusions) work without their own preamble.
_REPO_ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(_REPO_ROOT / "tests"))
sys.path.insert(0, str(_REPO_ROOT))
```

`pathlib` is already imported below the sentinel (line 350); `sys` is **not**,
which is why the `import sys` line is required. **Never edit above the
sentinel** — `tests/test_oracle_integrity.py` byte-compares that region against
upstream, including trailing whitespace.

- [ ] **Step 9: Strip the ten test preambles**

Re-derive the list first — earlier tasks may have added files:

```bash
rg -n "sys\.path\.insert" tests/
```

For each file, delete the insert line (and `import sys` if nothing else in the
file uses it). Then, **only for the seven files that inserted `tools/`**,
re-qualify their imports to `from tools.<module> import …`. The three that
inserted their own directory (`test_ignore_hook.py`, `test_oracle_integrity_unit.py`,
`test_registry_integrity.py`) import tests-tree modules such as `conftest` and
`conftest_exclusions` — those import statements are already correct and must
**not** be prefixed with `tools.`; Step 8's second insert is what keeps them
working.

- [ ] **Step 10: Run both gates**

```bash
mise run py:manifest
mise run py:gate && cat compat-report.json
mise run rust:check 2>&1 | tail -5
```
Expected: `py:gate` EXIT 0, ported `316 passed, 1 skipped, 77 xfailed`,
`denominator` 492, `gaps_by_class` unchanged from Task 1's post-state.
`rust:check` unaffected. If `py:gate` cannot find a tool, the `-m` conversion is
incomplete — fix it rather than restoring a `sys.path` insert.

- [ ] **Step 11: Prove the guards by mutation**

1. Re-add `sys.path.insert(0, "x")` to any one test file. Expected:
   `test_no_test_file_inserts_into_sys_path` fails naming that file.
2. Re-add one to `tools/gate_report.py`. Expected:
   `test_only_exempt_tools_insert_into_sys_path` fails naming it.
3. Delete the insert from `tools/count_tests.py`, then run — **outside pytest**,
   because under pytest the conftest insert now covers it:
   ```bash
   .venv/bin/python -c "from tools.count_tests import file_exclusion_test_counts; file_exclusion_test_counts()"
   ```
   Expected: `ModuleNotFoundError: No module named 'conftest_exclusions'`, which
   is what proves the exemption is real rather than merely asserted. Restore it.

Record all three observed results. Note in your report that mutation 3 must run
outside pytest — inside it, the conftest insert masks the breakage, and that
masking is itself worth stating.

- [ ] **Step 12: Commit**

```bash
git add tools/ tests/ .mise.toml python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "refactor(tools): make tools a package, one sys.path insert for tests

Fifteen files carried a preamble. tools/ becomes a package so three tool->tool
inserts dissolve, and tests/conftest.py carries one insert covering the ten test
files. Two tool inserts remain and are commented: they import
tests/conftest_exclusions.py, and tests/ must never become a package because it
holds the byte-verbatim upstream conftest.

py:gate now invokes the tools with -m, and test_gate_wiring.py's three
invocation pins are re-pointed at the module rather than the path. The pytest
lines are unchanged."
```

---

## Self-Review

**Spec coverage:** F1→Task 1. F2→Tasks 2 and 3. F3→Task 2. F4→Tasks 2 (M7) and
4 (M1, M2). F5→Task 4. F6/F7→Task 5. Rulings 1, 2→Task 1. Rulings 3, 4→Tasks 3
and 2. Ruling 5→this plan exists. Ruling 6→out of plan scope, handled by the
controller. Ruling 7→Task 5, at the scope F7 establishes.

**Type consistency:** `partial_symbol_names() -> set[str]` is defined in Task 1
and imported in Task 1 only; Task 5 changes its import path to
`tools.gen_manifest`. `FIELDS` is widened once, in Task 2, and Tasks 3 and 4 use
the six-field form.

**Ordering:** Task 1 captures its gate baseline before writing red-phase tests,
because `py:gate` runs under `set -e`. Tasks 2, 3 and 4 each regenerate the
manifest, because each edits a generator input. Task 3 may report BLOCKED
without affecting Tasks 4 or 5. Task 5 runs last because it changes the gate's
command line, and it re-points `test_gate_wiring.py` in the same commit.

**Known limitations, stated rather than hidden:**
- Task 3's entries cannot be behaviourally pinned from inside the normal suite;
  that would require an mpl-free import at test time. Step 8 says so and
  requires the report to say so.
- Task 3's xgi side is established from packaging metadata, not execution,
  because xgi cannot be installed without matplotlib. Step 3 says so explicitly.
- Task 4's two Stat pins share one `match=` string because the two sites emit
  byte-identical messages. Step 5 says so rather than implying the pins
  discriminate between the sites.
- Task 5's mutation 3 only demonstrates the exemption outside pytest; inside it,
  the new conftest insert masks the breakage. Step 11 says so.
