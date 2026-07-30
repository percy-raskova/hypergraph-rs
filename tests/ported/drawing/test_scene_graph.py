"""Scene-graph replacement for XGI's ``test_draw.py``/``test_draw_utils.py`` (spec §9.3, Task 13).

The matplotlib-free rendering contract: ``H.render(strategy)`` returns the Rust `SceneGraph`
(``crates/hypergraph-rs/src/viz/mod.rs``) as a JSON-shaped dict.
"""


def test_render_produces_valid_scene_graph():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph()
    H.add_edge(["a", "b", "c"], idx="e1")
    scene = H.render("hyperedge_aware")
    assert len(scene["nodes"]) == 3
    assert len(scene["hyperedges"]) == 1
    # `add_edge` passes members through a Python `set` before storing, faithfully
    # reproducing real XGI (`members = set(members)` in hypergraph.py; mirrored at
    # crates/hypergraph-rs-python/src/hypergraph.rs:1291-1294). Member ORDER is
    # therefore CPython hash-set iteration order, which varies per process with
    # PYTHONHASHSEED. Asserting a specific order pins one sample of XGI's own
    # nondeterminism and fails ~11 runs in 12 (spec F2). The membership SET is
    # the actual contract.
    assert set(scene["hyperedges"][0]["member_ids"]) == {"a", "b", "c"}
    assert len(scene["hyperedges"][0]["member_ids"]) == 3  # no duplicates
    assert scene["bounding_box"][2] > scene["bounding_box"][0]  # max_x > min_x


def test_render_default_strategy_is_hyperedge_aware():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([[1, 2, 3]])
    assert H.render() == H.render("hyperedge_aware")


def test_render_node_glyphs_carry_positions():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([[1, 2]])
    scene = H.render("spring")
    for node in scene["nodes"]:
        assert isinstance(node["x"], float)
        assert isinstance(node["y"], float)


def test_render_unknown_strategy_raises_xgi_error():
    import pytest
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([[1, 2]])
    with pytest.raises(xgi.exception.XGIError):
        H.render("not-a-real-strategy")
