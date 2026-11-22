import networkx as nx
import numpy as np
import pandas as pd
import pytest

import hypergraph_rs as xgi


@pytest.fixture
def edgelist1():
    return [{1, 2, 3}, {4}, {5, 6}, {6, 7, 8}]


@pytest.fixture
def edgelist2():
    return [{1, 2}, {3, 4}, {4, 5, 6}]


@pytest.fixture
def edgelist3():
    return [{1, 2, 3}, {3, 4}, {4, 5, 6}]


@pytest.fixture
def edgelist4():
    return [{1, 2, 3}, {2, 3, 4, 5}, {3, 4, 5}]


@pytest.fixture
def edgelist5():
    return [{0, 1, 2, 3}, {4}, {5, 6}, {6, 7, 8}]


@pytest.fixture
def edgelist6():
    return [{0, 1, 2}, {1, 2, 3}, {2, 3, 4}]


@pytest.fixture
def edgelist7():
    return [{0, 1, 2}, {1, 2, 3}, {2, 3, 4}, {4}]


@pytest.fixture
def edgelist8():
    return [
        {0, 1},
        {0, 1, 2},
        {0, 2, 3},
        {0, 1, 2, 3, 4},
        {2, 4, 5},
        {1, 3, 5},
        {0, 3, 4},
        {1, 6},
        {0, 6},
    ]


@pytest.fixture
def edgelist9():
    return [{1, 2, 3}, {2, 3, 4}, {3, 4, 5}]


@pytest.fixture
def edgelist10():
    return [{1}, {2}, {3}]


@pytest.fixture
def dict5():
    return {0: {0, 1, 2, 3}, 1: {4}, 2: {5, 6}, 3: {6, 7, 8}}


@pytest.fixture
def incidence5():
    I = np.zeros((9, 4))
    I[0, 0] = I[1, 0] = I[2, 0] = I[3, 0] = I[4, 1] = 1
    I[5, 2] = I[6, 2] = I[6, 3] = I[7, 3] = I[8, 3] = 1
    return I


@pytest.fixture
def dataframe5():
    data = [
        [0, 0],
        [1, 0],
        [2, 0],
        [3, 0],
        [4, 1],
        [5, 2],
        [6, 2],
        [6, 3],
        [7, 3],
        [8, 3],
    ]
    df = pd.DataFrame(data)
    df.columns = ["col1", "col2"]
    return df


@pytest.fixture
def bipartite_graph1():
    G = nx.Graph()
    G.add_nodes_from([1, 2, 3, 4], bipartite=0)
    G.add_nodes_from(["a", "b", "c"], bipartite=1)
    G.add_edges_from([(1, "a"), (1, "b"), (2, "b"), (2, "c"), (3, "c"), (4, "a")])
    return G


@pytest.fixture
def bipartite_graph2():
    G = nx.Graph()
    G.add_nodes_from([1, 2, 3, 4], bipartite=0)
    G.add_nodes_from(["a", "b", "c"], bipartite=2)
    G.add_edges_from([(1, "a"), (1, "b"), (2, "b"), (2, "c"), (3, "c"), (4, "a")])
    return G


@pytest.fixture
def bipartite_graph3():
    G = nx.Graph()
    G.add_nodes_from([1, 2, 3, 4])
    G.add_nodes_from(["a", "b", "c"])
    G.add_edges_from([(1, "a"), (1, "b"), (2, "b"), (2, "c"), (3, "c"), (4, "a")])
    return G


@pytest.fixture
def bipartite_graph4():
    # this is to test when the bipartite condition is not true
    G = nx.Graph()
    G.add_nodes_from([1, 2, 3, 4], bipartite=0)
    G.add_nodes_from(["a", "b", "c"], bipartite=1)
    G.add_edges_from([(1, "a"), ("a", "b"), (2, "b"), (2, "c"), (3, "c"), (4, "a")])
    return G


@pytest.fixture
def bipartite_digraph1():
    G = nx.DiGraph()
    G.add_nodes_from([1, 2, 3, 4], bipartite=0)
    G.add_nodes_from(["a", "b", "c"], bipartite=1)
    G.add_edges_from(
        [
            ("a", 1),
            (1, "b"),
            ("b", 2),
            (2, "c"),
            ("c", 3),
            (4, "a"),
        ]
    )
    return G


@pytest.fixture
def bipartite_digraph2():
    G = nx.DiGraph()
    G.add_nodes_from([1], bipartite=0)
    G.add_nodes_from(["a"], bipartite=1)
    G.add_edges_from(
        [
            ("a", 1, {"direction": "invalid"}),
        ]
    )
    return G


@pytest.fixture
def attr0():
    return {"color": "brown", "name": "camel"}


@pytest.fixture
def attr1():
    return {"color": "red", "name": "horse"}


@pytest.fixture
def attr2():
    return {"color": "blue", "name": "pony"}


@pytest.fixture
def attr3():
    return {"color": "yellow", "name": "zebra"}


@pytest.fixture
def attr4():
    return {"color": "red", "name": "orangutan", "age": 20}


@pytest.fixture
def attr5():
    return {"color": "blue", "name": "fish", "age": 2}


@pytest.fixture
def hyperwithdupsandattrs(attr1, attr2, attr3, attr4, attr5):
    edges = [
        ({1, 2}, {"color": "blue"}),
        ({1, 2}, {"color": "red", "weight": 2}),
        ({1, 2}, {"color": "yellow"}),
        ({3, 4, 5}, {"color": "purple"}),
        ({3, 4, 5}, {"color": "purple", "name": "test"}),
    ]
    H = xgi.Hypergraph()
    H.add_edges_from(edges)
    H.add_nodes_from(
        [
            (1, attr1),
            (2, attr2),
            (3, attr3),
            (4, attr4),
            (5, attr5),
        ]
    )
    return H


@pytest.fixture
def hyperwithattrs(edgelist4, attr1, attr2, attr3, attr4, attr5):
    H = xgi.Hypergraph(edgelist4)
    H.add_nodes_from(
        [
            (1, attr1),
            (2, attr2),
            (3, attr3),
            (4, attr4),
            (5, attr5),
        ]
    )
    return H


@pytest.fixture
def hypergraph1():
    H = xgi.Hypergraph()
    H.add_nodes_from(["a", "b", "c"])
    H.add_edges_from({"e1": ["a", "b"], "e2": ["a", "b", "c"], "e3": ["c"]})
    return H


@pytest.fixture
def hypergraph2():
    H = xgi.Hypergraph()
    H.add_nodes_from(["b", "c", 0])
    H.add_edges_from({"e1": [0, "b"], "e2": [0, "c"], "e3": [0, "b", "c"]})
    return H


@pytest.fixture
def simplicialcomplex1():
    S = xgi.SimplicialComplex()
    S.add_nodes_from(["b", "c", 0])
    S.add_simplices_from(
        {"e1": [0, "b"], "e2": [0, "c"], "e3": [0, "b", "c"], "e4": ["b", "c"]}
    )
    return S


@pytest.fixture
def dihypergraph1():
    H = xgi.DiHypergraph()
    H.add_nodes_from(["a", "b", "c", "d"])
    H.add_edges_from(
        {"e1": [{"a", "b"}, {"c"}], "e2": [{"b"}, {"c", "d"}], "e3": [{"b"}, {"c"}]}
    )
    return H


@pytest.fixture
def diedgelist1():
    return [({1, 2, 3}, {4}), ({5, 6}, {6, 7, 8})]


@pytest.fixture
def diedgelist2():
    return [({0, 1}, {2}), ({1, 2}, {4}), ({2, 3, 4}, {4, 5})]


@pytest.fixture
def diedgelist3():
    return [({0}, {1, 2}), ({1}, {2, 4}), ({2, 3, 4}, {4, 5})]


@pytest.fixture
def diedgedict1():
    return {0: ({1, 2, 3}, {4}), 1: ({5, 6}, {6, 7, 8})}


@pytest.fixture
def dihyperwithattrs(diedgelist2, attr0, attr1, attr2, attr3, attr4, attr5):
    H = xgi.DiHypergraph()
    H.add_nodes_from(
        [
            (0, attr0),
            (1, attr1),
            (2, attr2),
            (3, attr3),
            (4, attr4),
            (5, attr5),
        ]
    )
    H.add_edges_from(diedgelist2)
    H.set_edge_attributes({0: attr3, 1: attr4, 2: attr5})
    return H


### Fixtures for simpliciality


@pytest.fixture
def sc1_with_singletons():
    return xgi.Hypergraph([{1}, {2}, {3}, {1, 2}, {1, 3}, {2, 3}, {1, 2, 3}])


@pytest.fixture
def h_missing_one_singleton():
    return xgi.Hypergraph([{1}, {2}, {1, 2}, {1, 3}, {2, 3}, {1, 2, 3}])


@pytest.fixture
def h_missing_one_link():
    return xgi.Hypergraph([{1}, {2}, {3}, {1, 3}, {2, 3}, {1, 2, 3}])


@pytest.fixture
def h_links_and_triangles():
    return xgi.Hypergraph([{1, 3}, {2, 3}, {1, 2, 3}])


@pytest.fixture
def h_links_and_triangles2():
    return xgi.Hypergraph([{1, 3}, {2, 3}, {1, 2, 3}, {1, 4}, {2, 3, 4}, {2, 4}])


@pytest.fixture
def h1():
    return xgi.Hypergraph([{1, 2, 3}, {2, 3, 4, 5}, {5, 6, 7}, {5, 6}])
# === END VERBATIM XGI 1.1 conftest — repo-authored code below this line ===
# tests/test_oracle_integrity.py byte-compares everything ABOVE this marker
# against upstream's conftest.py. Do not move, reword, or duplicate it, and do
# not add repo-authored code above it.


# --- Task 15: conformance exclusion glue (spec §9.1) ---
import json
import pathlib

import pytest
from conftest_exclusions import (
    ENTRY_CLASSES,
    ENTRY_OWNERS,
    FILE_EXCLUSIONS,
    TEST_EXCLUSIONS,
    entry_matches_name,
)
from oracle_guard import VIOLATIONS, is_upstream, oracle_hidden, upstream_paths

from tools.import_tests import read_lock

# The upstream ported files (the lock's list). While one of them is collected or run,
# the real XGI is hidden and cannot be imported (tests/oracle_guard.py).
_UPSTREAM_PORTED = upstream_paths(read_lock())


@pytest.hookimpl(wrapper=True)
def pytest_runtest_protocol(item, nextitem):
    """Run setup, call and teardown of an upstream ported test with the oracle hidden."""
    if not is_upstream(item.path, _UPSTREAM_PORTED):
        return (yield)
    with oracle_hidden(item.nodeid):
        return (yield)


@pytest.hookimpl(wrapper=True)
def pytest_make_collect_report(collector):
    """Collect an upstream ported module with the oracle hidden."""
    if not (
        isinstance(collector, pytest.Module)
        and is_upstream(collector.path, _UPSTREAM_PORTED)
    ):
        return (yield)
    with oracle_hidden(str(collector.path)):
        return (yield)


def pytest_ignore_collect(collection_path, config):
    """Ignore file-excluded oracle modules; stay silent about everything else.

    Returning a bare bool is wrong here: pytest_ignore_collect is
    firstresult=True, so a `False` return short-circuits the hook chain and
    silently disables pytest's own --ignore / --ignore-glob handling for every
    path in the repo. `None` means "no opinion, ask the next implementation".
    """
    rel = str(collection_path).split("/ported/", 1)[-1]
    return True if rel in FILE_EXCLUSIONS else None


def _entry_matches(item, entry: str) -> bool:
    """Pytest-item adapter over `entry_matches_name` (the single matcher).

    Derives the item's registry-shaped name — `Class::method` for a class-scoped
    test, the bare name otherwise — then defers. All matching semantics live in
    `conftest_exclusions.entry_matches_name` so enforcement here and verification
    in tests/test_registry_integrity.py can never drift apart.
    """
    cls = getattr(item, "cls", None)
    name = f"{cls.__name__}::{item.name}" if cls is not None else item.name
    return entry_matches_name(name, entry)


# Classes whose tests genuinely cannot run. Everything else runs and is
# expected to fail, so it gets a STRICT xfail: if the gap is ever closed the
# test XPASSes and the build breaks, forcing the registry to be updated. That
# is what makes the register self-correcting rather than a document that rots
# into over-reporting (spec §5.1).
_SKIP_CLASSES = frozenset({"env", "upstream-bug", "out-of-scope"})

# How many collected tests each registry entry claimed in this run (`path::entry` -> count). A
# parametrized test is one entry and several tests, so the gate report compares the run's xfail
# count with these counts, not with the number of entries.
ENTRY_ITEMS: dict[str, int] = {}


def pytest_collection_modifyitems(config, items):
    for item in items:
        rel = str(item.fspath).split("/ported/", 1)[-1]
        for name, reason in TEST_EXCLUSIONS.get(rel, []):
            if not _entry_matches(item, name):
                continue
            cls = ENTRY_CLASSES.get((rel, name), "deferred-symbol")
            ENTRY_ITEMS[f"{rel}::{name}"] = ENTRY_ITEMS.get(f"{rel}::{name}", 0) + 1
            owner = ENTRY_OWNERS.get((rel, name))
            owner_tag = f"[owner:{owner}] " if owner else ""
            tag = f"{owner_tag}[conformance-exclusion:{cls}] {reason}"
            if cls in _SKIP_CLASSES:
                item.add_marker(pytest.mark.skip(reason=tag))
            else:
                item.add_marker(pytest.mark.xfail(reason=tag, strict=True))


def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    """Write the run's exact outcome counts for tools/gate_report.py.

    Read from the terminal reporter's own stats rather than scraping the
    printed summary line: that line's wording and order are presentation, and
    pinning it would be one more guard on a proxy.
    """
    if VIOLATIONS:
        # A strict expected failure cannot hide an oracle import: fail the session.
        for violation in VIOLATIONS:
            print(f"oracle guard: {violation}")
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
    reporter = session.config.pluginmanager.get_plugin("terminalreporter")
    if reporter is None:  # -p no:terminal, or a non-terminal runner
        return
    counts: dict = {key: len(value) for key, value in reporter.stats.items() if key}
    counts["entry_items"] = dict(sorted(ENTRY_ITEMS.items()))
    target = pathlib.Path(session.config.rootpath) / ".gate-stats.json"
    target.write_text(json.dumps(counts, indent=2, sort_keys=True) + "\n")
