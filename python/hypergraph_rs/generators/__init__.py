"""Generators (mirrors xgi/generators/__init__.py)."""
from .._deferred import make_deferred
from . import lattice

_BOUND = ['empty_hypergraph', 'trivial_hypergraph', 'complete_hypergraph', 'star_clique', 'sunflower', 'complement', 'ring_lattice', 'watts_strogatz_hypergraph', 'random_hypergraph', 'fast_random_hypergraph', 'chung_lu_hypergraph', 'random_simplicial_complex', 'uniform_configuration_model', 'uniform_erdos_renyi_model']
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
uniform_stochastic_block_model = make_deferred("uniform_stochastic_block_model")
uniform_planted_partition_model = make_deferred("uniform_planted_partition_model")
degree_corrected_stochastic_block_model_hypergraph = make_deferred(
    "degree_corrected_stochastic_block_model_hypergraph"
)
simplicial_chung_lu_hypergraph = make_deferred("simplicial_chung_lu_hypergraph")
random_nested_hypergraph = make_deferred("random_nested_hypergraph")
pseudofractal_simplicial_complex = make_deferred("pseudofractal_simplicial_complex")
apollonian_complex = make_deferred("apollonian_complex")
network_geometry_flavor = make_deferred("network_geometry_flavor")

__all__ = [
    "empty_hypergraph",
    "empty_simplicial_complex",
    "empty_dihypergraph",
    "trivial_hypergraph",
    "complete_hypergraph",
    "star_clique",
    "sunflower",
    "complement",
    "ring_lattice",
    "watts_strogatz_hypergraph",
    "random_hypergraph",
    "fast_random_hypergraph",
    "chung_lu_hypergraph",
    "simplicial_chung_lu_hypergraph",
    "degree_corrected_stochastic_block_model_hypergraph",
    "random_nested_hypergraph",
    "node_swap",
    "shuffle_hyperedges",
    "flag_complex",
    "flag_complex_d2",
    "random_flag_complex",
    "random_flag_complex_d2",
    "random_simplicial_complex",
    "uniform_configuration_model",
    "uniform_erdos_renyi_model",
    "uniform_stochastic_block_model",
    "uniform_planted_partition_model",
    "pseudofractal_simplicial_complex",
    "apollonian_complex",
    "network_geometry_flavor",
]
