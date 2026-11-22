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
//! Nothing is kept between calls: every result is a function of its input alone. Results are keyed by id and listed
//! in the input's order, so a caller can compare one call with the next.
//!
//! # Example
//!
//! ```
//! use hypergraph_rs::geometry::barycentre;
//!
//! let at = barycentre(&[(0.0, 0.0), (4.0, 0.0)], &[3.0, 1.0]).unwrap();
//! assert_eq!(at, Some((1.0, 0.0)));
//! ```

mod error;
mod place;
mod positions;

pub use error::GeometryError;
pub use place::{anchored_placement, barycentre, corner_mix};
pub use positions::{Placement, Positions};

/// A point `(x, y)`.
pub type Point = (f64, f64);

/// A rectangle `(min_x, min_y, max_x, max_y)`, as `layout::LayoutResult` uses.
pub type Rect = (f64, f64, f64, f64);
