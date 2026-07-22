def test_dual_dict():
    from hypergraph_rs.utils import dual_dict
    assert dual_dict({0: [1, 2, 3], 1: [4]}) == {1: [0], 2: [0], 3: [0], 4: [1]}


def test_to_graph_is_nx():
    import networkx as nx, hypergraph_rs as xgi
    G = xgi.to_graph(xgi.Hypergraph([[1, 2, 3]]))
    assert isinstance(G, nx.Graph) and G.number_of_nodes() == 3
