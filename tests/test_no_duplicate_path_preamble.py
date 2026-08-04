# tests/test_no_duplicate_path_preamble.py
"""No test file inserts into sys.path; python -m pytest and pytest's own insertions cover imports."""

import pathlib
import subprocess

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


def test_bare_pytest_collects_the_full_suite() -> None:
    """Regression for a real gap: without pyproject.toml's `[tool.pytest.
    ini_options] pythonpath`, bare `pytest tests/` (unlike `python -m
    pytest`, which puts the CWD on sys.path itself) aborted collection —
    measured 2026-08-04, `Interrupted: 6 errors during collection` — on the
    six repo-authored files that import `tools.*` at module scope. That is
    a pytest-native mechanism, not a per-file sys.path.insert, so it does
    not conflict with the two guards above. Single bounded subprocess call,
    collection only (no test execution)."""
    pytest_bin = ROOT / ".venv" / "bin" / "pytest"
    result = subprocess.run(
        [str(pytest_bin), "--collect-only", "-q", "tests/"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert result.returncode == 0, (
        f"bare pytest failed to collect the suite (exit {result.returncode}):\n"
        f"{result.stdout}\n{result.stderr}"
    )
    assert "errors during collection" not in result.stdout, result.stdout
