"""The ported oracle must stay byte-identical to XGI's own tests, modulo the import rewrite.

tests/ported/ holds copies of XGI's own suite, written only by tools/import_tests.py.
They are the differential oracle: every conformance claim on this port rests on them
being unmodified. The expectation is exact: each ported file is
`rewrite(upstream file)`, byte for byte, where `rewrite` is the importer's own function,
and compat/upstream_tests.toml agrees with the wheel installed as the oracle.

Repo-authored files (no upstream counterpart) are exempt; they are this repo's
own tests, not ports.
"""

import ast
import hashlib
import pathlib

import xgi

from tools.import_tests import read_lock, rewrite, sentinel_line

PORTED = pathlib.Path(__file__).parent / "ported"
# Derive the oracle path from the INSTALLED xgi rather than hardcoding a venv
# layout: CI has no `.venv` (it pip-installs xgi into a setup-python env), and a
# hardcoded `python3.13` breaks on any interpreter bump. XGI's wheel vendors its
# own test suite as a sibling of the package directory.
UPSTREAM = pathlib.Path(xgi.__file__).resolve().parent.parent / "tests"


# Upstream test files with no ported counterpart, each with the reason it was
# never ported. Measured in Task 3 Step 1 — do NOT add an entry to silence a
# failure. A file appearing here that you cannot justify from
# compat/exclusions.toml or tests/EXCLUSIONS.md is a deleted oracle, not an
# exemption.
#
# Task 3 Step 1 measured this set as EMPTY: every one of upstream xgi 0.10.2's
# 51 test_*.py files has a byte-present counterpart under tests/ported/ (which
# holds 67, the extra 16 being this repo's own tests, not ports). There is
# nothing to allowlist.
UNPORTED: dict[str, str] = {}


def _compare_tree(
    upstream: pathlib.Path, ported: pathlib.Path
) -> tuple[list[str], list[str]]:
    """Return (missing, drifted) relative paths, comparing UPSTREAM's file list.

    Iterating upstream rather than our own tree is the whole point: a deleted
    ported file must be a failure, and it cannot be one if the loop never
    visits it.
    """
    missing: list[str] = []
    drifted: list[str] = []
    # Bounded: one pass over upstream's finite file list.
    for path in sorted(upstream.rglob("test_*.py")):
        rel = str(path.relative_to(upstream))
        ours = ported / rel
        if not ours.exists():
            missing.append(rel)
            continue
        if ours.read_bytes() != rewrite(path.read_text(encoding="utf-8")).encode("utf-8"):
            drifted.append(rel)
    return missing, drifted


def _allowlist_verdict(
    missing: list[str], unported: dict[str, str]
) -> tuple[list[str], list[str]]:
    """Split measured `missing` against the allowlist.

    Returns (unexplained, stale): files missing with no recorded reason, and
    allowlist entries naming files that are in fact present. Extracted so both
    arms can be proven by mutation — with the tree currently complete, the
    live assertions only ever see the empty case.
    """
    unexplained = [m for m in missing if m not in unported]
    stale = sorted(set(unported) - set(missing))
    return unexplained, stale


# The sentinel names the XGI version the lock records; it is built from the same
# function the importer writes it with.
SENTINEL = sentinel_line(read_lock().xgi_version).encode("utf-8")


def _upstream_prefix(raw: bytes) -> bytes:
    """Everything above the sentinel, which must appear exactly once."""
    parts = raw.split(SENTINEL)
    assert len(parts) != 1, (
        "tests/conftest.py has lost its sentinel; without it the ~30 upstream "
        "fixtures every ported test depends on have no byte-fidelity coverage"
    )
    assert len(parts) == 2, (
        f"the sentinel must appear exactly once, found {len(parts) - 1}"
    )
    return parts[0]


def _redefined_below_sentinel(raw: bytes) -> list[str]:
    """Top-level names the upstream prefix defines, silently redefined below it.

    The byte comparison in test_shared_conftest_prefix_is_verbatim_upstream
    only pins the bytes ABOVE the sentinel. Python resolves module-level
    names last-definition-wins, so a repo-authored `def h1(): ...` (or
    `class ...`) placed below the sentinel silently replaces upstream's
    fixture of the same name at import time — no error, no visible change to
    the byte-pinned prefix above, and every ported test that depends on that
    fixture now runs against a different input. Sorted for a deterministic
    failure message.
    """
    sentinel_line = _upstream_prefix(raw).count(b"\n") + 1
    tree = ast.parse(raw.decode("utf-8"))
    above: set[str] = set()
    redefined: set[str] = set()
    # Bounded: one pass over the file's top-level statements.
    for node in tree.body:
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            continue
        if node.lineno <= sentinel_line:
            above.add(node.name)
        elif node.name in above:
            redefined.add(node.name)
    return sorted(redefined)


def test_shared_conftest_prefix_is_verbatim_upstream() -> None:
    upstream = UPSTREAM / "conftest.py"
    if not upstream.exists():
        raise AssertionError(f"upstream conftest not found at {upstream}")
    ours = (PORTED.parent / "conftest.py").read_bytes()
    assert _upstream_prefix(ours) == rewrite(upstream.read_text(encoding="utf-8")).encode(
        "utf-8"
    ), (
        "tests/conftest.py's upstream section differs from rewrite(XGI's conftest.py). Every "
        "ported test's inputs come from these fixtures; a change here silently "
        "alters what the oracle asserts."
    )
    redefined = _redefined_below_sentinel(ours)
    assert redefined == [], (
        "repo-authored code below the sentinel redefines a name the upstream "
        "prefix above already defines; Python resolves module-level names "
        "last-definition-wins, so the redefinition silently replaces what the "
        "byte-pinned prefix asserts:\n  " + "\n  ".join(redefined)
    )


def test_ported_oracle_files_are_verbatim_upstream() -> None:
    if not UPSTREAM.exists():
        raise AssertionError(
            f"upstream oracle not found at {UPSTREAM}; install the XGI the lock names"
        )
    missing, drifted = _compare_tree(UPSTREAM, PORTED)
    unexplained, stale = _allowlist_verdict(missing, UNPORTED)
    assert unexplained == [], (
        "upstream oracle files have no ported counterpart and no recorded "
        "reason — a deleted oracle silently removes the evidence behind every "
        "conformance claim that rested on it:\n  " + "\n  ".join(unexplained)
    )
    assert drifted == [], (
        "ported oracle files differ from rewrite(upstream) — every conformance "
        "claim rests on these being verbatim:\n  " + "\n  ".join(drifted)
    )
    assert stale == [], (
        "UNPORTED lists files that ARE now ported; remove them so the "
        "allowlist cannot hide a future deletion:\n  " + "\n  ".join(stale)
    )


def test_lock_agrees_with_the_installed_wheel() -> None:
    lock = read_lock()
    assert lock.xgi_version == xgi.__version__, (
        f"compat/upstream_tests.toml is for XGI {lock.xgi_version} but XGI "
        f"{xgi.__version__} is installed"
    )
    wheel = {
        str(p.relative_to(UPSTREAM)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in UPSTREAM.rglob("test_*.py")
    }
    locked = {f.path: f.sha256 for f in lock.files}
    assert locked == wheel, "the lock and the wheel hold different upstream test files"
    conftest = hashlib.sha256((UPSTREAM / "conftest.py").read_bytes()).hexdigest()
    assert lock.conftest_sha256 == conftest
