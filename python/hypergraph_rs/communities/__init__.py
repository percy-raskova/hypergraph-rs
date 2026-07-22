"""Communities (mirrors xgi/communities/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['spectral_clustering']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 12 binds it

__all__ = ['spectral_clustering']
