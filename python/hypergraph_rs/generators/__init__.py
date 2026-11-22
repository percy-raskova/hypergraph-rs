"""Generators (mirrors xgi/generators/__init__.py)."""
from .._deferred import make_deferred
from . import lattice

_BOUND = ['empty_hypergraph', 'trivial_hypergraph', 'complete_hypergraph', 'star_clique', 'sunflower', 'complement', 'ring_lattice', 'watts_strogatz_hypergraph', 'random_hypergraph', 'fast_random_hypergraph', 'chung_lu_hypergraph', 'random_simplicial_complex', 'uniform_hypergraph_configuration_model', 'uniform_erdos_renyi_hypergraph']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 10 binds it

# not in Phase 4's built list
empty_simplicial_complex = make_deferred("empty_simplicial_complex")
empty_dihypergraph = make_deferred("empty_dihypergraph")
node_swap = make_deferred("node_swap")
shuffle_hyperedges = make_deferred("shuffle_hyperedges")
flag_complex = make_deferred("flag_complex")
flag_complex_d2 = make_deferred("flag_complex_d2")
random_flag_complex = make_deferred("random_flag_complex")
random_flag_complex_d2 = make_deferred("random_flag_complex_d2")
dcsbm_hypergraph = make_deferred("dcsbm_hypergraph")
uniform_HSBM = make_deferred("uniform_HSBM")
uniform_HPPM = make_deferred("uniform_HPPM")

__all__ = ['empty_hypergraph', 'empty_simplicial_complex', 'empty_dihypergraph', 'trivial_hypergraph', 'complete_hypergraph', 'star_clique', 'sunflower', 'complement', 'ring_lattice', 'watts_strogatz_hypergraph', 'random_hypergraph', 'fast_random_hypergraph', 'chung_lu_hypergraph', 'dcsbm_hypergraph', 'node_swap', 'shuffle_hyperedges', 'flag_complex', 'flag_complex_d2', 'random_flag_complex', 'random_flag_complex_d2', 'random_simplicial_complex', 'uniform_hypergraph_configuration_model', 'uniform_erdos_renyi_hypergraph', 'uniform_HSBM', 'uniform_HPPM']
