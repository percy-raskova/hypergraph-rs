"""The library's random generators against XGI 1.1, statistically (plan 2026-11-21, Tasks 16, 18, 19, 20).

The library does not reproduce XGI's draws. It must give the same distribution. Each test draws 200 hypergraphs
from the library and 200 from XGI with disjoint seeds and compares summaries with a two-sample test. The seeds
and ALPHA are fixed by the plan: a failing comparison is a finding about the algorithm, never a reason to change
them.
"""

from collections import Counter
from collections.abc import Sequence

import pytest
import scipy.stats
import xgi

import hypergraph_rs as hr
from hypergraph_rs.generators.lattice import _ring_lattice_edges

OUR_SEEDS = range(200)
XGI_SEEDS = range(1_000_000, 1_000_200)
ALPHA = 0.001


def assert_same_distribution(ours: Sequence[float], theirs: Sequence[float], label: str) -> None:
    """Two-sample Kolmogorov-Smirnov test; the message gives both means."""
    p = scipy.stats.ks_2samp(ours, theirs).pvalue
    mean_ours = sum(ours) / len(ours)
    mean_theirs = sum(theirs) / len(theirs)
    assert p >= ALPHA, f"{label}: KS p={p:.3g}; mean ours {mean_ours:.4g}, mean XGI {mean_theirs:.4g}"


def assert_same_counts(ours: Counter, theirs: Counter, label: str) -> None:
    """Chi-square test of homogeneity on a 2 x K table over the union of keys."""
    keys = sorted(set(ours) | set(theirs))
    if len(keys) < 2:
        return  # a single shared key: nothing to compare
    table = [[ours[key] for key in keys], [theirs[key] for key in keys]]
    p = scipy.stats.chi2_contingency(table).pvalue
    assert p >= ALPHA, f"{label}: chi-square p={p:.3g}; ours {sorted(ours.items())}, XGI {sorted(theirs.items())}"


def test_assert_same_counts_accepts_equal_and_rejects_different():
    assert_same_counts(Counter({1: 100, 2: 100}), Counter({1: 98, 2: 102}), "equal")
    assert_same_counts(Counter({7: 5}), Counter({7: 900}), "one shared key")
    with pytest.raises(AssertionError):
        assert_same_counts(Counter({1: 190, 2: 10}), Counter({1: 10, 2: 190}), "different")


def test_assert_same_distribution_accepts_equal_and_rejects_different():
    assert_same_distribution(list(range(200)), list(range(200)), "equal")
    with pytest.raises(AssertionError):
        assert_same_distribution(list(range(200)), [x + 100 for x in range(200)], "different")


# --- watts_strogatz_hypergraph ------------------------------------------------------------------------------


def _members(H) -> dict:
    return H.edges.members(dtype=dict)


def rewired(H, params) -> int:
    """The number of edges whose member set differs from the lattice edge with the same id."""
    n, d, k, l, _ = params
    lattice = _ring_lattice_edges(n, d, k, l)
    members = _members(H)
    return sum(1 for e, original in enumerate(lattice) if set(members[e]) != set(original))


def pooled_degrees(hypergraphs) -> Counter:
    counts: Counter = Counter()
    for H in hypergraphs:
        counts.update(H.nodes.degree.asdict().values())
    return counts


def ring_distances(hypergraphs, params) -> Counter:
    """Pool `(z - lattice[e][0]) % n` over the replaced node `z` of each rewired edge."""
    n, d, k, l, _ = params
    lattice = _ring_lattice_edges(n, d, k, l)
    counts: Counter = Counter()
    for H in hypergraphs:
        members = _members(H)
        for e, original in enumerate(lattice):
            if set(members[e]) == set(original):
                continue
            (z,) = set(members[e]) - set(original[:-1])
            counts[(z - original[0]) % n] += 1
    return counts


@pytest.mark.parametrize("params", [(20, 3, 4, 2, 0.2), (20, 3, 4, 2, 0.5), (30, 2, 4, 0, 1.0)])
def test_watts_strogatz_matches_xgi(params):
    ours = [hr.watts_strogatz_hypergraph(*params, seed=s) for s in OUR_SEEDS]
    theirs = [xgi.watts_strogatz_hypergraph(*params, seed=s) for s in XGI_SEEDS]
    assert_same_distribution([rewired(H, params) for H in ours], [rewired(H, params) for H in theirs], "rewired")
    assert_same_counts(pooled_degrees(ours), pooled_degrees(theirs), "degree")
    assert_same_counts(ring_distances(ours, params), ring_distances(theirs, params), "replacement distance")
