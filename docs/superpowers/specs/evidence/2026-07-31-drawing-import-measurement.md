# Task 3 — measuring the drawing family's import gate

Scope: `draw`, `draw_nodes`, `draw_hyperedges` ship as plain `bound` in
`python/hypergraph_rs/compat/manifest.json`, but the source
(`python/hypergraph_rs/drawing/__init__.py`'s `_require_mpl` guard) raises
`ImportError` when matplotlib is not importable. Before this task, reading the
guard supported that fact by inference; nobody had observed it by actually
running the three symbols in an environment without matplotlib — the earlier
sweep (`2026-07-30-partial-symbols-sweep.md`, finding E) had to simulate the
absence by monkeypatching `HAS_MPL = False` in-process, because this repo's own
`.venv` carries matplotlib. This document replaces that simulation with a
direct measurement in a throwaway, genuinely matplotlib-free virtualenv.

## Step 1 — wheel selection

```
.venv/bin/python -c "import sys; print('need cp%d%d' % sys.version_info[:2])"
ls -la dist/*.whl target/wheels/*.whl 2>/dev/null
```

Output:

```
need cp313
-rw-rw-r-- 1 user user 1327089 Jul 22 18:20 dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl
-rw-rw-r-- 1 user user  207079 Jul 22 09:41 target/wheels/hypergraph_rs-0.1.0-cp312-cp312-linux_x86_64.whl
```

`.venv` needs `cp313`. Only `dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl`
carries that ABI tag; `target/wheels/…-cp312-…` is a pre-Phase-7 stub built for
the wrong interpreter. Selected: `dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl`.

## Step 2 — measurement, single shell invocation

Venv construction command (the exact block run, `set -e` plus a `trap` that
removes the scratch directory on exit — no separate cleanup step follows):

```bash
set -e
WHEEL=dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl   # from Step 1
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
.venv/bin/python -m venv "$SCRATCH/mpl-free"
"$SCRATCH/mpl-free/bin/pip" install --quiet "$WHEEL"
"$SCRATCH/mpl-free/bin/python" -c "
import importlib.util
print('matplotlib present:', importlib.util.find_spec('matplotlib') is not None)
"
"$SCRATCH/mpl-free/bin/python" -c "
import hypergraph_rs as hrs
H = hrs.Hypergraph()
H.add_edges_from([['a','b'], ['b','c']])
for name in ('draw', 'draw_nodes', 'draw_hyperedges'):
    fn = getattr(hrs, name)
    try:
        fn(H)
    except Exception as exc:
        print(name, '->', type(exc).__name__, ':', exc, '| module:', getattr(fn, '__module__', '<none>'))
    else:
        print(name, '-> NO RAISE')
"
```

Verbatim output (pip's update-available notice omitted; nothing else elided):

```
matplotlib present: False
draw -> ImportError : matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. For matplotlib-free rendering, use `H.render()` for a scene-graph. | module: hypergraph_rs.drawing
draw_nodes -> ImportError : matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. For matplotlib-free rendering, use `H.render()` for a scene-graph. | module: hypergraph_rs.drawing
draw_hyperedges -> ImportError : matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. For matplotlib-free rendering, use `H.render()` for a scene-graph. | module: hypergraph_rs.drawing
```

`matplotlib present: False` confirms the venv is genuinely matplotlib-free —
the wheel does not pull matplotlib in as a hard dependency. All three symbols
raised `ImportError`; none printed `NO RAISE`, so all three are import-gated
and each gets an entry below. The harness printed `__module__` as
`hypergraph_rs.drawing` for all three — that observed value is the `site`
value used in the entries (per the plan's schema note, `site` is "the
symbol's `__module__`, for env-gated entries with no call site").

## Step 3 — what xgi does (packaging metadata, not execution)

xgi 0.10.2 hard-depends on matplotlib: its `Requires-Dist` metadata carries an
unconditional `matplotlib>=3.4` entry (no `extra ==` qualifier), so xgi cannot
even install in a matplotlib-free environment — there is no call site to run
there. This wheel makes matplotlib an optional `mpl` extra instead. The
divergence recorded below is this packaging asymmetry, not a behavioral
difference observed at a shared call site: xgi's side comes from its
installed package metadata, never from executing xgi in the matplotlib-free
venv, because that is impossible by construction.

Commands and verbatim output:

```
.venv/bin/python -c "
import importlib.metadata as md
print('xgi:', [r for r in (md.metadata('xgi').get_all('Requires-Dist') or []) if 'matplotlib' in r])
"
```
```
xgi: ['matplotlib>=3.4', 'matplotlib>=3.3; extra == "docs"']
```

```
.venv/bin/python -c "
import zipfile
z = zipfile.ZipFile('dist/hypergraph_rs-0.1.0-cp313-cp313-linux_x86_64.whl')
name = [n for n in z.namelist() if n.endswith('METADATA')][0]
for line in z.read(name).decode().splitlines():
    if line.startswith(('Requires-Dist', 'Provides-Extra')):
        print(' ', line)
"
```
```
  Requires-Dist: numpy>=1.24
  Requires-Dist: scipy>=1.10
  Requires-Dist: matplotlib>=3.7 ; extra == 'mpl'
  Provides-Extra: mpl
```

xgi's `matplotlib>=3.4` entry carries no `extra ==` qualifier — unconditional,
confirming the hard dependency. This wheel gates its only `matplotlib`-related
entry behind `extra == 'mpl'`, with a matching `Provides-Extra: mpl`,
confirming that matplotlib stays optional here.

## Per-symbol summary

| symbol | exception | message | `__module__` observed |
|---|---|---|---|
| `draw` | `ImportError` | ``matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. For matplotlib-free rendering, use `H.render()` for a scene-graph.`` | `hypergraph_rs.drawing` |
| `draw_nodes` | `ImportError` | (same message) | `hypergraph_rs.drawing` |
| `draw_hyperedges` | `ImportError` | (same message) | `hypergraph_rs.drawing` |

No symbol printed `NO RAISE`; all three get an entry in
`compat/partial_symbols.toml`.

## Disposition

DONE — Step 1 found a matching wheel, Step 2's measurement ran to completion
with `matplotlib present: False` (the wheel does not pull matplotlib in as a
hard dependency, so no `DONE_WITH_CONCERNS` flag applies), and all three
symbols raised `ImportError` as inferred. Step 5 adds an entry to
`compat/partial_symbols.toml` for each of the three.

**Stated limitation.** The three `env` entries are NOT behaviorally pinned
inside the normal suite: no test executes `draw`/`draw_nodes`/
`draw_hyperedges` in an mpl-free environment (that would require this
document's throwaway venv at test time). The only suite coverage touching
them is `test_manifest_exposes_the_detail`, a TOML-to-manifest equality
check — serialization drift, not behavior. The register header's "pins the
raising behaviour for the sites it can reach" does not reach these three;
this measurement is their evidence.
