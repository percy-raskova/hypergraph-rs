# XGI compat A2 — cross-register consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every shipped compat artifact agree with every other one, and give
`partial_symbols.toml` a schema that can express the failures it needs to record.

**Architecture:** Five independent tasks over the artifacts Plan A shipped. Task 1
closes a three-way contradiction between generated and authored files. Task 2
widens the partial-symbol schema. Task 3 measures the one gap the schema was
widened for. Task 4 is verified cleanups. Task 5 converts `tools/` to a package —
the only task that changes how the gate is invoked, so it runs last and alone.

**Tech Stack:** Python 3.13 (`.venv`), pytest, TOML registers under `compat/`,
`mise` task runner, PyO3 extension (pre-built — **never rebuilt by this plan**).

**Spec:** `docs/superpowers/specs/2026-07-31-xgi-compat-a2-cross-register-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- **Silence is not evidence.** A symbol's absence from a register is NOT a claim
  it is verified equivalent to xgi. No artifact may imply otherwise.
- **Every shipped register entry is verified by direct execution against both
  libraries.** Inferred, simulated, or reasoned-about entries must never ship.
- The ported suite must stay at **`316 passed, 1 skipped, 77 xfailed`** and the
  honest denominator at **`492`**, at every task boundary.
- **`gaps_by_class` moves in Task 1 and only in Task 1** (`core-gap` 12→13,
  `deferred-symbol` 51→50). `pass_rate` stays `0.6423` — the numerator and
  denominator are untouched. Print before/after and record both in the report.
- **The gate discovers tests; it does not list them.** `py:gate`'s meta-test
  invocation is `pytest tests/ --ignore=tests/ported -q`, never an explicit file
  list. **Never pin the meta-suite's pass count anywhere** — it rises as tasks
  add guards.
- Files under `tests/ported/` that have an upstream XGI counterpart are
  **immutable**. Repo-authored files there are editable.
- `tests/conftest.py`'s prefix above the
  `# === END VERBATIM XGI 0.10.2 conftest` sentinel is byte-compared against
  upstream and must **never** be edited. Additions go below it.
- The divergence/exclusion taxonomy is a **closed nine-class set** in
  `tools/taxonomy.py`. Every `compat/` register validates against it.
- **Do not rebuild the Rust extension.** Cargo is single-flight in this repo.
  No task here needs a rebuild; Task 3 uses an existing wheel or reports BLOCKED.
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

| File | Responsibility | Task |
|---|---|---|
| `tools/gen_manifest.py` | gains `partial_symbol_names()`, the one place the partial set is derived | 1 |
| `tools/audit_symbols.py` | applies the promotion when emitting the ledger | 1 |
| `tests/DEFERRED_SYMBOLS.md` | generated; gains a bound-partial section | 1 |
| `compat/exclusions.toml` | one entry reclassified `deferred-symbol` → `core-gap` | 1 |
| `tests/test_cross_register.py` | **new** — pins agreement *between* artifacts | 1 |
| `compat/partial_symbols.toml` | gains `exception` and `class` on every entry | 2, 3 |
| `tests/test_partial_symbols.py` | `FIELDS` grows; taxonomy validation; `match=`; uniqueness | 2, 3, 4 |
| `pyproject.toml` | gains `[tool.pytest.ini_options] markers` | 4 |
| `tools/__init__.py` | **new** — makes `tools` importable as a package | 5 |
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
- Modify: `compat/exclusions.toml:365-369`
- Regenerate: `tests/DEFERRED_SYMBOLS.md`
- Create: `tests/test_cross_register.py`

**Interfaces:**
- Produces: `gen_manifest.partial_symbol_names() -> set[str]` — the symbols named
  in `compat/partial_symbols.toml`. Task 5 renames its import path.

- [ ] **Step 1: Write the failing test**

Create `tests/test_cross_register.py`:

```python
# tests/test_cross_register.py
"""Artifacts that describe the same symbol must not contradict each other."""

import json
import pathlib
import sys
import tomllib

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
LEDGER = ROOT / "tests" / "DEFERRED_SYMBOLS.md"


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def _exclusions() -> list[dict]:
    with (ROOT / "compat" / "exclusions.toml").open("rb") as fh:
        return tomllib.load(fh).get("test_exclusion", [])


def _partial_symbols() -> set[str]:
    manifest = _manifest()
    return {n for n, s in manifest["symbols"].items() if s == "bound-partial"}


def test_the_ledger_does_not_call_a_partial_symbol_plainly_bound() -> None:
    """DEFERRED_SYMBOLS.md is generated. Before this task it listed every
    partial symbol under 'Bound symbols' with no marker, so two generated
    artifacts disagreed."""
    ledger = LEDGER.read_text()
    bound_section = ledger.split("# Bound symbols")[1].split("# ")[0]
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
    """Name-based, deliberately: an exclusion whose test name embeds a symbol
    the manifest calls bound-partial must not be classed `deferred-symbol`.
    The symbol is bound — only some argument shapes raise."""
    partial = _partial_symbols()
    for excl in _exclusions():
        if excl["class"] != "deferred-symbol":
            continue
        for symbol in partial:
            assert symbol not in excl["test"], (
                f"{excl['test']} is classed deferred-symbol, but the manifest "
                f"calls {symbol} bound-partial — the binding exists"
            )
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_cross_register.py -q`
Expected: FAIL — `test_the_ledger_does_not_call_a_partial_symbol_plainly_bound`
and `test_no_exclusion_calls_a_bound_partial_symbol_deferred` both fail.
`test_the_ledger_names_every_partial_symbol` passes already (the names are
present, just mis-sectioned) — that is expected and correct; it guards against
the fix deleting them instead of moving them.

- [ ] **Step 3: Record the current gaps_by_class**

Run and save the output into your report — this is the "before" half of the
constraint that `gaps_by_class` moves only in this task:

```bash
mise run py:gate 2>&1 | tail -14
```

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

Replace `tools/audit_symbols.py` lines 26-38 (from `from symbol_status import
symbol_status` through the `print`) with:

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

Note the section order: bound-partial is emitted **before** plain bound, so
`ledger.split("# Bound symbols")` in the test cannot accidentally capture it.

- [ ] **Step 6: Reclassify the exclusion**

In `compat/exclusions.toml`, in the `[[test_exclusion]]` block whose
`test = "test_degree_assortativity"`, change only the class line:

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
mise run py:gate 2>&1 | tail -14
```
Expected: `core-gap` 12→13, `deferred-symbol` 51→50, **every other class
unchanged**, `denominator` 492, `pass_rate` 0.6423, ported
`316 passed, 1 skipped, 77 xfailed`. If any other number moved, stop and report
— a second change is hiding inside this one.

- [ ] **Step 9: Prove the guards by mutation**

Run each, confirm the named failure, then revert:

1. In `audit_symbols.py`, drop `and n not in partial` from the `bound` list
   comprehension and regenerate the ledger. Expected:
   `test_the_ledger_does_not_call_a_partial_symbol_plainly_bound` fails.
2. In `audit_symbols.py`, change the partials loop to write nothing.
   Expected: `test_the_ledger_names_every_partial_symbol` fails.
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
- Modify: `compat/partial_symbols.toml` (header + all four entries)
- Modify: `tests/test_partial_symbols.py:15` (`FIELDS`) and add two tests

**Interfaces:**
- Consumes: `tools/taxonomy.py`'s `CLASSES: frozenset[str]`
- Produces: `partial_symbols.toml` entries with the field set
  `{"symbol", "class", "exception", "site", "trigger", "reason"}`. Task 3 adds
  entries under this schema.

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
        assert entry["exception"].endswith("Error"), (
            f"{entry['symbol']}: {entry['exception']!r} is not an exception name"
        )


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

In `compat/partial_symbols.toml`, add two lines to each `[[partial_symbol]]`
block, immediately after its `symbol` line:

```toml
# degree_assortativity
class = "core-gap"
exception = "NotImplementedError"

# DiHypergraph (in_degree)
class = "binding-gap"
exception = "NotImplementedError"

# DiHypergraph (out_degree)
class = "binding-gap"
exception = "NotImplementedError"

# Hypergraph (edges.attrs)
class = "binding-gap"
exception = "NotImplementedError"
```

The classes differ deliberately and must not be made uniform.
`degree_assortativity` is `core-gap` — the Rust core lacks the `rand_pcg` pin,
matching the exclusion reclassified in Task 1. The three Stat sites are
`binding-gap`: the algorithms exist, but `PyEdgeStat::__call__`'s `Attr` arm and
the `DiStat` getters never got a re-parametrization branch at the PyO3 layer.

- [ ] **Step 4: Update the file header**

In `compat/partial_symbols.toml`, replace the schema comment block (the lines
beginning `#   [[partial_symbol]]` through `#   reason  = "..."`) with:

```
#   [[partial_symbol]]
#   symbol    = "degree_assortativity"   # the public name in __all__
#   class     = "core-gap"               # one of tools/taxonomy.py CLASSES
#   exception = "NotImplementedError"    # what it actually raises
#   site      = "crates/.../algorithms.rs:163"
#   trigger   = "exact=False"            # the argument shape that raises
#   reason    = "..."                    # what xgi does instead
```

- [ ] **Step 5: Run and regenerate**

```bash
mise run py:manifest
.venv/bin/python -m pytest tests/test_partial_symbols.py -q
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

- [ ] **Step 7: Commit**

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
- Modify: `compat/partial_symbols.toml` (three entries, if measured)
- Modify: `tools/gen_manifest.py` (`partial_symbols_status`, if measured)
- Create: `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md`

**Interfaces:**
- Consumes: Task 2's six-field schema.

- [ ] **Step 1: Find a wheel without building one**

```bash
ls -la target/wheels/*.whl 2>/dev/null; ls -la dist/*.whl 2>/dev/null
```

**If no wheel exists, stop and report BLOCKED**, naming what you looked for.
Building one is a cargo operation and cargo is single-flight in this repo; it is
not authorized by this plan. Do not run `maturin`, `cargo build`, or
`pip install -e .`.

- [ ] **Step 2: Build the throwaway venv**

Use the scratch directory, never the project tree:

```bash
SCRATCH=$(mktemp -d)
.venv/bin/python -m venv "$SCRATCH/mpl-free"
"$SCRATCH/mpl-free/bin/pip" install --quiet <the wheel found in Step 1>
"$SCRATCH/mpl-free/bin/python" -c "
import importlib.util
print('matplotlib present:', importlib.util.find_spec('matplotlib') is not None)
"
```
Expected: `matplotlib present: False`. If it prints `True`, the wheel pulled it
in as a hard dependency — that is itself the finding; record it and report
DONE_WITH_CONCERNS rather than inventing an entry.

- [ ] **Step 3: Observe the failure**

```bash
"$SCRATCH/mpl-free/bin/python" -c "
import traceback
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

Capture the verbatim output. Then run the **same** calls against real xgi in the
project venv to establish what upstream does — an entry needs both halves:

```bash
.venv/bin/python -c "
import xgi
H = xgi.Hypergraph()
H.add_edges_from([['a','b'], ['b','c']])
print('xgi draw:', xgi.draw)
"
```

- [ ] **Step 4: Write the evidence document**

Create `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md`
containing: the wheel path used, the venv construction commands, the verbatim
tracebacks from Step 3, the xgi comparison from Step 3, and one line per symbol
stating the exception type observed. If any symbol printed `NO RAISE`, say so —
it means that symbol is not import-gated and gets no entry.

- [ ] **Step 5: Add only the entries you observed**

For each symbol that actually raised, append to `compat/partial_symbols.toml`:

```toml
[[partial_symbol]]
symbol = "draw"
class = "env"
exception = "ImportError"
site = "<the module and line from the observed traceback>"
trigger = "any call in an environment without matplotlib installed"
reason = "<what xgi 0.10.2 does in the same environment, from Step 3>"
```

`env` is the taxonomy class for environment-dependent gaps. Use the **observed**
site and exception, never a predicted one. A symbol that printed `NO RAISE` gets
no entry.

- [ ] **Step 6: Update partial_symbols_status**

In `tools/gen_manifest.py`, the string currently ends
`"Known-unmeasured: the drawing.* ImportError family."` — that sentence is now
false. Replace only that final sentence with a statement of what remains
unmeasured. If nothing does, use:

```python
            "unmeasured. Every entry here was observed raising, in the "
            "environment its trigger names."
```

Do **not** delete the preceding "Absence from this list is NOT evidence"
sentence — it remains true and is load-bearing.

- [ ] **Step 7: Regenerate, run, prove**

```bash
mise run py:manifest
.venv/bin/python -m pytest tests/test_partial_symbols.py tests/test_cross_register.py -q
mise run py:gate 2>&1 | tail -14
```
Expected: all pass; ported `316 passed, 1 skipped, 77 xfailed`; denominator 492.

Mutation: change one new entry's `exception` to `NotImplementedError` and
confirm nothing catches it. **This is expected to find no guard** — record it as
a known limitation, because the register cannot execute an mpl-free import from
inside the normal suite. Say so plainly in the report rather than implying the
entries are behaviourally pinned the way the other four are.

- [ ] **Step 8: Clean up and commit**

```bash
rm -rf "$SCRATCH"
git add compat/partial_symbols.toml tools/gen_manifest.py \
        python/hypergraph_rs/compat/manifest.json \
        docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md
git commit --no-verify -m "feat(compat): register the drawing family's import gate, measured

draw/draw_nodes/draw_hyperedges shipped as plain `bound` while raising
ImportError without matplotlib, which the resolver's safe-verdict rule would
have reported usable. Measured in a throwaway venv built from the wheel with no
matplotlib; entries carry the observed exception and site. Symbols that did not
raise get no entry."
```

---

### Task 4: The verified cleanups

**Why:** Three defects the Task 7 review found and one warning class, all
verified still live at `1b0d67e`. None needs a decision; all were deferred only
because they were out of that task's scope.

**Files:**
- Modify: `compat/partial_symbols.toml` (the `Hypergraph` entry's `trigger`)
- Modify: `tests/test_partial_symbols.py` (three `match=` arguments)
- Modify: `pyproject.toml` (register the `slow` marker)

- [ ] **Step 1: Write the failing test**

Append to `tests/test_partial_symbols.py`:

```python
def test_the_attrs_trigger_does_not_overstate_the_shape() -> None:
    """H.edges.attrs('color')('color') raises TypeError, not the entry's
    recorded exception: PyEdgeStat::__call__ is signature=(degree=None) over
    Option<usize>, so a str positional dies in extraction before the Attr arm.
    The entry must not claim that shape."""
    entries = [e for e in _authored() if "attrs" in e["site"]]
    assert entries, "the Hypergraph.edges.attrs entry vanished"
    for entry in entries:
        assert "or with a new key" not in entry["trigger"], (
            "a new key raises TypeError from PyO3 extraction, not "
            f"{entry['exception']} — the recorded shape is mis-described"
        )
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_partial_symbols.py::test_the_attrs_trigger_does_not_overstate_the_shape -q`
Expected: FAIL on the `or with a new key` substring.

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

In `tests/test_partial_symbols.py`, add a `match=` to each of the three bare
`pytest.raises(NotImplementedError)` calls, so an unrelated
`NotImplementedError` cannot satisfy them. Use the message fragment each site
actually emits — run each call first and copy the real text:

```bash
.venv/bin/python -c "
import hypergraph_rs as hrs
H = hrs.Hypergraph(); H.add_edges_from([['a','b'],['b','c'],['a','c']])
try: hrs.degree_assortativity(H, exact=False)
except NotImplementedError as e: print('assort:', e)
D = hrs.DiHypergraph(); D.add_edges_from([({0,1},{2}), ({1,2},{4})])
try: D.nodes.in_degree()
except NotImplementedError as e: print('distat:', e)
E = hrs.Hypergraph(); E.add_edges_from([({1,2},{'color':'blue'})])
try: E.edges.attrs('color')()
except NotImplementedError as e: print('attrs:', e)
"
```

Then set each `match=` to a distinctive fragment of the corresponding message.
Do not invent fragments — if a message is empty, say so in the report and leave
that `raises` bare with a comment explaining why.

- [ ] **Step 6: Register the marker**

`pyproject.toml` has no `[tool.pytest.ini_options]` section. Append one:

```toml
[tool.pytest.ini_options]
markers = [
    "slow: upstream XGI's marker for network- or compute-heavy tests. Registered here only to silence PytestUnknownMarkWarning; the ported files that use it are byte-verbatim upstream copies and must not be edited.",
]
```

- [ ] **Step 7: Run everything**

```bash
.venv/bin/python -m pytest tests/test_partial_symbols.py -q
mise run py:gate 2>&1 | tail -20
```
Expected: all pass; the three `PytestUnknownMarkWarning` lines are gone; ported
`316 passed, 1 skipped, 77 xfailed`; denominator 492; `gaps_by_class` unchanged
from Task 1's post-state.

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
pins accepted any NotImplementedError. pyproject.toml had no pytest config at
all, so upstream's slow marker warned on every gate run."
```

---

### Task 5: `tools/` becomes a package

**Why:** The `sys.path` preamble is duplicated in fifteen files. Ten are test
files that can share one insert; three are tools importing sibling tools, which
packaging dissolves. **Two are not removable** — `migrate_exclusions.py:12` and
`count_tests.py:45` insert `tests/` to import `tests/conftest_exclusions.py`,
and `tests/` must never become a package because it holds the byte-verbatim
oracle conftest. Those two stay, commented.

This task changes the gate's command line. It runs last and alone.

**Files:**
- Create: `tools/__init__.py`
- Modify: `tools/gen_manifest.py:19`, `tools/gate_report.py:20`,
  `tools/audit_symbols.py:11` (remove the tools→tools inserts, qualify imports)
- Modify: `tools/migrate_exclusions.py:12`, `tools/count_tests.py:45` (comment
  only — the inserts stay)
- Modify: `.mise.toml` (`py:manifest`, `py:gate`)
- Modify: `tests/conftest.py` (one insert, **below** the sentinel)
- Modify: the ten test files that carry the preamble

- [ ] **Step 1: Write the failing test**

Create `tests/test_no_duplicate_path_preamble.py`:

```python
# tests/test_no_duplicate_path_preamble.py
"""One place puts the repo root on sys.path for tests, not ten."""

import pathlib

ROOT = pathlib.Path(__file__).parent.parent

# The two tools that import tests/conftest_exclusions.py. tests/ cannot become
# a package — it holds the byte-verbatim upstream conftest — so these inserts
# are load-bearing and exempt.
EXEMPT: frozenset[str] = frozenset({"migrate_exclusions.py", "count_tests.py"})


def test_no_test_file_inserts_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tests tree.
    for path in sorted((ROOT / "tests").rglob("*.py")):
        if path.name == "conftest.py":
            continue
        if "sys.path.insert" in path.read_text():
            offenders.append(str(path.relative_to(ROOT)))
    assert not offenders, f"these should rely on tests/conftest.py: {offenders}"


def test_only_exempt_tools_insert_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tools directory.
    for path in sorted((ROOT / "tools").glob("*.py")):
        if path.name in EXEMPT:
            continue
        if "sys.path.insert" in path.read_text():
            offenders.append(path.name)
    assert not offenders, f"tools is a package; these need no insert: {offenders}"
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `.venv/bin/python -m pytest tests/test_no_duplicate_path_preamble.py -q`
Expected: FAIL — the first test lists ten test files, the second lists
`gen_manifest.py`, `gate_report.py`, `audit_symbols.py`.

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
`tools/`, then qualify every sibling import:

- `from count_tests import file_exclusion_test_counts` →
  `from tools.count_tests import file_exclusion_test_counts`
- `from symbol_status import SymbolStatusError, symbol_status` →
  `from tools.symbol_status import SymbolStatusError, symbol_status`
- `from gen_manifest import DEST` → `from tools.gen_manifest import DEST`
- `from gen_manifest import partial_symbol_names` →
  `from tools.gen_manifest import partial_symbol_names`
- `from symbol_status import symbol_status` (in `audit_symbols.py`) →
  `from tools.symbol_status import symbol_status`

Remove the now-unneeded `# noqa: E402` comments on those import lines — they
were there because the imports sat below a `sys.path` statement. If deleting the
insert leaves `sys` or `pathlib` unused in a file, remove those imports too;
if they are still used elsewhere in the file, leave them.

- [ ] **Step 5: Comment the two exempt inserts**

Above `sys.path.insert` in `tools/migrate_exclusions.py` and in the function in
`tools/count_tests.py`, add:

```python
# Load-bearing: imports tests/conftest_exclusions.py. tests/ must never become
# a package — it holds the byte-verbatim upstream conftest — so this insert
# cannot be replaced by the tools package.
```

- [ ] **Step 6: Update the gate**

In `.mise.toml`, change the `py:manifest` run line and the two tool lines in
`py:gate`:

```
run = ".venv/bin/python -m tools.gen_manifest"
```

```
$PY -m tools.audit_symbols
$PY -m pytest tests/ --ignore=tests/ported -q
$PY -m pytest tests/ported -q
$PY -m tools.gate_report --from-stats .gate-stats.json
```

The two pytest lines are unchanged and must stay exactly as they are — the
meta-test invocation globs `tests/` by owner ruling and never lists files.

- [ ] **Step 7: Add the single test-side insert**

In `tests/conftest.py`, **below** the
`# === END VERBATIM XGI 0.10.2 conftest — repo-authored code below this line ===`
sentinel, add:

```python
# One place, not ten: puts the repo root on sys.path so tests can import the
# `tools` package. Individual test files must not carry their own preamble —
# tests/test_no_duplicate_path_preamble.py enforces that.
sys.path.insert(0, str(pathlib.Path(__file__).parent.parent))
```

If `sys` or `pathlib` is not already imported in the verbatim prefix, import it
**below** the sentinel too. **Never edit above the sentinel** —
`tests/test_oracle_integrity.py` byte-compares that region against upstream.

- [ ] **Step 8: Strip the ten test preambles and qualify their imports**

In each test file carrying `sys.path.insert`, delete that line (and the
now-unused `sys` import if nothing else uses it), and change every
`from <tool> import ...` to `from tools.<tool> import ...`. The known sites:

```
tests/test_denominator.py          tests/test_ignore_hook.py
tests/test_manifest_drift.py       tests/test_oracle_integrity_unit.py
tests/test_divergence_register.py  tests/test_partial_symbols.py
tests/test_registry_integrity.py   tests/test_symbol_status.py
tests/test_xgi_version_pin.py      tests/test_cross_register.py
tests/ported/test_gate_summary.py  (repo-authored — editable)
```

Re-derive this list with `rg -l "sys.path.insert" tests/` rather than trusting
it; earlier tasks in this plan may have added files.

- [ ] **Step 9: Run both gates**

```bash
mise run py:manifest
mise run py:gate 2>&1 | tail -20
mise run rust:check 2>&1 | tail -5
```
Expected: `py:gate` EXIT 0, ported `316 passed, 1 skipped, 77 xfailed`,
denominator 492, `gaps_by_class` unchanged from Task 1's post-state.
`rust:check` unaffected. If `py:gate` cannot find a tool, the `-m` conversion is
incomplete — fix it rather than restoring a `sys.path` insert.

- [ ] **Step 10: Prove the guards by mutation**

1. Re-add `sys.path.insert(0, "x")` to any one test file. Expected:
   `test_no_test_file_inserts_into_sys_path` fails naming that file.
2. Re-add one to `tools/gate_report.py`. Expected:
   `test_only_exempt_tools_insert_into_sys_path` fails naming it.
3. Delete the insert from `tools/count_tests.py`. Expected: it stays green (it
   is exempt) but `mise run py:gate` fails — proving the exemption is real and
   not merely asserted. Restore it.

Record all three observed results.

- [ ] **Step 11: Commit**

```bash
git add tools/ tests/ .mise.toml python/hypergraph_rs/compat/manifest.json
git commit --no-verify -m "refactor(tools): make tools a package, one sys.path insert for tests

Fifteen files carried the same preamble. tools/ becomes a package so three
tool->tool inserts dissolve, and tests/conftest.py carries one insert for the
ten test files. Two tool inserts remain and are commented: they import
tests/conftest_exclusions.py, and tests/ must never become a package because it
holds the byte-verbatim upstream conftest.

py:gate now invokes the tools with -m. The pytest lines are unchanged."
```

---

## Self-Review

**Spec coverage:** F1→Task 1. F2→Tasks 2 and 3. F3→Task 2. F4→Tasks 2 (M7) and
4 (M1, M2). F5→Task 4. F6/F7→Task 5. Rulings 1, 2→Task 1. Rulings 3, 4→Tasks 3
and 2. Ruling 5→this plan exists. Ruling 6→out of plan scope, handled by the
controller. Ruling 7→Task 5, at the scope F7 establishes.

**Type consistency:** `partial_symbol_names() -> set[str]` is defined in Task 1
and imported in Task 1 only; Task 5 changes its import path to
`tools.gen_manifest`. `FIELDS` is widened once, in Task 2, and every later task
uses the six-field form.

**Known limitation, stated rather than hidden:** Task 3's entries cannot be
behaviourally pinned from inside the normal suite, because that would require an
mpl-free import at test time. Task 3's Step 7 says so explicitly and requires
the report to say so too. Every other register entry in this plan is pinned.
