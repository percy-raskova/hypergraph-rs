"""Core hypergraph types (mirrors xgi/core/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ["Hypergraph", "DiHypergraph", "SimplicialComplex", "subhypergraph"]
for _n in _BOUND:
    try:
        globals()[_n] = getattr(
            __import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n
        )
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 4/6/7 binds it

from . import globalviews  # noqa: F401,E402 — submodule access (xgi.core.globalviews.subhypergraph);
# NOT added to __all__: xgi.core.__all__ is exactly the 4-symbol table in spec §A ("core | 4"),
# and "globalviews" isn't one of xgi's own top-level exports either.

__all__ = ["Hypergraph", "DiHypergraph", "SimplicialComplex", "subhypergraph"]
