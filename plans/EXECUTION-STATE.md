# hypergraph-rs port — execution state & continuation handoff

**Written 2026-07-19 ~11:45am EDT, on owner instruction to pause and preserve tokens.**
This file is the single source of truth for resuming the program. Read it top to bottom before
doing anything.

## Program goal

Owner-set session goal: **"the rust port is completed and all phases are complete"** — execute
Phases 0–10 of the hypergraph-rs port (standalone Rust port of XGI 0.10.2 as a rustworkx-core
plugin). Phase 11 (Babylon swap) stays deferred to ADR083. Umbrella directive: babylon
`.opencode/goal.md`.

This repo (`/home/user/projects/game/babylon/hypergraph-rs`) is its OWN git repo (branch
`main`), parent-ignored by babylon's `.gitignore` (`/hypergraph-rs/`, the `infra/` pattern).
No remote — publishing of any kind is an owner decision.

## State at pause

| Phase | Plan file (plans/) | Status | Evidence |
|---|---|---|---|
| 0+1 workspace + core | phase-0-1-workspace-core.md | **DONE + review approve** | 64 tests; commits b604c6d..767267e |
| 2 dihypergraph/simplicial/views | phase-2-dihypergraph-simplicial-views.md | **DONE + review approve** | 15/15 tasks, 135 tests, HEAD was `da208f4` |
| 3 linalg + algorithms | phase-3-linalg-algorithms.md | **DONE + review fixed** | 12/12 tasks (`a88c8e1`..`0308138`); review said fix-needed → all 4 findings fixed in `5fdb4a4` + `5630357`; 188 tests, clippy `--all-targets` clean |
| 4 generators/stats/readwrite | phase-4-generators-stats-readwrite.md | **PAUSED: tasks 1–5 of 19 committed** | `f65cc0e` `d9d80c7` `467b3ac` `af61224` `3f7ec00`; tree clean; run `wf_2cbb34ad-267` stopped mid-task-6 |
| 5 layout + viz | phase-5-layout-viz.md | not started | 11 tasks |
| 6 convert/dynamics/communities/utils | phase-6-convert-dynamics-communities-utils.md | not started | 17 tasks |
| 7 python conformance | phase-7-python-conformance.md | not started | 20 tasks (Task 0 = core `add_empty_edge`) |
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
