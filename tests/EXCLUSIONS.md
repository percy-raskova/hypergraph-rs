# XGI conformance triage protocol

When a ported test is red, classify it into exactly ONE bucket and act:

1. BINDING BUG — the Rust core is correct but the PyO3 layer mistranslates (wrong ID type,
   set-vs-list, missing kwarg, wrong exception class). → Fix the binding (src/*.rs). This is the
   default assumption; ~all reds should land here.

2. CORE BUG / GAP — the core computes a wrong value or lacks a built-but-needed method the binding
   assumed. → STOP. Do not paper over it in the binding. If it is a genuine XGI-semantics divergence in
   a *built* function, fix it in the core crate and regenerate that phase's Rust tests. The one
   pre-sanctioned additive method (`add_empty_edge`) is already landed in **Task 0**; any *other* core
   gap discovered here is NOT pre-approved — escalate rather than silently add core methods.

3. DOCUMENTED EXCLUSION — the test exercises a DEFERRED symbol, a pandas/network/matplotlib-image path,
   a private XGI internal, or an RNG/float-exact assertion unachievable with rand_pcg (§9.4). → Add it
   to `compat/exclusions.toml`, the enforced, authoritative registry (spec §4 C2), consumed at collection
   time by `tests/conftest.py` and validated by `tests/ported/test_gate_summary.py`. Every entry carries
   exactly one `class` from the closed 9-class taxonomy:

     deferred-symbol | core-gap | binding-gap | divergence | float-parity |
     upstream-private | env | upstream-bug | out-of-scope

   `env`, `upstream-bug`, and `out-of-scope` are the only classes that may skip; every other class must
   RUN and land as a strict `xfail` (`tests/conftest.py::_SKIP_CLASSES`, `tests/test_registry_integrity.py`
   enforces this). Never exclude to dodge a fixable binding bug.

   > **Superseded.** The scheme below — a bare (a)-(f) reason string with no enforced taxonomy — was the
   > original triage protocol. It predates `compat/exclusions.toml` and is no longer read by any tooling;
   > it is kept here only as a historical record of Task 15-18's triage vocabulary, most of which maps
   > directly onto today's classes ((a)→deferred-symbol, (b)→deferred-symbol/pandas, (c)→deferred-symbol,
   > (d)→deferred-symbol, (e)→float-parity, (f)→ did not end up in use). Do not add new exclusions using
   > this scheme; it lacks `core-gap`, `binding-gap`, `divergence`, `upstream-private`, `env`,
   > `upstream-bug`, and `out-of-scope`, all of which are now load-bearing.
   >
   > An exclusion needs one of these reasons:
   >   (a) deferred symbol (cite DEFERRED_SYMBOLS.md),
   >   (b) pandas (spec §4.4),
   >   (c) network (xgi-data spec §4.3; bigg-data @webtest, D7),
   >   (d) mpl image-compare (spec §9.3),
   >   (e) RNG/float-trajectory non-determinism (§9.4),
   >   (f) OQ-2 fetch of tests/readwrite/data/ unavailable at execution (disk-fixture readwrite tests only).

TARGET: every non-excluded test passes. The ledger is append-only and any exclusion lacking a valid
`class` from the 9-class taxonomy in `compat/exclusions.toml` is a triage failure.

## Owners, and what a re-import files

An entry may carry an `owner` (`tools/taxonomy.py` `OWNER_PATTERN`): `xgi-1-1-raise:T10` to `T20` names the
task of `plans/2026-11-21-xgi-1-1-raise.md` that clears it, and `later:online-loaders`,
`later:stat-functions` and `later:doc-examples` name a later plan of its own. The test's strict expected
failure carries the owner in its reason as `[owner:<owner>]`. An entry with no owner is a standing gap
that the raise does not close, and its reason says why.

After a re-import (`mise run py:import-tests <tag>`), run the ported suite, read each failure, and file it
by its real cause. A failure that the plan's table did not predict gets a reason that starts with
`found at the flip:` and is listed in the commit message. An entry whose test now passes is removed, since
a strict expected failure fails the run when it passes.
