//! Camera + perspective projection + painter's depth sort (Task 2).
//!
//! Pipeline (PHASE-R-MAP §3, deck `topology-deck.html` `cam.ry`/`cam.rx`
//! model, mined from `tui-3d-renderer/src/raster.rs::prepare_position` +
//! `src/clip.rs`, but pure f64 and rotate-Y-then-X per the deck):
//!
//! 1. Camera space: `v' = RotX(rx) · RotY(ry) · v`, then `z' += dist`.
//! 2. Near-plane clip FIRST: reject the whole primitive if any vertex has
//!    `z' <= NEAR_EPS` (never divide by a near-zero/negative `z'`).
//! 3. Perspective divide with focal length `f = 1 / tan(fov*pi/360)`,
//!    aspect-corrected for braille subcell space (`cols*2` x `rows*4`).
//! 4. Viewport remap to subcell coordinates; `depth = v'.z` (smaller = nearer).

use core::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::{Face, Strut, Vertex3};

/// Near-plane epsilon: a vertex at/behind the camera (`z' <= NEAR_EPS`)
/// makes its whole primitive rejected rather than risking a near-zero or
/// negative perspective divide (Power-of-10 rule 4; `tui-3d/src/clip.rs`).
pub const NEAR_EPS: f64 = 1e-6;

/// Camera model: rotate about Y then X, camera distance, vertical FOV
/// (degrees) — mirrors the deck's `cam.ry`/`cam.rx`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub ry: f64,
    pub rx: f64,
    pub dist: f64,
    pub fov: f64,
}

/// A vertex projected to subcell screen space; `depth` is camera-space z
/// after rotation+translation (before the perspective divide), smaller =
/// nearer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projected {
    pub sx: f64,
    pub sy: f64,
    pub depth: f64,
}

/// Rotate `v` about Y by `ry`, then about X by `rx`, then place the camera
/// at `+dist` on z looking toward the origin (camera-space z decreases as
/// world z increases — a vertex nearer the camera gets a smaller depth).
/// Returns the camera-space vertex.
fn to_camera_space(v: &Vertex3, cam: &Camera) -> Vertex3 {
    // Rotate about Y.
    let (sy, cy) = cam.ry.sin_cos();
    let x1 = v.x * cy + v.z * sy;
    let y1 = v.y;
    let z1 = -v.x * sy + v.z * cy;

    // Rotate about X.
    let (sx, cx) = cam.rx.sin_cos();
    let x2 = x1;
    let y2 = y1 * cx - z1 * sx;
    let z2 = y1 * sx + z1 * cx;

    Vertex3 {
        x: x2,
        y: y2,
        z: cam.dist - z2,
    }
}

/// Project a single 3D point into subcell screen space under `cam`, for a
/// `cols x rows` character grid (braille 2x4 subcells per cell). Returns
/// `None` if the vertex is at/behind the near plane (`z' <= NEAR_EPS`).
pub fn project(v: &Vertex3, cam: &Camera, cols: u16, rows: u16) -> Projected {
    let cs = to_camera_space(v, cam);

    let width = cols as f64 * 2.0;
    let height = rows as f64 * 4.0;

    // Guard the perspective divide: at/behind the near plane, clamp z' to
    // NEAR_EPS so 1/z' never yields inf/NaN (primitive-level rejection is
    // the caller's responsibility via `is_clipped`).
    let z = if cs.z <= NEAR_EPS { NEAR_EPS } else { cs.z };

    let f = 1.0 / (cam.fov * core::f64::consts::PI / 360.0).tan();
    let aspect = height / width;
    let ndc_x = f * cs.x / z * aspect;
    let ndc_y = f * cs.y / z;

    let sx = (ndc_x + 1.0) / 2.0 * (width - 1.0);
    let sy = (1.0 - ndc_y) / 2.0 * (height - 1.0);

    Projected {
        sx,
        sy,
        depth: cs.z,
    }
}

/// `true` if `v` is at/behind the near plane under `cam` (camera-space
/// `z' <= NEAR_EPS`) — the whole primitive containing it must be rejected,
/// never partially divided.
fn is_clipped(v: &Vertex3, cam: &Camera) -> bool {
    to_camera_space(v, cam).z <= NEAR_EPS
}

/// `true` if any vertex of the face is at/behind the near plane.
pub fn face_is_clipped(face: &Face, cam: &Camera) -> bool {
    face.verts.iter().any(|v| is_clipped(v, cam))
}

/// `true` if either endpoint of the strut is at/behind the near plane.
pub fn strut_is_clipped(strut: &Strut, cam: &Camera) -> bool {
    is_clipped(&strut.a, cam) || is_clipped(&strut.b, cam)
}

fn nan_safe_cmp(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// Stable painter's depth sort: farthest-first, keyed on each primitive's
/// centroid camera-space depth. Stable so equal-depth primitives keep
/// scene (insertion) order — no draw-order nondeterminism. Bounded: one
/// pass over each finite slice.
pub fn depth_sort(faces: &mut [Face], struts: &mut [Strut], cam: &Camera) {
    faces.sort_by(|a, b| {
        let da = face_centroid_depth(a, cam);
        let db = face_centroid_depth(b, cam);
        // Farthest first: descending depth.
        nan_safe_cmp(db, da)
    });
    struts.sort_by(|a, b| {
        let da = strut_centroid_depth(a, cam);
        let db = strut_centroid_depth(b, cam);
        nan_safe_cmp(db, da)
    });
}

fn face_centroid_depth(face: &Face, cam: &Camera) -> f64 {
    let cx = (face.verts[0].x + face.verts[1].x + face.verts[2].x) / 3.0;
    let cy = (face.verts[0].y + face.verts[1].y + face.verts[2].y) / 3.0;
    let cz = (face.verts[0].z + face.verts[1].z + face.verts[2].z) / 3.0;
    to_camera_space(&Vertex3 { x: cx, y: cy, z: cz }, cam).z
}

fn strut_centroid_depth(strut: &Strut, cam: &Camera) -> f64 {
    let cx = (strut.a.x + strut.b.x) / 2.0;
    let cy = (strut.a.y + strut.b.y) / 2.0;
    let cz = (strut.a.z + strut.b.z) / 2.0;
    to_camera_space(&Vertex3 { x: cx, y: cy, z: cz }, cam).z
}
