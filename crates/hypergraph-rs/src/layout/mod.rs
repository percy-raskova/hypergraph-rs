//! Deterministic hypergraph layout: node positions + per-hyperedge geometry.
//!
//! All strategies are hand-rolled — `rustworkx-core` 0.18 exposes NO layout
//! functions (empirically confirmed 2026-07-18). Randomised layouts seed
//! `rand_pcg::Pcg64Mcg`; non-randomised layouts (Bipartite, Radial) are pure
//! placement functions. Positions/geometries are emitted in the hypergraph's
//! insertion order (Constitution III.7 parity).

use serde::{Deserialize, Serialize};

pub mod hull;
pub mod orderings;
pub mod place;
pub mod spring;

pub use hull::convex_hull;
pub use orderings::{levi_ego_tree, paoh_ordering, LeviKind, LeviNode};
pub use place::bipartite_layout;
pub use place::radial_layout;
pub use spring::hyperedge_aware_layout;
pub use spring::spring_layout;

/// The default deterministic seed used when a strategy provides no explicit
/// seed (or `seed = None`). XGI's `seed=None` is non-deterministic (numpy global
/// RNG); we substitute a fixed seed for Constitution III.7 determinism.
pub const DEFAULT_LAYOUT_SEED: u64 = 0;

/// Column/row alignment for the bipartite layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BipartiteAlign {
    /// Agents in a left column, hyperedges in a right column (x separates roles).
    Vertical,
    /// Agents in a bottom row, hyperedges in a top row (y separates roles).
    Horizontal,
}

/// The four layout strategies (spec §5.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutStrategy {
    /// Fruchterman–Reingold force-directed layout on the 1-skeleton.
    Spring {
        iterations: usize,
        seed: Option<u64>,
    },
    /// Two-column/row bipartite placement of agents and hyperedge markers.
    Bipartite { align: BipartiteAlign },
    /// Spring positions for agents; each hyperedge is the padded convex hull of
    /// its members (a colored halo).
    HyperedgeAware {
        iterations: usize,
        halo_padding: f64,
    },
    /// Agents on concentric rings by membership count; hyperedges as arcs.
    Radial { rings: usize },
}

/// The geometry of a single hyperedge in a laid-out scene.
///
/// Defined here (not in `viz`) so that `viz` can depend on `layout` acyclically;
/// `viz` re-exports it as `viz::HyperedgeGeometry` (spec §5.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HyperedgeGeometry {
    /// Convex hull around member nodes (halo style).
    ConvexHull { points: Vec<(f64, f64)> },
    /// Closed spline / polyline through members (elliptical style).
    Spline { control_points: Vec<(f64, f64)> },
    /// Vertical stripe (bipartite layout).
    Stripe { x: f64, y_min: f64, y_max: f64 },
    /// Radial arc (radial layout).
    Arc {
        center: (f64, f64),
        radius: f64,
        angle_range: (f64, f64),
    },
}

/// The result of a layout computation: insertion-ordered node positions and
/// per-hyperedge geometry, plus the overall bounding box (min_x, min_y, max_x,
/// max_y).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutResult {
    pub node_positions: Vec<(String, (f64, f64))>,
    pub edge_geometries: Vec<(String, HyperedgeGeometry)>,
    pub bounding_box: (f64, f64, f64, f64),
}

/// Fold node positions + geometry extents into a bounding box
/// (min_x, min_y, max_x, max_y). Empty input yields the zero box.
pub(crate) fn bounding_box_of(
    positions: &[(String, (f64, f64))],
    geometries: &[(String, HyperedgeGeometry)],
) -> (f64, f64, f64, f64) {
    let mut bb = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    fn fold(bb: &mut (f64, f64, f64, f64), x: f64, y: f64) {
        if x < bb.0 {
            bb.0 = x;
        }
        if y < bb.1 {
            bb.1 = y;
        }
        if x > bb.2 {
            bb.2 = x;
        }
        if y > bb.3 {
            bb.3 = y;
        }
    }
    for (_, (x, y)) in positions {
        fold(&mut bb, *x, *y);
    }
    for (_, g) in geometries {
        match g {
            HyperedgeGeometry::ConvexHull { points }
            | HyperedgeGeometry::Spline {
                control_points: points,
            } => {
                for (x, y) in points {
                    fold(&mut bb, *x, *y);
                }
            }
            HyperedgeGeometry::Stripe { x, y_min, y_max } => {
                fold(&mut bb, *x, *y_min);
                fold(&mut bb, *x, *y_max);
            }
            HyperedgeGeometry::Arc { center, radius, .. } => {
                fold(&mut bb, center.0 - radius, center.1 - radius);
                fold(&mut bb, center.0 + radius, center.1 + radius);
            }
        }
    }
    if !bb.0.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    bb
}

use crate::core::hypergraph::Hypergraph;

/// Dispatch a `LayoutStrategy` to its implementation (spec §5.2).
pub fn compute_layout<N, E, M>(h: &Hypergraph<N, E, M>, strategy: LayoutStrategy) -> LayoutResult {
    match strategy {
        LayoutStrategy::Spring { iterations, seed } => spring::spring_layout(h, iterations, seed),
        LayoutStrategy::Bipartite { align } => place::bipartite_layout(h, align),
        LayoutStrategy::HyperedgeAware {
            iterations,
            halo_padding,
        } => spring::hyperedge_aware_layout(h, iterations, halo_padding),
        LayoutStrategy::Radial { rings } => place::radial_layout(h, rings),
    }
}
