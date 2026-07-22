def test_dual_dict():
    from hypergraph_rs.utils import dual_dict
    # XGI parity (verified xgi.utils.utilities.dual_dict docstring): values are `set`s, not
    # lists — `defaultdict(set)` (Task 16 execution-time fix, hypergraph-rs-python/src/utils.rs).
    assert dual_dict({0: [1, 2, 3], 1: [4]}) == {1: {0}, 2: {0}, 3: {0}, 4: {1}}


def test_to_graph_is_nx():
    import networkx as nx, hypergraph_rs as xgi
    G = xgi.to_graph(xgi.Hypergraph([[1, 2, 3]]))
    assert isinstance(G, nx.Graph) and G.number_of_nodes() == 3
