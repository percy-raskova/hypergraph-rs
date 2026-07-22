"""Matplotlib-based drawing for backward compat with XGI (spec §6.3).

OPTIONAL FEATURE: requires ``pip install 'hypergraph-rs[mpl]'``.
Primary viz pathway is the scene-graph via ``H.render()`` (matplotlib-free).

Layout functions (``random_layout``, ``pairwise_spring_layout``, ``bipartite_spring_layout``,
``circular_layout``) are bound in Rust (Task 13, ``crates/hypergraph-rs-python/src/layout.rs``).
Names with no Phase-5 analogue (``barycenter_spring_layout``,
``weighted_barycenter_spring_layout``, ``pca_transform``, ``spiral_layout``,
``barycenter_kamada_kawai_layout``, ``edge_positions_from_barycenters``) fall back to
``make_deferred`` below.
"""
from .._deferred import make_deferred

_LAYOUT_NAMES = [
    "random_layout",
    "pairwise_spring_layout",
    "barycenter_spring_layout",
    "weighted_barycenter_spring_layout",
    "pca_transform",
    "circular_layout",
    "spiral_layout",
    "barycenter_kamada_kawai_layout",
    "bipartite_spring_layout",
    "edge_positions_from_barycenters",
]
for _n in _LAYOUT_NAMES:
    try:
        globals()[_n] = getattr(__import__("hypergraph_rs._hypergraph_rs_core", fromlist=[_n]), _n)
    except (ImportError, AttributeError):
        globals()[_n] = make_deferred(_n)
del _n

try:
    import matplotlib.pyplot as plt  # noqa: F401

    HAS_MPL = True
except ImportError:
    HAS_MPL = False


def _require_mpl(fn):
    def wrapper(*args, **kwargs):
        if not HAS_MPL:
            raise ImportError(
                "matplotlib drawing requires the `mpl` extra: `pip install 'hypergraph-rs[mpl]'`. "
                "For matplotlib-free rendering, use `H.render()` for a scene-graph."
            )
        return fn(*args, **kwargs)

    wrapper.__name__ = fn.__name__
    return wrapper


@_require_mpl
def draw(H, pos=None, ax=None, **kwargs):
    """Draw a Hypergraph with matplotlib (XGI ``xgi.draw`` parity). Layout is computed in Rust
    (deterministic); matplotlib is a pure renderer over the scene-graph."""
    import matplotlib.pyplot as plt

    scene = H.render("hyperedge_aware")
    ax = ax or plt.subplots()[1]
    for he in scene["hyperedges"]:
        geometry = he.get("geometry") or {}
        pts = geometry.get("points") or geometry.get("control_points") or []
        if pts:
            ax.fill([p[0] for p in pts], [p[1] for p in pts], alpha=0.2)
    for node in scene["nodes"]:
        ax.plot(node["x"], node["y"], "o")
    return ax


draw_nodes = _require_mpl(lambda *a, **k: draw(*a, **k))
draw_hyperedges = _require_mpl(lambda *a, **k: draw(*a, **k))
draw_simplices = make_deferred("draw_simplices")
draw_node_labels = make_deferred("draw_node_labels")
draw_hyperedge_labels = make_deferred("draw_hyperedge_labels")
draw_multilayer = make_deferred("draw_multilayer")
draw_bipartite = make_deferred("draw_bipartite")
draw_undirected_dyads = make_deferred("draw_undirected_dyads")
draw_directed_dyads = make_deferred("draw_directed_dyads")

__all__ = [
    "random_layout",
    "pairwise_spring_layout",
    "barycenter_spring_layout",
    "weighted_barycenter_spring_layout",
    "pca_transform",
    "circular_layout",
    "spiral_layout",
    "barycenter_kamada_kawai_layout",
    "bipartite_spring_layout",
    "edge_positions_from_barycenters",
    "draw",
    "draw_nodes",
    "draw_hyperedges",
    "draw_simplices",
    "draw_node_labels",
    "draw_hyperedge_labels",
    "draw_multilayer",
    "draw_bipartite",
    "draw_undirected_dyads",
    "draw_directed_dyads",
]
