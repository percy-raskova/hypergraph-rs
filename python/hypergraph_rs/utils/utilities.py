"""Deep-import target: `from hypergraph_rs.utils.utilities import ...` (mirrors
xgi/utils/utilities.py). Re-exports everything from hypergraph_rs.utils; adds
the one symbol (`crest_r`) that lives only in the utilities submodule in XGI
and was never ported (private helper, not part of the __all__ surface)."""
import numpy as np

from .._deferred import make_deferred
from . import *  # noqa: F401,F403
from . import IDDict  # noqa: F401

crest_r = make_deferred("crest_r")


def min_where(dicty, where):
    """Minimum of `dicty`'s values among keys where `where[key]` is truthy — XGI parity:
    `xgi/utils/utilities.py::min_where`. Pure Python; no core dependency."""
    min_val = np.inf
    for key in dicty.keys():
        if where[key] and dicty[key] < min_val:
            min_val = dicty[key]
    return min_val
