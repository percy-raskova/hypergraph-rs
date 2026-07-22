"""`hypergraph_rs.core.globalviews` — mirrors `xgi/core/globalviews.py`'s module path.

The oracle tests import the function via the submodule path (`xgi.core.globalviews.subhypergraph`),
not just the re-exported `xgi.subhypergraph` top-level name — this module exists so that path
resolves too, delegating to the same native binding.
"""

from . import subhypergraph  # noqa: F401

__all__ = ["subhypergraph"]
