"""Deep-import target: `from hypergraph_rs.readwrite.json import read_json,
write_json` (mirrors xgi/readwrite/json.py). Neither symbol is part of
xgi.readwrite.__all__ (verified) so they stay out of the 164-symbol total;
they still need to resolve on import (Fact 2)."""
from .._deferred import make_deferred

read_json = make_deferred("read_json")
write_json = make_deferred("write_json")
