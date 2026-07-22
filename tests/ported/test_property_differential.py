"""ADR052-pattern differential test: random op sequences behave identically in hypergraph_rs
and real xgi (installed as a test-only oracle)."""
import random
import pytest

xgi_real = pytest.importorskip("xgi")   # skip if the oracle isn't installed
import hypergraph_rs as hr


def _apply(H, ops):
    for _, payload in ops:
        try:
            H.add_edge(payload)
        except Exception:
            pass
    return H


# PyKey stress (orchestrator ruling): exercise the id-translation layer over MANY hashable id TYPES,
# not just small ints — this is the load-bearing verification that ints/strings/tuples/floats/bools all
# round-trip by identity through hypergraph_rs exactly as through real xgi.
ID_POOL = [1, 2, 3, 0, -5, "a", "b", "node_x", (1, 2), (3, 4), 3.5, True, "1", 10, 99]


@pytest.mark.parametrize("seed", range(50))     # bounded: fixed 50 sequences (widened from 25)
def test_differential_membership_and_order(seed):
    rng = random.Random(seed)
    ops = [("add_edge", [ID_POOL[i] for i in rng.sample(range(len(ID_POOL)), rng.randint(1, 5))])
           for _ in range(rng.randint(1, 8))]
    a = _apply(hr.Hypergraph(), list(ops))
    b = _apply(xgi_real.Hypergraph(), list(ops))
    assert list(a.nodes) == list(b.nodes)         # insertion order (III.7) AND id-type identity
    assert [type(n) for n in a.nodes] == [type(n) for n in b.nodes]   # e.g. 1 stays int, "1" stays str
    assert a.num_nodes == b.num_nodes and a.num_edges == b.num_edges
    for e in b.edges:
        assert a.edges.members(e) == b.edges.members(e)
