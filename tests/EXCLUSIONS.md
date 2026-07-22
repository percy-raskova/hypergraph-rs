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
   to FILE_EXCLUSIONS/TEST_EXCLUSIONS with a one-line reason. An exclusion needs one of these reasons:
     (a) deferred symbol (cite DEFERRED_SYMBOLS.md),
     (b) pandas (spec §4.4),
     (c) network (xgi-data spec §4.3; bigg-data @webtest, D7),
     (d) mpl image-compare (spec §9.3),
     (e) RNG/float-trajectory non-determinism (§9.4),
     (f) OQ-2 fetch of tests/readwrite/data/ unavailable at execution (disk-fixture readwrite tests only).
   Never exclude to dodge a fixable binding bug.

TARGET: every non-excluded test passes. The ledger is append-only during Tasks 16–18 and reviewed at
Task 19 — any exclusion lacking an (a)–(e) reason is a triage failure.
