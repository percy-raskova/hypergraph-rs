"""Dynamics (mirrors xgi/dynamics/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['simulate_kuramoto', 'compute_kuramoto_order_parameter']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 12 binds it

# Phase 6 'Deferred'
simulate_simplicial_kuramoto = make_deferred("simulate_simplicial_kuramoto")
compute_simplicial_order_parameter = make_deferred("compute_simplicial_order_parameter")

__all__ = ['simulate_kuramoto', 'simulate_simplicial_kuramoto', 'compute_kuramoto_order_parameter', 'compute_simplicial_order_parameter']
