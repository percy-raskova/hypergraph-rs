# Whole-branch review — `feat/xgi-compat-plan-a` vs `main` (2026-08-04)

Audit record of the adversarial review that closed the xgi-compat program
(Plan A T0–T8 plus A2), and of the fix loop that followed it. Scope: the
full branch diff against the merge base `7413f3f` — 41 files, +4848/−159 at
the review head `3029f06`. The review and all fixes ran locally; nothing
merged, nothing pushed.

## Method

Six review dimensions (correctness, docs-and-claims accuracy, test quality,
constraint compliance, register/schema consistency, deferred-debt sweep),
each an independent Opus finder over the diff and the surrounding tree.
Every finding then faced three independent skeptics instructed to refute it
and to default to "refuted" when uncertain; a finding survived only when at
least two of three failed to refute it. 258 agents total. The first run lost
189 verifier agents to a session limit; a cache resume the next morning
replayed the six finders' results and re-ran every lost verifier, so each
finding carries a full three-skeptic verdict.

Process disclosures, for honesty:

- The fix-loop orchestration passed its arguments incorrectly (a JSON string
  where an object belonged), so agent briefs read "Read undefined" where the
  findings-file path belonged. All fourteen agents located the file
  themselves and named the defect in their reports; each report confirmed
  the correct group assignment.
- One fix group (small-fixes) failed its first review round: the reviewer
  caught a self-contradictory failure message in the first attempt, and the
  follow-up commit repaired it. Every other group passed review on the
  first round.
- The stray `uv.lock` that four reviewers flagged appeared as an untracked
  file mid-session (source unattributed) and the owner directed its deletion;
  the orchestrator removed it. The gate neither creates nor reads it.

## Tally

| Dimension | Findings | Confirmed |
|---|---|---|
| deferred-debt sweep | 26 | 13 |
| docs-and-claims | 13 | 4 |
| correctness | 12 | 4 |
| test quality | 12 | 5 |
| constraint compliance | 12 | 1 |
| register/schema | 9 | 4 |
| **total** | **84** | **31** |

Of the 31 confirmed: 13 are debt-status reports (below), and two duplicate
other confirmed findings across dimensions, leaving **16 unique defects** —
1 critical, 5 major, 10 minor. The owner ruled: fix all 16.

## Fixes — all seven groups approved

Each group: one Sonnet implementer, then an Opus reviewer who re-derived
every finding at HEAD, ran the gate personally, and proved each new or
strengthened guard with their own mutations (defect re-introduced, guard
observed red, mutation reverted, tree verified clean).

| Group | Defects | Commits | Reviewer's proof highlight |
|---|---|---|---|
| oracle-integrity | critical: symmetric `_normalise` accepts an un-swapped `import xgi` in a ported file (vacuous green); minor: a fixture redefined below the conftest sentinel evades the byte-prefix pin | `07e9df7` | Rewrote a ported file's import back to `import xgi`: the new guard goes red, the old code stays blind. Shadowed the upstream `h1` fixture below the sentinel: red. |
| ledger-drift | major: `py:gate` regenerated `tests/DEFERRED_SYMBOLS.md` in place before tests read it, so a corrupted committed ledger passed green; minor: `xgi` imported before the friendly missing-oracle guard; minor: stale module docstring | `3f2652e` | Corrupted the committed ledger header: the gate now exits 1 with a stale-ledger message and leaves the file byte-identical. The pre-fix command, run on the same corruption, silently restored it — the defect reproduced exactly. |
| gate-wiring | major: all five guards matched raw substrings of the `py:gate` script, so a fully commented-out gate stayed green end to end, CI included; two `run.index()` calls raised `ValueError` instead of asserting | `5e14ca6` | Commented out every gate command line: five guards passed pre-fix, five fail post-fix, each with its intended message. Four further guard-specific mutations all bite. |
| drawing-divergence | major: `draw_nodes`/`draw_hyperedges` are literal aliases of `draw`, and all three return an `Axes` where xgi returns tuples of artists — no register recorded it; major: the `drawing/test_draw.py` exclusion contradicted the manifest's `bound-partial` | `9b46ea9` | Re-measured the divergence independently (now shipped as D39; register total 36). Ran the 16 excluded tests un-ignored: 8 fail exactly as the new reason claims, 7 defer, 1 passes. Exclusion reclassified `divergence`, a valid taxonomy class. |
| register-claims | major: REV-1's machine-readable fields named a condition broader than the measured divergence; minor: D17's reproduction recipe worked only with id 0 free; minor: an "unconditionally" claim rested on one degenerate probe | `e212288`, `c595ba3` | Swept all 35 edge-size fixtures: the corrected REV-1 condition matches observed divergence with zero counterexamples. Both directions of the evidence-register verbatim pin mutation-proved. |
| small-fixes | four minors: stale "three partial sites" message, near-vacuous substring matching, unsorted iteration in a failure message, two-registry docstring | `8d12a5f`, `b8abd69` | First attempt rejected (its derived count read "0 partial sites …" on the only input that fires); the re-fix made the message count-free. Token-run matcher proved in both directions: catches a real contradiction, ignores the old false positive. |
| bare-pytest | minor: bare `pytest tests/` aborted with six collection errors and ran zero tests | `d456e21`, `24960c5` | Fixed with the pytest-native `pythonpath` ini option — the U14 ruling (no conftest `sys.path` inserts) holds. New guard red-lines all six collection errors on removing the option; the reviewer proved the follow-up's CI portability in a simulated no-`.venv` interpreter. |

## Debts verified by the sweep

The deferred-debt dimension confirmed, at their recorded commits: Task 7
minors M1 (`ed0b7e4`), M2 (`ed0b7e4`), M5 (`320300d`), M6 (`320300d`),
M7 (A2 Task 2), M8 (`320300d`); the `pytest.mark.slow` registration
(`ed0b7e4`); the removal of the 15-file `sys.path` duplication (`b34d0c3`,
none returned); and A2 Task 2's two fixture-shape advisories (closed by
Task 4 as scheduled). The sweep itself mutation-proved the four unmutated
Task 6 divergence guards — all four fail correctly — and discharged that
debt.

## Refuted findings

The following failed their three-skeptic majority. One line each, as
recorded; debt-dimension rows marked "STILL OPEN" are advisories the
skeptics judged inert rather than defects — they remain honestly recorded
in the A2 execution ledger. Two rows (marked ✝) duplicate findings that a
different dimension's copy confirmed; both fixes appear above.

| Location | Dimension | Severity | Claim (first sentence) |
|---|---|---|---|
| `tools/gate_report.py:35` | correctness | major | The gate's final consistency check equates the xfail count with the total number of `test_exclusion` entries, which is only true by accident today. |
| `compat/partial_symbols.toml:70` ✝ | correctness | major | The three drawing entries (lines 70, 78, 86) each assert "the divergence is in packaging, not at the call site". |
| `tools/__init__.py:1` | correctness | major | Promoting the repo-root `tools/` directory to a real package collides with a top-level `tools` namespace package that xgi 0.10.2 itself installs into site-packages. |
| `tools/audit_symbols.py:30` | correctness | minor | `open("tests/DEFERRED_SYMBOLS.md", "w")` uses a cwd-relative path and no `encoding=`. |
| `tests/test_partial_symbols.py:1` | correctness | minor | The three `env`-class drawing entries have no behavioural pin anywhere in the suite. |
| `tests/test_ignore_hook.py:28` | correctness | minor | `assert collection_path.name not in FILE_EXCLUSIONS` compares a bare basename against subdirectory-relative keys. |
| `tests/conftest.py:408` | correctness | minor | `pytest_sessionfinish` writes `.gate-stats.json` on every pytest session rooted in the repo. |
| `.mise.toml:70` | correctness | minor | `py:gate`'s description no longer matches the order of what the task runs. |
| `compat/divergences.toml:262` | claims | major | D37's reason ships an unkept commitment: "other members of this family may exist…" (a sweep it promised before T1's close). |
| `python/hypergraph_rs/compat/manifest.json:284` | claims | major | The shipped `divergences_status` asserts every entry is deliberate and observable, contradicted by the register's own class field. |
| `compat/divergences.toml:4` | claims | minor | The header attributes all entries to one evidence document, but D38 has no counterpart in it. |
| `docs/superpowers/specs/evidence/2026-07-30-divergence-register-measurement.md:11` | claims | minor | The verdict paragraph transposes two counts. |
| `compat/exclusions.toml:368` | claims | minor | The reclassified `test_degree_assortativity` exclusion's reason still opens with the superseded framing. |
| `compat/exclusions.toml:63` ✝ | claims | minor | The `drawing/test_draw.py` file exclusion is still `deferred-symbol` while the manifest calls the draw symbols `bound-partial`. |
| `compat/divergences.toml:251` | claims | minor | D36's `api` names four functions, but the evidence records only three. |
| `tools/gen_manifest.py:7` | claims | minor | The premise "a static capability claim belongs in the wheel" is never verified by any test or probe on this branch. |
| `tests/test_partial_symbols.py:27` | claims | minor | Stale assertion message: "three partial sites…". |
| `tests/test_divergence_register.py:97` | tests | major | The 35-entry divergence register has zero behavioural pins. |
| `tests/test_denominator.py:32` | tests | minor | `test_invisible_total_is_ninety_eight` asserts a sum over a literal dict declared 13 lines above. |
| `tests/test_partial_symbols.py:83` | tests | minor | The three `env`-class entries have no behavioural pin — their only coverage is a TOML-to-manifest equality check. |
| `tests/test_partial_symbols.py:178` | tests | minor | `test_the_attrs_trigger_does_not_overstate_the_shape` polices prose, not behaviour. |
| `tests/test_partial_symbols.py:159` | tests | minor | The `exception` field validates against a hardcoded seven-name builtin allowlist. |
| `tests/test_no_duplicate_path_preamble.py:19` | tests | minor | The `sys.path` guard greps the 51 byte-verbatim upstream oracle files too. |
| `tests/test_ignore_hook.py:28` | tests | minor | The precondition asserts against bare basenames while the hook keys on `/ported/`-relative paths. |
| `tools/gate_report.py:58` | constraints | major | The score is fail-open on failures and errors. |
| `tools/gate_report.py:35` | constraints | major | The xfail cross-check equates two quantities that are not equal by construction. |
| `tools/__init__.py:1` | constraints | major | Making `tools` importable collides with a top-level package the pinned oracle installs. |
| `tools/gate_report.py:50` | constraints | minor | `gaps_by_class` walks test exclusions only, though presented as the report's class breakdown. |
| `tools/audit_symbols.py:10` | constraints | minor | `def main():` carries no annotations, alone among the tools. |
| `tools/audit_symbols.py:30` | constraints | minor | The ledger writes to a cwd-relative path while every sibling tool resolves through `ROOT`. |
| `tools/audit_symbols.py:29` | constraints | minor | A partial entry naming a symbol absent from `__all__` is silently dropped here but raises in `gen_manifest`. |
| `tools/migrate_exclusions.py:75` | constraints | minor | A third, stale copy of the taxonomy survives in this tool's emitted header. |
| `tests/test_partial_symbols.py:27` | constraints | minor | Stale assertion message (duplicate of the claims-dimension row). |
| `compat/partial_symbols.toml:64` | constraints | minor | The three `env`-class drawing entries have no behavioural pin, unlike the four Rust-site entries. |
| `tests/test_cross_register.py:42` | constraints | minor | The `# Bounded:` loop annotation appears in `tools/` but not in tests. |
| `compat/exclusions.toml:368` | registers | minor | The branch's one exclusions edit did not update the entry's superseded reason text. |
| `compat/exclusions.toml:4` | registers | minor | The nine-class taxonomy is re-listed inline as a comment, duplicating `tools/taxonomy.py`. |
| `tests/test_partial_symbols.py:27` | registers | minor | Stale assertion message (third dimension to raise it). |
| `compat/exclusions.toml:54` | registers | minor | The `readwrite/test_bigg_data.py` reason ends "(D7)", now ambiguous against the divergence register's D7. |
| `compat/divergences.toml:3` | registers | minor | The header attributes all entries to one evidence document (duplicate of the claims-dimension row). |
| `.superpowers/sdd/…/progress.md:1` | debts | major | SYSTEMIC: every deferred debt lives only under the ignored `.superpowers/sdd/` tree. Mechanically true, but its universal claims failed — tracked plan and evidence documents do record the advisories. This report is the tracked record going forward. |
| `tests/test_cross_register.py:49` | debts | minor | A2 Task 1 advisory 2 — STILL OPEN; its trigger fired but is now harmless. |
| `tests/test_cross_register.py:65` | debts | minor | A2 Task 1 advisory 3 — STILL OPEN, inert today. |
| `tests/test_cross_register.py:13` | debts | minor | A2 Task 1 advisory 4 — STILL OPEN. |
| `tests/test_partial_symbols.py:165` | debts | minor | Ruff-format deviation advisory — STILL OPEN and under-recorded. |
| `compat/partial_symbols.toml:44` | debts | minor | A2 Task 2 advisory 4 — STILL OPEN, unchanged. |
| `tests/test_partial_symbols.py:201` | debts | minor | A2 Task 4 advisory 2 — STILL OPEN, unchanged. |
| `tests/test_no_duplicate_path_preamble.py:11` | debts | minor | A2 Task 5 advisories 2–4 — all STILL OPEN, confirmed by reading the shipped guard. |
| `tests/test_no_duplicate_path_preamble.py:25` | debts | minor | Unlisted: the `sys.path` guard exempts its own file wholesale. |
| `.mise.toml:78` | debts | minor | A2 Task 5 advisory 5 — STILL OPEN, inert as recorded. |
| `tools/migrate_exclusions.py:19` | debts | minor | A2 Task 5 advisory 6 — STILL OPEN, both items confirmed by running ruff. |
| `docs/superpowers/plans/2026-07-31-xgi-compat-a2-cross-register.md:1249` | debts | minor | A2 Task 5 advisory 1 — fixed in plan text; the shipped behaviour is fully recorded in the plan's Step 8. |
| `docs/superpowers/specs/evidence/2026-07-31-drawing-import-measurement.md:145` | debts | minor | The `env` entries' missing behavioural pin is honestly recorded, in a tracked file. |

## Advisories carried forward

New, from the fix-loop reviewers (none blocked approval; none moves the
gate). These are the open debts this report exists to preserve:

1. The two new red arms in `tests/test_oracle_integrity.py` (un-swapped
   ported import; below-sentinel redefinition) have no unit-test coverage in
   `test_oracle_integrity_unit.py`; in a healthy tree the live assertions
   only ever see the empty case. Two cheap unit tests mirroring the
   reviewer's mutations would pin them. Strongest advisory of the set.
2. `_redefined_below_sentinel` covers top-level definitions; a
   `from X import h1` below the sentinel where the import is itself a
   fixture remains a narrow silent hole.
3. `_effective_gate_run()` has no self-test; simplifying it back to the raw
   string would silently re-open the gate-wiring defect.
4. The gate-wiring guards police comment-stripped text, by sanctioned
   design; a gate disabled without `#` (an inserted `exit 0`, `if false`,
   removing `set -e`) still passes.
5. The `drawing/test_draw.py` exclusion's class has no regression pin: a
   consistent reversion in both `exclusions.toml` and the manifest is
   caught by nothing. Widening `test_no_exclusion_calls_a_bound_partial_
   symbol_deferred` to file exclusions is cheap and false-positive-free on
   today's data.
6. D39's `ours` field cites "0 patches, 4 lines" without naming the
   fixture; the numbers are fixture-bound. D39's `xgi` field's "10–15
   styling `kwargs`" understates two of the three signatures.
7. `NO_EVIDENCE_COUNTERPART = {"D38", "D39"}` is inert — the verbatim pin
   iterates the evidence block's 34 ids, which include neither.
8. The `degree_assortativity` reason's "measured: −0.647" lacks
   fixture/seed provenance.
9. No verbatim pin joins the partial-symbols sweep document's paste-source
   block to `compat/partial_symbols.toml`; the drift class that produced
   one earlier defect remains possible.
10. The evidence document's confidence legend lacks a CORRECTED entry,
    though four rows now carry that label (pre-existing).
11. `tests/test_cross_register.py`'s docstring overstates one clause
    ("structurally unreachable … regardless of matching strategy" — a
    case-folding strategy would reach it, uselessly). Token matching is
    also narrower than substring on glued suffixes and parametrized ids,
    both absent from today's data.
12. `pytest_sessionfinish` now also fires on `--collect-only` sessions,
    overwriting `.gate-stats.json` mid-run; harmless under the gate's
    last-writer ordering, worth a comment if the legs are ever reordered.
13. The bare-pytest guard hard-errors rather than skipping where no pytest
    console script sits beside `sys.executable`, and pins no floor on the
    collected count (464 today).
14. One drawing-review advisory claimed a new Vale `matplotlib` spelling
    finding; verification traced it to a Vale run outside the repo
    (the project vocabulary already accepts the term, and the
    document sits at its expected 70-finding baseline). Recorded as
    resolved, not carried.

## Final gates (verbatim)

`mise run py:gate` — green; ledger audit `bound=87 partial=6 deferred=71
total=164`; meta suite 70 passed; ported suite `316 passed, 1 skipped,
77 xfailed`; zero warnings. `compat-report.json`, complete:

```json
{
  "denominator": 492,
  "gaps_by_class": {
    "binding-gap": 4,
    "core-gap": 13,
    "deferred-symbol": 50,
    "divergence": 2,
    "float-parity": 2,
    "upstream-private": 6
  },
  "invisible": 98,
  "pass_rate": 0.6423,
  "passed": 316,
  "skipped": 1,
  "xfailed": 77
}
```

`mise run rust:check` — finished clean (doc build under `-D warnings`).
`PYTHONHASHSEED=0 .venv/bin/python -m pytest tests/ported -q` —
`316 passed, 1 skipped, 77 xfailed`. Full suite: 386 passed, 1 skipped,
77 xfailed. Bare `pytest tests/` now collects the identical 464 tests.

Invariant notes, disclosed rather than assumed: `gaps_by_class` did not
move despite the exclusion reclassification, because `gate_report.py`
builds it from test exclusions only — the invariant held tighter than the
fix plan disclosed. The meta-suite count rose 69 → 70 (the new bare-pytest
guard); that count is deliberately unpinned. The divergence register grew
35 → 36 (D39).

## Where things stand

Branch head after fixes: `24960c5` (ten fix commits atop the reviewed
`3029f06`). The register now holds 36 divergences, 7 partial-symbol entries
across 6 symbols, and a drift-guarded deferred-symbol ledger. The fix loop
closed every confirmed defect, each under independent re-review; the
refuted list and carried advisories above record what honestly remains.

Merge decision: owner's.
