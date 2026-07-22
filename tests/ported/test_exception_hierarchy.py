def test_id_not_found_is_keyerror():
    from hypergraph_rs.exception import IDNotFound, XGIException
    assert issubclass(IDNotFound, KeyError)
    assert issubclass(IDNotFound, XGIException)

def test_surface_matches_xgi_164():
    import hypergraph_rs as xgi
    assert len(set(xgi.__all__)) == 164
