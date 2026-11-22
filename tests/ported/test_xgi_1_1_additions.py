"""Tests written for this library for what XGI 1.1 added or changed (plan 2026-11-21, Tasks 2-15).

These are repo-authored: they import the library as `xgi` and never touch the real XGI.
"""

import copy
import pickle

import numpy as np
import pytest

import hypergraph_rs as xgi


@pytest.mark.parametrize(
    "cls,edges",
    [
        (xgi.Hypergraph, [[1, 2], [2, 3]]),
        (xgi.DiHypergraph, [([1], [2]), ([2], [3])]),
        (xgi.SimplicialComplex, [[1, 2], [3, 4]]),
    ],
)
def test_edge_uid_survives_copy_pickle_and_removal(cls, edges):
    H = cls(edges)
    expected = H._edge_uid
    last = list(H.edges)[-1]
    if cls is xgi.SimplicialComplex:
        H.remove_simplex_id(last)
    else:
        H.remove_edge(last)
    for clone in (H.copy(), copy.copy(H), copy.deepcopy(H), pickle.loads(pickle.dumps(H))):
        assert clone._edge_uid == expected
        assert list(clone.edges) == list(H.edges)


def test_simplicial_complex_pickles_every_simplex():
    S = xgi.SimplicialComplex([[1, 2, 3], [3, 4]])
    T = pickle.loads(pickle.dumps(S))
    assert list(T.edges) == list(S.edges)
    assert T.edges.members(dtype=list) == S.edges.members(dtype=list)


def test_edge_uid_is_read_only():
    with pytest.raises(AttributeError):
        xgi.Hypergraph()._edge_uid = 5


def test_memberships_dtype(edgelist1):
    H = xgi.Hypergraph(edgelist1)
    assert H.nodes.memberships(dtype=dict) == H.nodes.memberships()
    assert H.nodes.memberships(dtype=list) == [H.nodes.memberships(n) for n in H.nodes]
    assert H.nodes([1, 2]).memberships(dtype=list) == [H.nodes.memberships(1), H.nodes.memberships(2)]
    with pytest.raises(xgi.exception.XGIError, match="Unrecognized dtype"):
        H.nodes.memberships(dtype=tuple)


def test_di_memberships_dtype(diedgelist2):
    D = xgi.DiHypergraph(diedgelist2)
    assert D.nodes.dimemberships(dtype=list) == [D.nodes.dimemberships(n) for n in D.nodes]
    assert D.nodes.memberships(dtype=list) == [D.nodes.memberships(n) for n in D.nodes]


def test_sources_targets_warn_and_delegate(diedgelist2):
    D = xgi.DiHypergraph(diedgelist2)
    with pytest.warns(DeprecationWarning, match="DiEdgeView.sources is deprecated") as rec:
        assert D.edges.sources(0) == D.edges.tail(0)
    assert rec[0].filename == __file__
    with pytest.warns(DeprecationWarning, match="DiEdgeView.targets is deprecated"):
        assert D.edges.targets(dtype=list) == D.edges.head(dtype=list)


def test_add_weighted_edges_from_sets_weights():
    H = xgi.Hypergraph()
    H.add_weighted_edges_from([(0, 1, 0.3), (0, 2, 0.8)], color="red")
    assert H.edges.members(dtype=list) == [{0, 1}, {0, 2}]
    assert H.edges.attrs("weight").aslist() == [0.3, 0.8]
    assert H.edges.attrs("color").aslist() == ["red", "red"]
    H.add_weighted_edges_from([(3, 4, 2.0)], weight="w")
    assert H.edges.attrs("w").aslist()[-1] == 2.0
    H.add_weighted_edges_from(ebunch=[(5, 6, 1.5)])  # XGI's keyword name
    assert H.edges.attrs("weight").aslist()[-1] == 1.5


def test_add_weighted_adds_reject_empty_input():
    with pytest.raises(ValueError, match="Empty or invalid edges specified."):
        xgi.Hypergraph().add_weighted_edges_from([()])
    with pytest.raises(ValueError, match="Empty or invalid simplices specified."):
        xgi.SimplicialComplex().add_weighted_simplices_from([()])


def test_add_weighted_simplices_from_sets_weights():
    S = xgi.SimplicialComplex()
    S.add_weighted_simplices_from([(0, 1, 0.3), (0, 2, 0.8)])
    assert [S.edges[e]["weight"] for e in S.edges] == [0.3, 0.8]  # SC edge views have no attrs stat
    with pytest.raises(NotImplementedError):
        S.add_weighted_simplices_from([(1, 2, 3, 0.5)], max_order=1)


def test_weighted_adds_refuse_a_frozen_graph():
    H = xgi.Hypergraph([[1, 2]])
    H.freeze()
    with pytest.raises(xgi.exception.XGIError):
        H.add_weighted_edges_from([(1, 3, 0.5)])


def test_add_weighted_edges_from_keeps_the_edges_before_a_bad_entry():
    # XGI 1.1 feeds a generator to `add_edges_from`: each edge before the bad entry is added, then the
    # IndexError of `edge[-1]` on `()` becomes ValueError. A bad first entry adds nothing.
    H = xgi.Hypergraph()
    with pytest.raises(ValueError, match="Empty or invalid edges specified."):
        H.add_weighted_edges_from([(0, 1, 0.3), (2, 3, 0.5), (), (4, 5, 0.7)])
    assert H.edges.members(dtype=list) == [{0, 1}, {2, 3}]
    assert [H.edges[e]["weight"] for e in H.edges] == [0.3, 0.5]
    assert H._edge_uid == 2
    G = xgi.Hypergraph()
    with pytest.raises(ValueError, match="Empty or invalid edges specified."):
        G.add_weighted_edges_from([(), (0, 1, 0.3)])
    assert G.num_nodes == 0 and G.num_edges == 0


def test_add_weighted_simplices_from_keeps_the_simplices_before_a_bad_entry_without_their_faces():
    # XGI 1.1's `add_simplices_from` collects the subfaces and adds them AFTER its loop, so the error
    # raised inside the loop leaves the simplices read so far and none of their faces.
    S = xgi.SimplicialComplex()
    with pytest.raises(ValueError, match="Empty or invalid simplices specified."):
        S.add_weighted_simplices_from([(0, 1, 2, 0.1), (3, 4, 0.2), (), (5, 6, 0.3)])
    assert S.edges.members(dtype=list) == [{0, 1, 2}, {3, 4}]  # no [0, 1], [0, 2], [1, 2]
    assert [S.edges[e]["weight"] for e in S.edges] == [0.1, 0.2]
    assert S._edge_uid == 2
    T = xgi.SimplicialComplex()
    with pytest.raises(ValueError, match="Empty or invalid simplices specified."):
        T.add_weighted_simplices_from([(), (0, 1, 0.3)])
    assert T.num_nodes == 0 and T.num_edges == 0


def test_weighted_adds_refuse_a_frozen_graph_before_reading_the_bunch():
    H = xgi.Hypergraph([[1, 2]])
    H.freeze()
    with pytest.raises(xgi.exception.XGIError, match="Frozen"):
        H.add_weighted_edges_from([()])
    S = xgi.SimplicialComplex([[1, 2]])
    S.freeze()
    with pytest.raises(xgi.exception.XGIError, match="Frozen"):
        S.add_weighted_simplices_from([()])


def test_duplicate_groups_edges_and_nodes(edgelist1):
    H = xgi.Hypergraph([[0, 1], [2, 3], [1, 0], [3, 2], [4, 5]])
    assert [list(g) for g in H.edges.duplicate_groups()] == [[0, 2], [1, 3]]
    assert all(isinstance(g, type(H.edges)) for g in H.edges.duplicate_groups())
    assert xgi.Hypergraph([[0, 1], [1, 2]]).edges.duplicate_groups() == []
    assert {2, 3, 8} <= {i for g in xgi.Hypergraph(edgelist1).nodes.duplicate_groups() for i in g}
    S = xgi.SimplicialComplex([[1, 2, 3]])
    assert S.edges.duplicate_groups() == []


def test_duplicate_groups_of_a_filtered_view_cover_the_network():
    F = xgi.Hypergraph([[0, 1], [1, 0], [4, 5, 6], [6, 5, 4]])
    assert [list(g) for g in F.edges.filterby("size", 2).duplicate_groups()] == [[0, 1], [2, 3]]


def test_duplicate_groups_on_a_simplicial_complex_nodes():
    # Nodes 1 and 2 are in the simplex [1, 2] and nothing else: the same memberships. In the closure of
    # [3, 4, 5] no two nodes share their memberships (3 is in {34, 35, 345}, 4 in {34, 45, 345}).
    S = xgi.SimplicialComplex([[1, 2], [3, 4, 5]])
    groups = S.nodes.duplicate_groups()
    assert isinstance(groups, list)
    assert all(isinstance(g, type(S.nodes)) for g in groups)
    assert [list(g) for g in groups] == [[1, 2]]
    assert xgi.SimplicialComplex([[3, 4, 5]]).nodes.duplicate_groups() == []


def test_duplicate_groups_of_a_scoped_node_view_cover_the_network():
    # XGI 1.1's `IDView.duplicate_groups` (xgi/core/views.py) iterates `self._id_dict`, the WHOLE
    # network's, whatever ids the view holds. This is XGI's behaviour, not a defect: do not "repair" it.
    H = xgi.Hypergraph([[0, 1], [2, 3], [4, 5, 6]])
    assert [list(g) for g in H.nodes([0, 4]).duplicate_groups()] == [[0, 1], [2, 3], [4, 5, 6]]
    S = xgi.SimplicialComplex([[1, 2], [3, 4], [5, 6, 7]])
    scoped = S.nodes.duplicate_groups()[1]  # the view of nodes 3 and 4 alone
    assert list(scoped) == [3, 4]
    assert [list(g) for g in scoped.duplicate_groups()] == [[1, 2], [3, 4]]


def test_duplicate_groups_of_a_scoped_edge_view_cover_the_network():
    # Same XGI function as above: `IDView.duplicate_groups` iterates the network's `_id_dict`.
    H = xgi.Hypergraph([[0, 1], [1, 0], [2, 3], [3, 2], [4, 5, 6]])
    assert [list(g) for g in H.edges([0]).duplicate_groups()] == [[0, 1], [2, 3]]


def test_simplicial_complex_nodes_memberships():
    # simplex 0 is [1, 2], simplex 1 is [3, 4, 5]; the face ids that follow are the library's to number
    S = xgi.SimplicialComplex([[1, 2], [3, 4, 5]])
    assert S.nodes.memberships(1) == {0}
    three = S.nodes.memberships(3)
    assert len(three) == 3 and 1 in three  # [3, 4, 5], [3, 4], [3, 5]
    assert {S.edges.members(e) for e in three} == {frozenset({3, 4, 5}), frozenset({3, 4}), frozenset({3, 5})}
    everything = S.nodes.memberships()
    assert list(everything) == [1, 2, 3, 4, 5]
    assert everything[1] == everything[2] == {0} and everything[3] == three
    assert S.nodes.memberships(dtype=dict) == everything
    assert S.nodes.memberships(dtype=list) == list(everything.values())
    assert S.nodes.duplicate_groups()[0].memberships(dtype=list) == [{0}, {0}]
    with pytest.raises(xgi.exception.XGIError, match="Unrecognized dtype"):
        S.nodes.memberships(dtype=tuple)
    with pytest.raises(xgi.exception.IDNotFound):
        S.nodes.memberships(99)


def test_ring_lattice_positions_and_pos_stat():
    H = xgi.ring_lattice(8, 3, 2, 0, with_positions=True)
    pos = H.nodes.pos.asdict()
    assert set(pos) == set(range(8))
    assert pos[2] == pytest.approx((0.0, 1.0), abs=1e-12)
    assert xgi.ring_lattice(5, 2, 2, 0).nodes.pos.asdict() == {i: None for i in range(5)}
    assert H.pos()[2] == pos[2]
    assert "pos" in dir(H.nodes)  # upstream test_pos_stat checks this but stays a strict xfail (D23)
    D = xgi.DiHypergraph([([1], [2])])
    D.set_node_attributes({1: [0, 0]}, name="pos")
    assert D.nodes.pos.asdict() == {1: [0, 0], 2: None} and "pos" in dir(D.nodes)


def test_pos_stat_takes_no_argument_and_fails_when_evaluated():
    # XGI 1.1's `pos(net, bunch)` takes no argument. `IDStat.__call__` only stores the arguments, so
    # `H.nodes.pos("color")` is made without error and raises TypeError when the stat is evaluated.
    H = xgi.ring_lattice(5, 2, 2, 0, with_positions=True)
    H.set_node_attributes({0: "red"}, name="color")
    stat = H.nodes.pos("color")
    assert H.nodes.pos().asdict() == H.nodes.pos.asdict()
    with pytest.raises(TypeError, match=r"pos\(\) takes 2 positional arguments but 3 were given"):
        stat.asdict()
    with pytest.raises(TypeError):
        stat.aslist()
    with pytest.raises(TypeError):
        stat[0]
    with pytest.raises(xgi.exception.IDNotFound):
        stat[99]
    with pytest.raises(TypeError, match=r"pos\(\) got an unexpected keyword argument 'attr'"):
        H.nodes.pos(attr="color").asdict()
    D = xgi.DiHypergraph([([1], [2])])
    D.set_node_attributes({1: [0, 0]}, name="pos")
    dstat = D.nodes.pos("color")
    with pytest.raises(TypeError, match=r"pos\(\) takes 2 positional arguments but 3 were given"):
        dstat.asdict()
    assert D.nodes.pos().asdict() == {1: [0, 0], 2: None}


def test_ring_lattice_edges_by_path():
    assert xgi.generators.lattice._ring_lattice_edges(20, 3, 4, 2)[0] == [0, 3, 4]


def test_ring_lattice_edges_of_no_nodes_is_empty():
    # XGI 1.1's comprehension over `range(0)` never reaches `% n`: an empty list, not an error.
    assert xgi.generators.lattice._ring_lattice_edges(0, 3, 4, 2) == []


def assert_equal_results(a, b):
    if isinstance(a, dict):
        assert a.keys() == b.keys()
        for k in a:
            assert_equal_results(a[k], b[k])
    elif isinstance(a, (tuple, list)) and a and not isinstance(a[0], (int, str)):
        assert len(a) == len(b)
        for x, y in zip(a, b):
            assert_equal_results(x, y)
    elif isinstance(a, (np.ndarray, tuple)) or isinstance(a, list) and a and isinstance(a[0], float):
        assert np.allclose(a, b)
    else:
        assert a == b


def _shuffled(s):
    H = xgi.random_hypergraph(30, [0.15, 0.03], seed=11)
    H.random_edge_shuffle(seed=s)
    return H.edges.members(dtype=list)


_H3 = xgi.Hypergraph([[1, 2], [2, 3], [3, 4, 1], [4, 5], [1, 2, 3]])
_BIG = xgi.random_hypergraph(40, [0.1, 0.03], seed=3)

SEEDED = [
    lambda s: xgi.random_hypergraph(40, [0.1, 0.03], seed=s).edges.members(dtype=list),
    lambda s: xgi.fast_random_hypergraph(40, [0.1, 0.03], seed=s).edges.members(dtype=list),
    lambda s: xgi.chung_lu_hypergraph({i: 2 for i in range(20)}, {i: 4 for i in range(10)}, seed=s)
    .edges.members(dtype=list),
    lambda s: xgi.watts_strogatz_hypergraph(40, 3, 4, 2, 0.5, seed=s).edges.members(dtype=list),
    lambda s: xgi.uniform_erdos_renyi_model(20, 3, 0.2, seed=s).edges.members(dtype=list),
    lambda s: xgi.uniform_configuration_model({i: 3 for i in range(30)}, 3, seed=s)
    .edges.members(dtype=list),
    lambda s: xgi.random_simplicial_complex(20, [0.3, 0.05], seed=s).edges.members(dtype=list),
    lambda s: xgi.spectral_clustering(xgi.random_hypergraph(40, [0.2, 0.1], seed=3), 3, seed=s),
    lambda s: xgi.random_layout(_BIG, seed=s),
    lambda s: xgi.pairwise_spring_layout(_BIG, seed=s),
    _shuffled,
]


def _differ(a, b):
    try:
        assert_equal_results(a, b)
    except AssertionError:
        return True
    return False


@pytest.mark.parametrize("run", SEEDED)
def test_fresh_generators_from_one_seed_agree(run):
    assert_equal_results(run(np.random.default_rng(5)), run(np.random.default_rng(5)))


@pytest.mark.parametrize("run", SEEDED)
def test_generator_seed_is_consumed(run):
    rng = np.random.default_rng(5)
    before = rng.bit_generator.state
    run(rng)
    assert rng.bit_generator.state != before


def test_bipartite_spring_layout_consumes_a_generator_and_ignores_the_seed():
    rng = np.random.default_rng(5)
    before = rng.bit_generator.state
    a = xgi.bipartite_spring_layout(_H3, seed=rng)
    assert rng.bit_generator.state != before
    assert_equal_results(a, xgi.bipartite_spring_layout(_H3, seed=7))


@pytest.mark.parametrize("run", SEEDED)
def test_the_same_integer_seed_twice_gives_the_same_result(run):
    assert_equal_results(run(7), run(7))


@pytest.mark.parametrize("run", SEEDED)
def test_different_integer_seeds_give_different_results(run):
    assert _differ(run(1), run(2))


@pytest.mark.parametrize("run", SEEDED)
def test_different_generators_give_different_results(run):
    assert _differ(run(np.random.default_rng(1)), run(np.random.default_rng(2)))


def test_degree_assortativity_accepts_a_generator():
    # XGI 1.1 never draws on the exact path: `default_rng(gen)` hands the same Generator back.
    rng = np.random.default_rng(5)
    before = rng.bit_generator.state
    expected = xgi.degree_assortativity(_H3, exact=True)
    assert xgi.degree_assortativity(_H3, exact=True, seed=rng) == expected
    assert rng.bit_generator.state == before
    with pytest.raises(TypeError):
        xgi.degree_assortativity(_H3, exact=True, seed="x")
    with pytest.raises(ValueError):
        xgi.degree_assortativity(_H3, exact=True, seed=-1)


def test_degree_assortativity_sampling_is_not_ported_and_draws_nothing():
    # The sampling path (`exact=False`) is a recorded gap (compat/partial_symbols.toml): there is no
    # draw for a seed to steer, so the same-seed / different-seed checks of SEEDED do not apply to it.
    rng = np.random.default_rng(5)
    before = rng.bit_generator.state
    with pytest.raises(NotImplementedError):
        xgi.degree_assortativity(_H3, exact=False, seed=rng)
    assert rng.bit_generator.state == before


def test_a_frozen_graph_refuses_random_edge_shuffle_without_drawing():
    H = xgi.Hypergraph([[1, 2], [2, 3, 4], [4, 5], [1, 5, 6]])
    H.freeze()
    rng = np.random.default_rng(5)
    before = rng.bit_generator.state
    with pytest.raises(xgi.exception.XGIError, match="Frozen"):
        H.random_edge_shuffle(seed=rng)
    assert rng.bit_generator.state == before


@pytest.mark.parametrize("bad,err", [(-1, ValueError), (2**64, ValueError), ("x", TypeError), (1.5, TypeError)])
def test_bad_seeds_are_refused(bad, err):
    with pytest.raises(err):
        xgi.random_hypergraph(10, [0.3], seed=bad)


def test_numpy_integer_and_bool_seeds_are_ints():
    assert SEEDED[0](np.int64(4)) == SEEDED[0](4)
    assert SEEDED[0](True) == SEEDED[0](1)


def test_members_default_is_a_dict_and_list_keeps_view_order(edgelist1):
    H = xgi.Hypergraph(edgelist1)
    assert H.edges.members() == {e: H.edges.members(e) for e in H.edges}
    sub = H.edges.filterby("size", 2)
    assert sub.members(dtype=list) == [H.edges.members(e) for e in sub]
    with pytest.raises(xgi.exception.XGIError, match="Unrecognized dtype"):
        H.edges.members(dtype=tuple)
    with pytest.raises(xgi.exception.XGIError, match="Unrecognized dtype"):
        xgi.SimplicialComplex([[1, 2]]).edges.members(dtype=tuple)


def test_dimembers_default(diedgelist2):
    D = xgi.DiHypergraph(diedgelist2)
    assert D.edges.dimembers() == {e: D.edges.dimembers(e) for e in D.edges}
    assert D.edges.tail() == {e: D.edges.tail(e) for e in D.edges}
    assert D.edges.head() == {e: D.edges.head(e) for e in D.edges}
    assert D.edges.members() == {e: D.edges.members(e) for e in D.edges}
    assert D.edges.tail(dtype=list) == [D.edges.tail(e) for e in D.edges]


def test_reprs_are_dict_shaped():
    assert repr(xgi.Hypergraph()) == "Hypergraph({})"
    assert repr(xgi.Hypergraph([[1, 2]])) == "Hypergraph({0: {1, 2}})"
    H = xgi.Hypergraph()
    H.add_edge([])
    assert repr(H) == "Hypergraph({0: set()})"
    assert repr(xgi.DiHypergraph([([1], [2])])) == "DiHypergraph({0: ({1}, {2})})"
    assert repr(xgi.SimplicialComplex([[1, 2]])).startswith("SimplicialComplex({")


def test_directed_edge_must_be_a_pair():
    with pytest.raises(TypeError, match="Directed edge must be a list or tuple!"):
        xgi.DiHypergraph().add_edge({1, 2})


def test_directed_edge_type_checks_in_every_adder():
    with pytest.raises(TypeError, match="Directed edge must be a list or tuple!"):
        xgi.DiHypergraph().add_edges_from({"a": {1, 2}})
    with pytest.raises(TypeError, match="Directed edge must be a list or tuple!"):
        xgi.DiHypergraph().add_edge("ab")
    # the type is checked, not the length: a one-element list is an IndexError in XGI, never the TypeError
    with pytest.raises(IndexError):
        xgi.DiHypergraph().add_edge([[1]])
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        xgi.DiHypergraph().add_edges_from({"a": (1, 2)})
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        xgi.DiHypergraph().add_edges_from([((1, 2), 5)])


def test_invalid_direction():
    D = xgi.DiHypergraph([([1], [2])])
    with pytest.raises(ValueError, match="Invalid direction!"):
        D.add_node_to_edge(0, 3, direction="sideways")
    with pytest.raises(ValueError, match="Invalid direction!"):
        D.remove_node_from_edge(0, 1, direction="sideways")


def test_simplex_members_as_a_string():
    with pytest.raises(TypeError, match="Members cannot be specified as a string"):
        xgi.SimplicialComplex().add_simplices_from(["a", "b", "c"])
    with pytest.raises(TypeError, match="Members cannot be specified as a string"):
        xgi.SimplicialComplex().add_simplices_from([("foo", {"color": "red"})])
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        xgi.SimplicialComplex().add_simplices_from([0, 1, 2])
    with pytest.raises(TypeError, match="The simplex cannot be cast to a frozenset."):
        xgi.SimplicialComplex().add_simplex(5)
    S = xgi.SimplicialComplex()
    S.add_simplex("abc")
    assert set(S.nodes) == {"a", "b", "c"}


def test_simplex_exception_types_are_not_xgi_errors():
    for bad in ([0, 1, 2],):
        with pytest.raises(ValueError) as info:
            xgi.SimplicialComplex().add_simplices_from(bad)
        assert not isinstance(info.value, xgi.exception.XGIError)
    with pytest.raises(TypeError) as info:
        xgi.SimplicialComplex().add_simplex(None)
    assert not isinstance(info.value, xgi.exception.XGIError)


_NODE_CLASSES = [xgi.Hypergraph, xgi.DiHypergraph, xgi.SimplicialComplex]


def _node_attrs(H):
    """Node attributes as {node: dict}; a SimplicialComplex reads them from its pickle state."""
    if isinstance(H, xgi.Hypergraph):
        return H.nodes.attrs.asdict()
    if isinstance(H, xgi.DiHypergraph):
        return {n: H.nodes[n] for n in H.nodes}
    return dict(H.__reduce__()[1][0]["nodes"])


def _check_attrs(H, expected):
    assert _node_attrs(H) == expected


@pytest.mark.parametrize("cls", _NODE_CLASSES)
def test_add_nodes_from_dict_with_global_attrs(cls):
    H = cls()
    H.add_nodes_from({1: {"color": "red"}, 2: {"color": "blue", "weight": 5}}, weight=1)
    assert list(H.nodes) == [1, 2]
    _check_attrs(H, {1: {"color": "red", "weight": 1}, 2: {"color": "blue", "weight": 5}})


@pytest.mark.parametrize("cls", _NODE_CLASSES)
def test_two_tuple_is_node_and_attrs(cls):
    H = cls()
    H.add_nodes_from([(1, None), (2, {}), (3, {"a": 1})])
    assert list(H.nodes) == [1, 2, 3]
    _check_attrs(H, {1: {}, 2: {}, 3: {"a": 1}})
    with pytest.raises(TypeError, match="the second element must be a dictionary or None"):
        H.add_nodes_from([(1, 2)])
    H.add_nodes_from([(7, 8, 9)])  # a 3-tuple is a node id
    assert (7, 8, 9) in H.nodes


@pytest.mark.parametrize("cls", _NODE_CLASSES)
def test_add_nodes_from_updates_an_existing_node(cls):
    H = cls()
    H.add_nodes_from([(1, {"a": 1, "b": 2})])
    H.add_nodes_from([(1, {"b": 3})], c=4)
    _check_attrs(H, {1: {"a": 1, "b": 3, "c": 4}})


# --- the reviews' loose ends on tasks 8 to 12 -------------------------------------------------------------


def test_simplicial_constructor_goes_through_add_simplices_from():
    # a non-iterable element is a ValueError, as in add_simplices_from (it was a TypeError)
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        xgi.SimplicialComplex([5])
    # a list of pairs is ONE simplex {1, 2} with the id 'a' (XGI: from_hyperedge_list -> add_simplices_from)
    S = xgi.SimplicialComplex([([1, 2], "a")])
    assert S.edges.members(dtype=dict) == {"a": frozenset({1, 2})}
    # triples are (members, id, attrs)
    S = xgi.SimplicialComplex([([1, 2], "a", {"color": "red"})])
    assert S.edges.members(dtype=dict) == {"a": frozenset({1, 2})}
    assert S.edges["a"] == {"color": "red"}
    # a dict is handed over as the pairs (members, uid)
    S = xgi.SimplicialComplex({"x": [1, 2, 3]})
    assert S.edges.members(dtype=dict)["x"] == frozenset({1, 2, 3})
    assert len(S.edges) == 4
    # a duplicate id warns WITH the full stop that add_simplices_from writes
    with pytest.warns(UserWarning, match=r"uid a already exists, cannot add simplex \{3, 4\}\.$"):
        xgi.SimplicialComplex([([1, 2], "a"), ([3, 4], "a")])


def test_simplicial_constructor_from_a_plain_list_builds_what_it_built():
    data = [[1, 2, 3], [3, 4], [4, 5, 6]]
    S = xgi.SimplicialComplex(data)
    T = xgi.SimplicialComplex()
    for members in data:
        T.add_simplex(members)
    assert S.edges.members(dtype=dict) == T.edges.members(dtype=dict)
    assert S.edges.members(dtype=dict)[0] == frozenset({1, 2, 3})
    assert S.edges.members(dtype=dict)[4] == frozenset({3, 4})


def test_di_add_edge_duplicate_id_warns_before_tail_and_head_are_read():
    # XGI: the type test and members[0], members[1] first, then the duplicate id warns and returns,
    # and only then are the tail and the head iterated.
    D = xgi.DiHypergraph()
    D.add_edge(([1], [2]), idx="a")
    with pytest.warns(UserWarning, match="uid a already exists, cannot add edge"):
        D.add_edge((5, [2]), idx="a")
    assert list(D.edges) == ["a"]
    assert list(D.nodes) == [1, 2]
    assert D.edges.dimembers("a") == ({1}, {2})
    # the type test and the indices still come first
    with pytest.raises(TypeError, match="Directed edge must be a list or tuple!"):
        D.add_edge({1, 2}, idx="a")
    with pytest.raises(IndexError):
        D.add_edge([[1]], idx="a")


def test_di_add_edges_from_dict_duplicate_id_warns_before_the_type_check():
    D = xgi.DiHypergraph()
    D.add_edge(([1], [2]), idx="a")
    with pytest.warns(UserWarning, match=r"uid a already exists, cannot add edge .*\.$"):
        D.add_edges_from({"a": {1, 2}})  # not a list or tuple, but the duplicate id comes first
    with pytest.warns(UserWarning, match="uid a already exists"):
        D.add_edges_from({"a": (5, [2])})
    assert list(D.edges) == ["a"]
    assert list(D.nodes) == [1, 2]
    # the list forms warn first, too: members are indexed only for a new id
    with pytest.warns(UserWarning, match="uid a already exists"):
        D.add_edges_from([(5, "a")])
    assert list(D.edges) == ["a"]


def test_add_simplices_from_dict_form_raises_cpythons_type_error_before_the_duplicate_check():
    S = xgi.SimplicialComplex()
    S.add_simplex([1, 2], idx="a")
    # XGI evaluates `not members or self.has_simplex(members)` first: frozenset(5) is a TypeError
    with pytest.raises(TypeError) as info:
        S.add_simplices_from({"a": 5})
    assert not isinstance(info.value, xgi.exception.XGIError)
    with pytest.raises(TypeError):
        S.add_simplices_from({"b": [[1], 2]})  # an unhashable element
    # falsy members are skipped silently
    S.add_simplices_from({"c": []})
    assert list(S.edges) == ["a"]
    # in the list forms iter(members) is tested first: ValueError("Invalid ebunch format")
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        S.add_simplices_from([[1, 2], 5])  # {1, 2} is present and skipped; 5 is not iterable
    with pytest.raises(ValueError, match="Invalid ebunch format"):
        S.add_simplices_from([5])


def test_add_simplices_from_dict_form_sets_no_keyword_attributes():
    # XGI (simplicialcomplex.py, add_simplices_from, format 5): `self._add_simplex(frozenset(members), idx)`
    # is called WITHOUT **attr, so the keyword attributes reach no simplex of the dict form.
    S = xgi.SimplicialComplex()
    S.add_simplices_from({"a": [1, 2]}, color="red")
    assert S.edges["a"] == {}
    # the list forms do set them
    S.add_simplices_from([([3, 4], "b")], color="red")
    assert S.edges["b"] == {"color": "red"}


def test_none_cannot_be_a_new_node_of_add_simplex_or_the_dict_form():
    S = xgi.SimplicialComplex()
    with pytest.raises(ValueError, match="None cannot be a node"):
        S.add_simplex([None, 1])
    assert list(S.nodes) == [] and list(S.edges) == [] and S._edge_uid == 0
    with pytest.raises(ValueError, match="None cannot be a node"):
        S.add_simplex([None, 1], idx="a")
    assert list(S.nodes) == [] and list(S.edges) == [] and S._edge_uid == 0
    with pytest.raises(ValueError, match="None cannot be a node"):
        S.add_simplices_from({"a": [None, 1]})
    assert list(S.nodes) == [] and list(S.edges) == [] and S._edge_uid == 0
    # the list forms add their nodes inline and accept None
    S.add_simplices_from([[None, 1]])
    assert None in S.nodes
    # None is a node now: add_simplex raises nothing
    S.add_simplex([None, 2])
    assert 2 in S.nodes
