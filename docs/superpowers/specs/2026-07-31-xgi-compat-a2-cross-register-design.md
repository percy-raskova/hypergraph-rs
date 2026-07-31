# XGI compat A2 — cross-register consistency and schema honesty

**Date:** 2026-07-31
**Status:** approved (owner rulings recorded below)
**Predecessor:** `2026-07-30-xgi-compat-layer-t1-design.md` → Plan A, complete 8/8 at `1b0d67e`
**Successor:** Plan B (types layer), Plan C (`check()`) — both specced, unwritten

## 1. Why this exists

Plan A shipped an honest compatibility register: 35 measured divergences, a
`bound-partial` status for symbols that are bound and still raise, and manifest
`schema_version` 2. Its eight per-task reviews raised nine Minor findings that
were deliberately not folded in, because several needed an owner decision rather
than an implementer's judgement. This document records those decisions and the
reasoning behind them.

The through-line is the same one that governs the whole T1 slice: **silence is
not evidence.** Plan A applied that rule inside `manifest.json`. A2 applies it
*across* artifacts — three shipped files currently disagree about the same
symbol — and to the schema itself, which today can only express one kind of
failure.

## 2. Findings that motivate the work

Each verified against the tree at `1b0d67e`, not carried from a stale note.

**F1 — three artifacts contradict each other.** For `degree_assortativity`,
`manifest.json` says `bound-partial`, `tests/DEFERRED_SYMBOLS.md:75` lists it
under "Bound symbols (93/164)" with no marker, and `compat/exclusions.toml:366`
classes it `deferred-symbol`. `Hypergraph` and `DiHypergraph` are in the same
state in the first two. The cause is mechanical: `tools/audit_symbols.py` reads
`symbol_status()` raw and never applies `_promote_partials`, so its generated
artifact shows the pre-Task-7 picture.

**F2 — `bound-partial` cannot express `ImportError`.** `symbols_status` defines
it as `NotImplementedError`-specific. `draw`, `draw_nodes` and `draw_hyperedges`
are all in `__all__` and all carry status `bound`. They raise `ImportError`
without matplotlib. `resolver-design.md:192` emits the safe verdict for anything
`bound` carrying no divergence ids, so Plan C would report them usable to a
consumer with no `mpl` extra. Nothing false ships *today* only because
`partial_symbols_status` names the family as known-unmeasured.

**F3 — `partial_symbols.toml` cannot join the taxonomy.** It is the only
`compat/` register not validated against the closed nine-class set in
`tools/taxonomy.py`. It has no `class` field, and Plan A's Task 7 brief mandates
`set(entry) == FIELDS`, so adding one is a schema change, not an edit.

**F4 — three cheap defects, all still live.** `partial_symbols.toml:46` says the
`Hypergraph.edges.attrs` gap triggers on a re-call "or with a new key", but a
`str` positional dies as `TypeError` inside PyO3 extraction before reaching the
`Attr` arm — the recorded shape is mis-described. The three behavioural pins in
`tests/test_partial_symbols.py` (lines 81, 93, 104) use bare
`pytest.raises(NotImplementedError)` with no `match=`, so any
`NotImplementedError` satisfies them. Neither register has a uniqueness guard;
a double-pasted entry is accepted silently.

**F5 — `pytest.mark.slow` is unregistered.** `pyproject.toml` has no
`[tool.pytest.ini_options]` section at all. Eight uses across three ported
files; three reach the gate as `PytestUnknownMarkWarning`.

**F6 — the `sys.path` preamble is in 15 files, not 6.** Ten test files, five
tools. The earlier "6" was written during Task 3 and never updated.

**F7 — the five tool preambles are two different things.** This narrows F6 and
is the reason ruling 7 cannot be executed as stated. Three
(`gen_manifest.py:19`, `gate_report.py:20`, `audit_symbols.py:11`) insert
`tools/` so a tool can import a sibling tool — these dissolve when `tools/`
becomes a package. Two (`migrate_exclusions.py:12`, `count_tests.py:45`) insert
`tests/` so a tool can import `tests/conftest_exclusions.py` — a cross-tree
import that packaging `tools/` does not touch. Removing those would require
either making `tests/` a package (it holds the byte-verbatim oracle conftest and
must not gain an `__init__.py`) or relocating `conftest_exclusions.py`, which is
imported by the oracle-adjacent test tree. **Both stay, with a comment naming
why they are load-bearing.**

## 3. Owner rulings

Recorded 2026-07-31. Rulings 1–7 answer questions raised by A–D.

| # | Decision | Rationale |
|---|---|---|
| A | Propagate `bound-partial` to **both** `DEFERRED_SYMBOLS.md` and `exclusions.toml` | A reader comparing two shipped files must not find an unexplained contradiction |
| B | Broaden `bound-partial` to **any** exception, via an `exception` field | The schema must be able to say what Plan C needs to know |
| C | Add a taxonomy-validated `class` field to `partial_symbols.toml`; amend `FIELDS` | Every `compat/` register validates against one closed set |
| D | Fix F4 and F5, and consolidate F6 | All verified, all cheap |
| 1 | The reclassified exclusion becomes **`core-gap`**, not `binding-gap` | Its own reason names a missing Rust-core capability (the Phase-4 `rand_pcg` pin). The binding is present and `exact=True` works. `gaps_by_class`: `core-gap` 12→13, `deferred-symbol` 51→50 |
| 2 | **No new** exclusion entries for the `Hypergraph.edges.attrs` / `DiHypergraph.nodes.in_degree` paths | `exclusions.toml` records why a ported *test* is invisible to the gate. No ported test calls those shapes — they were found by measuring the PyO3 surface directly. Their absence is correct |
| 3 | Measure `drawing.*` in a **throwaway venv** built from the wheel with no matplotlib | A `meta_path` blocker fabricates the failure rather than observing it — the same category of evidence refused for finding E. Uninstalling from `.venv` risks leaving the oracle broken mid-run |
| 4 | Broaden the `exception` field **now**, across all four existing entries | Independent of the drawing measurement; avoids a second migration when those entries land |
| 5 | A2 is a **new plan**; Plan A's ledger stays sealed at 8/8 | Task 8 certified that plan green; work after it must not reopen that certification |
| 6 | The whole-branch review runs **after** A2 | One review over the state that actually merges. A2's own tasks still get per-task gates |
| 7 | `sys.path`: **tests and tools** | Owner overrode a tests-only recommendation. See F7 — the achievable scope is 10 test files plus 3 of 5 tools |

## 4. Scope

**In:** `tools/audit_symbols.py` promotion; one `exclusions.toml`
reclassification; the `exception` and `class` fields on
`compat/partial_symbols.toml` with matching guards; a real `drawing.*`
measurement; F4 and F5; the `tools/` package conversion and test-preamble
consolidation.

**Out:** Plan B's types layer. Plan C's `check()` and resolver. Any new
divergence measurement beyond `drawing.*`. Rebuilding the Rust extension —
cargo is single-flight and nothing here needs one.

## 5. Invariants that must survive

Verified at every task boundary, as in Plan A:

- Ported suite: **`316 passed, 1 skipped, 77 xfailed`**
- Honest denominator: **`492`**
- `tests/conftest.py`'s prefix above the sentinel: byte-identical to upstream
- Files under `tests/ported/` with an upstream counterpart: immutable
- The meta-suite's pass count is **never pinned** anywhere

`pass_rate` and `gaps_by_class` **will move** under ruling 1 — that is the
intended effect, and the new values must be re-baselined deliberately and shown
before they are committed, not absorbed silently.

## 6. Risks

**R1 — the `tools/` package conversion changes the gate's command line.**
`.mise.toml` invokes `$PY tools/audit_symbols.py` and
`$PY tools/gate_report.py --from-stats`; both become `-m tools.…`. Tests that
import `from gen_manifest import …` become `from tools.gen_manifest import …`.
This is the highest-regression item in A2 and gets its own task, executed last.

**R2 — the throwaway venv needs the built wheel.** If no current wheel exists,
building one is a cargo operation and must be single-flight. The measurement
task must check for an existing artifact before assuming it can build.

**R3 — `gaps_by_class` moving could mask a second change.** Ruling 1 shifts two
counters. Any other movement in the same commit would be invisible. The
reclassification lands alone, with the before/after printed.

## 7. Evidence gaps

Stated so no reader mistakes silence for coverage:

- `drawing.*` has never been observed failing. Every claim about it in this
  document is an inference from `resolver-design.md` and the `__all__` listing
  until ruling 3's measurement is performed.
- The nine Minor findings are the ones Task 7's and Task 8's reviewers *found*.
  No claim is made that the diff contains no others.
- `gaps_by_class` after ruling 1 is arithmetic from the current counts, not a
  measured value. It must be confirmed by regeneration.
