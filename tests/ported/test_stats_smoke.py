"""Task 11 failing-first smoke test for the `stats` Stat proxy (spec §9.1 conformance harness).

Not one of the 51 ported oracle files — a hand-authored Task-11 gate proving `H.degree()` and
`H.nodes.degree.asdict()` agree (mirrors `tests/stats/test_nodestats.py:12`).
"""


def test_degree_stat_dict_and_list():
    import hypergraph_rs as xgi

    H = xgi.Hypergraph([{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}])
    assert H.degree() == {1: 1, 2: 1, 3: 1, 4: 1, 5: 1, 6: 2, 7: 1, 8: 1}
    assert H.nodes.degree.asdict() == H.degree()
