//! Deterministic placement layouts (no RNG): bipartite two-column and radial.

use crate::core::hypergraph::Hypergraph;

use super::{bounding_box_of, BipartiteAlign, HyperedgeGeometry, LayoutResult};
use std::f64::consts::PI;

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
            (
                eid.clone(),
                HyperedgeGeometry::Stripe {
                    x: ex,
                    y_min,
                    y_max,
                },
            )
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}

/// Radial layout: agents on concentric rings bucketed by membership count
/// (higher membership ⇒ outer ring), evenly spread by angle in insertion order.
/// Each hyperedge emits an `Arc` at the mean radius of its members spanning
/// their angular range. Deterministic — no RNG.
pub fn radial_layout<N, E, M>(h: &Hypergraph<N, E, M>, rings: usize) -> LayoutResult {
    let node_ids = h.node_ids();
    let rings = rings.max(1);
    let n = node_ids.len();

    let degree = |id: &str| -> usize { h.memberships(id).map(|m| m.len()).unwrap_or(0) };
    let max_deg = node_ids.iter().map(|id| degree(id)).max().unwrap_or(0);

    let node_positions: Vec<(String, (f64, f64))> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let ring = if max_deg == 0 {
                0
            } else {
                degree(id) * (rings - 1) / max_deg
            };
            let radius = (ring as f64 + 1.0) / (rings as f64 + 1.0);
            let angle = if n == 0 {
                0.0
            } else {
                2.0 * PI * (i as f64) / (n as f64)
            };
            (id.clone(), (radius * angle.cos(), radius * angle.sin()))
        })
        .collect();

    let pos_of = |id: &str| -> (f64, f64) {
        node_positions
            .iter()
            .find(|(nn, _)| nn == id)
            .map(|(_, p)| *p)
            .unwrap_or((0.0, 0.0))
    };

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let (radius, a_min, a_max) = if members.is_empty() {
                (0.0, 0.0, 0.0)
            } else {
                let mut rsum = 0.0;
                let mut amin = f64::INFINITY;
                let mut amax = f64::NEG_INFINITY;
                for m in &members {
                    let (x, y) = pos_of(m);
                    rsum += (x * x + y * y).sqrt();
                    let ang = y.atan2(x);
                    if ang < amin {
                        amin = ang;
                    }
                    if ang > amax {
                        amax = ang;
                    }
                }
                (rsum / members.len() as f64, amin, amax)
            };
            (
                eid.clone(),
                HyperedgeGeometry::Arc {
                    center: (0.0, 0.0),
                    radius,
                    angle_range: (a_min, a_max),
                },
            )
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
