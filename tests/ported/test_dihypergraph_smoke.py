"""Task 6 failing-first smoke test for PyDiHypergraph (spec §9.1 conformance harness).

Not one of the 51 ported oracle files — a hand-authored Task-6 gate proving the directed
`(tail, head)` `add_edge` shape and the `DiEdgeView.dimembers` proxy round-trip.
"""


def test_directed_members_roundtrip():
    import hypergraph_rs as xgi

    H = xgi.DiHypergraph([({1, 2, 3}, {4}), ({5, 6}, {6, 7, 8})])  # diedgelist1
    assert H.edges.dimembers(0) == ({1, 2, 3}, {4})
    assert H.num_nodes == 8
