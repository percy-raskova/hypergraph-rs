"""The ported oracle must stay byte-identical to XGI 0.10.2 modulo imports.

tests/ported/ holds verbatim copies of XGI's own suite. They are the
differential oracle: every conformance claim on this port rests on them being
unmodified. Nothing executable enforced that until now — gutting an oracle
file's assertions left the entire suite green.

Repo-authored files (no upstream counterpart) are exempt; they are this repo's
own tests, not ports.
"""

import pathlib
import re

import xgi

PORTED = pathlib.Path(__file__).parent / "ported"
# Derive the oracle path from the INSTALLED xgi rather than hardcoding a venv
# layout: CI has no `.venv` (it pip-installs xgi into a setup-python env), and a
# hardcoded `python3.13` breaks on any interpreter bump. xgi 0.10.2 vendors its
# own test suite as a sibling of the package directory.
UPSTREAM = pathlib.Path(xgi.__file__).resolve().parent.parent / "tests"

# The port aliases its import (`import hypergraph_rs as xgi`) rather than
# rewriting every `xgi.Foo(...)` call site, so everywhere BELOW the import
# line both files already read identically as `xgi.Foo(...)`. A blanket
# `\bxgi\b` substitution over the whole file is too blunt: it also rewrites
# the trailing alias name in `import hypergraph_rs as xgi`, producing
# `import hypergraph_rs as hypergraph_rs` on the ported side against
# `import hypergraph_rs` (upstream's `import xgi` normalised) on the other —
# a false positive on every file that imports at all. Anchoring the
# substitution to the import statement's root-module position only (start of
# line, only leading whitespace before `import`/`from`) normalises exactly
# the two shapes the port's `port_tests.sh` swap produces and leaves every
# other `xgi` token (call sites, error-message text, identifiers like
# `load_xgi_data`) untouched on both sides.
_IMPORT_ROOT = re.compile(rb"(?m)^(\s*)import (?:xgi|hypergraph_rs as xgi)\s*$")
_FROM_ROOT = re.compile(rb"(?m)^(\s*)from (?:xgi|hypergraph_rs)\b")


def _normalise(raw: bytes) -> bytes:
    """Collapse the one sanctioned difference: the xgi -> hypergraph_rs import swap."""
    raw = _IMPORT_ROOT.sub(rb"\1import PKG", raw)
    raw = _FROM_ROOT.sub(rb"\1from PKG", raw)
    return raw


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
        if _normalise(ours.read_bytes()) != _normalise(path.read_bytes()):
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


# The em dash below is written as its UTF-8 escape sequence (\xe2\x80\x94)
# rather than the literal character: bytes literals only permit ASCII source
# characters, so embedding "—" directly is a SyntaxError. This is the exact
# same three bytes the character encodes to; tests/conftest.py's sentinel
# comment contains the literal character, which Python encodes to these same
# bytes when the file is read via read_bytes().
SENTINEL = b"# === END VERBATIM XGI 0.10.2 conftest \xe2\x80\x94 repo-authored code below this line ==="


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


def test_shared_conftest_prefix_is_verbatim_upstream() -> None:
    upstream = UPSTREAM / "conftest.py"
    if not upstream.exists():
        raise AssertionError(f"upstream conftest not found at {upstream}")
    ours = (PORTED.parent / "conftest.py").read_bytes()
    assert _normalise(_upstream_prefix(ours)) == _normalise(upstream.read_bytes()), (
        "tests/conftest.py's upstream section differs from XGI 0.10.2's. Every "
        "ported test's inputs come from these fixtures; a change here silently "
        "alters what the oracle asserts."
    )


def test_ported_oracle_files_are_verbatim_upstream() -> None:
    if not UPSTREAM.exists():
        raise AssertionError(
            f"upstream oracle not found at {UPSTREAM}; install xgi==0.10.2"
        )
    missing, drifted = _compare_tree(UPSTREAM, PORTED)
    unexplained, stale = _allowlist_verdict(missing, UNPORTED)
    assert unexplained == [], (
        "upstream oracle files have no ported counterpart and no recorded "
        "reason — a deleted oracle silently removes the evidence behind every "
        "conformance claim that rested on it:\n  " + "\n  ".join(unexplained)
    )
    assert drifted == [], (
        "ported oracle files differ from upstream XGI 0.10.2 beyond the import "
        "swap — every conformance claim rests on these being verbatim:\n  "
        + "\n  ".join(drifted)
    )
    assert stale == [], (
        "UNPORTED lists files that ARE now ported; remove them so the "
        "allowlist cannot hide a future deletion:\n  " + "\n  ".join(stale)
    )
