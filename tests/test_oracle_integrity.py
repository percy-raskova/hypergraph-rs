"""The ported oracle must stay byte-identical to XGI 0.10.2 modulo imports.

tests/ported/ holds verbatim copies of XGI's own suite. They are the
differential oracle: every conformance claim on this port rests on them being
unmodified. Nothing executable enforced that until now — gutting an oracle
file's assertions left the entire suite green.

Repo-authored files (no upstream counterpart) are exempt; they are this repo's
own tests, not ports.
"""

import ast
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
# the one shape each side is allowed to carry, and leaves every other `xgi`
# token (call sites, error-message text, identifiers like `load_xgi_data`)
# untouched on both sides.
#
# The two sides get SEPARATE regexes and separate normalise functions —
# deliberately not one pattern accepting either spelling on either side. A
# shared pattern would rewrite both `import xgi` and `import hypergraph_rs as
# xgi` to the identical `import PKG` token regardless of which file carries
# which, so a ported file left un-swapped — still reading the real `import
# xgi` — would normalise to the same token as a correctly-swapped port and
# compare equal to upstream. That is testing xgi against itself and calling
# it byte-verbatim (a vacuous green). `_normalise_ported` only recognises the
# port's own sanctioned spelling (`hypergraph_rs`); a bare `import xgi` left
# on the ported side is untouched by it and therefore fails to match
# upstream's normalised `import PKG`, which is the drift it must be.
_UPSTREAM_IMPORT_ROOT = re.compile(rb"(?m)^(\s*)import xgi\s*$")
_UPSTREAM_FROM_ROOT = re.compile(rb"(?m)^(\s*)from xgi\b")
_PORTED_IMPORT_ROOT = re.compile(rb"(?m)^(\s*)import hypergraph_rs as xgi\s*$")
_PORTED_FROM_ROOT = re.compile(rb"(?m)^(\s*)from hypergraph_rs\b")


def _normalise_upstream(raw: bytes) -> bytes:
    """Collapse upstream's own import spelling (`import xgi`) to a common token."""
    raw = _UPSTREAM_IMPORT_ROOT.sub(rb"\1import PKG", raw)
    raw = _UPSTREAM_FROM_ROOT.sub(rb"\1from PKG", raw)
    return raw


def _normalise_ported(raw: bytes) -> bytes:
    """Collapse the port's own sanctioned import swap to the same common token.

    Deliberately does NOT also accept a bare `import xgi`/`from xgi` — see
    the module comment above the regexes. A ported file that still imports
    the real `xgi` must fail the comparison, not be normalised away.
    """
    raw = _PORTED_IMPORT_ROOT.sub(rb"\1import PKG", raw)
    raw = _PORTED_FROM_ROOT.sub(rb"\1from PKG", raw)
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
        if _normalise_ported(ours.read_bytes()) != _normalise_upstream(
            path.read_bytes()
        ):
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
    assert _normalise_ported(_upstream_prefix(ours)) == _normalise_upstream(
        upstream.read_bytes()
    ), (
        "tests/conftest.py's upstream section differs from XGI 0.10.2's. Every "
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
