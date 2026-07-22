"""Stats (mirrors xgi/stats/__init__.py)."""
from .._deferred import make_deferred

_BOUND = []
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 11 binds it

# XGI decorator/dispatch machinery (spec Sec4.4) — the binding synthesizes the observable H.nodes.degree.asdict() surface over Stat<T> (Task 11) instead of re-exporting these
nodestat_func = make_deferred("nodestat_func")
edgestat_func = make_deferred("edgestat_func")
dinodestat_func = make_deferred("dinodestat_func")
diedgestat_func = make_deferred("diedgestat_func")
dispatch_stat = make_deferred("dispatch_stat")
dispatch_many_stats = make_deferred("dispatch_many_stats")

__all__ = ['nodestat_func', 'edgestat_func', 'dinodestat_func', 'diedgestat_func', 'dispatch_stat', 'dispatch_many_stats']
