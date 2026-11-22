"""Tests written for this library for what XGI 1.1 added or changed (plan 2026-11-21, Tasks 2-15).

These are repo-authored: they import the library as `xgi` and never touch the real XGI.
"""

import copy
import pickle

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
