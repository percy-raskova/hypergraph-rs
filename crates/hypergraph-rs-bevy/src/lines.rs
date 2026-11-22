//! Paths, hatching and ribbons: pure functions from outlines to numbers and meshes.
//!
//! Nothing here touches a window or a world. Hatching is computed without a
//! triangulation: every edge of every path reports where it crosses the
//! lines of a lattice fixed to the origin, and the crossings are paired by
//! the even-odd rule.

use std::f64::consts::FRAC_1_SQRT_2;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use hypergraph_rs::geometry::{ClosedPath, Outline, Point};

use crate::error::FieldError;
use crate::style::HatchDirection;

/// The fewest segments a round outline may be drawn with.
const MIN_SEGMENTS: u32 = 8;

/// Places a plan point `(x, y)` in the scene at `(x, height, -y)`.
///
/// This is the one place the plan's coordinates are turned into the scene's.
///
/// ```
/// use hypergraph_rs_bevy::plan_to_scene;
/// let p = plan_to_scene((2.0, 3.0), 1.5);
/// assert_eq!((p.x, p.y, p.z), (2.0, 1.5, -3.0));
/// ```
#[must_use]
pub fn plan_to_scene(p: Point, height: f32) -> Vec3 {
    Vec3::new(p.0 as f32, height, -(p.1 as f32))
}

fn check_segments(segments: u32) -> Result<(), FieldError> {
    if segments < MIN_SEGMENTS {
        return Err(FieldError::BadParameter {
            name: "segments",
            value: f64::from(segments),
        });
    }
    Ok(())
}

fn ring(center: Point, radius: f64, segments: u32) -> ClosedPath {
    (0..segments)
        .map(|k| {
            let angle = std::f64::consts::TAU * f64::from(k) / f64::from(segments);
            (
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            )
        })
        .collect()
}

/// An arc round `center` of `radius`, from the unit normal `start`, turning
/// counter-clockwise by `sweep` in `steps` steps; both ends are included.
fn arc(
    center: Point,
    radius: f64,
    start: (f64, f64),
    sweep: f64,
    steps: u32,
    out: &mut ClosedPath,
) {
    let (sin, cos) = (sweep / f64::from(steps)).sin_cos();
    let mut n = start;
    for _ in 0..=steps {
        out.push((center.0 + radius * n.0, center.1 + radius * n.1));
        n = (n.0 * cos - n.1 * sin, n.0 * sin + n.1 * cos);
    }
}

/// The closed paths of an outline, counter-clockwise for outer paths.
///
/// A `Ring` gives one path of `segments` vertices, vertex `k` at angle
/// `2πk / segments` from `+x`. A `Capsule` gives one path of its two end arcs
/// and the two tangent sides; when one circle holds the other, it is the ring
/// of the larger circle. A `Membrane` gives its paths unchanged.
///
/// # Errors
///
/// `BadParameter` when `segments` is below 8, `NonFinite` when a number of a
/// ring or capsule is not finite, `UnsupportedOutline` for a kind this crate
/// does not draw.
///
/// ```
/// use hypergraph_rs::geometry::Outline;
/// use hypergraph_rs_bevy::outline_paths;
/// let paths = outline_paths(&Outline::Ring { center: (0.0, 0.0), radius: 1.0 }, 8).unwrap();
/// assert_eq!(paths[0].len(), 8);
/// ```
pub fn outline_paths(outline: &Outline, segments: u32) -> Result<Vec<ClosedPath>, FieldError> {
    check_segments(segments)?;
    let non_finite = || FieldError::NonFinite {
        item: "outline".to_owned(),
    };
    match outline {
        Outline::Ring { center, radius } => {
            if ![center.0, center.1, *radius].iter().all(|v| v.is_finite()) {
                return Err(non_finite());
            }
            Ok(vec![ring(*center, *radius, segments)])
        }
        Outline::Capsule {
            a,
            radius_a,
            b,
            radius_b,
        } => {
            if ![a.0, a.1, b.0, b.1, *radius_a, *radius_b]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err(non_finite());
            }
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let d = dx.hypot(dy);
            if d <= (radius_a - radius_b).abs() {
                return Ok(vec![if radius_a >= radius_b {
                    ring(*a, *radius_a, segments)
                } else {
                    ring(*b, *radius_b, segments)
                }]);
            }
            let u = (dx / d, dy / d);
            let v = (-u.1, u.0);
            let s = (radius_a - radius_b) / d;
            let c = (1.0 - s * s).sqrt();
            let n_plus = (s * u.0 + c * v.0, s * u.1 + c * v.1);
            let n_minus = (s * u.0 - c * v.0, s * u.1 - c * v.1);
            let theta = c.atan2(s);
            let steps = segments / 2;
            let mut path = Vec::with_capacity(2 * (steps as usize + 1));
            arc(*b, *radius_b, n_minus, 2.0 * theta, steps, &mut path);
            arc(
                *a,
                *radius_a,
                n_plus,
                std::f64::consts::TAU - 2.0 * theta,
                steps,
                &mut path,
            );
            Ok(vec![path])
        }
        Outline::Membrane { paths } => Ok(paths.clone()),
        _ => Err(FieldError::UnsupportedOutline),
    }
}

/// Hatch strokes over the area the paths enclose by the even-odd rule.
///
/// The lines sit at `n·x = k·spacing` for whole `k`, so the hatching is fixed
/// to the origin and does not shimmer as a shape changes. A vertex exactly on
/// a line is counted once, and strokes of no length are dropped. A `spacing`
/// that is not finite or not above zero gives no strokes. The output is
/// ordered by line, then along the line.
///
/// ```
/// use hypergraph_rs_bevy::{hatch_segments, HatchDirection};
/// let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
/// assert_eq!(hatch_segments(&[square], HatchDirection::Deg0, 0.5).len(), 4);
/// ```
#[must_use]
pub fn hatch_segments(
    paths: &[ClosedPath],
    direction: HatchDirection,
    spacing: f64,
) -> Vec<(Point, Point)> {
    if !(spacing.is_finite() && spacing > 0.0) {
        return Vec::new();
    }
    let d = match direction {
        HatchDirection::Deg0 => (1.0, 0.0),
        HatchDirection::Deg45 => (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        HatchDirection::Deg90 => (0.0, 1.0),
        HatchDirection::Deg135 => (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    };
    let n = (-d.1, d.0);
    // Beyond this many lines from the origin an index no longer holds exactly.
    const MAX_LINE: f64 = 4.0e15;
    let mut crossings: Vec<(i64, f64)> = Vec::new();
    for path in paths {
        for (i, &p) in path.iter().enumerate() {
            let q = path[(i + 1) % path.len()];
            let sp = n.0 * p.0 + n.1 * p.1;
            let sq = n.0 * q.0 + n.1 * q.1;
            if !(sp.is_finite() && sq.is_finite() && q.0.is_finite() && q.1.is_finite())
                || sp == sq
                || sp.abs() / spacing > MAX_LINE
                || sq.abs() / spacing > MAX_LINE
            {
                continue;
            }
            let (lo, hi) = if sp < sq { (sp, sq) } else { (sq, sp) };
            let mut k = (lo / spacing).ceil() as i64;
            while ((k - 1) as f64) * spacing >= lo {
                k -= 1;
            }
            while (k as f64) * spacing < lo {
                k += 1;
            }
            loop {
                let line = (k as f64) * spacing;
                if line >= hi {
                    break;
                }
                let u = (line - sp) / (sq - sp);
                let x = p.0 + u * (q.0 - p.0);
                let y = p.1 + u * (q.1 - p.1);
                crossings.push((k, d.0 * x + d.1 * y));
                k += 1;
            }
        }
    }
    crossings.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut strokes = Vec::new();
    let mut at = 0;
    while at < crossings.len() {
        let k = crossings[at].0;
        let end = at + crossings[at..].iter().take_while(|c| c.0 == k).count();
        let line = (k as f64) * spacing;
        let mut j = at;
        while j + 1 < end {
            let (t0, t1) = (crossings[j].1, crossings[j + 1].1);
            if (t1 - t0).abs() >= 1e-9 * spacing {
                strokes.push((
                    (t0 * d.0 + line * n.0, t0 * d.1 + line * n.1),
                    (t1 * d.0 + line * n.0, t1 * d.1 + line * n.1),
                ));
            }
            j += 2;
        }
        at = end;
    }
    strokes
}

/// The segments of a closed path, whole or dashed.
///
/// `None` gives every edge in order. `Some((on, off))` walks the closed path
/// from its first vertex and keeps the stretches
/// `[j·(on+off), j·(on+off) + on)`; a dash crossing a vertex is split there.
/// An `on` or `off` that is not finite, an `on` not above zero or a negative
/// `off` gives no segments.
///
/// ```
/// use hypergraph_rs_bevy::dashed;
/// let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
/// assert_eq!(dashed(&square, Some((1.0, 1.0))).len(), 4);
/// ```
#[must_use]
pub fn dashed(path: &[Point], dash: Option<(f64, f64)>) -> Vec<(Point, Point)> {
    let n = path.len();
    if n < 2 {
        return Vec::new();
    }
    let edges = (0..n).map(|i| (path[i], path[(i + 1) % n]));
    let Some((on, off)) = dash else {
        return edges.collect();
    };
    if !(on.is_finite() && off.is_finite() && on > 0.0 && off >= 0.0) {
        return Vec::new();
    }
    let period = on + off;
    let mut out = Vec::new();
    let mut s0 = 0.0_f64;
    for (p, q) in edges {
        let len = (q.0 - p.0).hypot(q.1 - p.1);
        if len <= 0.0 || !len.is_finite() {
            continue;
        }
        let s1 = s0 + len;
        let at = |s: f64| {
            let u = (s - s0) / len;
            (p.0 + u * (q.0 - p.0), p.1 + u * (q.1 - p.1))
        };
        let mut j = (s0 / period).floor();
        while j * period < s1 {
            let lo = (j * period).max(s0);
            let hi = (j * period + on).min(s1);
            if hi > lo {
                out.push((at(lo), at(hi)));
            }
            j += 1.0;
        }
        s0 = s1;
    }
    out
}

/// One flat quad per segment, in the plane at `height`, `width` wide.
///
/// Each segment `(p, q)` becomes four vertices `p ± w·m` and `q ± w·m`, with
/// `m` the unit normal of `q − p` in the plan and `w = width / 2`, and six
/// indices. Normals all point up. Segments shorter than `1e-9` are skipped.
///
/// ```
/// use hypergraph_rs_bevy::ribbon_mesh;
/// let mesh = ribbon_mesh(&[((0.0, 0.0), (2.0, 0.0))], 0.5, 1.0);
/// assert_eq!(mesh.count_vertices(), 4);
/// ```
#[must_use]
pub fn ribbon_mesh(segments: &[(Point, Point)], width: f32, height: f32) -> Mesh {
    let w = f64::from(width) / 2.0;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for &(p, q) in segments {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = dx.hypot(dy);
        if !(len.is_finite() && len >= 1e-9) {
            continue;
        }
        let m = (-dy / len * w, dx / len * w);
        let base = positions.len() as u32;
        for corner in [
            (p.0 + m.0, p.1 + m.1),
            (p.0 - m.0, p.1 - m.1),
            (q.0 + m.0, q.1 + m.1),
            (q.0 - m.0, q.1 - m.1),
        ] {
            positions.push(plan_to_scene(corner, height).to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    let normals = vec![[0.0_f32, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
