def test_pyi_shipped():
    import importlib.util, os
    root = os.path.dirname(importlib.util.find_spec("hypergraph_rs").origin)
    assert os.path.exists(os.path.join(root, "py.typed"))
    assert os.path.exists(os.path.join(root, "_hypergraph_rs_core.pyi"))
