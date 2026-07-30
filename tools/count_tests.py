"""AST test counting for files pytest never collects (spec §4.1, F3).

A whole-file exclusion removes its tests from BOTH the pass and skip counts,
so the reported denominator silently shrinks by exactly the tests we can least
account for. pytest cannot count them — some of these files cannot even import
— so they are counted statically instead.
"""

import ast
import pathlib

PORTED = pathlib.Path(__file__).parent.parent / "tests" / "ported"


def count_tests_in_file(path: pathlib.Path) -> int:
    """Number of test functions/methods defined in `path`, via AST."""
    tree = ast.parse(path.read_text())
    n = 0
    # Bounded: one pass over the module's AST nodes.
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.name.startswith("test_"):
                n += 1
    return n


def file_exclusion_test_counts() -> dict[str, int]:
    """`{relpath: test count}` for every whole-file exclusion."""
    import sys

    sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tests"))
    from conftest_exclusions import FILE_EXCLUSIONS

    out: dict[str, int] = {}
    # Bounded: one pass over the registry's file entries.
    for rel in FILE_EXCLUSIONS:
        path = PORTED / rel
        out[rel] = count_tests_in_file(path) if path.exists() else 0
    return out


if __name__ == "__main__":
    counts = file_exclusion_test_counts()
    # Bounded: one pass over the computed counts.
    for rel, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"{n:4d}  {rel}")
    print(f"{sum(counts.values()):4d}  TOTAL INVISIBLE")
