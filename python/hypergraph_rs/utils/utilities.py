"""Deep-import target: `from hypergraph_rs.utils.utilities import ...` (mirrors
xgi/utils/utilities.py). Re-exports everything from hypergraph_rs.utils; adds
the one symbol (`crest_r`) that lives only in the utilities submodule in XGI
and was never ported (private helper, not part of the __all__ surface)."""
from .._deferred import make_deferred
from . import *  # noqa: F401,F403
from . import IDDict  # noqa: F401

crest_r = make_deferred("crest_r")
