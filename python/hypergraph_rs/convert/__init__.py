"""Convert (mirrors xgi/convert/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['from_bipartite_edgelist', 'to_bipartite_edgelist', 'from_bipartite_graph', 'to_bipartite_graph', 'to_graph', 'cut_to_order', 'from_hyperedge_dict', 'to_hyperedge_dict', 'from_hyperedge_list', 'to_hyperedge_list', 'to_hypergraph_dict', 'from_hypergraph_dict']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 12 binds it

# Phase 6 'Deferred'; pandas pair never ported per spec Sec4.4
to_hif_dict = make_deferred("to_hif_dict")
from_hif_dict = make_deferred("from_hif_dict")
to_encapsulation_dag = make_deferred("to_encapsulation_dag")
empirical_subsets_filter = make_deferred("empirical_subsets_filter")
to_line_graph = make_deferred("to_line_graph")
from_simplex_dict = make_deferred("from_simplex_dict")
from_max_simplices = make_deferred("from_max_simplices")
k_skeleton = make_deferred("k_skeleton")
from_incidence_matrix = make_deferred("from_incidence_matrix")
to_incidence_matrix = make_deferred("to_incidence_matrix")
to_hypergraph = make_deferred("to_hypergraph")
to_dihypergraph = make_deferred("to_dihypergraph")
to_simplicial_complex = make_deferred("to_simplicial_complex")
from_bipartite_pandas_dataframe = make_deferred("from_bipartite_pandas_dataframe")
to_bipartite_pandas_dataframe = make_deferred("to_bipartite_pandas_dataframe")

__all__ = ['from_bipartite_edgelist', 'to_bipartite_edgelist', 'from_bipartite_graph', 'to_bipartite_graph', 'to_encapsulation_dag', 'empirical_subsets_filter', 'to_graph', 'to_hif_dict', 'from_hif_dict', 'to_hypergraph', 'to_dihypergraph', 'to_simplicial_complex', 'cut_to_order', 'from_hyperedge_dict', 'to_hyperedge_dict', 'from_hyperedge_list', 'to_hyperedge_list', 'to_hypergraph_dict', 'from_hypergraph_dict', 'from_incidence_matrix', 'to_incidence_matrix', 'to_line_graph', 'from_bipartite_pandas_dataframe', 'to_bipartite_pandas_dataframe', 'from_simplex_dict', 'from_max_simplices', 'k_skeleton']
