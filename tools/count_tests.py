"""AST test counting for files pytest never collects (spec §4.1, F3).

A whole-file exclusion removes its tests from BOTH the pass and skip counts,
so the reported denominator silently shrinks by exactly the tests we can least
account for. pytest cannot count them — some of these files cannot even import
— so they are counted statically instead.
"""

import ast
import pathlib
import sys

PORTED = pathlib.Path(__file__).parent.parent / "tests" / "ported"


def count_tests_in_file(path: pathlib.Path) -> int:
    """Number of tests pytest would collect from `path`, via AST.

    Counts UNIQUE (class, name) identities rather than `FunctionDef` nodes.
    Python is last-definition-wins, so a module that defines the same test name
    twice exposes only one of them — `tests/ported/stats/test_core_stats_functions.py`
    does exactly this twice over, and counting nodes overstated it by 2. Two
    different classes may each define the same method name and pytest collects
    both, hence the class-scoped key rather than a bare name set.
    """
    tree = ast.parse(path.read_bytes())
    seen: set[tuple[str | None, str]] = set()
    # Bounded: module body, then one class body per class.
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                seen.add((None, node.name))
        elif isinstance(node, ast.ClassDef):
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    if sub.name.startswith("test_"):
                        seen.add((node.name, sub.name))
    return len(seen)


def file_exclusion_test_counts() -> dict[str, int]:
    """`{relpath: test count}` for every whole-file exclusion."""
    _tests_dir = str(pathlib.Path(__file__).parent.parent / "tests")
    # Load-bearing: imports tests/conftest_exclusions.py. tests/ must never become
    # a package (an __init__.py would move pytest's prepend insertion to the repo
    # root and break the bare tests-tree imports), so this insert cannot be
    # replaced by the tools package. tests/test_no_duplicate_path_preamble.py
    # exempts these two files by name.
    if _tests_dir not in sys.path:
        sys.path.insert(0, _tests_dir)
    from conftest_exclusions import FILE_EXCLUSIONS

    out: dict[str, int] = {}
    # Bounded: one pass over the registry's file entries.
    for rel in FILE_EXCLUSIONS:
        path = PORTED / rel
        if not path.exists():
            raise FileNotFoundError(
                f"registry names {rel!r} but it does not exist under {PORTED}; "
                "the invisible count would silently shrink"
            )
        out[rel] = count_tests_in_file(path)
    return out


if __name__ == "__main__":
    counts = file_exclusion_test_counts()
    # Bounded: one pass over the computed counts.
    for rel, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"{n:4d}  {rel}")
    print(f"{sum(counts.values()):4d}  TOTAL INVISIBLE")
