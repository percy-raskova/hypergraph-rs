//! Subcell depth buffer, integer Bresenham lines, edge-function scanline
//! triangle fill, and braille 2x4 glyph composition (Task 3).
//!
//! Mined from `tui-3d-renderer/src/raster.rs` for the z-buffer / edge
//! function / barycentric shape only (per CRITICAL REFERENCE CORRECTION 1
//! in the plan: the line walk is a fresh **integer** Bresenham, never the
//! upstream floating-point DDA, and the braille glyph floor is designed
//! fresh here — the reference ships half-blocks only).

use super::{camera::Projected, Cell, CellGrid, Rgb};

/// A 2-wide x 4-tall-per-cell subcell buffer (`w = cols*2`, `h = rows*4`).
/// `depth` starts at `f64::INFINITY` (nothing drawn yet); `color` starts
/// `None`. The nearer-wins z-test lives in `fill_triangle`/`line_bresenham`.
pub(crate) struct SubBuffer {
    w: usize,
    h: usize,
    depth: Vec<f64>,
    color: Vec<Option<Rgb>>,
}

impl SubBuffer {
    pub(crate) fn new(w: usize, h: usize) -> Self {
        let n = w * h;
        Self {
            w,
            h,
            depth: vec![f64::INFINITY; n],
            color: vec![None; n],
        }
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    /// Z-test and write one subcell (Finding 4: caller passes already
    /// bounds-clamped `x,y` — never called with an out-of-range index).
    fn set(&mut self, x: usize, y: usize, depth: f64, color: Rgb) {
        let i = self.idx(x, y);
        if depth < self.depth[i] {
            self.depth[i] = depth;
            self.color[i] = Some(color);
        }
    }
}

/// Clamp a projected subcell coordinate into `0..len` (saturating on NaN
/// or out-of-range values — Finding 4, never panics on an extreme camera).
fn clamp_coord(v: f64, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if !v.is_finite() {
        return 0;
    }
    let max = (len - 1) as f64;
    v.round().clamp(0.0, max) as usize
}

/// Integer Bresenham line walk over subcell coordinates — no floating-point
/// accumulation (CRITICAL REFERENCE CORRECTION 1). Depth is linearly
/// interpolated per step and z-tested at each subcell.
pub(crate) fn line_bresenham(buf: &mut SubBuffer, a: Projected, b: Projected, color: Rgb) {
    let x0 = clamp_coord(a.sx, buf.w) as isize;
    let y0 = clamp_coord(a.sy, buf.h) as isize;
    let x1 = clamp_coord(b.sx, buf.w) as isize;
    let y1 = clamp_coord(b.sy, buf.h) as isize;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: isize = if x0 < x1 { 1 } else { -1 };
    let sy: isize = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    // Fixed upper bound: a Bresenham walk between two points inside a
    // w x h buffer takes at most w+h+1 steps (Power-of-10 rule 2).
    let max_steps = buf.w + buf.h + 1;
    let total = (dx - dy) as f64; // Manhattan-ish step count for interpolation (dy is <= 0).
    let total = if total <= 0.0 { 1.0 } else { total };

    let mut x = x0;
    let mut y = y0;
    let mut step = 0.0_f64;
    for _ in 0..=max_steps {
        let t = (step / total).clamp(0.0, 1.0);
        let depth = a.depth + (b.depth - a.depth) * t;
        buf.set(x as usize, y as usize, depth, color);

        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        step += 1.0;
    }
}

/// Pineda edge function, as specified in the plan: `edge(a,b,c) =
/// (c.x-a.x)*(b.y-a.y) - (c.y-a.y)*(b.x-a.x)`. Positive when `c` is to the
/// right of the directed edge `a->b`.
#[inline]
fn edge(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (c.0 - a.0) * (b.1 - a.1) - (c.1 - a.1) * (b.0 - a.0)
}

/// Top-left fill-rule tie-break for a directed edge `a->b` in a
/// (already normalized, area>0 under `edge()`) winding. `edge(a,b,c)>0`
/// means `c` is on the screen-left of `a->b` (y increasing downward) — so
/// this winding is screen-CCW and interior is to the left of each edge.
/// Under that convention a "top" edge is exactly horizontal and runs
/// **leftward** (`a.y==b.y && b.x<a.x`, so interior-left is below it); a
/// "left" edge runs **downward** (`b.y>a.y`, so interior-left is to its
/// +x side). Watertight shared edges: exactly one of the two triangles
/// sharing an edge owns it (per Task 3's flagged gap in `tui-3d`).
#[inline]
fn is_top_left(a: (f64, f64), b: (f64, f64)) -> bool {
    let is_top = a.1 == b.1 && b.0 < a.0;
    let is_left = b.1 > a.1;
    is_top || is_left
}

/// Edge-function bounding-box scanline fill with an explicit top-left
/// tie-break (the `tui-3d` repo omits this — a flagged gap). Depth is
/// barycentric-interpolated and z-tested per subcell. Bounded by the
/// (finite, buffer-clamped) triangle bounding box.
pub(crate) fn fill_triangle(buf: &mut SubBuffer, verts: [Projected; 3], color: Rgb) {
    let v0 = (verts[0].sx, verts[0].sy);
    let mut v1 = (verts[1].sx, verts[1].sy);
    let mut v2 = (verts[2].sx, verts[2].sy);
    let d0 = verts[0].depth;
    let mut d1 = verts[1].depth;
    let mut d2 = verts[2].depth;

    let mut area = edge(v0, v1, v2);
    if area == 0.0 {
        return; // Degenerate triangle: zero screen-space area, nothing to fill.
    }
    if area < 0.0 {
        // Normalize winding so area>0 (CCW under `edge()`'s convention);
        // swap v1/v2 to flip orientation without changing the triangle.
        core::mem::swap(&mut v1, &mut v2);
        core::mem::swap(&mut d1, &mut d2);
        area = -area;
    }

    let min_x = clamp_coord(v0.0.min(v1.0).min(v2.0), buf.w);
    let max_x = clamp_coord(v0.0.max(v1.0).max(v2.0), buf.w);
    let min_y = clamp_coord(v0.1.min(v1.1).min(v2.1), buf.h);
    let max_y = clamp_coord(v0.1.max(v1.1).max(v2.1), buf.h);

    let bias0 = if is_top_left(v1, v2) {
        0.0
    } else {
        -f64::EPSILON.max(1e-9)
    };
    let bias1 = if is_top_left(v2, v0) {
        0.0
    } else {
        -f64::EPSILON.max(1e-9)
    };
    let bias2 = if is_top_left(v0, v1) {
        0.0
    } else {
        -f64::EPSILON.max(1e-9)
    };

    // Bounded: bounding-box area is finite (buffer-clamped on all sides).
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f64, y as f64);
            let w0 = edge(v1, v2, p);
            let w1 = edge(v2, v0, p);
            let w2 = edge(v0, v1, p);

            let inside = (w0 + bias0 >= 0.0) && (w1 + bias1 >= 0.0) && (w2 + bias2 >= 0.0);
            if !inside {
                continue;
            }

            let bc0 = w0 / area;
            let bc1 = w1 / area;
            let bc2 = w2 / area;
            let depth = bc0 * d0 + bc1 * d1 + bc2 * d2;
            buf.set(x, y, depth, color);
        }
    }
}

/// Braille dot -> bit map (Unicode U+2800 base), per the plan's table:
/// `(col0,row0)=0x01 (col1,row0)=0x08 (col0,row1)=0x02 (col1,row1)=0x10
/// (col0,row2)=0x04 (col1,row2)=0x20 (col0,row3)=0x40 (col1,row3)=0x80`.
#[inline]
fn braille_bit(sub_col: usize, sub_row: usize) -> u8 {
    match (sub_col, sub_row) {
        (0, 0) => 0x01,
        (1, 0) => 0x08,
        (0, 1) => 0x02,
        (1, 1) => 0x10,
        (0, 2) => 0x04,
        (1, 2) => 0x20,
        (0, 3) => 0x40,
        (1, 3) => 0x80,
        _ => 0,
    }
}

/// Read the subcell buffer into a `cols x rows` `CellGrid`: each cell's
/// glyph is `0x2800 + set_bits` over its 8 subcells; `fg` is the color of
/// the nearest lit subcell (Task 4 finalizes color mixing); `bg` is black.
/// Bounded loops: `cols*rows` cells x 8 subcells each.
pub(crate) fn compose_braille(buf: &SubBuffer, cols: u16, rows: u16) -> CellGrid {
    let mut cells = Vec::with_capacity(cols as usize * rows as usize);

    for row in 0..rows as usize {
        for col in 0..cols as usize {
            let mut bits: u8 = 0;
            let mut nearest_depth = f64::INFINITY;
            let mut nearest_color = None;

            for sub_row in 0..4usize {
                for sub_col in 0..2usize {
                    let x = col * 2 + sub_col;
                    let y = row * 4 + sub_row;
                    let i = buf.idx(x, y);
                    if let Some(c) = buf.color[i] {
                        bits |= braille_bit(sub_col, sub_row);
                        if buf.depth[i] < nearest_depth {
                            nearest_depth = buf.depth[i];
                            nearest_color = Some(c);
                        }
                    }
                }
            }

            let ch = char::from_u32(0x2800 + bits as u32).unwrap_or('\u{2800}');
            cells.push(Cell {
                ch,
                fg: nearest_color.unwrap_or(Rgb(0, 0, 0)),
                bg: Rgb(0, 0, 0),
            });
        }
    }

    CellGrid { cols, rows, cells }
}
