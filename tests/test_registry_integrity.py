"""Integrity tests for the conformance exclusion registry (spec F1).

These guard the REGISTRY, not the port. They deliberately live outside
tests/ported/ so the exclusion glue in conftest.py never rewrites them.
"""

import ast
import pathlib

from conftest_exclusions import (
    ENTRY_CLASSES,
    ENTRY_OWNERS,
    FILE_EXCLUSIONS,
    FILE_OWNERS,
    TEST_EXCLUSIONS,
    entry_matches_name,
)
from tools.taxonomy import OWNER_PATTERN

PORTED = pathlib.Path(__file__).parent / "ported"


def _test_names(path: pathlib.Path) -> list[str]:
    """Every bare test function/method name defined in `path`, via AST."""
    tree = ast.parse(path.read_text())
    out: list[str] = []
    # Bounded: one pass over the module's AST nodes.
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                out.append(node.name)
    return out


def _qualified_test_names(path: pathlib.Path) -> list[str]:
    """Test names as the registry must key them: `Class::method` or `func`."""
    tree = ast.parse(path.read_text())
    out: list[str] = []
    # Bounded: module body, then one class body per class.
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                out.append(node.name)
        elif isinstance(node, ast.ClassDef):
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if sub.name.startswith("test_"):
                        out.append(f"{node.name}::{sub.name}")
    return out


def test_every_exclusion_entry_names_a_real_test():
    """A phantom entry is a silent no-op; the D5 note records a past instance."""
    phantom: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            phantom.append(f"{rel}  (file does not exist)")
            continue
        known = set(_test_names(path)) | set(_qualified_test_names(path))
        for entry, _ in entries:
            if entry not in known:
                phantom.append(f"{rel}::{entry}")
    assert phantom == [], (
        "registry names tests that do not exist:\n  " + "\n  ".join(phantom)
    )


def test_entry_matches_name_rejects_prefix_siblings() -> None:
    """The F1 regression guard. A bare `startswith` fails every `not` below."""
    assert entry_matches_name("test_write_hif", "test_write_hif")
    assert not entry_matches_name("test_write_hif_compressed", "test_write_hif")
    assert not entry_matches_name("test_empty_order", "test_empty")
    # pytest's parametrization suffix must still match; `[` cannot occur in an
    # identifier, so this tolerance cannot reach a sibling test.
    assert entry_matches_name("test_foo[1-2]", "test_foo")
    # Class scoping is symmetric: neither shape matches the other.
    assert entry_matches_name("TestA::test_x", "TestA::test_x")
    assert not entry_matches_name("TestB::test_x", "TestA::test_x")
    assert not entry_matches_name("TestA::test_x", "test_x")
    assert not entry_matches_name("test_x", "TestA::test_x")


def test_each_entry_matches_exactly_one_test() -> None:
    """Every registry entry must claim exactly the one test it names.

    Supersedes the earlier prefix-text assertions. Those asked whether the
    registry's TEXT contained prefix relationships, which exact matching renders
    harmless — and which Task 5 deliberately restores. This asks the real
    question, through the matcher itself, so it survives Task 5 unchanged.
    """
    problems: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            problems.append(f"{rel}  (file does not exist)")
            continue
        names = set(_qualified_test_names(path))
        # Bounded: entries for this file, times names in this file.
        for entry, _ in entries:
            hits = sorted(n for n in names if entry_matches_name(n, entry))
            if len(hits) != 1:
                problems.append(f"{rel}::{entry} matches {len(hits)}: {hits}")
    assert problems == [], (
        "registry entries that do not claim exactly one test:\n  "
        + "\n  ".join(problems)
    )


def test_every_file_exclusion_names_a_real_file():
    missing = [rel for rel in FILE_EXCLUSIONS if not (PORTED / rel).exists()]
    assert missing == [], f"FILE_EXCLUSIONS names files that do not exist: {missing}"


def test_registry_is_not_empty() -> None:
    """A TOML parse that silently yields nothing would make the integrity
    tests above pass vacuously (Task 1 review, deferred minor)."""
    assert FILE_EXCLUSIONS, "FILE_EXCLUSIONS parsed empty from compat/exclusions.toml"
    assert TEST_EXCLUSIONS, "TEST_EXCLUSIONS parsed empty from compat/exclusions.toml"
    assert ENTRY_CLASSES, "ENTRY_CLASSES parsed empty from compat/exclusions.toml"


def test_skip_and_xfail_markers_land_on_the_right_classes(monkeypatch) -> None:
    """Pins BEHAVIOUR: which marker the hook actually applies per class.

    The previous version asserted the contents of `_SKIP_CLASSES`, which an
    inversion of the branch that consumes it does not change — flipping
    `if cls in _SKIP_CLASSES` to `not in` turns all 77 strict xfails into skips
    and every test in this repo still passes. Verified by mutation. No live test
    entry carries a skip class any more, so the skip path is exercised here with
    synthetic registry entries rather than real ones.
    """
    import conftest

    class _FakeItem:
        def __init__(self, name: str) -> None:
            self.fspath = "/repo/tests/ported/fake/test_x.py"
            self.name = name
            self.cls = None
            self.own_markers: list = []

        def add_marker(self, marker: object) -> None:
            self.own_markers.append(marker)

    rel = "fake/test_x.py"
    monkeypatch.setitem(
        conftest.TEST_EXCLUSIONS,
        rel,
        [("test_skipme", "synthetic"), ("test_xfailme", "synthetic")],
    )
    monkeypatch.setitem(conftest.ENTRY_CLASSES, (rel, "test_skipme"), "env")
    monkeypatch.setitem(conftest.ENTRY_CLASSES, (rel, "test_xfailme"), "core-gap")

    skip_item = _FakeItem("test_skipme")
    xfail_item = _FakeItem("test_xfailme")
    conftest.pytest_collection_modifyitems(None, [skip_item, xfail_item])

    assert [m.name for m in skip_item.own_markers] == ["skip"]
    assert [m.name for m in xfail_item.own_markers] == ["xfail"]
    assert xfail_item.own_markers[0].kwargs["strict"] is True


def test_skip_classes_are_exactly_the_unrunnable_ones() -> None:
    """Pins the constant itself — catches a class added, removed, or renamed."""
    import conftest

    assert conftest._SKIP_CLASSES == frozenset({"env", "upstream-bug", "out-of-scope"})


def test_no_test_entry_hides_behind_a_skip_class() -> None:
    """Every test-level gap must RUN and be pinned by strict xfail.

    Reclassifying an entry to a skip class silently returns a running, failing
    gap to the hidden column, and no other guard catches it: the drift check
    faithfully reproduces the reclassification. Zero test entries carry a skip
    class today. If a genuinely un-runnable test-level gap ever appears, update
    this deliberately and record why — the same discipline EXPECTED_INVISIBLE
    carries.
    """
    import conftest

    hidden = sorted(
        f"{path}::{name}"
        for (path, name), cls in ENTRY_CLASSES.items()
        if cls in conftest._SKIP_CLASSES
    )
    assert hidden == [], (
        "test entries classed into a skip tier, hiding a running gap:\n  "
        + "\n  ".join(hidden)
    )


def test_every_owner_is_known() -> None:
    """An owner names a later task of the raise to XGI 1.1 or a later plan of its own."""
    bad = sorted(
        f"{key}: {owner}"
        for key, owner in [*ENTRY_OWNERS.items(), *FILE_OWNERS.items()]
        if not OWNER_PATTERN.match(owner)
    )
    assert bad == [], "owners that match no task or plan:\n  " + "\n  ".join(bad)


def test_flip_entries_carry_an_owner_or_a_reason() -> None:
    """A test entry the flip added without an owner says why no task of the plan clears it."""
    import tomllib

    data = tomllib.loads(
        (pathlib.Path(__file__).parent.parent / "compat" / "exclusions.toml").read_text()
    )
    flip_without_owner = {
        ("core/test_hypergraph.py", "test_cleanup_empties"),
        ("core/test_simplicialcomplex.py", "test_add_weighted_simplices_from"),
        ("core/test_views.py", "test_pos_stat"),
        ("drawing/test_layout.py", "test_spring_layouts_accept_rng"),
        ("drawing/test_layout.py", "test_spring_layouts_rng_semantics"),
        ("algorithms/test_centrality.py", "test_node_edge_centrality_xgi_data"),
        ("algorithms/test_centrality.py", "test_h_eigenvector_centrality_xgi_data"),
        ("generators/test_random.py", "test_degree_corrected_stochastic_block_model_hypergraph"),
        ("generators/test_uniform.py", "test_uniform_stochastic_block_model"),
        ("generators/test_uniform.py", "test_uniform_planted_partition_model"),
    }
    reasons = {
        (e["path"], e["test"]): e["reason"]
        for e in data["test_exclusion"]
        if "owner" not in e
    }
    for key in sorted(flip_without_owner):
        assert key in reasons, f"{key} has an owner or is gone"
        assert len(reasons[key]) > 40, f"{key}: the reason is too short to say why nothing clears it"
