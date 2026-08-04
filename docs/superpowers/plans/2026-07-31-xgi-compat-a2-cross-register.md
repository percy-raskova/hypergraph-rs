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
  Task 3 is the one place where "both libraries" needs care — read its Step 3.
- The ported suite must stay at **`316 passed, 1 skipped, 77 xfailed`** and the
  honest denominator at **`492`**, at every task boundary.
- **`gaps_by_class` moves in Task 1 and only in Task 1** (`core-gap` 12→13,
  `deferred-symbol` 51→50). `pass_rate` stays `0.6423` — numerator and
  denominator are untouched. Print before/after and record both in the report.
  (The spec's §5 predicted `pass_rate` would move; the measured prediction is
  that it does not — reclassification moves an entry between classes without
  touching the counts. This plan overrides the spec here, deliberately.)
- **`compat-report.json` is 16 lines.** Read it with `cat compat-report.json`,
  never `tail -14` — that drops the opening brace and the first key, producing
  output that is not valid JSON and hides `denominator`.
- **The gate discovers tests; it does not list them.** `py:gate`'s meta-test
  invocation is `pytest tests/ --ignore=tests/ported -q`, never an explicit file
  list. **Never pin the meta-suite's pass count anywhere.**
- **`py:gate` runs under `set -e`, and the meta suite runs before the report**
  (the gate's third command, after the stats cleanup and the symbol audit). Any step that runs `mise run py:gate` while a deliberately-failing
  red-phase test exists will abort before reaching the report. Capture
  baselines *before* writing red-phase tests.
- Files under `tests/ported/` that have an upstream XGI counterpart are
  **immutable**. Repo-authored files there are editable
  (`tests/ported/test_gate_summary.py` is repo-authored).
- `tests/conftest.py`'s prefix above the
  `# === END VERBATIM XGI 0.10.2 conftest` sentinel is byte-compared against
  upstream and must **never** be edited. Additions go below it.
- The divergence/exclusion taxonomy is a **closed nine-class set** in
  `tools/taxonomy.py`. After Task 2, every `compat/` register validates against
  it — today `partial_symbols.toml` is the one that does not, which is Task 2's
  reason to exist.
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
| `tools/gen_manifest.py` | gains `partial_symbol_names()`; `partial_symbols_status` updated | 1, 3, 5 |
| `tools/audit_symbols.py` | applies the promotion when emitting the ledger | 1, 5 |
| `tests/DEFERRED_SYMBOLS.md` | generated; gains a bound-partial section | 1, 3 |
| `compat/exclusions.toml` | one entry reclassified `deferred-symbol` → `core-gap` | 1 |
| `tests/test_cross_register.py` | **new** — pins agreement *between* artifacts | 1 |
| `compat/partial_symbols.toml` | gains `exception` and `class`; one trigger trimmed | 2, 3, 4 |
| `tests/test_partial_symbols.py` | `FIELDS` grows; taxonomy check; `match=`; uniqueness | 2, 4, 5 |
| `pyproject.toml` | gains `[tool.pytest.ini_options] markers` | 4 |
| `tools/__init__.py` | **new** — makes `tools` importable as a package | 5 |
| `tests/test_gate_wiring.py` | its two invocation pins re-pointed at the module | 5 |
| `.mise.toml` | `$PY tools/x.py` → `$PY -m tools.x` | 5 |
| `tools/gate_report.py` | imports qualified; docstring + argv usage re-pointed | 5 |
| `tools/migrate_exclusions.py`, `tools/count_tests.py` | kept inserts gain comments | 5 |
| `tests/test_no_duplicate_path_preamble.py` | **new** — the no-preamble guard | 5 |
| the ten preamble-carrying test files | preambles stripped, imports re-qualified | 5 |
| `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md` | **new** — measurement evidence | 3 |

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
and runs the meta suite (its third command) before the report, so once a
deliberately-failing test is on disk the gate aborts before printing the report.

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


def _test_exclusions() -> list[dict]:
    """`test_exclusion` ONLY, deliberately.

    `file_exclusion` entries are keyed on a path, and the guard below matches
    symbol names as bare substrings. `drawing/test_draw.py` is classed
    `deferred-symbol` and contains the substring `draw`, so once Task 3
    registers `draw` as bound-partial a path-matching guard would fail on a
    collision rather than a contradiction. Test names embed the symbol under
    test; file paths do not, so the heuristic is sound only here.
    """
    with (ROOT / "compat" / "exclusions.toml").open("rb") as fh:
        return tomllib.load(fh).get("test_exclusion", [])


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
        assert f"- `{symbol}`\n" in ledger, f"{symbol} vanished from the ledger"


def test_no_exclusion_calls_a_bound_partial_symbol_deferred() -> None:
    """Name-based, deliberately: a test exclusion whose test name embeds a
    symbol the manifest calls bound-partial must not be classed
    `deferred-symbol`. The symbol is bound — only some argument shapes or
    environments raise."""
    partial = _partial_symbols()
    for excl in _test_exclusions():
        if excl["class"] != "deferred-symbol":
            continue
        for symbol in partial:
            assert symbol not in excl["test"], (
                f"{excl['test']} is classed deferred-symbol, but the manifest "
                f"calls {symbol} bound-partial — the binding exists"
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
        f.write("These are bound, but some argument shapes or environments "
                "raise. See `compat/partial_symbols.toml` for the exact "
                "triggers.\n\n")
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

Mutations 1 and 2 regenerate `tests/DEFERRED_SYMBOLS.md` from deliberately
broken code, and reverting `audit_symbols.py` does not un-write the file.
Task 1 commits nothing until Step 10, so a `git diff` against HEAD cannot
serve as the check — the ledger legitimately differs from HEAD by this
task's own transformation. Prove the round trip by content instead. Before
mutation 1, save Step 7's clean output:

```bash
cp tests/DEFERRED_SYMBOLS.md /tmp/a2-task1-ledger-step7.md
```

After the final revert, regenerate from clean sources and compare:

```bash
.venv/bin/python tools/audit_symbols.py
diff /tmp/a2-task1-ledger-step7.md tests/DEFERRED_SYMBOLS.md
```

Expected: exit 0, no output — the regenerated ledger is byte-identical to
Step 7's clean output, so Step 10 commits the clean-source version.

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
schema comment; the prose above names `NotImplementedError` specifically
(line 4) and would contradict the widened schema. Replace the entire comment
header — every line
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
#   site      = "crates/.../algorithms.rs:163"  # or the symbol's __module__, for env-gated entries with no call site
#   trigger   = "exact=False"            # the argument shape that raises
#   reason    = "..."                    # xgi's side of the gap: its differing behaviour, or the packaging fact when no call-site comparison exists
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
- Modify: `tools/gen_manifest.py` (`partial_symbols_status` and `symbols_status`, only if measured)
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
    except Exception as exc:
        print(name, '->', type(exc).__name__, ':', exc, '| module:', getattr(fn, '__module__', '<none>'))
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
symbol stating the exception type and `__module__` observed. State explicitly that xgi's side is
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
site = "<the symbol's __module__, verbatim from the Step 2 output>"
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

In `tools/gen_manifest.py`, the `"partial_symbols_status"` value currently
reads, in full:

```python
        "partial_symbols_status": (
            "A measured, INCOMPLETE record. Absence from this list is NOT "
            "evidence a symbol is free of raising paths — it may simply be "
            "unmeasured. Known-unmeasured: the drawing.* ImportError family."
        ),
```

The final sentence becomes false once the entries exist. Replace the whole
value so it reads, in full:

```python
        "partial_symbols_status": (
            "A measured, INCOMPLETE record. Absence from this list is NOT "
            "evidence a symbol is free of raising paths — it may simply be "
            "unmeasured. Every entry here was observed raising, in the "
            "environment its trigger names."
        ),
```

The "Absence from this list is NOT evidence" sentence stays — it remains true
and is load-bearing.

In the same edit, widen the sibling `symbols_status` value: its "raise from
Rust for particular arguments" becomes false once the drawing family — which
raises from the Python-side `_require_mpl` import gate, for every argument
shape — joins `bound-partial`. The value currently reads, in full:

```python
        "symbols_status": (
            "`bound` means 'not routed through the _deferred stub factory', NOT "
            "'fully implemented'. Some bound symbols raise from Rust for "
            "particular arguments; only those MEASURED so far are marked "
            "`bound-partial` and recorded in `partial_symbols`. Read `symbols` "
            "together with `exclusions`, `partial_symbols` and `divergences`."
        ),
```

Replace the whole value so it reads, in full:

```python
        "symbols_status": (
            "`bound` means 'not routed through the _deferred stub factory', NOT "
            "'fully implemented'. Some bound symbols raise for particular "
            "arguments (from Rust) or environments (from Python-side import "
            "gates); only those MEASURED so far are marked `bound-partial` and "
            "recorded in `partial_symbols`. Read `symbols` together with "
            "`exclusions`, `partial_symbols` and `divergences`."
        ),
```

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

Mutation: change one new entry's `exception` to `NotImplementedError` and
re-run the suite **without regenerating the manifest**. Expected:
`test_manifest_exposes_the_detail` fails — the authored TOML no longer matches
the committed manifest. That is a serialization-drift guard, not a behavioural
pin: **no test observes the actual exception type**, because the register
cannot execute an mpl-free import from inside the normal suite, so these
entries are *not* behaviourally pinned the way the other four are. Revert the
entry. Record both halves in your report as a known limitation — do not let
the drift failure imply a behavioural guard exists.

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
- Modify: `tests/test_partial_symbols.py` (two new tests, three pins re-derived via a new `_expected()` helper)
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


def test_a_new_key_really_does_raise_typeerror() -> None:
    """The behavioural half. Without this, the test above pins the absence of
    one English phrase — prose, not behaviour. If PyO3 ever gains a str
    overload, the phrase check would stay green while the register's claim
    became wrong in the other direction."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([({1, 2}, {"color": "blue"})])
    with pytest.raises(TypeError):
        H.edges.attrs("color")("color")
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
    except Exception as exc:
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
changed, use the observed text, not the table:

```bash
.venv/bin/python -c "
import hypergraph_rs as hrs
H = hrs.Hypergraph(); H.add_edges_from([['a','b'],['b','c'],['a','c']])
try: hrs.degree_assortativity(H, exact=False)
except NotImplementedError as e: print('assort:', repr(str(e)))
D = hrs.DiHypergraph(); D.add_edges_from([({0,1},{2}), ({1,2},{4})])
try: D.nodes.in_degree()
except NotImplementedError as e: print('distat:', repr(str(e)))
E = hrs.Hypergraph(); E.add_edges_from([({1,2},{'color':'blue'})])
try: E.edges.attrs('color')()
except NotImplementedError as e: print('attrs :', repr(str(e)))
"
```

**Then make the `exception` field load-bearing.** As written, Task 2's
`test_every_entry_names_the_exception_it_raises` only checks that the string is
a known builtin name — nothing joins it to what the site actually raises, so an
entry could claim `ImportError` for a `NotImplementedError` site and no test
would notice. Resolve the expected type from the register instead of hardcoding
it. Rewrite each of the three pins on this pattern:

```python
def _expected(site_fragment: str) -> type[BaseException]:
    """The exception type the register claims for this site."""
    import builtins

    entries = [e for e in _authored() if site_fragment in e["site"]]
    assert len(entries) == 1, f"{site_fragment}: {len(entries)} entries, want 1"
    return getattr(builtins, entries[0]["exception"])


def test_degree_assortativity_exact_false_still_raises() -> None:
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([["a", "b"], ["b", "c"], ["a", "c"]])
    with pytest.raises(_expected("algorithms.rs"), match="call with exact=True"):
        hrs.degree_assortativity(H, exact=False)
```

Use `"algorithms.rs"` for the assortativity site, `"in_degree"` /`"out_degree"`
for the two `DiStat` sites, and `"PyEdgeStat::__call__"` for the attrs site —
confirm each fragment selects exactly one entry before relying on it. Now
changing a register entry's `exception` to a wrong type makes its pin fail —
unless the wrong type is a SUPERCLASS of the real one: `pytest.raises` matches
subclasses, so `RuntimeError` (on Task 2's allow-list) would still pass for a
`NotImplementedError` site. The pins' honest claim is "raises an instance of
the named type"; Step 8's second mutation proves the register value is
load-bearing using a non-ancestor type.

The two `DiStat` fragments imply TWO tests: today's
`test_di_stat_call_still_raises` loops over both stats under one
`pytest.raises`. Split it — one test per stat, each passing its own fragment
to `_expected` — so the single-entry assert holds and each test pins its
site's claimed exception independently.

- [ ] **Step 6: Register the marker**

`pyproject.toml` has no `[tool.pytest.ini_options]` section. Append one:

```toml
[tool.pytest.ini_options]
markers = [
    "slow: upstream XGI's marker for network- or compute-heavy tests. Registered here only to silence PytestUnknownMarkWarning; the ported files that use it are byte-verbatim upstream copies and must not be edited.",
]
```

pytest already treats `pyproject.toml` as its config file (measured:
`--collect-only` prints `configfile: pyproject.toml` today, with no
`[tool.pytest.ini_options]` table present), so adding the table changes what
pytest reads from the file, not which file it reads. Step 7's full run still
re-confirms the ported counts as cheap insurance.

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

1. For one of the three pins, change its `match=` to a string the real message
   does not contain. Expected: that test fails with pytest's "DID NOT MATCH"
   style error, proving the argument is load-bearing rather than decorative.
   Revert.
2. Change one entry's `exception` in `compat/partial_symbols.toml` to a
   non-ancestor wrong type (`ValueError`), run `mise run py:manifest`, then
   `.venv/bin/python -m pytest tests/test_partial_symbols.py -q`. Expected:
   that site's pin fails — the site raises its real exception, which is not a
   `ValueError`. This proves the register value is load-bearing in the pins.
   Do NOT use a superclass such as `RuntimeError` for this mutation:
   `pytest.raises` matches subclasses and the pin would pass. Revert the entry
   and run `mise run py:manifest` again.

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

**The preambles are four different things.** Do not treat them uniformly:

| Kind | Files | Inserts | Fate |
|---|---|---|---|
| test → `tools/` | `test_denominator`, `test_divergence_register`, `test_manifest_drift`, `test_partial_symbols`, `test_symbol_status`, `test_xgi_version_pin`, `tests/ported/test_gate_summary` | `…/tools` | removed; imports re-qualified |
| test → `tests/` | `test_ignore_hook`, `test_oracle_integrity_unit`, `test_registry_integrity` | own directory | removed; pytest's basedir insertion covers them |
| tool → `tests/` | `migrate_exclusions:12`, `count_tests:45` | `…/tests` | **kept**, commented |
| tool → `tools/` | `gen_manifest:19`, `gate_report:20`, `audit_symbols:11` | `…/tools` | removed; packaging dissolves them |

The two kept inserts import `tests/conftest_exclusions.py`, and `tests/` must
never become a package: a `tests/__init__.py` would move pytest's prepend-mode
insertion to the repo root, breaking every bare tests-tree import. (Either
way the byte-verbatim oracle conftest stays untouched — the byte-compare
covers its prefix, which a sibling file cannot alter.) The three
tool→tool inserts (`gen_manifest:19`, `gate_report:20`, `audit_symbols:11`)
dissolve under packaging.

This task changes the gate's command line. It runs last and alone.

**Files:**
- Create: `tools/__init__.py`
- Modify: `tools/gen_manifest.py`, `tools/gate_report.py`, `tools/audit_symbols.py`
- Modify: `tools/migrate_exclusions.py`, `tools/count_tests.py` (comments only)
- Modify: `.mise.toml` (`py:manifest`, `py:gate`)
- Modify: `tests/test_gate_wiring.py` (two assertions pin the old invocation)
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
"""No test file inserts into sys.path; python -m pytest and pytest's own insertions cover imports."""

import pathlib

ROOT = pathlib.Path(__file__).parent.parent
SELF = pathlib.Path(__file__).resolve()

# Assembled at runtime so this file's own source never contains the literal it
# searches for — belt-and-braces alongside the SELF exemption below.
NEEDLE = "sys.path" + ".insert"

# The two tools that import tests/conftest_exclusions.py. tests/ cannot become
# a package (an __init__.py would break pytest's tests-tree imports), so these
# inserts are load-bearing and exempt.
EXEMPT: frozenset[str] = frozenset({"migrate_exclusions.py", "count_tests.py"})


def test_no_test_file_inserts_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tests tree. The sweep
    # includes tests/ported/ upstream files (immutable); measured 2026-08-03,
    # none contain the needle — only repo-authored files did.
    for path in sorted((ROOT / "tests").rglob("*.py")):
        if path.resolve() == SELF:
            continue
        if NEEDLE in path.read_text():
            offenders.append(str(path.relative_to(ROOT)))
    assert not offenders, f"rely on python -m pytest and pytest's own insertions, not per-file inserts: {offenders}"


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

Note: `python -m tools.<name>` would work even without this marker (implicit
namespace package); the explicit `__init__.py` pins the intent, carries the
docstring, and keeps type-checkers treating `tools/` as one package.

- [ ] **Step 4: Qualify the tools→tools imports**

In each of `tools/gen_manifest.py`, `tools/gate_report.py`,
`tools/audit_symbols.py`: delete the `sys.path.insert(...)` line that points at
`tools/`, then qualify **every** sibling import. Re-derive the full list rather
than trusting this one — and note the command must **not** be `^`-anchored,
because three of the six sites are function-level imports indented inside `main()`:

```bash
rg -n "from (count_tests|symbol_status|gen_manifest|taxonomy) import" tools/
```

Expect six hits across three files. The list must at minimum cover:

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
deleting them all.

If removing an insert leaves `sys` **or `pathlib`** unused in a file, remove that
import too; if either is still used elsewhere, leave it. Measured against this
tree: `audit_symbols.py` uses both **only** inside the insert, so it loses both.
`gen_manifest.py` uses `sys` only there but keeps `pathlib` for `ROOT`.
`gate_report.py` uses both elsewhere and keeps both. Re-check rather than
trusting these three lines.

`tools/gate_report.py`'s module docstring documents its own invocation as a path
in two places (lines 10-11: `Usage: python tools/gate_report.py --from-stats
<path>` and the manual-fallback line below it). Direct-path execution stops
working the moment its sibling import is qualified, so update both lines to
`python -m tools.gate_report …` in the same edit. The argv-error message at
`tools/gate_report.py:82-83` (`"usage: gate_report.py <passed> <skipped>
<xfailed>\n   or: gate_report.py --from-stats <path>"`) has the same problem —
re-point both of its lines to `python -m tools.gate_report` too. The comment
at `tests/conftest.py:409` ("…for tools/gate_report.py.") names the consumer
FILE, not an invocation — leave it unchanged.

- [ ] **Step 5: Comment the two exempt inserts**

Above `sys.path.insert` in `tools/migrate_exclusions.py` and in the function in
`tools/count_tests.py`, add:

```python
# Load-bearing: imports tests/conftest_exclusions.py. tests/ must never become
# a package (an __init__.py would move pytest's prepend insertion to the repo
# root and break the bare tests-tree imports), so this insert cannot be
# replaced by the tools package. tests/test_no_duplicate_path_preamble.py
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

`tests/test_gate_wiring.py` byte-pins the old invocation string in **two** places
and will fail the moment Step 6 lands. Re-derive with
`rg -n "tools/" tests/test_gate_wiring.py` (expect exactly two hits, both naming
`tools/gate_report.py`; no assertion names `audit_symbols`), then re-point each
at the tool rather than its path:

- line 17: `assert "tools/gate_report.py" in _gate_run()` →
  `assert "tools.gate_report" in _gate_run()`
- line 34: `assert run.index("tests/ported") < run.index("tools/gate_report.py")` →
  `assert run.index("pytest tests/ported") < run.index("tools.gate_report")`

  The needle gains its `pytest ` prefix deliberately: bare `"tests/ported"`
  first matches the `--ignore=tests/ported` fragment on the META-suite line
  (measured: index 116, vs the ported run at 139), so the old pin held even
  for orderings it exists to forbid. `"pytest tests/ported"` matches only the
  ported-run line.

Keep every assertion's message text: the guard's purpose is unchanged, only the
string it matches. Do not weaken an assertion to a substring that would also
match the old form.

- [ ] **Step 8: Confirm no test-side insert is needed**

Measured 2026-08-03 (probes in the closeout plan Task 1 Step 7, extended by
pass 3): `$PY -m pytest` puts the repo root (CWD) on `sys.path`, which covers
every `tools.` import from tests. FOUR files import tests-tree modules, kept
working by two pytest-side mechanisms: prepend-mode basedir insertion of
`tests/` covers `test_ignore_hook.py`, `test_oracle_integrity_unit.py` and
`test_registry_integrity.py`; for `tests/ported/test_gate_summary.py` (whose
basedir is `tests/ported/`), loading `tests/conftest.py` is what prepends
`tests/` and resolves its `conftest_exclusions` import (measured: it fails
under `--confcutdir=tests/ported`). No mise task invokes bare `pytest`.
`tests/conftest.py` therefore gets NO insert — do not add one. **Stated
limitation, not hidden:** bare `.venv/bin/pytest` exists and adds no CWD
entry, so after this task six of the seven `tools.*`-importing test files
are uncollectible under it (the seventh, `tests/test_partial_symbols.py`,
imports tools only inside test bodies — it collects, and three of its tests
fail at call time); `python -m pytest` is the supported invocation. Put
that in your report's limitations. If a future invocation style needs an
insert, measure first. (Task 4's `[tool.pytest.ini_options]` addition does not
disturb these measurements: `pyproject.toml` is already pytest's configfile,
and a `markers` key has no `sys.path` effect.)

- [ ] **Step 9: Strip the ten test preambles**

Re-derive the list first — earlier tasks may have added files:

```bash
rg -n "sys\.path\.insert" tests/
```

For each file, delete the insert line (and `import sys` or `import pathlib` if
nothing else in the file uses them — check each; four of the seven
tools-inserting files use `pathlib` only in the insert). Then re-derive the
imports — **they are not all at module level
and not all the same shape**:

```bash
rg -n "^\s*(from (count_tests|symbol_status|gen_manifest|taxonomy) import|import (count_tests|symbol_status|gen_manifest|taxonomy)\b)" tests/
```

Re-qualify **only the seven files that inserted `tools/`**. Two import shapes
exist and the correct rewrite differs:

- `from <tool> import <names>` → `from tools.<tool> import <names>`.
- **A bare `import <tool>`** → `from tools import <tool>`, which keeps every
  `<tool>.attr` call site working unchanged. `tests/test_xgi_version_pin.py:12`
  is this shape: `import gen_manifest`, followed by seven attribute accesses
  (four distinct names: `gen_manifest.XGI_TARGET`, `.require_xgi_target()`,
  `.ManifestError`, `.build()`). Rewriting it as
  `from tools.gen_manifest import …` would require editing all seven call
  sites; `from tools import gen_manifest` requires none.

FOUR files import tests-tree modules such as `conftest` and
`conftest_exclusions`: the three that inserted their own directory
(`test_ignore_hook.py`, `test_oracle_integrity_unit.py`,
`test_registry_integrity.py`) plus `tests/ported/test_gate_summary.py`'s
function-level `conftest_exclusions` import. Those statements are already
correct and must **not** be prefixed with `tools.`; pytest's own insertions
keep them working — basedir insertion of `tests/` for the first three, the
`tests/conftest.py` load for the ported file (Step 8 records both
measurements).

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

Run each, confirm the named failure, then revert:

1. Re-add `sys.path.insert(0, "x")` to any one test file, then run
   `.venv/bin/python -m pytest tests/test_no_duplicate_path_preamble.py -q`
   (the guard reads file TEXT, so running only the guard file sidesteps any
   collection error in the mutated file). Expected:
   `test_no_test_file_inserts_into_sys_path` fails naming that file. Revert.
2. Re-add one to `tools/gate_report.py`; same guard-file run. Expected:
   `test_only_exempt_tools_insert_into_sys_path` fails naming it. Revert.
3. In `tools/count_tests.py`, delete the **whole guard** — both the
   `if _tests_dir not in sys.path:` line and its indented
   `sys.path.insert(0, _tests_dir)` body (lines 44-45 today; Step 5's comment
   shifts them — re-derive), leaving
   `from conftest_exclusions import FILE_EXCLUSIONS` as the next statement.
   Deleting only the insert line leaves the `if` with an empty body and yields
   `IndentationError`, not the failure this mutation is testing for. Then run —
   **outside pytest**, because under pytest the basedir insertion of `tests/` covers it:
   ```bash
   .venv/bin/python -c "from tools.count_tests import file_exclusion_test_counts; file_exclusion_test_counts()"
   ```
   Expected: `ModuleNotFoundError: No module named 'conftest_exclusions'`, which
   is what proves the exemption is real rather than merely asserted. Restore
   both lines.
4. In `.mise.toml`, swap the ported-suite line and the gate-report line (so
   `$PY -m tools.gate_report …` runs before `$PY -m pytest tests/ported -q`),
   then run `.venv/bin/python -m pytest tests/test_gate_wiring.py -q`.
   Expected: `test_gate_still_runs_the_ported_suite_last` FAILS. This is the
   discrimination proof the old pin never had — the pre-Step-7 needle
   (`"tests/ported"`) would have PASSED this same mutation against the
   pre-Step-6 gate line, because its first match sits inside the
   `--ignore=tests/ported` fragment. Revert.

After the final revert, prove every mutation is unwound before Step 12 stages
whole directories:

```bash
.venv/bin/python -m pytest tests/test_no_duplicate_path_preamble.py tests/test_gate_wiring.py -q
.venv/bin/python -c "from tools.count_tests import file_exclusion_test_counts; file_exclusion_test_counts()"
```

Expected: every guard test passes and the import round-trip exits 0 —
mutations 1, 2 and 4 unwound, mutation 3's two lines restored. Step 12's
`git add tools/ tests/` stages whole directories, so a forgotten revert would
land in the commit silently; this run is what prevents that.

Record all four observed results. Note in your report that mutation 3 must run
outside pytest — inside it, pytest's basedir insertion masks the breakage, and
that masking is itself worth stating.

- [ ] **Step 12: Commit**

```bash
git add tools/ tests/ .mise.toml python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "refactor(tools): make tools a package; tests need no sys.path inserts

Fifteen files carried a preamble. tools/ becomes a package so three tool->tool
inserts dissolve; the ten test files need no replacement insert — python -m
pytest and pytest's basedir insertion cover them (measured). Two tool inserts
remain and are commented: they import
tests/conftest_exclusions.py, and tests/ must never become a package — an
__init__.py would move pytest's prepend insertion to the repo root and break
the bare tests-tree imports.

py:gate now invokes the tools with -m, and test_gate_wiring.py's two invocation
pins are re-pointed at the module rather than the path. The pytest lines are
unchanged."
```

---

## Self-Review

**Spec coverage:** F1→Task 1. F2→Tasks 2 and 3. F3→Task 2. F4→Task 2 (Step 1's
uniqueness guard) and Task 4 (Step 4's trigger re-description, Step 5's match=
narrowing). F5→Task 4. F6/F7→Task 5. Rulings 1, 2→Task 1. Rulings 3, 4→Tasks 3
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
  pytest's basedir insertion masks the breakage. Step 11 says so.
- After Task 5, six of the seven `tools.*`-importing test files are
  uncollectible under bare `.venv/bin/pytest` (no CWD entry on `sys.path`);
  the seventh, `tests/test_partial_symbols.py`, fails at call time instead.
  `python -m pytest` is the supported invocation. Step 8 says so.

---

## UNFINISHED — resume here

**State at pause, 2026-07-31.** This plan has had TWO adversarial pre-flight
passes. Pass 1 raised 49 findings (15 confirmed, 2 refuted) and drove the
rewrite at `418ab84`. Pass 2 raised 42 findings (17 confirmed, 4 refuted) — of
which **10 were regressions introduced by that rewrite**. Nine of pass 2's
seventeen are applied above. The rest are NOT. Do not execute this plan until
they are.

**Still outstanding, with the pass-2 finding id** *(historical — the
2026-08-03 repair applied every item below; see the RESOLUTION record at the
end of this section)*:

- **U3** — the Self-Review's "Spec coverage" line reads `F4→Tasks 2 (M7) and 4
  (M1, M2)`. `M1`, `M2` and `M7` are identifiers from Plan A's Task 7 review;
  they appear nowhere in this plan or its spec. Replace them with the F-numbers
  or spell the findings out.
- **U12 / U5** — Task 3 Step 5's `site` field says "the module and line from the
  observed traceback", but Step 2's harness prints only
  `type(exc).__name__` and `str(exc)` — it captures **no traceback**, so there is
  no source for that field. Either add `traceback.print_exc()` to the harness or
  drop `site` to the symbol's `__module__`.
- **U6** — Task 2 Step 4's replacement header still defines `reason` as "what xgi
  does instead", a call-site contract Task 3's packaging-asymmetry entries
  structurally cannot satisfy. Widen the definition.
- **U17** — Task 3 Step 6 says replace "only that final sentence" of
  `partial_symbols_status`. The real final sentence is
  `Known-unmeasured: the drawing.* ImportError family.` but the replacement text
  begins mid-sentence with `unmeasured. `, so the boundary is ambiguous. Quote
  the full before/after.
- **U7** — Task 3's two measurement snippets use `except BaseException`, which
  the Global Constraints ban ("no generic catch-alls") with no carve-out stated.
  Either narrow to `Exception` or state the exemption and why.
- **U14** — neither `sys.path.insert` line Task 5 Step 8 adds is exercised by any
  command in the plan: `py:gate` runs `$PY -m pytest`, and `python -m` already
  puts CWD on `sys.path`. Confirm the inserts are load-bearing at all, or the
  whole conftest change may be unnecessary.
- **U16 / U19 / U20** — smaller: `gate_report.py`'s docstring paths (partly
  addressed in Step 4), the `^`-anchor (addressed), the missing re-measurement
  command (addressed). Re-check these three are fully closed.

**The signal worth acting on.** Twelve of pass 2's seventeen confirmed findings
are in **Task 5**, the `tools/` package conversion — the task where the owner
overrode a tests-only recommendation in favour of tests-and-tools. The decision
stands and is not to be re-argued, but a third pre-flight pass should be run
after the remaining fixes land, and it should weight Task 5 heavily. Two passes
have not yet converged on that task.

**Do not skip the third pass.** Pass 2 proved that fixing this plan introduces
new defects at roughly two-thirds the rate it removes them.

**RESOLUTION — 2026-08-03 (closeout plan Task 1).** All eight outstanding
findings applied as surgical edits:

- **U3** spelled out: F4 now maps to Task 2 (Step 1's uniqueness and
  exception guards) and Task 4 (Step 4's trigger re-description, Step 5's
  match= narrowing).
- **U6** widened: `reason` now names xgi's side of the gap — behaviour, or
  the packaging fact when no call-site comparison exists.
- **U7** narrowed to `except Exception` at both sites (Task 3 Step 2, Task 4
  Step 3). Line 866's `type[BaseException]` stayed: a return-type annotation
  feeding `pytest.raises`, not a handler — the catch-all ban does not reach
  it.
- **U12/U5**: the Step 2 harness now prints each symbol's `__module__`, the
  evidence document records it, and `site` cites that observed value — no
  traceback needed.
- **U17** quoted in full: Step 6 now carries the complete before/after
  `partial_symbols_status` value, verified against
  `tools/gen_manifest.py:150-154` on 2026-08-03.
- **U14** resolved by measurement — DROP path. Probes (2026-08-03, repo
  root, `.venv` cp313):
  - A: `.venv/bin/python -m pytest <scratch>/test_cwd.py`, asserting
    `os.getcwd() in sys.path` → `1 passed` (python -m puts CWD on sys.path).
  - B: `.venv/bin/pytest` exists; the same probe → `1 failed` (bare pytest
    adds no CWD). No mise task invokes bare pytest.
  - C: sibling import (`import helper_mod` beside the test) with no insert
    → `1 passed` (prepend-mode basedir insertion).
  - D: no `tests/__init__.py`; no `importmode` override in pyproject.toml,
    pytest.ini, setup.cfg or tox.ini.
  - `tests/conftest.py` contains no `sys.path` text at all, and no conftest
    under tests/ contains the insert needle.
  Eleven edits, not the six the closeout plan predicted — the Kind-table
  row, the Files-list conftest line, and the guard's `conftest.py`
  exemption (removable per the last measurement above) surfaced during
  execution.
- **U16/U19/U20** re-verified closed: the both-docstring-lines instruction
  sits in Task 5 Step 4's final paragraph; the not-`^`-anchored warning
  sits in the same step; Task 3 Step 7 and Task 4 Step 7 both carry
  regenerate-and-rerun blocks.

**THIRD PRE-FLIGHT PASS STILL REQUIRED before execution** — this record
does not lift the do-not-execute bar; only a PASS 3 record below it may do
that.

**PASS 3 — 2026-08-03.** Ten Opus lens finders (five whole-plan, five
Task-5-only), one refute-by-default Opus skeptic per finding. 125 raised,
110 after dedup, 76 verified under the per-lens cap of 8: **45 confirmed, 31
refuted**; the 34 cap-dropped findings were hand-triaged afterwards — none
dismissed unread. After merging cross-scope duplicates: ~30 unique defects,
18 anchored in Task 5. **The Task 5 convergence gate FIRED** (strict
majority), and the owner ruled: absorb the fixes, then run a pass-4
pre-flight over Task 5 alone. All unique confirmed defects plus 16 of the 34
dropped produced surgical edits above (the rest duplicated confirmed fixes).
Headline fixes: `symbols_status` joins Task 3 Step 6 with a full before/after
quote (the CRITICAL); Task 1 Step 9 now regenerates the ledger after its
mutations; Task 4 Step 8 gains a non-ancestor `exception` mutation
(`pytest.raises` matches subclasses); Task 5's stale counts corrected
throughout and the gate-wiring ordering pin re-anchored on
`"pytest tests/ported"` (the bare needle matched `--ignore=tests/ported`
first). Four confirmed findings were regressions from the 2026-08-03 repair
itself — the two-thirds law held again. **THE DO-NOT-EXECUTE BAR REMAINS
UP** until a PASS 4 record below this one clears Task 5.

**PASS 4 — 2026-08-03, Task 5 only.** Five Opus lenses, one refute-by-default
skeptic per finding, cap 8 (one dropped and hand-triaged: the preamble
taxonomy gained its fourth table row). 39 raised, 28 after dedup, 8 confirmed
— collapsing to **five unique defects (2 major, 3 minor)**, down from ~18 in
pass 3, with none of pass 3's stale-count character recurring. The majors
were mutation hygiene: Step 11's mutations 1-2 lacked reverts while Step 12
stages whole directories, and the re-anchored ordering pin shipped without a
discrimination proof — both fixed above (mutation 4 now swaps the gate lines
and expects the pin to FAIL; a post-revert verification block guards the
whole-directory `git add`). Also fixed: the bare-pytest limitation corrected
to six-of-seven (`test_partial_symbols` collects; two tests fail at call
time),
the four-site "tests/ must never become a package" justification replaced
with the real prepend-insertion mechanism, and the guard comment's false
load-bearing claim. A dedicated Opus verifier re-checked all eleven fix edits
against the tree: five CLEAN verdicts; its two cosmetic notes landed above.
**Convergence judgment:** the defect count collapsed and no finding indicts
Task 5's scope, so per the owner's absorb-and-pass-4 ruling the split path is
not taken. **THIS RECORD LIFTS THE DO-NOT-EXECUTE BAR — it clears the plan
for execution.**
