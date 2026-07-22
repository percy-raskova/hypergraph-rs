"""Utils (mirrors xgi/utils/__init__.py)."""
from .._deferred import make_deferred

_BOUND = ['convert_labels_to_integers', 'dual_dict', 'get_network_type', 'powerset', 'subfaces']
for _n in _BOUND:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)  # until Task 12 binds it

# Phase 6 built only dual_dict/powerset/subfaces/get_network_type/convert_labels_to_integers
binomial_sequence = make_deferred("binomial_sequence")
find_triangles = make_deferred("find_triangles")
geometric = make_deferred("geometric")
hist = make_deferred("hist")
pairwise_incidence = make_deferred("pairwise_incidence")
request_json_from_url = make_deferred("request_json_from_url")
request_json_from_url_cached = make_deferred("request_json_from_url_cached")
ttsv1 = make_deferred("ttsv1")
ttsv2 = make_deferred("ttsv2")
update_uid_counter = make_deferred("update_uid_counter")

class IDDict(dict):
    """A dict subclass keyed by node/edge IDs (deep-import target: `from xgi.utils import IDDict`).

    Not part of xgi.utils.__all__ (verified) so excluded from __all__ below to preserve the
    164-symbol total.
    """


__all__ = ['binomial_sequence', 'convert_labels_to_integers', 'dual_dict', 'find_triangles', 'geometric', 'get_network_type', 'hist', 'pairwise_incidence', 'powerset', 'request_json_from_url', 'request_json_from_url_cached', 'subfaces', 'ttsv1', 'ttsv2', 'update_uid_counter']
