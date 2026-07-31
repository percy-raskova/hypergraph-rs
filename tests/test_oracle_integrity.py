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


def test_ported_oracle_files_are_verbatim_upstream() -> None:
    if not UPSTREAM.exists():
        raise AssertionError(
            f"upstream oracle not found at {UPSTREAM}; install xgi==0.10.2"
        )
    drifted: list[str] = []
    # Bounded: one pass over the ported tree.
    for path in sorted(PORTED.rglob("test_*.py")):
        rel = path.relative_to(PORTED)
        upstream = UPSTREAM / rel
        if not upstream.exists():
            continue  # repo-authored, not a port
        if _normalise(path.read_bytes()) != _normalise(upstream.read_bytes()):
            drifted.append(str(rel))
    assert drifted == [], (
        "ported oracle files differ from upstream XGI 0.10.2 beyond the import "
        "swap — every conformance claim rests on these being verbatim:\n  "
        + "\n  ".join(drifted)
    )
