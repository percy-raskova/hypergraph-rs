"""Integrity tests for the conformance exclusion registry (spec F1).

These guard the REGISTRY, not the port. They deliberately live outside
tests/ported/ so the exclusion glue in conftest.py never rewrites them.
"""

import ast
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from conftest_exclusions import (  # noqa: E402
    ENTRY_CLASSES,
    FILE_EXCLUSIONS,
    TEST_EXCLUSIONS,
    entry_matches_name,
)

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
