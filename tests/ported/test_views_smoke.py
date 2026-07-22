"""Task 5 failing-first smoke test for PyNodeView/PyEdgeView (spec §9.1 conformance harness).

Not one of the 51 ported oracle files — a hand-authored Task-5 gate proving H.edges.members()
returns a set (deviation shim) and H.nodes.degree wires through to the Task-11 Stat proxy.
"""


def test_edge_members_is_a_set():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([[1, 2, 3], [4]])
    assert H.edges.members(0) == {1, 2, 3}  # a set, ints preserved
    assert isinstance(H.edges.members(0), set)


def test_node_degree_asdict():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])  # edgelist1
    assert H.nodes.degree.asdict() == {1: 1, 2: 1, 3: 1, 4: 1, 5: 1, 6: 2, 7: 1, 8: 1}
