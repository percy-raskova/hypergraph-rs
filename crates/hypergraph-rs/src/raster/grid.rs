//! Subcell depth buffer, integer Bresenham lines, edge-function scanline
//! triangle fill, and braille 2x4 glyph composition (Task 3).
//!
//! Mined from `tui-3d-renderer/src/raster.rs` for the z-buffer / edge
//! function / barycentric shape only (per CRITICAL REFERENCE CORRECTION 1
//! in the plan: the line walk is a fresh **integer** Bresenham, never the
//! upstream floating-point DDA, and the braille glyph floor is designed
//! fresh here — the reference ships half-blocks only).

use super::{
    camera::{
        depth_sort, depth_sort_nodes, face_is_clipped, node_is_clipped, project_wh,
        strut_is_clipped, Projected,
    },
    color::{pick_fg_bg, LitSubcell},
    Camera, Cell, CellGrid, Rgb, SceneGraph3D,
};

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

    /// Read the buffer out as a row-major `Rgb` plate: every subcell/pixel
    /// carries a color unconditionally, `clear` filling in anything the
    /// z-test never wrote (Task 14: the Tier-1 pixel path's glyph-floor
    /// analogue — every pixel is fully defined, mirroring Tier-0's
    /// braille-cell contract). Bounded: one pass over `w*h` entries.
    /// Only called from the `raster-png`-gated pixel path.
    #[cfg(feature = "raster-png")]
    pub(crate) fn to_rgb_plate(&self, clear: Rgb) -> Vec<Rgb> {
        self.color.iter().map(|c| c.unwrap_or(clear)).collect()
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

/// Depth-tested filled disc — the `Node3` point primitive (review MAJOR 3).
/// `radius` is already in target units (subcells for the Tier-0 braille
/// buffer, pixels for the Tier-1 pixel buffer): the caller multiplies the
/// node's world-space `radius` by `Projected::scale`, mirroring the deck's
/// own `r * p.s / 300` disc radii (`topology-deck.html:308,325,357,412`).
///
/// The center subcell is ALWAYS written (subject to the z-test), even for a
/// sub-unit radius — a node in the scene is never invisible, which is what
/// makes the Tier-0 frame "fully informative by construction". Bounded: the
/// bounding box is clamped into the buffer on all four sides, so the worst
/// case is one pass over `w*h` (Power-of-10 rule 2). NaN-safe: `clamp_coord`
/// maps non-finite coordinates to 0 and the `radius > 0.5` test is false for
/// `NaN`, so a degenerate node degrades to a single center subcell.
pub(crate) fn fill_disc(buf: &mut SubBuffer, p: Projected, radius: f64, color: Rgb) {
    let cx = clamp_coord(p.sx, buf.w);
    let cy = clamp_coord(p.sy, buf.h);
    buf.set(cx, cy, p.depth, color);

    if radius.is_nan() || radius <= 0.5 {
        return; // sub-unit (or NaN) radius: the center subcell IS the disc.
    }

    let min_x = clamp_coord(p.sx - radius, buf.w);
    let max_x = clamp_coord(p.sx + radius, buf.w);
    let min_y = clamp_coord(p.sy - radius, buf.h);
    let max_y = clamp_coord(p.sy + radius, buf.h);
    let r2 = radius * radius;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f64 - p.sx;
            let dy = y as f64 - p.sy;
            if dx * dx + dy * dy <= r2 {
                buf.set(x, y, p.depth, color);
            }
        }
    }
}

/// The ONE scene->buffer draw core both raster tiers call (reuse ledger —
/// Tier-0 braille `rasterize` and Tier-1 `png::render_pixels` differ only in
/// their target resolution and their read-out, never in geometry).
///
/// Order: near-plane clip, stable painter's depth sort, then faces, then
/// struts, then node discs — every write z-tested, so draw order only
/// decides exact-depth ties, and each pass is itself stably sorted, keeping
/// the frame a pure function of `(scene, cam, width, height)`. Bounded: one
/// pass per primitive over finite scene collections.
pub(crate) fn draw_scene(
    buf: &mut SubBuffer,
    scene: &SceneGraph3D,
    cam: &Camera,
    width: f64,
    height: f64,
) {
    let mut faces: Vec<_> = scene
        .faces
        .iter()
        .filter(|f| !face_is_clipped(f, cam))
        .cloned()
        .collect();
    let mut struts: Vec<_> = scene
        .struts
        .iter()
        .filter(|s| !strut_is_clipped(s, cam))
        .cloned()
        .collect();
    let mut nodes: Vec<_> = scene
        .nodes
        .iter()
        .filter(|n| !node_is_clipped(n, cam))
        .cloned()
        .collect();
    depth_sort(&mut faces, &mut struts, cam);
    depth_sort_nodes(&mut nodes, cam);

    for f in &faces {
        let p0 = project_wh(&f.verts[0], cam, width, height);
        let p1 = project_wh(&f.verts[1], cam, width, height);
        let p2 = project_wh(&f.verts[2], cam, width, height);
        fill_triangle(buf, [p0, p1, p2], f.fill);
    }
    for s in &struts {
        let a = project_wh(&s.a, cam, width, height);
        let b = project_wh(&s.b, cam, width, height);
        line_bresenham(buf, a, b, s.color);
    }
    for n in &nodes {
        let p = project_wh(&n.pos, cam, width, height);
        fill_disc(buf, p, n.radius * p.scale, n.color);
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
/// glyph is `0x2800 + set_bits` over its 8 subcells; `fg`/`bg` are picked
/// under the braille "one fg + one bg" constraint (Task 4: majority-lit
/// color for `fg`, ties broken by nearest depth then fixed subcell scan
/// order — `color::pick_fg_bg`). Bounded loops: `cols*rows` cells x 8
/// subcells each.
pub(crate) fn compose_braille(buf: &SubBuffer, cols: u16, rows: u16) -> CellGrid {
    let mut cells = Vec::with_capacity(cols as usize * rows as usize);

    for row in 0..rows as usize {
        for col in 0..cols as usize {
            let mut bits: u8 = 0;
            let mut lit: Vec<LitSubcell> = Vec::with_capacity(8);

            for sub_row in 0..4usize {
                for sub_col in 0..2usize {
                    let x = col * 2 + sub_col;
                    let y = row * 4 + sub_row;
                    let i = buf.idx(x, y);
                    if let Some(c) = buf.color[i] {
                        bits |= braille_bit(sub_col, sub_row);
                        lit.push(LitSubcell {
                            color: c,
                            depth: buf.depth[i],
                        });
                    }
                }
            }

            let ch = char::from_u32(0x2800 + bits as u32).unwrap_or('\u{2800}');
            let (fg, bg) = pick_fg_bg(&lit);
            cells.push(Cell { ch, fg, bg });
        }
    }

    CellGrid { cols, rows, cells }
}
