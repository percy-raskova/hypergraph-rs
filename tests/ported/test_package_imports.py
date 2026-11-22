def test_package_imports():
    import hypergraph_rs as xgi

    assert xgi.__version__ == "1.1-xgi-parity"


def test_all_has_165_symbols():
    import hypergraph_rs as xgi

    assert len(set(xgi.__all__)) == 165


def test_deep_import_targets_resolve():
    from hypergraph_rs.exception import IDNotFound, XGIError  # noqa: F401
    from hypergraph_rs.utils import dual_dict, IDDict  # noqa: F401
