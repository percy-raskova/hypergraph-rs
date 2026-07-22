def test_random_hypergraph_structural():
    import hypergraph_rs as xgi

    H = xgi.random_hypergraph(10, [0.2], seed=1)
    assert H.num_nodes == 10                  # structural (§9.4 option a)
    assert all(isinstance(n, int) for n in H.nodes)
