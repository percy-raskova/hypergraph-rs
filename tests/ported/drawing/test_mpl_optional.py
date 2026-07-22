"""The matplotlib `mpl` extra is optional (spec §6.3, Task 13): the core install never imports
matplotlib, and `draw()`/`draw_nodes()`/`draw_hyperedges()` raise a clear `ImportError` instead of
an `ImportError: No module named 'matplotlib'` from deep inside a call.
"""


def test_draw_requires_mpl_extra():
    import hypergraph_rs as xgi
    import hypergraph_rs.drawing as d

    if not d.HAS_MPL:
        import pytest

        with pytest.raises(ImportError, match="mpl"):
            xgi.draw(xgi.Hypergraph([[1, 2]]))


def test_layout_functions_do_not_require_mpl():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([[1, 2, 3]])
    pos = xgi.random_layout(H, seed=1)
    assert set(pos.keys()) == set(H.nodes)
