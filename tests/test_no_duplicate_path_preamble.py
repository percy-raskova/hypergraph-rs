# tests/test_no_duplicate_path_preamble.py
"""No test file inserts into sys.path; python -m pytest and pytest's own insertions cover imports."""

import pathlib

ROOT = pathlib.Path(__file__).parent.parent
SELF = pathlib.Path(__file__).resolve()

# Assembled at runtime so this file's own source never contains the literal it
# searches for — belt-and-braces alongside the SELF exemption below.
NEEDLE = "sys.path" + ".insert"

# The two tools that import tests/conftest_exclusions.py. tests/ cannot become
# a package (an __init__.py would break pytest's tests-tree imports), so these
# inserts are load-bearing and exempt.
EXEMPT: frozenset[str] = frozenset({"migrate_exclusions.py", "count_tests.py"})


def test_no_test_file_inserts_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tests tree. The sweep
    # includes tests/ported/ upstream files (immutable); measured 2026-08-03,
    # none contain the needle — only repo-authored files did.
    for path in sorted((ROOT / "tests").rglob("*.py")):
        if path.resolve() == SELF:
            continue
        if NEEDLE in path.read_text():
            offenders.append(str(path.relative_to(ROOT)))
    assert not offenders, f"rely on python -m pytest and pytest's own insertions, not per-file inserts: {offenders}"


def test_only_exempt_tools_insert_into_sys_path() -> None:
    offenders: list[str] = []
    # Bounded: one pass over a finite glob of the tools directory.
    for path in sorted((ROOT / "tools").glob("*.py")):
        if path.name in EXEMPT:
            continue
        if NEEDLE in path.read_text():
            offenders.append(path.name)
    assert not offenders, f"tools is a package; these need no insert: {offenders}"
