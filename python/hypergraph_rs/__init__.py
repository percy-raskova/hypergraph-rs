"""hypergraph_rs — a Rust port of XGI 0.10.2, import-compatible as ``import hypergraph_rs as xgi``.

The public surface mirrors ``xgi/__init__.py`` (164 symbols). Symbols no Rust phase implemented raise
``NotImplementedError`` on call (see ``tests/DEFERRED_SYMBOLS.md``); they still *import* so any test
module referencing them at top level collects cleanly.
"""

# Importing the native module at package import time is deliberate: a broken
# cdylib build must fail HERE, loudly, not at first attribute access deep in
# a consumer (babylon Constitution III.11 — Loud Failure).
from hypergraph_rs import _hypergraph_rs_core as _core  # noqa: F401
from hypergraph_rs._hypergraph_rs_core import _reconstruct_hypergraph  # noqa: F401
from hypergraph_rs._hypergraph_rs_core import _reconstruct_dihypergraph  # noqa: F401

from . import (  # noqa: F401
    utils,
    core,
    exception,
    algorithms,
    communities,
    convert,
    drawing,
    dynamics,
    generators,
    linalg,
    readwrite,
    stats,
)
from .utils import *  # noqa: F401,F403
from .core import *  # noqa: F401,F403
from .algorithms import *  # noqa: F401,F403
from .communities import *  # noqa: F401,F403
from .convert import *  # noqa: F401,F403
from .drawing import *  # noqa: F401,F403
from .dynamics import *  # noqa: F401,F403
from .generators import *  # noqa: F401,F403
from .linalg import *  # noqa: F401,F403
from .readwrite import *  # noqa: F401,F403
from .stats import *  # noqa: F401,F403

__version__ = "0.10.2-xgi-parity"  # PORT of XGI 0.10.2, not XGI itself (spec Sec6.1)

__all__ = (
    core.__all__
    + algorithms.__all__
    + communities.__all__
    + convert.__all__
    + drawing.__all__
    + dynamics.__all__
    + generators.__all__
    + linalg.__all__
    + readwrite.__all__
    + stats.__all__
    + utils.__all__
)
