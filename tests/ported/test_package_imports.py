def test_package_imports():
    import hypergraph_rs as xgi

    assert xgi.__version__ == "0.10.2-xgi-parity"


def test_all_has_164_symbols():
    import hypergraph_rs as xgi

    assert len(set(xgi.__all__)) == 164


def test_deep_import_targets_resolve():
    from hypergraph_rs.exception import IDNotFound, XGIError  # noqa: F401
    from hypergraph_rs.utils import dual_dict, IDDict  # noqa: F401
    from hypergraph_rs.readwrite.json import read_json, write_json  # noqa: F401
