def test_add_edge_blocked_on_sc():
    import pytest, hypergraph_rs as xgi
    from hypergraph_rs.exception import XGIError
    S = xgi.SimplicialComplex()
    with pytest.raises(XGIError):
        S.add_edge([1, 2, 3])


def test_closure_under_faces():
    import hypergraph_rs as xgi
    S = xgi.SimplicialComplex()
    S.add_simplices_from([[1, 2, 3]])
    assert {1, 2} in [set(S.edges.members(e)) for e in S.edges]
