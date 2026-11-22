//! Spreading overlapping points apart in a fixed number of steps.
//!
//! Points go into square cells of the side `min_distance`; each point meets only the points of the nine cells
//! around it, at most `max_neighbours` of them, so the cost is `n * max_neighbours * steps` and never `n * n`.
//! The push between two points on one spot is chosen from their two ids, never from their indices.

use super::{GeometryError, Point, Positions, Rect};

/// The default cap on the points one point examines in a step.
pub const DEFAULT_MAX_NEIGHBOURS: usize = 32;

/// The parameters of [`spread`].
#[derive(Clone, Debug, PartialEq)]
pub struct SpreadParams {
    /// The distance no two points may be closer than. Finite and above zero.
    pub min_distance: f64,
    /// The number of steps. Zero changes nothing.
    pub steps: u32,
    /// The most candidate partners one point examines in a step. At least 1.
    pub max_neighbours: usize,
    /// When set, every point is clamped into this rectangle after each step. Its minimum must not exceed its maximum on either axis.
    pub bounds: Option<Rect>,
}

impl SpreadParams {
    /// Parameters with [`DEFAULT_MAX_NEIGHBOURS`] and no bounds.
    #[must_use]
    pub fn new(min_distance: f64, steps: u32) -> Self {
        Self {
            min_distance,
            steps,
            max_neighbours: DEFAULT_MAX_NEIGHBOURS,
            bounds: None,
        }
    }
}

/// What a call of [`spread`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpreadReport {
    /// The pairs and obstacles examined over all steps.
    pub comparisons: u64,
    /// The pairs examined after the last step that are still closer than `min_distance` (less a tolerance of 1e-9
    /// of it). The final count is not part of `comparisons`.
    pub overlaps_left: usize,
}

/// The sine and cosine of 45 degrees (a constant, so no trigonometry runs).
const H: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Sixteen unit vectors at multiples of 22.5 degrees, as literals so no trigonometry runs.
const DIRECTIONS: [Point; 16] = [
    (1.0, 0.0),
    (0.9238795325112867, 0.3826834323650898),
    (H, H),
    (0.3826834323650898, 0.9238795325112867),
    (0.0, 1.0),
    (-0.3826834323650898, 0.9238795325112867),
    (-H, H),
    (-0.9238795325112867, 0.3826834323650898),
    (-1.0, 0.0),
    (-0.9238795325112867, -0.3826834323650898),
    (-H, -H),
    (-0.3826834323650898, -0.9238795325112867),
    (0.0, -1.0),
    (0.3826834323650898, -0.9238795325112867),
    (H, -H),
    (0.9238795325112867, -0.3826834323650898),
];

/// 64-bit FNV-1a.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn direction(hash: u64) -> Point {
    DIRECTIONS[(hash & 15) as usize]
}

type Cell = (i64, i64, usize);

fn cell_of(at: Point, side: f64) -> (i64, i64) {
    // Saturating casts: a finite coordinate far from the origin lands in an extreme cell, which is still a cell.
    ((at.1 / side).floor() as i64, (at.0 / side).floor() as i64)
}

fn sorted_cells(points: &[Point], side: f64) -> Vec<Cell> {
    let mut cells: Vec<Cell> = points
        .iter()
        .enumerate()
        .map(|(k, at)| {
            let (r, c) = cell_of(*at, side);
            (r, c, k)
        })
        .collect();
    cells.sort_unstable();
    cells
}

/// The entries of `cells` in the cell `(row, col)`, in index order.
fn in_cell(cells: &[Cell], row: i64, col: i64) -> &[Cell] {
    let start = cells.partition_point(|&(r, c, _)| (r, c) < (row, col));
    let end = cells.partition_point(|&(r, c, _)| (r, c) <= (row, col));
    &cells[start..end]
}

/// Visits, for each point in index order, the points `j > i` of the nine cells around its own (rows first, then
/// columns), until `max_neighbours` have been examined for `i`. `visit(i, j, distance)` is called for each. Returns
/// the number of pairs examined.
fn scan_pairs(
    points: &[Point],
    cells: &[Cell],
    side: f64,
    max_neighbours: usize,
    mut visit: impl FnMut(usize, usize, f64),
) -> u64 {
    let mut comparisons: u64 = 0;
    for (i, at) in points.iter().enumerate() {
        let (row, col) = cell_of(*at, side);
        let mut examined = 0usize;
        'cells: for dr in -1i64..=1 {
            for dc in -1i64..=1 {
                // Past the last representable cell there is no neighbour; never revisit the edge cell.
                let (Some(r), Some(c)) = (row.checked_add(dr), col.checked_add(dc)) else {
                    continue;
                };
                let cell = in_cell(cells, r, c);
                // Entries of a cell are sorted by index: skip straight to the first `j > i`.
                let cell = &cell[cell.partition_point(|&(_, _, j)| j <= i)..];
                for &(_, _, j) in cell {
                    if examined >= max_neighbours {
                        break 'cells;
                    }
                    examined += 1;
                    comparisons += 1;
                    let (dx, dy) = (points[j].0 - at.0, points[j].1 - at.1);
                    visit(i, j, (dx * dx + dy * dy).sqrt());
                }
            }
        }
    }
    comparisons
}

fn validate(
    positions: &Positions,
    obstacles: &[Point],
    params: &SpreadParams,
) -> Result<(), GeometryError> {
    let d = params.min_distance;
    if !(d.is_finite() && d > 0.0) {
        return Err(GeometryError::BadParameter {
            name: "min_distance",
            value: d,
        });
    }
    if params.max_neighbours < 1 {
        return Err(GeometryError::BadParameter {
            name: "max_neighbours",
            value: 0.0,
        });
    }
    if let Some((x0, y0, x1, y1)) = params.bounds {
        for value in [x0, y0, x1, y1] {
            if !value.is_finite() {
                return Err(GeometryError::BadParameter {
                    name: "bounds",
                    value,
                });
            }
        }
        if x0 > x1 {
            return Err(GeometryError::BadParameter {
                name: "bounds",
                value: x0,
            });
        }
        if y0 > y1 {
            return Err(GeometryError::BadParameter {
                name: "bounds",
                value: y0,
            });
        }
    }
    for (id, at) in positions.iter() {
        if !(at.0.is_finite() && at.1.is_finite()) {
            return Err(GeometryError::NonFinitePosition {
                item: id.to_owned(),
            });
        }
    }
    for (k, at) in obstacles.iter().enumerate() {
        if !(at.0.is_finite() && at.1.is_finite()) {
            return Err(GeometryError::NonFinitePosition {
                item: format!("#{k}"),
            });
        }
    }
    Ok(())
}

/// One step: every point moves at once from the positions at the start of the step. Returns the comparisons.
fn step(
    positions: &mut Positions,
    hashes: &[u64],
    obstacles: &[Point],
    obstacle_cells: &[Cell],
    params: &SpreadParams,
) -> u64 {
    let d = params.min_distance;
    let half = d / 2.0;
    let points: Vec<Point> = positions.iter().map(|(_, at)| at).collect();
    let cells = sorted_cells(&points, d);
    let mut shift: Vec<Point> = vec![(0.0, 0.0); points.len()];

    let mut comparisons = {
        let view: &Positions = positions;
        scan_pairs(&points, &cells, d, params.max_neighbours, |i, j, dist| {
            if dist >= d {
                return;
            }
            let (dx, dy) = (points[j].0 - points[i].0, points[j].1 - points[i].1);
            // The displacement of `i`; `j` gets its negation.
            let push = if dist == 0.0 {
                let (small, large) = if view.id_at(i) < view.id_at(j) {
                    (i, j)
                } else {
                    (j, i)
                };
                let u = direction(hashes[small].wrapping_mul(31) ^ hashes[large]);
                let sign = if i == large { 1.0 } else { -1.0 };
                (sign * u.0 * half, sign * u.1 * half)
            } else {
                let amount = (d - dist) / 2.0;
                (-(dx / dist) * amount, -(dy / dist) * amount)
            };
            shift[i].0 += push.0;
            shift[i].1 += push.1;
            shift[j].0 -= push.0;
            shift[j].1 -= push.1;
        })
    };

    for s in &mut shift {
        let len = (s.0 * s.0 + s.1 * s.1).sqrt();
        if len > half {
            let scale = half / len;
            s.0 *= scale;
            s.1 *= scale;
        }
    }

    if !obstacles.is_empty() {
        for (i, at) in points.iter().enumerate() {
            let (row, col) = cell_of(*at, d);
            for dr in -1i64..=1 {
                for dc in -1i64..=1 {
                    let (Some(r), Some(c)) = (row.checked_add(dr), col.checked_add(dc)) else {
                        continue;
                    };
                    for &(_, _, k) in in_cell(obstacle_cells, r, c) {
                        comparisons += 1;
                        let (dx, dy) = (at.0 - obstacles[k].0, at.1 - obstacles[k].1);
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist >= d {
                            continue;
                        }
                        if dist == 0.0 {
                            let u = direction(hashes[i]);
                            shift[i].0 += u.0 * d;
                            shift[i].1 += u.1 * d;
                        } else {
                            let amount = d - dist;
                            shift[i].0 += dx / dist * amount;
                            shift[i].1 += dy / dist * amount;
                        }
                    }
                }
            }
        }
    }

    for (i, s) in shift.iter().enumerate() {
        let at = positions.point_mut(i);
        at.0 += s.0;
        at.1 += s.1;
        if let Some((x0, y0, x1, y1)) = params.bounds {
            at.0 = at.0.max(x0).min(x1);
            at.1 = at.1.max(y0).min(y1);
        }
    }
    comparisons
}

/// Pushes points apart until none is closer than `min_distance` to another, within a fixed number of steps.
///
/// Rules, in order of a step: every point meets the points of the nine cells around it that come later in
/// insertion order, at most `max_neighbours` of them; a close pair is pushed apart by half the shortfall each along
/// the line between them; two points on one spot part along a direction chosen from the two ids, so adding a far
/// node changes no other node's bits; each point's summed push is cut to half the distance; `obstacles` (which
/// never move) push points out to the distance in full; all points move at once; then `bounds`, if set, clamp.
///
/// `steps = 0` changes nothing. Overlaps that remain (for example when `bounds` are narrower than the distance) are
/// reported in [`SpreadReport::overlaps_left`], never looped on.
///
/// # Errors
///
/// [`GeometryError::BadParameter`] for a `min_distance` that is not finite and above zero, a `max_neighbours` of
/// zero, or invalid `bounds`; [`GeometryError::NonFinitePosition`] for a point (named by id) or an obstacle (named
/// `#<index>`) that is not finite.
pub fn spread(
    positions: &mut Positions,
    obstacles: &[Point],
    params: &SpreadParams,
) -> Result<SpreadReport, GeometryError> {
    validate(positions, obstacles, params)?;
    let hashes: Vec<u64> = (0..positions.len())
        .map(|i| fnv1a(positions.id_at(i).as_bytes()))
        .collect();
    let obstacle_cells = sorted_cells(obstacles, params.min_distance);
    let mut comparisons: u64 = 0;
    for _ in 0..params.steps {
        comparisons += step(positions, &hashes, obstacles, &obstacle_cells, params);
    }

    let d = params.min_distance;
    let points: Vec<Point> = positions.iter().map(|(_, at)| at).collect();
    let cells = sorted_cells(&points, d);
    let mut overlaps_left = 0usize;
    scan_pairs(&points, &cells, d, params.max_neighbours, |_, _, dist| {
        if dist < d * (1.0 - 1e-9) {
            overlaps_left += 1;
        }
    });
    Ok(SpreadReport {
        comparisons,
        overlaps_left,
    })
}
