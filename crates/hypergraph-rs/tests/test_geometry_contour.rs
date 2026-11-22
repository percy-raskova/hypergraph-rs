#![cfg(feature = "geometry")]

use hypergraph_rs::geometry::{contains, iso_lines, signed_area, GeometryError, ScalarGrid};

fn grid(cols: usize, rows: usize, cell: f64, values: &[f64]) -> ScalarGrid {
    let mut g = ScalarGrid::zeros((0.0, 0.0), cell, cols, rows).unwrap();
    g.values.copy_from_slice(values);
    g
}

fn has_vertex(path: &[(f64, f64)], v: (f64, f64)) -> bool {
    path.iter()
        .any(|p| (p.0 - v.0).abs() < 1e-12 && (p.1 - v.1).abs() < 1e-12)
}

#[test]
fn one_node_above_gives_one_diamond() {
    let g = grid(1, 1, 2.0, &[1.0]);
    let paths = iso_lines(&g, 0.5);
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].len(), 4);
    for v in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
        assert!(has_vertex(&paths[0], v), "missing {v:?} in {:?}", paths[0]);
    }
    assert_eq!(signed_area(&paths[0]), 2.0);
}

#[test]
fn crossings_are_interpolated() {
    let g = grid(2, 1, 1.0, &[1.0, 0.0]);
    let paths = iso_lines(&g, 0.25);
    assert_eq!(paths.len(), 1);
    assert!(has_vertex(&paths[0], (0.75, 0.0)), "{:?}", paths[0]);
}

#[test]
fn a_saddle_is_joined_when_its_centre_is_inside() {
    let apart = iso_lines(&grid(2, 2, 1.0, &[1.0, 0.0, 0.0, 1.0]), 0.5);
    assert_eq!(apart.len(), 2);
    let joined = iso_lines(&grid(2, 2, 1.0, &[1.0, 0.2, 0.2, 1.0]), 0.5);
    assert_eq!(joined.len(), 1);
}

#[test]
fn a_hole_runs_clockwise() {
    let mut values = vec![1.0; 25];
    values[2 * 5 + 2] = 0.0;
    let g = grid(5, 5, 1.0, &values);
    let paths = iso_lines(&g, 0.5);
    assert_eq!(paths.len(), 2);
    let areas: Vec<f64> = paths.iter().map(|p| signed_area(p)).collect();
    assert!(areas.iter().any(|a| *a > 0.0), "{areas:?}");
    assert!(areas.iter().any(|a| *a < 0.0), "{areas:?}");
    assert!(!contains(&paths, (2.0, 2.0)));
    assert!(contains(&paths, (0.5, 0.5)));
    assert!(!contains(&paths, (-5.0, 0.5)));
}

#[test]
fn sample_is_bilinear_and_none_outside() {
    let g = grid(2, 2, 1.0, &[0.0, 1.0, 2.0, 3.0]);
    assert_eq!(g.sample((0.5, 0.5)), Some(1.5));
    assert_eq!(g.sample((-0.1, 0.0)), None);
    assert_eq!(g.sample((0.0, 1.1)), None);
    assert_eq!(g.sample((1.0, 1.0)), Some(3.0));
    assert_eq!(g.get(1, 1), 3.0);
}

fn lcg_grid(cols: usize, rows: usize) -> ScalarGrid {
    let mut state = 20_261_121_u64;
    let mut g = ScalarGrid::zeros((0.0, 0.0), 1.0, cols, rows).unwrap();
    for v in &mut g.values {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *v = (state >> 40) as f64 / (1_u64 << 24) as f64;
    }
    g
}

#[test]
fn the_same_grid_gives_the_same_bits_twice() {
    let bits = |paths: Vec<Vec<(f64, f64)>>| -> Vec<Vec<(u64, u64)>> {
        paths
            .into_iter()
            .map(|p| {
                p.into_iter()
                    .map(|v| (v.0.to_bits(), v.1.to_bits()))
                    .collect()
            })
            .collect()
    };
    let first = bits(iso_lines(&lcg_grid(40, 30), 0.5));
    let second = bits(iso_lines(&lcg_grid(40, 30), 0.5));
    assert!(first.len() > 3);
    assert_eq!(first, second);
}

#[test]
fn every_path_has_three_vertices_and_no_repeat() {
    for path in iso_lines(&lcg_grid(40, 30), 0.5) {
        assert!(path.len() >= 3);
        assert_ne!(path.first(), path.last());
    }
}

#[test]
fn zeros_refuses_a_bad_grid() {
    let bad = |origin, cell, cols, rows| ScalarGrid::zeros(origin, cell, cols, rows);
    assert!(matches!(
        bad((0.0, 0.0), 0.0, 2, 2),
        Err(GeometryError::BadParameter { .. })
    ));
    assert!(matches!(
        bad((0.0, 0.0), f64::NAN, 2, 2),
        Err(GeometryError::BadParameter { .. })
    ));
    assert!(matches!(
        bad((0.0, 0.0), -1.0, 2, 2),
        Err(GeometryError::BadParameter { .. })
    ));
    assert!(matches!(
        bad((f64::NAN, 0.0), 1.0, 2, 2),
        Err(GeometryError::BadParameter { .. })
    ));
    assert!(matches!(
        bad((0.0, 0.0), 1.0, 0, 2),
        Err(GeometryError::BadParameter { .. })
    ));
    assert!(matches!(
        bad((0.0, 0.0), 1.0, 2, 0),
        Err(GeometryError::BadParameter { .. })
    ));
}

#[test]
fn nan_is_outside_and_a_malformed_grid_panics() {
    let g = grid(2, 1, 1.0, &[f64::NAN, 1.0]);
    assert_eq!(iso_lines(&g, 0.5).len(), 1);
    let mut broken = grid(2, 2, 1.0, &[0.0; 4]);
    broken.values.pop();
    assert!(std::panic::catch_unwind(|| iso_lines(&broken, 0.5)).is_err());
}

#[test]
fn a_big_grid_of_ones_gives_one_outer_path() {
    let g = grid(200, 200, 1.0, &vec![1.0; 40_000]);
    let paths = iso_lines(&g, 0.5);
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].len(), 4 * 200);
    assert!(signed_area(&paths[0]) > 0.0);
}
