//! Geometry for weighted sets and fixed placements.
//!
//! This module draws the outline of a hyperedge from the numbers on its memberships, places nodes at a weighted
//! mix of given positions, and spreads overlapping points apart. Everything is a pure function of its input.
//!
//! # Determinism
//!
//! Nothing here is random, reads a clock or iterates a hash map, and only `+ - * /`, `sqrt`, `abs`, `floor`, `min`
//! and `max` touch a float, so results are bit for bit the same on every machine. A test over every file of this
//! module enforces the rule.
//!
//! # Stability
//!
//! Nothing is kept between calls: every result is a function of its input alone, a measurement and not a
//! simulation. Stability from one call to the next comes from four rules, each pinned by a test:
//!
//! 1. Results are keyed by id and listed in the input's order ([`Positions`] keeps insertion order).
//! 2. The push that separates two points on one spot is chosen from the two ids, never from their indices, so
//!    adding or removing a far-away node changes no other node's bits.
//! 3. A membrane's grid sits on a lattice of `cell` fixed at the origin `(0, 0)`, so the outline of a set does not
//!    shimmer when its extent changes.
//! 4. The same input gives the same bits on every run.
//!
//! Easing between one call's result and the next belongs to the caller.
//!
//! # The two layers
//!
//! The lower layer works on plain points and numbers: [`barycentre`], [`corner_mix`], [`spread`], [`iso_lines`]
//! and [`weighted_outline`]. The upper layer reads a [`Hypergraph`](crate::Hypergraph) through the reads the core
//! already serves (`node_ids` and `members_with_data`): [`anchored_placement`] and [`edge_outline`]. The number on
//! a membership is read through a closure the caller gives, so the membership value may be of any type.
//!
//! # Example
//!
//! Three edges: two anchors, `top` and `bottom`, and one set, `club`, of three weighted members, each of which is
//! also in `top` or `bottom`, so every member is placed. The nodes are placed at the weighted mix of the anchors,
//! spread apart, and the set is outlined.
//!
//! ```
//! use hypergraph_rs::geometry::{
//!     anchored_placement, edge_outline, spread, OutlineParams, Positions, SpreadParams,
//! };
//! use hypergraph_rs::Hypergraph;
//!
//! let mut h: Hypergraph<(), (), f64> = Hypergraph::new();
//! for (node, edge, weight) in [
//!     ("a", "top", 3.0),
//!     ("a", "bottom", 1.0),
//!     ("b", "top", 1.0),
//!     ("b", "bottom", 3.0),
//!     ("c", "top", 2.0),
//!     ("c", "bottom", 2.0),
//!     ("a", "club", 5.0),
//!     ("b", "club", 4.0),
//!     ("c", "club", 6.0),
//! ] {
//!     h.add_node_to_edge_with_data(edge, node, weight).unwrap();
//! }
//!
//! let mut anchors = Positions::new();
//! anchors.insert("top", (0.0, 10.0));
//! anchors.insert("bottom", (0.0, 0.0));
//!
//! let placement = anchored_placement(&h, &anchors, |w| *w).unwrap();
//! assert!(placement.unplaced.is_empty());
//! let mut positions = placement.positions;
//! spread(&mut positions, &[], &SpreadParams::new(1.0, 8)).unwrap();
//!
//! let params = OutlineParams::new(4.0, 0.5, 0.9);
//! let outline = edge_outline(&h, "club", &positions, |w| *w, &params)
//!     .unwrap()
//!     .expect("the set has weight");
//! let drawn = &outline.outline;
//! assert!(drawn.held_weight >= 0.9 * drawn.total_weight);
//! ```

mod contour;
mod error;
mod outline;
mod place;
mod positions;
mod spread;

pub use contour::{contains, iso_lines, signed_area, ClosedPath, ScalarGrid};
pub use error::GeometryError;
pub use outline::{
    edge_outline, weighted_outline, EdgeOutline, Outline, OutlineParams, WeightedOutline,
    DEFAULT_LEVEL_FRACTION, DEFAULT_MAX_GRID_NODES,
};
pub use place::{anchored_placement, barycentre, corner_mix};
pub use positions::{Placement, Positions};
pub use spread::{spread, SpreadParams, SpreadReport, DEFAULT_MAX_NEIGHBOURS};

/// A point `(x, y)`.
pub type Point = (f64, f64);

/// A rectangle `(min_x, min_y, max_x, max_y)`, as `layout::LayoutResult` uses.
pub type Rect = (f64, f64, f64, f64);
