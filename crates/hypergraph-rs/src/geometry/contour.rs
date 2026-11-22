//! Iso-lines of a scalar grid, by marching squares.
//!
//! A [`ScalarGrid`] holds one number at each node of a regular lattice. [`iso_lines`] draws the closed paths that
//! separate the nodes above a threshold from the rest. One virtual ring of nodes around the grid is always below the
//! threshold, so every path closes.

use indexmap::IndexMap;

use super::{GeometryError, Point};

/// A closed path: its first vertex is not repeated at the end.
pub type ClosedPath = Vec<Point>;

/// A regular grid of numbers.
///
/// Node `(col, row)` stands at `origin + (col * cell, row * cell)` and holds `values[row * cols + col]`. The fields
/// are public; a grid whose `values` do not hold exactly `cols * rows` numbers is malformed, and the functions that
/// read it panic.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarGrid {
    /// The position of node `(0, 0)`.
    pub origin: Point,
    /// The distance between neighbouring nodes.
    pub cell: f64,
    /// The count of columns of nodes.
    pub cols: usize,
    /// The count of rows of nodes.
    pub rows: usize,
    /// The numbers, row by row.
    pub values: Vec<f64>,
}

impl ScalarGrid {
    /// A grid of zeros.
    ///
    /// # Errors
    ///
    /// [`GeometryError::BadParameter`] for a `cell` that is not finite and above zero, an `origin` that is not
    /// finite, or `cols` or `rows` of zero, or a `cols * rows` that overflows (named `cols`).
    pub fn zeros(
        origin: Point,
        cell: f64,
        cols: usize,
        rows: usize,
    ) -> Result<Self, GeometryError> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err(GeometryError::BadParameter {
                name: "cell",
                value: cell,
            });
        }
        for value in [origin.0, origin.1] {
            if !value.is_finite() {
                return Err(GeometryError::BadParameter {
                    name: "origin",
                    value,
                });
            }
        }
        if cols == 0 {
            return Err(GeometryError::BadParameter {
                name: "cols",
                value: 0.0,
            });
        }
        if rows == 0 {
            return Err(GeometryError::BadParameter {
                name: "rows",
                value: 0.0,
            });
        }
        let nodes = cols.checked_mul(rows).ok_or(GeometryError::BadParameter {
            name: "cols",
            value: cols as f64,
        })?;
        Ok(Self {
            origin,
            cell,
            cols,
            rows,
            values: vec![0.0; nodes],
        })
    }

    /// The number at node `(col, row)`.
    ///
    /// # Panics
    ///
    /// When `col >= cols`, `row >= rows`, or the grid is malformed.
    #[must_use]
    pub fn get(&self, col: usize, row: usize) -> f64 {
        assert!(col < self.cols && row < self.rows, "node out of range");
        self.check();
        self.values[row * self.cols + col]
    }

    /// The bilinear value at `at`, or `None` outside the extent of the nodes (or for a position that is `NaN`, or on a grid without nodes).
    ///
    /// # Panics
    ///
    /// When the grid is malformed.
    #[must_use]
    pub fn sample(&self, at: Point) -> Option<f64> {
        self.check();
        if self.cols == 0 || self.rows == 0 {
            return None;
        }
        let u = (at.0 - self.origin.0) / self.cell;
        let v = (at.1 - self.origin.1) / self.cell;
        if !(u >= 0.0 && u <= (self.cols - 1) as f64 && v >= 0.0 && v <= (self.rows - 1) as f64) {
            return None;
        }
        let c0 = (u.floor() as usize).min(self.cols.saturating_sub(2));
        let r0 = (v.floor() as usize).min(self.rows.saturating_sub(2));
        let c1 = (c0 + 1).min(self.cols - 1);
        let r1 = (r0 + 1).min(self.rows - 1);
        let (fx, fy) = (u - c0 as f64, v - r0 as f64);
        let at_node = |c: usize, r: usize| self.values[r * self.cols + c];
        let top = at_node(c0, r0) + (at_node(c1, r0) - at_node(c0, r0)) * fx;
        let bottom = at_node(c0, r1) + (at_node(c1, r1) - at_node(c0, r1)) * fx;
        Some(top + (bottom - top) * fy)
    }

    fn check(&self) {
        assert!(
            self.values.len() == self.cols * self.rows,
            "grid holds {} numbers, expected {} * {}",
            self.values.len(),
            self.cols,
            self.rows
        );
    }
}

/// No crossing.
const NONE: usize = usize::MAX;

/// The numbering of the crossing edges. Edges run over the grid and its virtual ring: nodes with column in
/// `-1..=cols` and row in `-1..=rows`. Row by row from row `-1`, a row's horizontal edges (from node `(c, r)` to
/// `(c + 1, r)`, `c` from `-1`) come first, then the vertical edges from row `r` to row `r + 1` (`c` from `-1`).
struct Edges {
    cols: usize,
    stride: usize,
}

impl Edges {
    fn new(grid: &ScalarGrid) -> Self {
        Self {
            cols: grid.cols,
            stride: 2 * grid.cols + 3,
        }
    }

    fn count(&self, grid: &ScalarGrid) -> usize {
        (grid.rows + 2) * self.stride
    }

    /// The edge from node `(c, r)` to `(c + 1, r)`.
    fn horizontal(&self, c: i64, r: i64) -> usize {
        ((r + 1) as usize) * self.stride + (c + 1) as usize
    }

    /// The edge from node `(c, r)` to `(c, r + 1)`.
    fn vertical(&self, c: i64, r: i64) -> usize {
        ((r + 1) as usize) * self.stride + self.cols + 1 + (c + 1) as usize
    }
}

/// The value at node `(c, r)`, or `None` for a node of the virtual ring.
fn node_value(grid: &ScalarGrid, c: i64, r: i64) -> Option<f64> {
    if c < 0 || r < 0 || c >= grid.cols as i64 || r >= grid.rows as i64 {
        None
    } else {
        Some(grid.values[r as usize * grid.cols + c as usize])
    }
}

fn is_inside(grid: &ScalarGrid, threshold: f64, c: i64, r: i64) -> bool {
    node_value(grid, c, r).is_some_and(|v| v > threshold)
}

/// Where the line from value `a` (at 0) to value `b` (at 1) meets `threshold`; the middle when that is undefined.
fn crossing_fraction(threshold: f64, a: f64, b: f64) -> f64 {
    let t = (threshold - a) / (b - a);
    if (0.0..=1.0).contains(&t) {
        t
    } else {
        0.5
    }
}

/// The vertex of a crossing edge: by linear interpolation between two real nodes, at the middle of an edge to a
/// virtual node.
fn edge_vertex(grid: &ScalarGrid, threshold: f64, edges: &Edges, edge: usize) -> Point {
    let row = (edge / edges.stride) as i64 - 1;
    let rem = edge % edges.stride;
    let (ox, oy) = grid.origin;
    if rem < edges.cols + 1 {
        let c = rem as i64 - 1;
        let t = match (node_value(grid, c, row), node_value(grid, c + 1, row)) {
            (Some(a), Some(b)) => crossing_fraction(threshold, a, b),
            _ => 0.5,
        };
        (ox + (c as f64 + t) * grid.cell, oy + row as f64 * grid.cell)
    } else {
        let c = (rem - edges.cols - 1) as i64 - 1;
        let t = match (node_value(grid, c, row), node_value(grid, c, row + 1)) {
            (Some(a), Some(b)) => crossing_fraction(threshold, a, b),
            _ => 0.5,
        };
        (ox + c as f64 * grid.cell, oy + (row as f64 + t) * grid.cell)
    }
}

/// The segments of the cell whose lower-left node is `(cc, rr)`, as `(from edge, to edge)` pairs with the inside
/// on the left of each. `emit` is called for each.
///
/// Walking the cell's four edges counter-clockwise, a crossing where the walk leaves the inside is an exit and one
/// where it enters is an entry. A segment joins an exit to the next entry; for a saddle whose centre is not inside,
/// to the previous entry instead.
fn cell_segments(
    grid: &ScalarGrid,
    threshold: f64,
    edges: &Edges,
    cc: i64,
    rr: i64,
    mut emit: impl FnMut(usize, usize),
) {
    // Corners a, b, c, d counter-clockwise from the lower left.
    let corners = [(cc, rr), (cc + 1, rr), (cc + 1, rr + 1), (cc, rr + 1)];
    let inside = corners.map(|(c, r)| is_inside(grid, threshold, c, r));
    let sides = [
        edges.horizontal(cc, rr),
        edges.vertical(cc + 1, rr),
        edges.horizontal(cc, rr + 1),
        edges.vertical(cc, rr),
    ];
    // (edge, is an exit), in walking order.
    let mut crossings = [(NONE, false); 4];
    let mut n = 0;
    for k in 0..4 {
        let next = (k + 1) % 4;
        if inside[k] != inside[next] {
            crossings[n] = (sides[k], inside[k]);
            n += 1;
        }
    }
    match n {
        2 => {
            let (from, to) = if crossings[0].1 {
                (crossings[0].0, crossings[1].0)
            } else {
                (crossings[1].0, crossings[0].0)
            };
            emit(from, to);
        }
        4 => {
            let value = |c: i64, r: i64| node_value(grid, c, r).unwrap_or(f64::NEG_INFINITY);
            let sum =
                ((value(cc, rr) + value(cc + 1, rr)) + value(cc + 1, rr + 1)) + value(cc, rr + 1);
            let joined = sum / 4.0 > threshold;
            for k in 0..4 {
                if crossings[k].1 {
                    let other = if joined { (k + 1) % 4 } else { (k + 3) % 4 };
                    emit(crossings[k].0, crossings[other].0);
                }
            }
        }
        _ => {}
    }
}

/// The closed paths along which the grid crosses `threshold`.
///
/// A node is inside when its value is above `threshold` (a `NaN` is outside). Crossings between two nodes are
/// placed by linear interpolation; a crossing to the virtual ring around the grid sits half a cell outside it. A
/// saddle is joined when the mean of its four corners is above `threshold`.
///
/// Each path has the inside on its left: outer boundaries run counter-clockwise (positive [`signed_area`]), holes
/// clockwise. No vertex repeats within a path and no path has fewer than three vertices or zero area: where a line
/// touches itself at a node holding exactly `threshold`, it is cut there into separate loops. Paths start at the vertex that comes
/// first in row-major order of the crossing edges (horizontal edges of a row before its vertical edges) and are
/// listed in that order. The cost is linear in the count of nodes.
///
/// # Panics
///
/// When `grid.values.len() != grid.cols * grid.rows`.
#[must_use]
pub fn iso_lines(grid: &ScalarGrid, threshold: f64) -> Vec<ClosedPath> {
    grid.check();
    let edges = Edges::new(grid);
    let mut next = vec![NONE; edges.count(grid)];
    for rr in -1..grid.rows as i64 {
        for cc in -1..grid.cols as i64 {
            cell_segments(grid, threshold, &edges, cc, rr, |from, to| next[from] = to);
        }
    }
    let mut paths = Vec::new();
    for start in 0..next.len() {
        if next[start] == NONE {
            continue;
        }
        let mut path: Vec<Point> = Vec::new();
        let mut seen: IndexMap<(u64, u64), usize> = IndexMap::new();
        let mut edge = start;
        while next[edge] != NONE {
            let vertex = edge_vertex(grid, threshold, &edges, edge);
            // `+ 0.0` turns a negative zero into zero, so equal points have equal keys.
            let key = ((vertex.0 + 0.0).to_bits(), (vertex.1 + 0.0).to_bits());
            // A node outside the line that holds exactly `threshold` is met by several crossing edges, all
            // placed on it. When the walk comes back to a vertex it already holds, the path touches itself
            // there: the part walked since is cut off as a loop of its own.
            if let Some(&at) = seen.get(&key) {
                let cut = path.split_off(at + 1);
                for dropped in &cut {
                    seen.swap_remove(&((dropped.0 + 0.0).to_bits(), (dropped.1 + 0.0).to_bits()));
                }
                keep_loop(&mut paths, std::iter::once(vertex).chain(cut).collect());
            } else {
                seen.insert(key, path.len());
                path.push(vertex);
            }
            let following = next[edge];
            next[edge] = NONE;
            edge = following;
        }
        keep_loop(&mut paths, path);
    }
    paths
}

/// Keeps `path` as a closed path when it has three vertices and a non-zero area.
fn keep_loop(paths: &mut Vec<ClosedPath>, path: Vec<Point>) {
    if path.len() >= 3 && signed_area(&path) != 0.0 {
        paths.push(path);
    }
}

/// The signed area of a closed path by the shoelace formula: positive when it runs counter-clockwise.
#[must_use]
pub fn signed_area(path: &[Point]) -> f64 {
    let mut twice = 0.0_f64;
    for (k, p) in path.iter().enumerate() {
        let q = path[(k + 1) % path.len()];
        twice += p.0 * q.1 - q.0 * p.1;
    }
    twice / 2.0
}

/// Whether the segment from `p` to `q` crosses the ray going right from `at`.
fn crosses_ray(p: Point, q: Point, at: Point) -> bool {
    if (p.1 > at.1) == (q.1 > at.1) {
        return false;
    }
    let x = p.0 + (at.1 - p.1) * (q.0 - p.0) / (q.1 - p.1);
    x > at.0
}

/// Whether `at` is inside the paths by the even-odd rule: a ray from `at` crosses an odd number of segments.
#[must_use]
pub fn contains(paths: &[ClosedPath], at: Point) -> bool {
    let mut inside = false;
    for path in paths {
        for (k, p) in path.iter().enumerate() {
            let q = path[(k + 1) % path.len()];
            if crosses_ray(*p, q, at) {
                inside = !inside;
            }
        }
    }
    inside
}

/// Whether `contains(&iso_lines(grid, threshold), at)` would be true, decided from the one cell that holds `at`.
///
/// It computes the cell's crossings with the same functions `iso_lines` uses and counts the cell's segments that
/// the ray going right crosses; the side of the cell's right edge, where the ray leaves, is read from the corners.
/// The two agree bit for bit except for a point lying exactly on a path.
///
/// # Panics
///
/// When `grid.values.len() != grid.cols * grid.rows`.
pub(crate) fn inside_cell(grid: &ScalarGrid, threshold: f64, at: Point) -> bool {
    grid.check();
    let u = ((at.0 - grid.origin.0) / grid.cell).floor();
    let v = ((at.1 - grid.origin.1) / grid.cell).floor();
    // Cells run from -1 to cols - 1 (rows - 1): the virtual ring makes half a cell of margin.
    if !(u >= -1.0 && u < grid.cols as f64 && v >= -1.0 && v < grid.rows as f64) {
        return false;
    }
    let (cc, rr) = (u as i64, v as i64);
    let edges = Edges::new(grid);
    let mut crossed = false;
    cell_segments(grid, threshold, &edges, cc, rr, |from, to| {
        let p = edge_vertex(grid, threshold, &edges, from);
        let q = edge_vertex(grid, threshold, &edges, to);
        if crosses_ray(p, q, at) {
            crossed = !crossed;
        }
    });
    // Where the ray meets the cell's right edge, between nodes b (below) and c (above).
    let below = is_inside(grid, threshold, cc + 1, rr);
    let above = is_inside(grid, threshold, cc + 1, rr + 1);
    let at_edge = if below == above {
        below
    } else {
        let y = edge_vertex(grid, threshold, &edges, edges.vertical(cc + 1, rr)).1;
        if at.1 < y {
            below
        } else {
            above
        }
    };
    at_edge != crossed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 40) as f64 / (1_u64 << 24) as f64
    }

    #[test]
    fn inside_cell_agrees_with_contains() {
        let mut state = 20_261_121_u64;
        let mut grid = ScalarGrid::zeros((0.0, 0.0), 1.0, 40, 30).unwrap();
        for value in &mut grid.values {
            *value = lcg(&mut state);
        }
        let paths = iso_lines(&grid, 0.5);
        let mut second = 7_u64;
        let mut inside = 0;
        for _ in 0..2_000 {
            let p = (lcg(&mut second) * 39.0, lcg(&mut second) * 29.0);
            let expected = contains(&paths, p);
            assert_eq!(inside_cell(&grid, 0.5, p), expected, "at {p:?}");
            inside += usize::from(expected);
        }
        assert!(inside > 300 && inside < 1_700, "{inside} inside");
        assert!(!inside_cell(&grid, 0.5, (-3.0, 4.0)));
    }
}
