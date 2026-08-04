# T1b sweep — `bound` symbols that raise `NotImplementedError` (or equivalent) for some input

> **A correction landed on one entry after this document's first draft.** Finding A's
> `degree_assortativity` reason in the ready-to-paste TOML below originally claimed xgi
> "returns a float unconditionally." Measurement disproved this: xgi returns a Python float
> only when the sampled degree pairs have variance, and the literal int `0` when `corrcoef`
> is NaN from zero variance — exactly what the uniform-triangle fixture this entry's own test
> uses hits. Corrected in place below; footnote [^1] records what was originally claimed and
> what measurement replaced it. Everything else stands as written.

Scope: every public hypergraph_rs callable that is `bound` in
`python/hypergraph_rs/compat/manifest.json` (i.e. NOT routed through `_deferred.make_deferred`)
but raises for at least one reachable argument/environment combination. Feeds
`compat/partial_symbols.toml` (fields: `symbol`, `site`, `trigger`, `reason`).

## Method

1. `rg -n "NotImplementedError"` across `crates/` and `python/` — exhaustive, 3 raise sites total
   in the whole workspace (all in `crates/hypergraph-rs-python/src/`; zero in the core
   `hypergraph-rs` crate; zero `unimplemented!()`/`todo!()` macros anywhere).
2. Read each enclosing function, its `#[pyo3]` signature/getter/method name, and traced which
   *public* Python name(s) reach it and under what call shape.
3. Verified every site by calling it against the built `hypergraph_rs` **and** real `xgi 0.10.2`
   in the same process, from `/home/user/projects/game/hypergraph-rs/.venv/bin/python`.
4. Additionally read every `.py` file under `python/hypergraph_rs/` in full (all are short,
   7–105 lines) looking for hand-written stubs that raise/return-None/pass without going through
   `NotImplementedError`. Found one extra family (an `ImportError`-raising shim, listed
   separately below since it does not fit the `NotImplementedError` schema literally).

## Findings table

| # | symbol (public reach) | site | trigger | xgi's behaviour | verified? |
|---|---|---|---|---|---|
| A | `degree_assortativity(H, exact=False)` | `crates/hypergraph-rs-python/src/algorithms.rs:163` | `exact=False` (any `kind`/`num_samples`/`seed`) — the default is `exact=False`, so **even the parameterless call `degree_assortativity(H)` raises** | Returns a float (real run: `0`, with a `RuntimeWarning: invalid value encountered in divide` from the sampling path) | **Yes**, executed both sides |
| B | `DiHypergraph.nodes.in_degree` (as a *call*, e.g. `D.nodes.in_degree(order=1)`) | `crates/hypergraph-rs-python/src/dihypergraph.rs:2190` (`DiStat::__call__`, reached because the `in_degree` getter at `dihypergraph.rs:1470-1484` sets both `degree_source: None` and `edge_source: None`) | **Any call at all**, including zero args: `D.nodes.in_degree()` raises exactly like `D.nodes.in_degree(order=1)`. The bare property access `D.nodes.in_degree` (no `()`) does NOT raise — only invoking it as a function does. | `D.nodes.in_degree()` → re-dispatches, returns a re-parametrized Stat (`.asdict()` unchanged); `D.nodes.in_degree(order=1)` → filtered dict, e.g. `{1:0,2:0,3:0,4:0}` | **Yes**, executed both sides, both call shapes |
| C | `DiHypergraph.nodes.out_degree` (as a *call*) | Same site as B, `dihypergraph.rs:2190` (the `out_degree` getter at `dihypergraph.rs:1488-1502` also sets both sources `None`) | Same as B: any call, zero-arg included | `D.nodes.out_degree()` → `{1:1,2:2,3:0,4:0}`; `D.nodes.out_degree(order=1)` → `{1:0,2:0,3:0,4:0}` | **Yes**, executed both sides, both call shapes |
| D | `Hypergraph.edges.attrs(key)` **re-called**, e.g. `H.edges.attrs('color')()` | `crates/hypergraph-rs-python/src/stats.rs:471` (`PyEdgeStat::__call__`, `EdgeStatKind::Attr` arm) | Invoking the `EdgeStat` object returned by `.attrs(key)` as a function, with or without a new key. Only the `Attr` kind raises — `Size`/`Order` EdgeStats re-parametrize fine via the same `__call__`. **Asymmetric with the node side**: `H.nodes.attrs('color')()` (`stats.rs` `PyNodeStat::__call__`, `NodeStatKind::Attr` arm, lines 280-293) does **not** raise — it happily re-parametrizes and returns `{'a': 'blue'}`. | `H.edges.attrs('color')()` → `{0: {'color': 'red'}}` (resets to whole-attr-dict semantics, does not raise) | **Yes**, executed both sides, plus the Node/Edge asymmetry contrast |

Note on B/C: the recon's original phrasing (`D.nodes.in_degree(order=1)`) undersold the trigger —
it is not "calling with `order=`" that raises, it is **calling at all**. This matters for
`compat/partial_symbols.toml`'s `trigger` field: any AST call-site on `.in_degree`/`.out_degree`
(zero-arg included) must be flagged, not only ones that pass keyword arguments.

## Extra finding from the step-4 stub probe (not `NotImplementedError` — flagged separately)

| # | symbol | site | trigger | behaviour | verified? |
|---|---|---|---|---|---|
| E | `drawing.draw`, `drawing.draw_nodes`, `drawing.draw_hyperedges` | `python/hypergraph_rs/drawing/__init__.py:42-52` (`_require_mpl` decorator) — no Rust site, pure Python shim | Calling any of the three when `matplotlib` is not importable (the `hypergraph-rs[mpl]` extra not installed) | Raises `ImportError`, not `NotImplementedError`: `"matplotlib drawing requires the mpl extra..."` | **Yes, but by simulation**: `matplotlib` IS installed in this `.venv` (`import matplotlib` succeeds), so I could not reproduce the natural absence. I verified the *code path* honestly by monkeypatching `hypergraph_rs.drawing.HAS_MPL = False` in-process and calling `draw(H)`/`draw_nodes(H)`/`draw_hyperedges(H)`, which reproduced the exact `ImportError` text from the source. This is the real guarded branch (`if not HAS_MPL: raise ImportError(...)`), not a fabricated result, but it is a simulated trigger condition rather than a naturally absent dependency — flagged so the reader can tell the difference from A-D's direct reproductions. |

All three are `bound` in the manifest (`draw`, `draw_nodes`, `draw_hyperedges` → `"bound"`,
checked directly against `manifest.json`). Whether `partial_symbols.toml`'s schema should also
carry non-`NotImplementedError` exception classes is an open design question for the owner —
included here because it is exactly the "bound symbol that lies to `check()`" failure mode the
task is trying to close, just via a different exception type and an environment-condition
trigger instead of an argument-value trigger.

## Ruled out (checked, not a divergence / not reachable)

- `hypergraph_rs.exception.frozen(*args, **kwargs)` — unconditionally raises `XGIError`, but this
  is a verbatim mirror of `xgi.exception.frozen` (both raise identically); not a manifest
  top-level symbol; not a partial-symbol candidate. Verified: both `xgi.exception.frozen()` and
  `hypergraph_rs.exception.frozen()` raise the identical `XGIError` message.
- `SimplicialComplex` has no `.attrs()` method on either its node or edge views at all (no
  `fn attrs` in `crates/hypergraph-rs-python/src/simplicial.rs`), so it cannot reach stats.rs:471
  — this is a missing-method gap (already known per the T1 recon), not a partial-bound-symbol.
- `DiHypergraph` has no `.attrs()` on `nodes`/`edges` views either (no `fn attrs` in
  `dihypergraph.rs`) — same as above, out of scope for this sweep.
- Every `_BOUND` list / `make_deferred` fallback read in full across all 18 `.py` files under
  `python/hypergraph_rs/` — no additional hand-written raise/return-None/pass shims found beyond
  the drawing module's `_require_mpl` (finding E) and the ruled-out `frozen` helper.
- Core `hypergraph-rs` crate (non-Python) — zero `NotImplementedError`, zero
  `unimplemented!()`/`todo!()` macros; the divergence is entirely a PyO3-layer phenomenon,
  consistent with the T1 recon's carry-over #6.

## Ready-to-paste TOML (`compat/partial_symbols.toml`)

Schema: exactly the four fields named in the task (`symbol`, `site`, `trigger`, `reason`). `A`
already has a near-identical entry in `compat/exclusions.toml:366-368` (class
`deferred-symbol`/test-level) — this is the symbol-level companion the manifest itself can key
on. B/C/D have no existing entry anywhere in `exclusions.toml`. E is included but marked
`schema-question` in its reason since it is not a `NotImplementedError`; the owner should decide
whether `bound-partial` covers non-`NotImplementedError` exceptions or whether E needs a fifth
status.

```toml
[[partial_symbol]]
symbol = "degree_assortativity"
site = "crates/hypergraph-rs-python/src/algorithms.rs:163"
trigger = "exact=False (the default) — any kind/num_samples/seed; the bare call degree_assortativity(H) already hits this"
reason = "Random-sampling assortativity path awaits the Phase-4 rand_pcg pin (module doc, algorithms.rs). Only exact=True is ported. xgi's exact=False path returns a Python float when the sampled degree pairs have variance (measured: -0.647 on a varied-degree fixture) but returns the literal int 0, not a float, when corrcoef is NaN from zero variance — the uniform-triangle fixture this entry's own test uses hits exactly that path, with a 'RuntimeWarning: invalid value encountered in divide'. Verified against real xgi 0.10.2." # CORRECTED 2026-08-04, see footnote [^1]

[[partial_symbol]]
symbol = "DiHypergraph.nodes.in_degree"
site = "crates/hypergraph-rs-python/src/dihypergraph.rs:2190 (DiStat::__call__, via the in_degree getter at :1470-1484)"
trigger = "Invoking the Stat as a function at all — D.nodes.in_degree(...) or even D.nodes.in_degree() with zero arguments. The bare property access D.nodes.in_degree (no call) does not raise."
reason = "The in_degree getter constructs DiStat with both degree_source and edge_source None, so __call__'s fallback branch always raises. xgi re-dispatches and returns a filtered/re-parametrized Stat instead. Not covered by any ported test (untested per the T1 recon) and not in exclusions.toml."

[[partial_symbol]]
symbol = "DiHypergraph.nodes.out_degree"
site = "crates/hypergraph-rs-python/src/dihypergraph.rs:2190 (DiStat::__call__, via the out_degree getter at :1488-1502)"
trigger = "Invoking the Stat as a function at all, including zero-arg D.nodes.out_degree()."
reason = "Identical root cause and fix to in_degree: the out_degree getter also constructs DiStat with both sources None. xgi returns a valid re-parametrized Stat; ours raises unconditionally on call."

[[partial_symbol]]
symbol = "Hypergraph.edges.attrs"
site = "crates/hypergraph-rs-python/src/stats.rs:471 (PyEdgeStat::__call__, EdgeStatKind::Attr arm)"
trigger = "Re-invoking the EdgeStat returned by H.edges.attrs(key) as a function — H.edges.attrs('color')() or with a new key. Only the Attr kind raises; Size/Order re-parametrize fine on the same __call__."
reason = "PyEdgeStat's Attr arm is the one EdgeStatKind that never got a re-parametrization branch (contrast PyNodeStat::Attr at stats.rs:280-293, which does re-parametrize and does not raise for the identical H.nodes.attrs(key)() shape). xgi returns a re-dispatched Stat, not an error."

[[partial_symbol]]
symbol = "drawing.draw / drawing.draw_nodes / drawing.draw_hyperedges"
site = "python/hypergraph_rs/drawing/__init__.py:42-70 (_require_mpl decorator; no Rust site)"
trigger = "Calling any of the three when matplotlib is not importable (hypergraph-rs[mpl] extra not installed)."
reason = "SCHEMA QUESTION FOR OWNER: raises ImportError, not NotImplementedError, so it may not belong under a status literally named for NotImplementedError. Verified by simulation (HAS_MPL monkeypatched False in-process; matplotlib IS installed in this .venv so the natural absence could not be reproduced directly). Flagged because it is the same class of check()-lies-about-bound problem, just environment-triggered rather than argument-triggered."
```

[^1]: **Correction, 2026-08-04 (register-claims group, post-freeze).** Finding A's reason
originally read "xgi returns a float unconditionally; verified against real xgi 0.10.2." from
a single degenerate probe. Re-measured: xgi's `exact=False` path returns a Python float when
the sampled degree pairs have variance (`-0.647` on a varied-degree fixture) but the literal
int `0`, not a float, when `corrcoef` is NaN from zero variance — exactly the uniform-triangle
fixture this entry's own test (`tests/test_partial_symbols.py:100-103`) exercises, with a
`RuntimeWarning: invalid value encountered in divide`. Entry text corrected above and in
`compat/partial_symbols.toml`.
