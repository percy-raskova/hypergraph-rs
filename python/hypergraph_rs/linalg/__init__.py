"""Linalg (mirrors xgi/linalg/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['adjacency_matrix', 'incidence_matrix', 'intersection_profile', 'clique_motif_matrix', 'degree_matrix', 'laplacian', 'multiorder_laplacian', 'normalized_hypergraph_laplacian']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 8 binds it

# Phase 3 'Deferred': Hodge/boundary/tensor
boundary_matrix = make_deferred("boundary_matrix")
hodge_laplacian = make_deferred("hodge_laplacian")
adjacency_tensor = make_deferred("adjacency_tensor")

__all__ = ['boundary_matrix', 'hodge_laplacian', 'adjacency_matrix', 'incidence_matrix', 'intersection_profile', 'clique_motif_matrix', 'degree_matrix', 'adjacency_tensor', 'laplacian', 'multiorder_laplacian', 'normalized_hypergraph_laplacian']
