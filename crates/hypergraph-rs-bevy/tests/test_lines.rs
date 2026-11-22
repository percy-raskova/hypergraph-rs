use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use hypergraph_rs::geometry::{signed_area, Outline};
use hypergraph_rs_bevy::{
    dashed, hatch_segments, outline_paths, plan_to_scene, ribbon_mesh, FieldError, HatchDirection,
};

fn square(side: f64) -> Vec<(f64, f64)> {
    vec![(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
}

fn near(a: (f64, f64), b: (f64, f64), eps: f64) -> bool {
    (a.0 - b.0).abs() <= eps && (a.1 - b.1).abs() <= eps
}

fn dist_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

#[test]
fn plan_points_land_with_y_reversed() {
    assert_eq!(plan_to_scene((2.0, 3.0), 1.5), Vec3::new(2.0, 1.5, -3.0));
}

#[test]
fn a_ring_is_a_counter_clockwise_polygon() {
    let paths = outline_paths(
        &Outline::Ring {
            center: (0.0, 0.0),
            radius: 1.0,
        },
        8,
    )
    .unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].len(), 8);
    assert!(near(paths[0][0], (1.0, 0.0), 1e-12));
    assert!(near(paths[0][2], (0.0, 1.0), 1e-12));
    assert!(signed_area(&paths[0]) > 0.0);
}

#[test]
fn a_capsule_is_two_ends_and_two_sides() {
    let paths = outline_paths(
        &Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 1.0,
            b: (4.0, 0.0),
            radius_b: 1.0,
        },
        48,
    )
    .unwrap();
    assert_eq!(paths.len(), 1);
    let exact = 8.0 + std::f64::consts::PI;
    let area = signed_area(&paths[0]);
    assert!(area > 0.0);
    assert!((area - exact).abs() / exact < 0.01, "area {area}");
    for p in &paths[0] {
        let d = dist_to_segment(*p, (0.0, 0.0), (4.0, 0.0));
        assert!((d - 1.0).abs() < 1e-9, "distance {d}");
    }
}

#[test]
fn a_capsule_inside_one_end_is_that_end() {
    let ring2 = outline_paths(
        &Outline::Ring {
            center: (0.0, 0.0),
            radius: 2.0,
        },
        48,
    )
    .unwrap();
    let held = outline_paths(
        &Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 2.0,
            b: (0.5, 0.0),
            radius_b: 1.0,
        },
        48,
    )
    .unwrap();
    assert_eq!(held, ring2);
    let ring1 = outline_paths(
        &Outline::Ring {
            center: (0.0, 0.0),
            radius: 1.0,
        },
        48,
    )
    .unwrap();
    let same = outline_paths(
        &Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 1.0,
            b: (0.0, 0.0),
            radius_b: 1.0,
        },
        48,
    )
    .unwrap();
    assert_eq!(same, ring1);
}

#[test]
fn a_membrane_passes_through_unchanged() {
    let paths = vec![square(2.0)];
    let got = outline_paths(
        &Outline::Membrane {
            paths: paths.clone(),
        },
        48,
    )
    .unwrap();
    assert_eq!(got, paths);
}

#[test]
fn hatching_fills_a_square_and_leaves_its_hole() {
    let strokes = hatch_segments(&[square(2.0)], HatchDirection::Deg0, 0.5);
    assert_eq!(strokes.len(), 4);
    for (k, s) in strokes.iter().enumerate() {
        let y = 0.5 * k as f64;
        assert!(
            near(s.0, (0.0, y), 1e-12) && near(s.1, (2.0, y), 1e-12),
            "{s:?}"
        );
    }
    let hole = vec![(0.5, 0.5), (0.5, 1.5), (1.5, 1.5), (1.5, 0.5)];
    let strokes = hatch_segments(&[square(2.0), hole], HatchDirection::Deg0, 0.5);
    let at_half: Vec<_> = strokes
        .iter()
        .filter(|s| (s.0 .1 - 0.5).abs() < 1e-12)
        .collect();
    assert_eq!(at_half.len(), 2);
    assert!(near(at_half[0].0, (0.0, 0.5), 1e-12) && near(at_half[0].1, (0.5, 0.5), 1e-12));
    assert!(near(at_half[1].0, (1.5, 0.5), 1e-12) && near(at_half[1].1, (2.0, 0.5), 1e-12));
}

#[test]
fn a_vertex_on_a_hatch_line_is_counted_once() {
    let diamond = vec![(1.0, 0.0), (2.0, 1.0), (1.0, 2.0), (0.0, 1.0)];
    let strokes = hatch_segments(&[diamond], HatchDirection::Deg0, 1.0);
    assert_eq!(strokes.len(), 1, "{strokes:?}");
    assert!(near(strokes[0].0, (0.0, 1.0), 1e-12));
    assert!(near(strokes[0].1, (2.0, 1.0), 1e-12));
}

#[test]
fn two_directions_cross() {
    for direction in [HatchDirection::Deg45, HatchDirection::Deg135] {
        let strokes = hatch_segments(&[square(2.0)], direction, 0.5);
        assert!(!strokes.is_empty());
        for (p, q) in strokes {
            for e in [p, q] {
                assert!(
                    (-1e-9..=2.0 + 1e-9).contains(&e.0) && (-1e-9..=2.0 + 1e-9).contains(&e.1),
                    "{e:?}"
                );
            }
        }
    }
}

#[test]
fn hatching_stays_on_its_lattice_when_the_shape_grows() {
    let small = hatch_segments(&[square(2.0)], HatchDirection::Deg0, 0.5);
    let large = hatch_segments(&[square(3.0)], HatchDirection::Deg0, 0.5);
    let large_ys: Vec<u64> = large.iter().map(|s| s.0 .1.to_bits()).collect();
    assert!(!small.is_empty());
    for s in &small {
        assert!(large_ys.contains(&s.0 .1.to_bits()), "{s:?}");
    }
}

#[test]
fn a_bad_spacing_gives_no_strokes() {
    for spacing in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(hatch_segments(&[square(2.0)], HatchDirection::Deg0, spacing).is_empty());
    }
}

#[test]
fn a_tiny_spacing_over_a_large_shape_does_not_size_by_lines() {
    // Cost is the crossings: 2 edges x 40 lines here, never one slot per line of the plane.
    let strokes = hatch_segments(&[square(1.0e6)], HatchDirection::Deg0, 25_000.0);
    assert_eq!(strokes.len(), 40);
}

#[test]
fn dashes_split_a_path_by_length() {
    let path = square(2.0);
    let dashes = dashed(&path, Some((1.0, 1.0)));
    assert_eq!(dashes.len(), 4);
    for (p, q) in &dashes {
        let len = ((q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)).sqrt();
        assert!((len - 1.0).abs() < 1e-12, "{len}");
    }
    assert_eq!(dashed(&path, None).len(), 4);
}

#[test]
fn a_ribbon_is_one_flat_quad_per_segment() {
    let mesh = ribbon_mesh(
        &[((0.0, 0.0), (2.0, 0.0)), ((1.0, 1.0), (1.0, 1.0))],
        0.5,
        3.0,
    );
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("no positions");
    };
    assert_eq!(positions.len(), 4);
    let Some(Indices::U32(indices)) = mesh.indices() else {
        panic!("no u32 indices");
    };
    assert_eq!(indices.len(), 6);
    let mut got: Vec<[f32; 3]> = positions.clone();
    got.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut want = vec![
        [0.0, 3.0, -0.25],
        [0.0, 3.0, 0.25],
        [2.0, 3.0, -0.25],
        [2.0, 3.0, 0.25],
    ];
    want.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(got, want);
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!("no normals");
    };
    assert!(normals.iter().all(|n| *n == [0.0, 1.0, 0.0]));
}

#[test]
fn few_segments_are_refused() {
    let err = outline_paths(
        &Outline::Ring {
            center: (0.0, 0.0),
            radius: 1.0,
        },
        4,
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            FieldError::BadParameter {
                name: "segments",
                ..
            }
        ),
        "{err:?}"
    );
}
