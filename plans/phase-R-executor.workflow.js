export const meta = {
  name: 'hypergraph-rs-phase-R-execute',
  description: 'Phase-R executor: run the cell-rasterizer plan as sequential TDD task agents, then an Opus review grounded on the reference deck + determinism contract (NOT XGI — Phase R is not a port)',
  whenToUse: 'Invoke with args {phase:"R", plan, tasks} once the plan is committed',
  phases: [
    { title: 'Execute', detail: 'sequential Sonnet TDD task agents, single-flight cargo, feature-aware tests, commit per task' },
    { title: 'Review', detail: 'Opus review: new modules line-by-line + deck-number reproduction + determinism + full feature-matrix gates' },
  ],
}

// Phase R is a NEW capability (rasterizer + 3D instruments), NOT an XGI port.
// Ground truth = the reference deck + the plan's golden numbers, never XGI.
const REPO = '/home/user/projects/game/hypergraph-rs'
const DECK = REPO + '/plans/reference/topology-deck.html'
const MAP = REPO + '/plans/PHASE-R-MAP.md'
const cfg = typeof args === 'string' ? JSON.parse(args) : args
if (!cfg || !Number.isInteger(cfg.tasks) || cfg.tasks < 1 || !cfg.plan || !cfg.phase) {
  throw new Error('executor needs args {phase, plan, tasks>=1}; got: ' + JSON.stringify(args).slice(0, 200))
}
log('Executing Phase ' + cfg.phase + ' (rasterizer): ' + cfg.tasks + ' tasks from ' + cfg.plan)

const TASK_SCHEMA = {
  type: 'object',
  required: ['task', 'status', 'commit_sha', 'tests_passing', 'deviations'],
  properties: {
    task: { type: 'integer' },
    status: { type: 'string', enum: ['pass', 'fail'] },
    commit_sha: { type: 'string' },
    tests_passing: { type: 'integer', description: 'total tests passing across the test files this task touches, from the final run (include feature-gated runs)' },
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
      required: ['tests_default', 'tests_cells3d', 'tests_png', 'clippy', 'fmt', 'no_default', 'deck_numbers'],
      properties: {
        tests_default: { type: 'string', description: 'cargo test --workspace --locked (default features)' },
        tests_cells3d: { type: 'string', description: 'cargo test --workspace --features cells3d --locked' },
        tests_png: { type: 'string', description: 'cargo test --workspace --features raster-png --locked' },
        clippy: { type: 'string', description: 'clippy --all-targets -D warnings at default AND --features cells3d AND --features raster-png' },
        fmt: { type: 'string' },
        no_default: { type: 'string', description: 'cargo build --no-default-features (+ --features raster) still compiles' },
        deck_numbers: { type: 'string', description: 'do the Stage-0 golden tests reproduce the deck numbers (r=0.4608359187088229, S=198.3, net-T=0 both zooms, Pi0=13, mulberry32 vectors)?' },
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
  const r = await agent('You are executing Task ' + i + ' of the Phase ' + cfg.phase + ' TDD plan for the hypergraph-rs Rust workspace — the cell rasterizer + Lawverian 3D instruments.\n\n' +
    'THE PLAN: ' + cfg.plan + '\nFirst read the plan header + Global Constraints/deviations (top of file), then locate the heading "### Task ' + i + '" (or "### Task ' + i + ':") and read that ENTIRE task section through to the next task heading. The plan was authored against the reference deck, adversarially reviewed, and revised — the on-disk text is authoritative. Follow it EXACTLY, including every Note and Finding reference.\n\n' +
    'THIS IS NOT AN XGI PORT. Phase R builds a new rendering capability. The ground truth for any ambiguity is the REFERENCE DECK (read-only): ' + DECK + ' (the JS reference implementation whose computed numbers the Rust code must reproduce), and the component map ' + MAP + '. There is no XGI counterpart — do not consult XGI.\n\n' +
    'THE REPO: ' + REPO + ' — standalone git repo (branch main). Earlier tasks are committed; the core crate is crates/hypergraph-rs/. Work ONLY inside ' + REPO + '; cd there (never git -C).\n\n' +
    'PROCESS — strict TDD:\n' +
    '1. Execute each checkbox step in order: failing test first where specified (RUN and CONFIRM the expected failure), then the implementation exactly as written, then confirm pass.\n' +
    '2. FEATURE-AWARE TESTING: the raster feature is default-on, but cells3d (the 3 instruments) and raster-png (the PNG task) are OFF by default. If this task creates/touches a module or test behind cells3d or raster-png, you MUST run its cargo test/clippy WITH that feature enabled (e.g. cargo test --features cells3d, or --features raster-png) — the default gate will not compile them. Verify the test file carries the matching #![cfg(feature = "...")] header the plan specifies.\n' +
    '3. ONE cargo command at a time, never parallel builds.\n' +
    '4. DETERMINISM IS SACRED: no wall-clock, no entropy, no unseeded RNG, no HashMap iteration order in output paths. Byte-exact goldens for pure-arithmetic paths; tolerance-bounded (the plan says 1e-12) for exp/ln paths (psa, occY) and platform-pinned for PNG-compressed bytes. Honor these exactly.\n' +
    '5. Commit with plain git commit using the plan commit message (trailer: Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>). Verify HEAD moved with git log --oneline -1; report the short SHA.\n' +
    '6. On conflict between plan text and compiler/tests: make the MINIMAL correct fix, record it precisely in "deviations", continue. If a golden number differs after an honest fix, STOP and record it in notes (a differing deck number is a red flag, not a rubber-stamp). If genuinely blocked, return status "fail" with the error in notes — never fake success.\n' +
    '7. Do not modify the plan file or anything outside ' + REPO + '.\n\n' +
    'Return via StructuredOutput. Your final text IS the data.', { label: 'pR-task' + i, model: 'sonnet', effort: 'medium', schema: TASK_SCHEMA, phase: 'Execute' })
  results.push(r)
  if (!r || r.status !== 'pass') {
    log('Phase ' + cfg.phase + ' Task ' + i + ' FAILED — aborting')
    return { phase: cfg.phase, aborted_at: i, results }
  }
  log('PR task ' + i + '/' + cfg.tasks + ' @ ' + r.commit_sha + (r.deviations.length ? ' (' + r.deviations.length + ' dev)' : ''))
}

phase('Review')
const review = await agent('Adversarial review of Phase ' + cfg.phase + ' of the hypergraph-rs port — the cell rasterizer + Lawverian 3D instruments, just executed as ' + cfg.tasks + ' TDD commits in ' + REPO + ' (standalone git repo, branch main).\n\n' +
  'THIS IS NOT AN XGI PORT — do NOT ground against XGI. Ground truth:\n' +
  '- The phase plan ' + cfg.plan + ' (including its Global Constraints + deviations + every Finding reference).\n' +
  '- The REFERENCE DECK ' + DECK + ' — the JS reference implementation. The Rust instruments must REPRODUCE its computed numbers. The plan pins these goldens: raw r=0.47102137767220903, displayed r after one startup tick=0.4608359187088229, S=198.3, net-T=0 at BOTH zoom levels, Pi0=13, org=[0.2,0.2,0.2,1.0], the two mulberry32 golden vectors (seed 1917 and 22928=1917^0x5EED), pw=1.8250060787286078 POST-tick; psa is tolerance-bounded (exp is non-reproducible).\n' +
  '- The component map ' + MAP + '.\n\n' +
  'Do all of the following, ONE cargo command at a time:\n' +
  '1. Read every NEW/CHANGED Rust source + test file this phase added (git log/diff since the phase started tells you which; read line by line).\n' +
  '2. Run and record ACTUAL output of the FULL feature matrix (cells3d + raster-png are OFF by default, so the default gate does NOT exercise the instruments or PNG — you MUST run the feature legs):\n' +
  '   - cargo test --workspace --locked (default)\n' +
  '   - cargo test --workspace --features cells3d --locked (the 3 instruments + Stage-0 deck-number goldens)\n' +
  '   - cargo test --workspace --features raster-png --locked (the PNG task)\n' +
  '   - cargo clippy --workspace --all-targets --locked -- -D warnings, AND again with --features cells3d, AND with --features raster-png (--all-targets is mandatory — plain clippy skips test targets)\n' +
  '   - cargo fmt --all -- --check\n' +
  '   - cargo build --no-default-features --features raster (must still compile; confirms h3o/png/communities do not leak into the raster-only surface)\n' +
  '   - RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked\n' +
  '3. Confirm the Stage-0 golden tests ACTUALLY reproduce the deck numbers above (grep the test output / read the asserts). A missing or wrong deck number is a blocker — this is the whole point of the phase.\n' +
  '4. Hunt for real defects: determinism holes (unseeded/hash-ordered/wall-clock anything in an output path; unstable sort; float non-reproducibility claimed byte-exact); reachable panics from the safe public API (NaN partial_cmp unwrap, unclamped subcell/pixel index, near-plane divide); reuse violations (reimplementing shipped convex_hull/spectral/connected_components/svg-fmt instead of calling them); S9 violations (a 3D layout leaking into a default/non-cells3d path); H3 String regressions or bit-algebra bleeding into graph semantics (D7); PNG cross-platform DEFLATE assumptions asserted as universally byte-exact rather than platform-pinned; any deviation the code makes that the plan does NOT declare.\n' +
  'Do NOT fix anything — report. verdict "approve" ONLY if ALL gate legs pass AND the deck numbers reproduce AND there is no blocker/major finding.\n' +
  'Return via StructuredOutput. Your final text IS the data.', { label: 'review:phaseR', model: 'opus', effort: 'high', schema: REVIEW_SCHEMA, phase: 'Review' })

log('Phase ' + cfg.phase + ' review: ' + (review ? review.verdict : 'agent failed'))
return { phase: cfg.phase, results, review }
