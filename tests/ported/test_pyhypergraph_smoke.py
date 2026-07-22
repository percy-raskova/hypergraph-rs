"""Task 4 failing-first smoke test for PyHypergraph (spec §9.1 conformance harness).

Not one of the 51 ported oracle files — a hand-authored Task-4 gate proving the PyId
translation layer (Task 2) round-trips through the real Hypergraph binding, and that the
duplicate-idx / missing-id deviation shims behave like XGI.
"""


def test_node_ids_preserve_python_type():  # THE Task-2 behavioral proof
    import hypergraph_rs as xgi

    H = xgi.Hypergraph()
    H.add_edge([1, 2, 3], idx="e1")
    assert list(H.nodes) == [1, 2, 3]  # ints, not "1","2","3"
    assert all(isinstance(n, int) for n in H.nodes)
    assert H.num_nodes == 3 and H.num_edges == 1


def test_duplicate_idx_warns_and_noops():
    import warnings

    import hypergraph_rs as xgi

    H = xgi.Hypergraph()
    H.add_edge([1, 2], idx="e1")
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        H.add_edge([3, 4], idx="e1")
        assert any(issubclass(x.category, UserWarning) for x in w)
    assert H.num_edges == 1  # no-op


def test_missing_node_raises_idnotfound():
    import pytest

    import hypergraph_rs as xgi
    from hypergraph_rs.exception import IDNotFound

    H = xgi.Hypergraph()
    with pytest.raises(IDNotFound):
        H.remove_node(99)
