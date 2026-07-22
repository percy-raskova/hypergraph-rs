# hypergraph-rs port — execution state & continuation handoff

**Written 2026-07-19 ~11:45am EDT, on owner instruction to pause and preserve tokens.**
This file is the single source of truth for resuming the program. Read it top to bottom before
doing anything.

## 2026-07-22 UPDATE — read this FIRST (supersedes stale facts below)

**The repo moved.** This repo now lives at `/home/user/projects/game/hypergraph-rs`
(sibling of the babylon checkout — moved OUT of babylon's tree per the owner's
2026-07-22 instruction; babylon's `.gitignore` keeps `/hypergraph-rs/` as a guard
against re-mounting). Any path below reading
`/home/user/projects/game/babylon/hypergraph-rs` means THIS repo at its new home.

**A second lineage exists — reconciliation is the next agent's first task.**
Branch `conformance-lineage` (in this repo) is a snapshot of babylon's
`feature/hypergraph-rs-phase-0-1` (45 commits, merged into babylon dev
2026-07-22 — full history reachable there via merge commit 764c6229). It forked
from the same Phase 0/1 root as `main` and carries what `main` lacks:

- the executable XGI conformance harness (`crates/hypergraph-rs/conformance/
  generate_fixtures.py` + `xgi_ground_truth.json` + `tests/conformance/main.rs`,
  ~2900 lines of replay tests) with the **divergence register D1-D17** — pinned
  against the XGI **runtime**, never docstrings;
- reviewed Phase-2 error-hardening (MembershipError family, freeze guards D12,
  D9 bulk-removal semantics, D10/D11 counter rules).

`main` raced ahead in breadth (Phases 0-3 done + reviewed, Phase 4 paused at
task 5/19) but did NOT absorb those corrections — e.g. `main` still rejects
empty edges while the register's D1 pins XGI's actual create-empty-edge
behavior (that is precisely Phase 7 Task 0 here). Reconciliation plan:

1. Graft the conformance harness (fixtures generator + JSON + the Rust replay
   tests) from `conformance-lineage` onto `main`; expect the replay tests to
   NOT compile/pass initially — each failure is a real, documented divergence
   between the lineages. Adjudicate each against the D1-D17 register (the
   register wins unless a `main` review explicitly ruled otherwise).
2. Fold register-mandated behavior into `main`'s core (D1 empty edges is the
   known first case; sweep D2-D17 systematically).
3. Wire `rust:fixtures` back into `.mise.toml` once the harness lands (task
   text is preserved in babylon history; the generator runs XGI from babylon's
   venv, read-only oracle use — sanctioned).
4. Only then resume Phase 4 task 6 via the executor (below, unchanged).
Already grafted from the lineage (2026-07-22): `rust-toolchain.toml`
(1.91.1 pin, MSRV floor 1.85 — `cargo test --workspace --locked` verified
green on `main` under the pin).

**Oracle relocation (2026-07-22 ~10:50am):** babylon's venv is now python3.12
(uv-managed) and its `import xgi` is BROKEN — `xopen` resolves `backports.zstd`
to a python3.13 nix-store path (cross-version contamination; babylon-side
issue, not ours to fix). The XGI 0.10.2 oracle now lives in THIS repo's own
venv: `.venv/` (python3.13, gitignored), created via
`uv venv --python 3.13 .venv && uv pip install xgi==0.10.2` — the
Phase 7-sanctioned subrepo venv, provisioned early for the fixtures
generator. Any `rust:fixtures` wiring and oracle probes should use
`.venv/bin/python`, NOT babylon's venv. Also fixed the same day: the stale
`REPO`/`XGI` constants in `plans/phase-executor.workflow.js` (pointed at the
pre-move babylon-tree path).

**Packaging + task runner now exist (2026-07-22).** Root `pyproject.toml`
(maturin backend; module `hypergraph_rs._hypergraph_rs_core`; pure-python
shell at `python/hypergraph_rs/`; version single-sourced from the cargo
workspace), `.mise.toml` (`rust:*` gate ported from babylon + `py:build`),
`.github/workflows/ci.yml` (dormant until a remote exists — publishing stays
an owner decision). **Babylon consumes this repo TODAY** as an opt-in uv
dependency-group: in babylon, `uv sync --group hypergraph` builds the wheel
and `import hypergraph_rs` works (verified end-to-end). Constraints that
follow: (a) babylon's CI materializes a metadata stub for this package —
if this pyproject ever gains **python dependencies** or moves
requires-python, babylon's `.github/workflows/ci.yml` stub must mirror it
or babylon's lock-check goes red (loudly, by design); (b) do not rename the
distribution ("hypergraph-rs") or module without a paired babylon change.

**Babylon as it actually exists on dev (2026-07-22) — the Phase 11 target:**
- Python env is **uv-managed** (Poetry fully retired: hatchling backend,
  `uv.lock`, `.python-version` pins 3.12). Never write `poetry ...` anywhere.
- Its flake builds the player closure via uv2nix; this repo's group is
  deliberately OUTSIDE that closure until Phase 7 + ADR083.
- XGI call sites to swap at Phase 11 (`import xgi` → `import hypergraph_rs as
  xgi`), exactly 7 files: `src/babylon/engine/graph_wrappers.py`,
  `src/babylon/engine/systems/community.py`,
  `src/babylon/domain/bifurcation/{analysis,axis,bridges,consciousness}.py`,
  `src/babylon/engine/bifurcation_monitor.py`; then drop `xgi` from
  `[project.dependencies]` and add an `import xgi` ban.
- Minimal binding surface those sites need (Phase 7 priority order):
  `Hypergraph()`, `add_node`, `add_edge(members, idx=..., **attrs)`, `.nodes`
  (contains/iterate/`memberships()`), `.edges` (contains/iterate/`members()`/
  `[id]` attrs), `num_edges`, and
  `incidence_matrix(H, sparse=False, index=True) -> (ndarray, rowdict, coldict)`.
- Acceptance for the swap: babylon `mise run qa:regression` byte-identical +
  `nix build .#babylon` green (group still excluded from the closure).
- The design spec + plans on the babylon side live at
  `docs/superpowers/specs/2026-07-18-hypergraph-rs-design.md` (now carries the
  D1-D17 register) and `docs/superpowers/plans/2026-07-18-hypergraph-rs-*.md`;
  where babylon's "Phases 3-11" roadmap doc and this repo's `plans/` disagree,
  **this repo's plans are canonical** (they are the corrected, preflight-
  reviewed versions; the babylon roadmap predates the fork).

## Program goal

Owner-set session goal: **"the rust port is completed and all phases are complete"** — execute
Phases 0–10 of the hypergraph-rs port (standalone Rust port of XGI 0.10.2 as a rustworkx-core
plugin). Phase 11 (Babylon swap) stays deferred to ADR083. Umbrella directive: babylon
`.opencode/goal.md`.

This repo (`/home/user/projects/game/babylon/hypergraph-rs`) is its OWN git repo (branch
`main`), parent-ignored by babylon's `.gitignore` (`/hypergraph-rs/`, the `infra/` pattern).
No remote — publishing of any kind is an owner decision.

## 2026-07-22 UPDATE 2 — lineage reconciliation DONE

The `conformance-lineage` reconciliation (13 tasks, plan `plans/reconciliation-plan.md`) is
**complete**: HEAD is `703b334` after this close-out commit lands. The executable XGI 0.10.2
conformance harness (`crates/hypergraph-rs/conformance/generate_fixtures.py` +
`xgi_ground_truth.json` + `tests/conformance/main.rs`, 57 replay tests) is grafted onto `main`
and green under `mise run rust:check` (296 tests total across the workspace, conformance
target included). The D1-D17 divergence register and rulings ledger live in
`plans/reconciliation-plan.md` §2 ("Rulings ledger") — that is now the durable record of every
core-vs-lineage adjudication; no separate rulings file exists for this program.

Consequence for the phase table below: **Phase 7 Task 0** ("core `add_empty_edge`") is
**subsumed** by reconciliation Task 3 (D1 — empty edges/simplices are now first-class; XGI
create-empty-edge semantics landed on `main`, `EdgeError::EmptyMembers` removed). Phase 7's
plan text still lists it; skip it when Phase 7 executes and start from its Task 1.

## 2026-07-22 UPDATE 3 — Phase 4 DONE (+ review fixes)

Phase 4 (19 tasks) completed via the executor (run `wf_d77f7d10-cef`): tasks 6-19 executed
live (37daba5..e8af8b4), tasks 1-5 verified as pre-committed. Opus review verdict was
fix-needed (1 blocker, 1 major, 3 lesser); all fixed oracle-verified in `05d7a6f`
(comb/u128 overflow guards, zero-combination order skip, BIGG reaction-name attrs, JSON
numeric member sort; review's JSON "name"-default claim was REFUTED by direct oracle probe
and pinned as-is). Workspace: 353 tests green, full `mise run rust:check` clean.

**Sequencing note:** Phase 6 executes next. Phase 5 (layout/viz) is HELD pending the
owner's ruling on `plans/PROPOSAL-raster-replan.md` (TUI/raster re-scope of Phases
5/8/9/10, incl. the Phase R cell rasterizer + Lawverian 3D instruments).

## 2026-07-22 UPDATE 4 — Phase 6 DONE, review APPROVE

Phase 6 (17 tasks, run `wf_b9542453-5b5`) completed 0370e1d..e3632da; Opus review verdict
**approve** — first clean phase review of the program. Workspace: 406 tests green, full
gate clean. Review's two doc findings fixed in the plan's deviations block same day; one
advisory PARKED (add to end-of-program ceremony list): `simulate_kuramoto` panics (index
OOB) on caller-supplied omega/theta shorter than n — XGI also rejects (numpy shape error),
parity preserved; optionally add a typed LengthMismatch error or document the precondition.

**Program position:** Phases 0-4 + 6 done; reconciliation done. Remaining: Phase 5 (HELD on
raster-proposal ruling), Phase 7 (needs Phase 5 for its layout/render bindings), 8/9/10
(shape depends on same ruling). The critical path now blocks on the owner's
PROPOSAL-raster-replan.md decision.

## 2026-07-22 UPDATE 5 — proposal APPROVED; Phase 5 DONE (+review fixes); Phase R mapped

Owner rulings (2026-07-22 afternoon): PROPOSAL-raster-replan.md **APPROVED as proposed**
(Phase 8 deferred, Phase 10 cancelled, Phase 9 promoted, Phase R after Phase 7; H3
first-class snapping deferred to Phase R). Phase 5 executed with owner-amendment Task 12
(ordering providers paoh_ordering + levi_ego_tree) — 12/12 tasks (3596eec..36532ac, run
wf_51643f61-287); review fix-needed on one blocker (plan-supplied 3.14 literal vs
clippy approx_constant) + paoh docstring accuracy — both fixed same day; 428 tests
green, full gate clean. PARKED advisory (end-of-program ceremony): consider gating
`rand` as optional to shrink the no-default-features footprint. `plans/PHASE-R-MAP.md`
records the complete deck→Rust component map for Phase R (reference artifact deck v3,
verification stages 0-3, four open owner decisions). Next: Phase 7 (skip Task 0 — D1
subsumed it; executor runs Task 1 onward).

**Phase 4 resumes at task 6** (tasks 1-5 were already committed pre-reconciliation and are
untouched by any reconciliation task; the reconciliation work landed entirely in `crates/`
core, `tests/conformance/`, and `.mise.toml`, not in `phase-4-generators-stats-readwrite.md`'s
task list).

## State at pause

| Phase | Plan file (plans/) | Status | Evidence |
|---|---|---|---|
| 0+1 workspace + core | phase-0-1-workspace-core.md | **DONE + review approve** | 64 tests; commits b604c6d..767267e |
| 2 dihypergraph/simplicial/views | phase-2-dihypergraph-simplicial-views.md | **DONE + review approve** | 15/15 tasks, 135 tests, HEAD was `da208f4` |
| 3 linalg + algorithms | phase-3-linalg-algorithms.md | **DONE + review fixed** | 12/12 tasks (`a88c8e1`..`0308138`); review said fix-needed → all 4 findings fixed in `5fdb4a4` + `5630357`; 188 tests, clippy `--all-targets` clean |
| 4 generators/stats/readwrite | phase-4-generators-stats-readwrite.md | **PAUSED: tasks 1–5 of 19 committed; resumes at task 6** | `f65cc0e` `d9d80c7` `467b3ac` `af61224` `3f7ec00`; tree clean; run `wf_2cbb34ad-267` stopped mid-task-6; unaffected by reconciliation (703b334) |
| 5 layout + viz | phase-5-layout-viz.md | not started | 11 tasks |
| 6 convert/dynamics/communities/utils | phase-6-convert-dynamics-communities-utils.md | not started | 17 tasks |
| 7 python conformance | phase-7-python-conformance.md | not started | 20 tasks; **Task 0 (core `add_empty_edge`) already subsumed by reconciliation D1 (`703b334`) — start at Task 1** |
| 8 wasm | phase-8-wasm.md | not started | 8 tasks |
| 9 cli | phase-9-cli.md | not started | 11 tasks |
| 10 react package | phase-10-react-package.md | not started | 6 tasks |

All Phase 2–10 plans in `plans/` are the **corrected** versions (preflight-reviewed, defects
fixed, orchestrator rulings folded in — the rulings ledger is
`scratchpad/phase-plans/rulings-phase-7-10.md` in the OLD session scratchpad, but everything
binding from it is already IN the plan texts, so the plans stand alone).

## How to continue

### The executor

`plans/phase-executor.workflow.js` (durable copy of the session executor script) runs ONE
phase: sequential Sonnet TDD task agents (effort medium) + a final Opus review (effort high)
with full-workspace gates (`cargo test --workspace`; `cargo clippy --workspace --all-targets
-- -D warnings` — `--all-targets` is mandatory, plain clippy skips test targets and hid a
Phase 3 defect; `cargo fmt --all -- --check`).

Invoke per phase with the Workflow tool:

```
Workflow({
  scriptPath: "<path to phase-executor.workflow.js>",
  args: {"phase": N, "plan": "/home/user/projects/game/babylon/hypergraph-rs/plans/<file>.md", "tasks": T}
})
```

Task counts: P4=19, P5=11, P6=17, P7=20, P8=8, P9=11, P10=6.

**GOTCHA (burned twice):** the Workflow tool may deliver `args` as a JSON STRING. The script
already guards (`typeof args === 'string' ? JSON.parse(args) : args` + loud throw), keep that
guard if the script is ever rewritten. Never trust an "approve" whose results array is empty.

### Resuming Phase 4 specifically

- **Same session** (session id `86ea08c6-f377-43cb-a0c3-b48c4e14e93f`): resume with
  `resumeFromRunId: "wf_2cbb34ad-267"` and the session script path
  `~/.claude/projects/-home-user-projects-game-babylon--claude-worktrees-docs-hypergraph-rs-design/86ea08c6-f377-43cb-a0c3-b48c4e14e93f/workflows/scripts/hypergraph-rs-phase-execute-wf_9d04fc12-c8b.js`
  and the exact same args (`{"phase": 4, "plan": ".../plans/phase-4-generators-stats-readwrite.md", "tasks": 19}`).
  Tasks 1–5 replay from cache; task 6 onward runs live. (The script was edited after launch —
  only the REVIEW prompt changed, so task-agent cache hits are unaffected.)
- **New session** (resumeFromRunId is same-session-only): invoke
  `plans/phase-executor.workflow.js` fresh with the same args. Task agents 1–5 will each find
  their work already committed and self-heal — this exact pattern worked in Phase 2 (the task-2
  agent detected task 1 pre-committed and simply proceeded). Slightly wasteful, fully safe:
  every completed task is a git commit; the tree is clean.

### After each phase completes

1. Read the review verdict from the workflow result (or
   `<session>/subagents/workflows/<runId>/journal.jsonl` — the review record has
   `verdict`/`gates`/`findings`).
2. **approve** → launch the next phase.
3. **fix-needed** → fix blockers/majors BEFORE the next phase (Phase 3 precedent: one Sonnet
   agent, TDD, oracle values computed by running real XGI read-only from babylon's venv
   `/home/user/projects/game/babylon/.venv/bin/python`). Advisories: fix if semantic
   (XGI-divergence) — Phase 7's conformance gate will catch them anyway but fixing early is
   cheaper — else park (list below).
4. Phases run strictly ONE AT A TIME (cargo single-flight, dev-box safety).

## Hard constraints (unchanged, non-negotiable)

- **NO publishing ever**: no npm/cargo/PyPI/twine/gh-release/wasm-pack publish. Build,
  pack-dry-run, smoke only. Publishing (and any remote/GitHub repo for this subrepo) = owner
  decision.
- **Cargo single-flight**: one build workflow at a time; parallel agents only for read-only
  doc/plan work. Never fan out anything that runs pytest/cargo concurrently.
- **Phase 7 creates the subrepo's OWN venv** (`python3.13 -m venv .venv` in the subrepo);
  NEVER touch babylon's `.venv` (read-only oracle use is fine).
- Subagent models: NEVER Fable; Sonnet for mechanical/execution, Opus for review/synthesis
  (always set `model:` explicitly).
- Subrepo commits: plain `git commit` (no hooks), conventional messages, trailer
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`, always verify HEAD moved.
- XGI oracle corpus (51 test files): OLD-session scratchpad
  `/tmp/claude-1000/-home-user-projects-game-babylon--claude-worktrees-docs-hypergraph-rs-design/a9bdcfa3-d5c2-4825-afe9-6cfbcdc5df12/scratchpad/xgi-0.10.2/tests/`.
  If /tmp was wiped: shallow-clone github.com/xgi-org/xgi at tag v0.10.2 (Phase 7's plan also
  fetches `tests/readwrite/data/` itself). Installed xgi package for symbol inventory:
  babylon `.venv/lib/python3.13/site-packages/xgi`.
- wasm-pack is NOT installed; Phase 8 Task includes its install (prebuilt route).
  wasm32-unknown-unknown target IS installed; maturin 1.14.1 via pipx; node v25.9/npm 11.12.

## Parked advisories (fold into the end-of-program ceremony)

From Phase 0+1 review: completion-summary frozen-guard wording; spec §4.1 signature sync
(node_ids→Vec<String>, remove_node strong flag); members/memberships adjacency-order
docstrings; remove_node_from_edge error-enum refinement; add_node_to_edge counter-bump
deviation note; Null-attr coercion note.
From Phase 2 review: shuffle/swap reset per-membership attrs M to default (docstring note);
merge_duplicate_edges MergeRename::New swallows AlreadyExists on collision (unreachable —
note only); cleanup() deviation #6 already declared.
From Phase 3 review: katz all-zero guard — ALREADY documented in code (`5630357`); nothing
further.

## End-of-program ceremony (babylon side, after Phase 10)

In the babylon repo (branch off `dev`, PR back to `dev`):
1. ADR in `ai/decisions/` (individual `ADR0NN_*.yaml` + `index.yaml` entry): subrepo ruling +
   port program record.
2. `ai/state.yaml` update.
3. Apply the parked doc advisories to the spec
   (`docs/superpowers/specs/2026-07-18-hypergraph-rs-design.md`).
4. Conventional commits via `mise run commit`; verify HEAD moved (hooks can silently abort).

## Key session artifacts (session `86ea08c6-...`, for same-session resume only)

- Executor script: `workflows/scripts/hypergraph-rs-phase-execute-wf_9d04fc12-c8b.js`
- Run IDs: P2 exec `wf_ac047bea-ce2` (done), P3 exec `wf_9171fce8-f33` (done), P4 exec
  `wf_2cbb34ad-267` (STOPPED after task 5), P7-10 planning `wf_6d13d15e-9e1` (done), P7-10 fix
  `wf_20e834f4-e92` (done).
- Program memory (auto-loaded each session):
  `~/.claude/projects/-home-user-projects-game-babylon/memory/hypergraph-rs-port-program.md`
  — points here.
