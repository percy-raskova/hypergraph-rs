"""Readwrite (mirrors xgi/readwrite/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['load_bigg_data', 'read_bipartite_edgelist', 'write_bipartite_edgelist', 'parse_bipartite_edgelist', 'read_edgelist', 'write_edgelist', 'parse_edgelist', 'read_hif', 'write_hif', 'read_incidence_matrix', 'write_incidence_matrix']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 12 binds it

# network (gated off) + Phase 4 built undirected HIF only
load_xgi_data = make_deferred("load_xgi_data")
download_xgi_data = make_deferred("download_xgi_data")
read_hif_collection = make_deferred("read_hif_collection")
write_hif_collection = make_deferred("write_hif_collection")

__all__ = ['load_bigg_data', 'read_bipartite_edgelist', 'write_bipartite_edgelist', 'parse_bipartite_edgelist', 'read_edgelist', 'write_edgelist', 'parse_edgelist', 'read_hif', 'write_hif', 'read_hif_collection', 'write_hif_collection', 'read_incidence_matrix', 'write_incidence_matrix', 'load_xgi_data', 'download_xgi_data']
