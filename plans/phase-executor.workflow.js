export const meta = {
  name: 'hypergraph-rs-phase-execute',
  description: 'Generic executor: run one phase plan of the hypergraph-rs port as sequential TDD task agents, then an Opus phase review with full-workspace gates',
  whenToUse: 'Re-invoke with args {phase, plan, tasks} for each phase 2-6',
  phases: [
    { title: 'Execute', detail: 'sequential Sonnet TDD task agents, single-flight cargo, commit per task' },
    { title: 'Review', detail: 'Opus phase review: new modules line-by-line + full-workspace test/clippy/fmt gates' },
  ],
}

const REPO = '/home/user/projects/game/babylon/hypergraph-rs'
const XGI = '/home/user/projects/game/babylon/.venv/lib/python3.13/site-packages/xgi'
const cfg = typeof args === 'string' ? JSON.parse(args) : args
if (!cfg || !Number.isInteger(cfg.tasks) || cfg.tasks < 1 || !cfg.plan || !cfg.phase) {
  throw new Error('executor needs args {phase, plan, tasks>=1}; got: ' + JSON.stringify(args).slice(0, 200))
}
log('Executing Phase ' + cfg.phase + ': ' + cfg.tasks + ' tasks from ' + cfg.plan)

const TASK_SCHEMA = {
  type: 'object',
  required: ['task', 'status', 'commit_sha', 'tests_passing', 'deviations'],
  properties: {
    task: { type: 'integer' },
    status: { type: 'string', enum: ['pass', 'fail'] },
    commit_sha: { type: 'string' },
    tests_passing: { type: 'integer', description: 'total tests passing across the test files this task touches, from the final run' },
    deviations: { type: 'array', items: { type: 'string' } },
    notes: { type: 'string' },
  },
}

const REVIEW_SCHEMA = {
  type: 'object',
  required: ['verdict', 'gates', 'findings'],
  properties: {
    verdict: { type: 'string', enum: ['approve', 'fix-needed'] },
    gates: {
      type: 'object',
      required: ['tests', 'clippy', 'fmt'],
      properties: {
        tests: { type: 'string' },
        clippy: { type: 'string' },
        fmt: { type: 'string' },
      },
    },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['severity', 'description'],
        properties: {
          severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'advisory'] },
          file: { type: 'string' },
          description: { type: 'string' },
          suggested_fix: { type: 'string' },
        },
      },
    },
  },
}

const results = []
for (let i = 1; i <= cfg.tasks; i++) {
  phase('Execute')
  const r = await agent(`You are executing Task ${i} of the Phase ${cfg.phase} TDD implementation plan for the hypergraph-rs Rust workspace.

THE PLAN: ${cfg.plan}
First read the plan's header and global-constraints/deviations sections (the top of the file), then locate the heading "### Task ${i}" (or "### Task ${i}:") and read that ENTIRE task section through to the next task heading. The plan was authored against the real XGI 0.10.2 source, adversarially reviewed, and corrected — the on-disk text is authoritative. Follow it EXACTLY, including every Note.

THE REPO: ${REPO} — a standalone git repository (branch main). Earlier phases are already committed; the core crate lives at crates/hypergraph-rs/. Work ONLY inside ${REPO}; cd there for all commands (never git -C).

REFERENCE (read-only, for resolving ambiguities): the real XGI 0.10.2 Python source at ${XGI} — if a plan step is ambiguous or conflicts with reality, the actual XGI behavior + the plan's documented deviations decide.

PROCESS — strict TDD:
1. Execute each checkbox step in order: failing test first where specified (RUN and CONFIRM the expected failure), then the implementation exactly as written, then confirm pass.
2. ONE cargo command at a time, never parallel builds.
3. Commit with plain git commit using the plan's commit message (trailer: Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>). Verify HEAD moved with git log --oneline -1; report the short SHA.
4. On conflict between plan text and compiler/tests: make the MINIMAL correct fix, record it precisely in "deviations", continue. If an expected test count differs after an honest fix, record the actual. If genuinely blocked, return status "fail" with the error in notes — never fake success.
5. Do not modify the plan file or anything outside ${REPO}.

Return via StructuredOutput. Your final text IS the data.`, { label: 'p' + cfg.phase + '-task' + i, model: 'sonnet', effort: 'medium', schema: TASK_SCHEMA, phase: 'Execute' })
  results.push(r)
  if (!r || r.status !== 'pass') {
    log('Phase ' + cfg.phase + ' Task ' + i + ' FAILED — aborting')
    return { phase: cfg.phase, aborted_at: i, results }
  }
  log('P' + cfg.phase + ' task ' + i + '/' + cfg.tasks + ' @ ' + r.commit_sha + (r.deviations.length ? ' (' + r.deviations.length + ' dev)' : ''))
}

phase('Review')
const review = await agent(`Adversarial review of Phase ${cfg.phase} of the hypergraph-rs port, just executed as ${cfg.tasks} TDD commits in ${REPO} (standalone git repo, branch main).

Ground truth: the phase plan ${cfg.plan} (including its deviations block), the design spec at /home/user/projects/game/babylon/.claude/worktrees/docs-hypergraph-rs-design/docs/superpowers/specs/2026-07-18-hypergraph-rs-design.md, and the REAL XGI 0.10.2 source at ${XGI} for every ported behavior (docstrings lie — read code).

Do all of the following, one cargo command at a time:
1. Read every NEW/CHANGED Rust source + test file this phase added (git log/diff since the phase started tells you which; read them line by line).
2. Run and record ACTUAL output of the full-workspace gates: cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings (--all-targets is mandatory — plain clippy skips test targets and hid a Phase 3 defect); cargo fmt --all -- --check. Any regression in earlier-phase tests is a blocker.
3. Verify the plan's completion criteria for this phase are actually met, and that every deviation the plan declares matches what the code does.
4. Hunt for real defects: semantic divergence from XGI not declared as deviation, determinism holes (unseeded/hash-ordered anything), borrow/logic errors, unreachable or dead code, panics reachable from the safe public API.
Do NOT fix anything — report. verdict "approve" only if all gates pass AND no blocker/major findings.
Return via StructuredOutput. Your final text IS the data.`, { label: 'review:phase' + cfg.phase, model: 'opus', effort: 'high', schema: REVIEW_SCHEMA, phase: 'Review' })

log('Phase ' + cfg.phase + ' review: ' + (review ? review.verdict : 'agent failed'))
return { phase: cfg.phase, results, review }