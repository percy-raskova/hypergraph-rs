use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use hypergraph_rs_bevy::{glyph_mesh, Glyph};

const ALL: [Glyph; 5] = [
    Glyph::Disc,
    Glyph::Ring,
    Glyph::Square,
    Glyph::SquareFrame,
    Glyph::Streak,
];

fn positions(mesh: &Mesh) -> Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("positions are not Float32x3: {other:?}"),
    }
}

fn normals(mesh: &Mesh) -> Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("normals are not Float32x3: {other:?}"),
    }
}

#[test]
fn every_glyph_is_flat_and_faces_up() {
    for glyph in ALL {
        let mesh = glyph_mesh(glyph, 16);
        let p = positions(&mesh);
        assert!(!p.is_empty(), "{glyph:?}");
        assert!(p.iter().all(|v| v[1] == 0.0), "{glyph:?} is not flat");
        let n = normals(&mesh);
        assert_eq!(n.len(), p.len());
        assert!(n.iter().all(|v| *v == [0.0, 1.0, 0.0]), "{glyph:?}");
        let Some(Indices::U32(i)) = mesh.indices() else {
            panic!("{glyph:?}: indices are not U32");
        };
        assert!(!i.is_empty() && i.len() % 3 == 0, "{glyph:?}");
        assert!(i.iter().all(|&k| (k as usize) < p.len()), "{glyph:?}");
    }
}

#[test]
fn a_disc_and_its_ring_have_the_same_outer_size() {
    let radii = |g: Glyph| -> Vec<f32> {
        positions(&glyph_mesh(g, 16))
            .iter()
            .map(|v| v[0].hypot(v[2]))
            .collect()
    };
    let disc = radii(Glyph::Disc);
    let ring = radii(Glyph::Ring);
    let max = |r: &[f32]| r.iter().copied().fold(f32::MIN, f32::max);
    let min = |r: &[f32]| r.iter().copied().fold(f32::MAX, f32::min);
    assert!((max(&disc) - 1.0).abs() < 1e-6);
    assert!((max(&ring) - 1.0).abs() < 1e-6);
    assert!((min(&ring) - 0.6).abs() < 1e-6);
}

#[test]
fn a_square_and_its_frame_have_the_same_outer_size() {
    for g in [Glyph::Square, Glyph::SquareFrame] {
        let p = positions(&glyph_mesh(g, 16));
        let max_x = p.iter().map(|v| v[0].abs()).fold(0.0, f32::max);
        let max_z = p.iter().map(|v| v[2].abs()).fold(0.0, f32::max);
        assert_eq!((max_x, max_z), (1.0, 1.0), "{g:?}");
    }
    let frame = positions(&glyph_mesh(Glyph::SquareFrame, 16));
    assert!(frame.contains(&[0.6, 0.0, 0.6]));
}

#[test]
fn a_streak_trails_behind_its_head() {
    let p = positions(&glyph_mesh(Glyph::Streak, 16));
    let max_x = p.iter().map(|v| v[0]).fold(f32::MIN, f32::max);
    let min_x = p.iter().map(|v| v[0]).fold(f32::MAX, f32::min);
    let max_z = p.iter().map(|v| v[2].abs()).fold(0.0, f32::max);
    assert_eq!((max_x, min_x, max_z), (0.0, -1.0, 0.125));
}

#[test]
fn the_same_glyph_gives_the_same_bytes_twice() {
    let a: Vec<[u32; 3]> = positions(&glyph_mesh(Glyph::Ring, 32))
        .iter()
        .map(|v| v.map(f32::to_bits))
        .collect();
    let b: Vec<[u32; 3]> = positions(&glyph_mesh(Glyph::Ring, 32))
        .iter()
        .map(|v| v.map(f32::to_bits))
        .collect();
    assert_eq!(a, b);
}
