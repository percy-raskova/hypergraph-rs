//! The weighted outline of a set of points.
//!
//! Each member of a set has a position and a number. Only members with a number above zero take part. The outline
//! is a [`Ring`](Outline::Ring) round one member, a [`Capsule`](Outline::Capsule) round two, or a
//! [`Membrane`](Outline::Membrane) of closed paths round three or more, drawn from a field that every member
//! raises to a hill as wide as `reach`, scaled by its number relative to the largest.

use super::contour::{inside_cell, iso_lines, ClosedPath, ScalarGrid};
use super::{GeometryError, Point};

/// The default fraction of a member's level that the line of a membrane is drawn at.
pub const DEFAULT_LEVEL_FRACTION: f64 = 0.5;

/// The default most nodes the outline's grid may hold.
pub const DEFAULT_MAX_GRID_NODES: usize = 4_194_304;

/// The parameters of [`weighted_outline`].
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineParams {
    /// How far a member's hill reaches: the field of a member is zero at this distance. Finite and above zero.
    pub reach: f64,
    /// The distance between grid nodes. Finite, above zero and at most `reach / 2`.
    pub cell: f64,
    /// The share of the total number the outline must hold, in `(0, 1]`.
    pub share: f64,
    /// The fraction of the level of the last member held at which a membrane's line is drawn, in `(0, 1]`.
    pub level_fraction: f64,
    /// The most nodes the grid may hold; a larger grid is [`GeometryError::GridTooLarge`].
    pub max_grid_nodes: usize,
}

impl OutlineParams {
    /// Parameters with [`DEFAULT_LEVEL_FRACTION`] and [`DEFAULT_MAX_GRID_NODES`].
    #[must_use]
    pub fn new(reach: f64, cell: f64, share: f64) -> Self {
        Self {
            reach,
            cell,
            share,
            level_fraction: DEFAULT_LEVEL_FRACTION,
            max_grid_nodes: DEFAULT_MAX_GRID_NODES,
        }
    }

    fn validate(&self) -> Result<(), GeometryError> {
        let bad = |name: &'static str, value: f64| Err(GeometryError::BadParameter { name, value });
        if !(self.reach.is_finite() && self.reach > 0.0) {
            return bad("reach", self.reach);
        }
        if !(self.cell.is_finite() && self.cell > 0.0 && self.cell <= self.reach / 2.0) {
            return bad("cell", self.cell);
        }
        if !(self.share > 0.0 && self.share <= 1.0) {
            return bad("share", self.share);
        }
        if !(self.level_fraction > 0.0 && self.level_fraction <= 1.0) {
            return bad("level_fraction", self.level_fraction);
        }
        Ok(())
    }
}

/// The shape of an outline.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Outline {
    /// A circle round one member.
    Ring {
        /// The member's position.
        center: Point,
        /// `reach * (0.5 + 0.5 * relative weight)`.
        radius: f64,
    },
    /// The hull of two circles, round two members.
    Capsule {
        /// The position of the member with the lower index.
        a: Point,
        /// The radius at `a`.
        radius_a: f64,
        /// The position of the other member.
        b: Point,
        /// The radius at `b`.
        radius_b: f64,
    },
    /// Closed paths round three or more members.
    Membrane {
        /// Outer paths run counter-clockwise, holes clockwise; the fill is even-odd.
        paths: Vec<ClosedPath>,
    },
}

/// An outline and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedOutline {
    /// The shape.
    pub outline: Outline,
    /// The indices, ascending, of every member with a number above zero that lies inside the outline as drawn.
    pub held: Vec<usize>,
    /// The sum of the numbers of the held members, in index order.
    pub held_weight: f64,
    /// The sum of the numbers above zero, in index order.
    pub total_weight: f64,
}

/// A member with a number above zero.
struct Part {
    index: usize,
    at: Point,
    weight: f64,
    relative: f64,
}

/// The radius of a ring or of an end of a capsule.
fn radius_of(reach: f64, relative: f64) -> f64 {
    reach * (0.5 + 0.5 * relative)
}

fn too_large(limit: usize) -> GeometryError {
    GeometryError::GridTooLarge {
        nodes: usize::MAX,
        limit,
    }
}

/// The grid on the lattice of `cell` through the origin that covers every part widened by `reach + cell`, and the
/// field on it.
fn build_field(parts: &[Part], params: &OutlineParams) -> Result<ScalarGrid, GeometryError> {
    let (reach, cell) = (params.reach, params.cell);
    let margin = reach + cell;
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for part in parts {
        min_x = min_x.min(part.at.0);
        min_y = min_y.min(part.at.1);
        max_x = max_x.max(part.at.0);
        max_y = max_y.max(part.at.1);
    }
    let lo_col = ((min_x - margin) / cell).floor();
    let lo_row = ((min_y - margin) / cell).floor();
    let hi_col = ((max_x + margin) / cell).floor() + 1.0;
    let hi_row = ((max_y + margin) / cell).floor() + 1.0;
    let count = |lo: f64, hi: f64| -> Result<usize, GeometryError> {
        let n = hi - lo + 1.0;
        if n.is_finite() && n >= 1.0 && n < usize::MAX as f64 {
            Ok(n as usize)
        } else {
            Err(too_large(params.max_grid_nodes))
        }
    };
    let cols = count(lo_col, hi_col)?;
    let rows = count(lo_row, hi_row)?;
    let nodes = cols
        .checked_mul(rows)
        .ok_or_else(|| too_large(params.max_grid_nodes))?;
    if nodes > params.max_grid_nodes {
        return Err(GeometryError::GridTooLarge {
            nodes,
            limit: params.max_grid_nodes,
        });
    }
    let mut grid = ScalarGrid::zeros((lo_col * cell, lo_row * cell), cell, cols, rows)?;
    let reach2 = reach * reach;
    let (ox, oy) = grid.origin;
    let span = |p: f64, o: f64, n: usize| -> (usize, usize) {
        let last = (n - 1) as f64;
        let lo = ((p - reach - o) / cell).floor().max(0.0).min(last);
        let hi = (((p + reach - o) / cell).floor() + 1.0).max(0.0).min(last);
        (lo as usize, hi as usize)
    };
    for part in parts {
        let (c0, c1) = span(part.at.0, ox, cols);
        let (r0, r1) = span(part.at.1, oy, rows);
        for r in r0..=r1 {
            let dy = oy + r as f64 * cell - part.at.1;
            for c in c0..=c1 {
                let dx = ox + c as f64 * cell - part.at.0;
                let d2 = dx * dx + dy * dy;
                if d2 < reach2 {
                    let rest = 1.0 - d2 / reach2;
                    grid.values[r * cols + c] += part.relative * (rest * rest);
                }
            }
        }
    }
    Ok(grid)
}

/// The held members and their weight at a threshold of a membrane, by one pass of `inside_cell`.
fn held_by_membrane(grid: &ScalarGrid, threshold: f64, parts: &[Part]) -> (Vec<usize>, f64) {
    let mut held = Vec::new();
    let mut weight = 0.0;
    for part in parts {
        if inside_cell(grid, threshold, part.at) {
            held.push(part.index);
            weight += part.weight;
        }
    }
    (held, weight)
}

fn held_by(parts: &[Part], inside: impl Fn(Point) -> bool) -> (Vec<usize>, f64) {
    let mut held = Vec::new();
    let mut weight = 0.0;
    for part in parts {
        if inside(part.at) {
            held.push(part.index);
            weight += part.weight;
        }
    }
    (held, weight)
}

/// Whether `at` lies within the capsule: its distance to the segment `a b` is at most the radius interpolated along
/// the segment.
fn in_capsule(at: Point, a: Point, ra: f64, b: Point, rb: f64) -> bool {
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let len2 = ex * ex + ey * ey;
    let t = if len2 > 0.0 {
        (((at.0 - a.0) * ex + (at.1 - a.1) * ey) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (at.0 - (a.0 + ex * t), at.1 - (a.1 + ey * t));
    let r = ra + (rb - ra) * t;
    dx * dx + dy * dy <= r * r
}

/// The outline of a set of weighted points.
///
/// `members` are `(position, number)` pairs. Only members with a number above zero take part; a zero member's
/// position is never read, so it may be anything. With none above zero the result is `Ok(None)`.
///
/// Only the numbers relative to the largest count. A member's field is `w * (1 - q^2)^2` for `q = distance / reach
/// < 1`, with `w` the number divided by the largest, summed over members on a grid whose nodes lie on the lattice of
/// `cell` through `(0, 0)`. The members the outline must hold are the shortest prefix, by field level at their own
/// positions (highest first, ties by index), whose numbers sum to at least `share` of the total. One member gives a
/// [`Ring`](Outline::Ring) of radius `reach * (0.5 + 0.5 * w)`, two a [`Capsule`](Outline::Capsule) with such a
/// radius at each end, three or more a [`Membrane`](Outline::Membrane) drawn at `level_fraction` of the level of
/// the last member of that prefix.
///
/// `held` is measured on the outline as drawn. If a membrane holds less than `share` of the total, its level steps
/// down, to `level_fraction` of the level of the member with the next lower level, until it does (found by
/// bisection) or every member is the reference; `held` then reports what it holds.
///
/// # Errors
///
/// [`GeometryError::BadParameter`] naming `reach`, `cell`, `share` or `level_fraction`;
/// [`GeometryError::BadWeight`] for a number that is negative or not finite (`item` is `#<index>`);
/// [`GeometryError::NonFinitePosition`] for a member above zero whose position is not finite;
/// [`GeometryError::GridTooLarge`] when the grid would hold more than `max_grid_nodes` nodes, or a count that does
/// not fit a `usize`. The check comes before anything is allocated.
pub fn weighted_outline(
    members: &[(Point, f64)],
    params: &OutlineParams,
) -> Result<Option<WeightedOutline>, GeometryError> {
    params.validate()?;
    let mut total = 0.0_f64;
    let mut largest = 0.0_f64;
    for (index, &(at, weight)) in members.iter().enumerate() {
        if !(weight.is_finite() && weight >= 0.0) {
            return Err(GeometryError::BadWeight {
                item: format!("#{index}"),
                value: weight,
            });
        }
        if weight > 0.0 {
            if !(at.0.is_finite() && at.1.is_finite()) {
                return Err(GeometryError::NonFinitePosition {
                    item: format!("#{index}"),
                });
            }
            total += weight;
            largest = largest.max(weight);
        }
    }
    let parts: Vec<Part> = members
        .iter()
        .enumerate()
        .filter(|(_, &(_, weight))| weight > 0.0)
        .map(|(index, &(at, weight))| Part {
            index,
            at,
            weight,
            relative: weight / largest,
        })
        .collect();
    if parts.is_empty() {
        return Ok(None);
    }
    let grid = build_field(&parts, params)?;
    let level = |part: &Part| grid.sample(part.at).unwrap_or(0.0);
    // Positions in `parts`, by level (highest first), ties by index.
    let mut order: Vec<usize> = (0..parts.len()).collect();
    let levels: Vec<f64> = parts.iter().map(level).collect();
    order.sort_by(|&a, &b| levels[b].total_cmp(&levels[a]).then(a.cmp(&b)));
    let wanted = params.share * total;
    let mut prefix = 0.0;
    let mut k = order.len();
    for (taken, &p) in order.iter().enumerate() {
        prefix += parts[p].weight;
        if prefix >= wanted {
            k = taken + 1;
            break;
        }
    }
    let radius = |p: &Part| radius_of(params.reach, p.relative);
    let (outline, (held, held_weight)) = match k {
        1 => {
            let part = &parts[order[0]];
            let (center, r) = (part.at, radius(part));
            let held = held_by(&parts, |at| {
                let (dx, dy) = (at.0 - center.0, at.1 - center.1);
                dx * dx + dy * dy <= r * r
            });
            (Outline::Ring { center, radius: r }, held)
        }
        2 => {
            let (first, second) = (order[0].min(order[1]), order[0].max(order[1]));
            let (pa, pb) = (&parts[first], &parts[second]);
            let (a, ra, b, rb) = (pa.at, radius(pa), pb.at, radius(pb));
            let held = held_by(&parts, |at| in_capsule(at, a, ra, b, rb));
            (
                Outline::Capsule {
                    a,
                    radius_a: ra,
                    b,
                    radius_b: rb,
                },
                held,
            )
        }
        _ => {
            let threshold_of = |j: usize| params.level_fraction * levels[order[j - 1]];
            let reaches = |j: usize| held_by_membrane(&grid, threshold_of(j), &parts).1 >= wanted;
            let n = order.len();
            let chosen = if reaches(k) || k == n {
                k
            } else if !reaches(n) {
                n
            } else {
                // `lo` falls short, `hi` reaches: the smallest j that reaches lies in `(lo, hi]`.
                let (mut lo, mut hi) = (k, n);
                while hi - lo > 1 {
                    let mid = lo + (hi - lo) / 2;
                    if reaches(mid) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                hi
            };
            let threshold = threshold_of(chosen);
            let held = held_by_membrane(&grid, threshold, &parts);
            (
                Outline::Membrane {
                    paths: iso_lines(&grid, threshold),
                },
                held,
            )
        }
    };
    Ok(Some(WeightedOutline {
        outline,
        held,
        held_weight,
        total_weight: total,
    }))
}
