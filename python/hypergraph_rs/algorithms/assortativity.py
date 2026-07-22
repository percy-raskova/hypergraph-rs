"""Deep-import target: `from hypergraph_rs.algorithms.assortativity import
_choose_degrees` (mirrors xgi/algorithms/assortativity.py). `_choose_degrees`
is a private helper, not part of the port (D4 mandatory: without this module,
`tests/ported/algorithms/test_assortativity.py`'s module-level import fails at
collection, taking the wanted-green `test_dynamical_assortativity` /
`test_degree_assortativity` cases with it)."""
from .._deferred import make_deferred

_choose_degrees = make_deferred("_choose_degrees")  # private helper; not part of the port
