//! Deterministic placement layouts (no RNG): bipartite two-column and radial.

use crate::core::hypergraph::Hypergraph;

use super::{bounding_box_of, BipartiteAlign, HyperedgeGeometry, LayoutResult};

/// Evenly spread index `i` of `count` items into [0, 1]; a single item sits at
/// the midpoint 0.5.
fn spread(count: usize, i: usize) -> f64 {
    if count <= 1 {
        0.5
    } else {
        i as f64 / (count as f64 - 1.0)
    }
}

/// Bipartite layout: agents in one column/row, hyperedge markers in the other.
/// Deterministic — no RNG. Each hyperedge emits a `Stripe` spanning the
/// (oriented) y-range of its members at the marker's x. `Horizontal` align
/// swaps the role axis; see the module deviation note (its stripe is degenerate).
pub fn bipartite_layout<N, E, M>(h: &Hypergraph<N, E, M>, align: BipartiteAlign) -> LayoutResult {
    let node_ids = h.node_ids();
    let edge_ids = h.edge_ids();

    let orient = |x: f64, y: f64| -> (f64, f64) {
        match align {
            BipartiteAlign::Vertical => (x, y),
            BipartiteAlign::Horizontal => (y, x),
        }
    };

    let node_positions: Vec<(String, (f64, f64))> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), orient(0.0, spread(node_ids.len(), i))))
        .collect();

    let pos_of = |id: &str| -> (f64, f64) {
        node_positions
            .iter()
            .find(|(n, _)| n == id)
            .map(|(_, p)| *p)
            .unwrap_or((0.0, 0.0))
    };

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = edge_ids
        .iter()
        .enumerate()
        .map(|(i, eid)| {
            let (ex, _ey) = orient(1.0, spread(edge_ids.len(), i));
            let members = h.members(eid).unwrap_or_default();
            let (y_min, y_max) = if members.is_empty() {
                (0.0, 0.0)
            } else {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for m in &members {
                    let (_mx, my) = pos_of(m);
                    if my < lo {
                        lo = my;
                    }
                    if my > hi {
                        hi = my;
                    }
                }
                (lo, hi)
            };
            (eid.clone(), HyperedgeGeometry::Stripe { x: ex, y_min, y_max })
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
