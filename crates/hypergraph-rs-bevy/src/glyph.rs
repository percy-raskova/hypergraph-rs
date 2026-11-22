//! Flat glyph meshes for particles and marks.
//!
//! Every glyph lies in the plane `y = 0`, centred on the origin and of unit
//! size, with normals `[0, 1, 0]`. The caller's size is the entity's uniform
//! scale. A full glyph and its hollow pair have the same outer size.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::style::Glyph;

/// The fewest rim segments a round glyph is drawn with.
const MIN_SEGMENTS: u32 = 8;
/// Inner size of a hollow glyph, as a share of the outer size.
const INNER: f32 = 0.6;
/// Half the width of a streak.
const STREAK_HALF_WIDTH: f32 = 0.125;

/// A rim of `segments` vertices at `radius`, vertex `k` at angle
/// `2πk / segments` from `+x`.
fn rim(radius: f32, segments: u32) -> impl Iterator<Item = [f32; 3]> {
    (0..segments).map(move |k| {
        let angle = std::f32::consts::TAU * k as f32 / segments as f32;
        [radius * angle.cos(), 0.0, radius * angle.sin()]
    })
}

/// The corners of a square of half-side `half`, counter-clockwise from
/// `(+half, -half)`.
fn corners(half: f32) -> [[f32; 3]; 4] {
    [
        [half, 0.0, -half],
        [half, 0.0, half],
        [-half, 0.0, half],
        [-half, 0.0, -half],
    ]
}

/// Triangles joining an outer loop to an inner loop of the same length that
/// begin at matching vertices; `inner_at` is the index of the inner loop's
/// first vertex.
fn band(count: u32, inner_at: u32, indices: &mut Vec<u32>) {
    for k in 0..count {
        let next = (k + 1) % count;
        indices.extend_from_slice(&[k, next, inner_at + k]);
        indices.extend_from_slice(&[next, inner_at + next, inner_at + k]);
    }
}

/// The mesh of one glyph.
///
/// `segments` is the number of rim vertices of a round glyph; below 8 it is
/// taken as 8, and the square glyphs ignore it. The mesh is an indexed
/// triangle list, flat in `y = 0`, with every normal `[0, 1, 0]`:
///
/// - `Disc`: a centre and a rim at radius 1.
/// - `Ring`: rims at radius 0.6 and 1.
/// - `Square`: corners at `(±1, 0, ±1)`.
/// - `SquareFrame`: outer corners `±1`, inner corners `±0.6`.
/// - `Streak`: the rectangle `x` in `[-1, 0]`, `z` in `[-0.125, 0.125]`, so
///   its head is at the origin and it trails behind along `-x`.
///
/// ```
/// use hypergraph_rs_bevy::{glyph_mesh, Glyph};
/// assert_eq!(glyph_mesh(Glyph::Disc, 16).count_vertices(), 17);
/// ```
#[must_use]
pub fn glyph_mesh(glyph: Glyph, segments: u32) -> Mesh {
    let segments = segments.max(MIN_SEGMENTS);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    match glyph {
        Glyph::Disc => {
            positions.push([0.0, 0.0, 0.0]);
            positions.extend(rim(1.0, segments));
            for k in 0..segments {
                indices.extend_from_slice(&[0, 1 + k, 1 + (k + 1) % segments]);
            }
        }
        Glyph::Ring => {
            positions.extend(rim(1.0, segments));
            positions.extend(rim(INNER, segments));
            band(segments, segments, &mut indices);
        }
        Glyph::Square => {
            positions.extend(corners(1.0));
            indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        }
        Glyph::SquareFrame => {
            positions.extend(corners(1.0));
            positions.extend(corners(INNER));
            band(4, 4, &mut indices);
        }
        Glyph::Streak => {
            positions.extend_from_slice(&[
                [-1.0, 0.0, -STREAK_HALF_WIDTH],
                [0.0, 0.0, -STREAK_HALF_WIDTH],
                [0.0, 0.0, STREAK_HALF_WIDTH],
                [-1.0, 0.0, STREAK_HALF_WIDTH],
            ]);
            indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        }
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
