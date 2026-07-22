"""hypergraph-rs — Rust port of XGI as a rustworkx-core plugin.

Python surface. The native module ``hypergraph_rs._hypergraph_rs_core`` is a
STUB until Phase 7 (plans/phase-7-python-conformance.md) lands the full
XGI-conformant binding; importing this package today proves only that the
maturin/PyO3 build + link path works end to end.
"""

from importlib.metadata import version as _version

# Importing the native module at package import time is deliberate: a broken
# cdylib build must fail HERE, loudly, not at first attribute access deep in
# a consumer (babylon Constitution III.11 — Loud Failure).
from hypergraph_rs import _hypergraph_rs_core as _core  # noqa: F401

__version__ = _version("hypergraph-rs")

__all__: list[str] = ["__version__"]
