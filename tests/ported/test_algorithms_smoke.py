def test_is_connected_and_components():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])  # 3 components
    assert xgi.is_connected(H) is False
    assert xgi.number_connected_components(H) == 3
