"""Integrity tests for the conformance exclusion registry (spec F1).

These guard the REGISTRY, not the port. They deliberately live outside
tests/ported/ so the exclusion glue in conftest.py never rewrites them.
"""

import ast
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS  # noqa: E402

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


def test_no_exclusion_entry_claims_a_test_it_does_not_name():
    """F1: `startswith` matching lets one entry sweep up its prefix-siblings."""
    offenders: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            continue
        names = _test_names(path)
        owned = {entry for entry, _ in entries}
        for entry, _ in entries:
            for name in names:
                if name != entry and name.startswith(entry) and name not in owned:
                    offenders.append(f"{rel}::{name}  <- swept up by entry {entry!r}")
    assert offenders == [], (
        "registry entries claim tests they do not name:\n  " + "\n  ".join(offenders)
    )


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


def test_class_scoped_tests_are_qualified_in_the_registry():
    """A bare entry matching a method in 2+ classes is ambiguous (test_spectral.py)."""
    ambiguous: list[str] = []
    for rel, entries in TEST_EXCLUSIONS.items():
        path = PORTED / rel
        if not path.exists():
            continue
        qualified = _qualified_test_names(path)
        for entry, _ in entries:
            if "::" in entry:
                continue
            hits = [q for q in qualified if q.endswith(f"::{entry}")]
            if hits:
                ambiguous.append(f"{rel}::{entry} is class-scoped; matches {hits}")
    assert ambiguous == [], (
        "bare registry entries target class-scoped tests:\n  " + "\n  ".join(ambiguous)
    )


def test_every_file_exclusion_names_a_real_file():
    missing = [rel for rel in FILE_EXCLUSIONS if not (PORTED / rel).exists()]
    assert missing == [], f"FILE_EXCLUSIONS names files that do not exist: {missing}"
