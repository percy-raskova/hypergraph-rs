use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use hypergraph_rs::geometry::Point;
use hypergraph_rs_bevy::{FieldError, GroundForm, GroundTopology, RegionShape};

fn square(id: &str, dx: f64) -> RegionShape {
    let p = |x: f64, y: f64| -> Point { (x + dx, y) };
    RegionShape {
        id: id.to_string(),
        triangles: vec![
            [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
            [p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
        ],
        rings: vec![vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]],
    }
}

fn two() -> Vec<RegionShape> {
    vec![square("a", 0.0), square("b", 1.0)]
}

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

fn colours(mesh: &Mesh) -> Vec<[f32; 4]> {
    match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(v)) => v.clone(),
        other => panic!("colours are not Float32x4: {other:?}"),
    }
}

fn bits3(v: &[[f32; 3]]) -> Vec<[u32; 3]> {
    v.iter().map(|p| p.map(f32::to_bits)).collect()
}

fn bits4(v: &[[f32; 4]]) -> Vec<[u32; 4]> {
    v.iter().map(|p| p.map(f32::to_bits)).collect()
}

const WHITE: LinearRgba = LinearRgba::WHITE;

#[test]
fn blocks_rise_and_sink_with_walls() {
    let topo = GroundTopology::new(&two(), GroundForm::Blocks).unwrap();
    let mesh = topo.body_mesh(&[2.0, -1.0], &[WHITE, WHITE]);
    let p = positions(&mesh);
    assert_eq!(p.len(), 60);
    assert!(p[..6].iter().all(|v| v[1] == 2.0));
    assert!(p[30..36].iter().all(|v| v[1] == -1.0));
    assert!(p[36..].iter().all(|v| v[1] == 0.0 || v[1] == -1.0));
}

#[test]
fn the_surface_meets_at_shared_corners() {
    let topo = GroundTopology::new(&two(), GroundForm::Surface).unwrap();
    let mesh = topo.body_mesh(&[2.0, -1.0], &[WHITE, WHITE]);
    let p = positions(&mesh);
    assert_eq!(p.len(), 36);
    let mut seen_shared = 0;
    for v in &p {
        // scene (x, h, -y)
        if v[0] == 1.0 && (v[2] == 0.0 || v[2] == -1.0) {
            assert_eq!(v[1], 0.5, "{v:?}");
            seen_shared += 1;
        }
        if v[0] == 0.0 && v[2] == 0.0 {
            assert_eq!(v[1], 2.0, "{v:?}");
        }
    }
    assert!(seen_shared > 0);
    // a's centroids: first 18 vertices are a; centroid is the third of each 3
    for k in 0..6 {
        assert_eq!(p[3 * k + 2][1], 2.0);
    }
}

#[test]
fn both_forms_have_the_same_outline_in_plan() {
    let heights = [2.0, -1.0];
    let blocks = GroundTopology::new(&two(), GroundForm::Blocks).unwrap();
    let surface = GroundTopology::new(&two(), GroundForm::Surface).unwrap();
    let a = positions(&blocks.outline_mesh(&heights, 0.004));
    let b = positions(&surface.outline_mesh(&heights, 0.004));
    assert_eq!(a.len(), 16);
    assert_eq!(a.len(), b.len());
    for (u, v) in a.iter().zip(&b) {
        assert_eq!(u[0].to_bits(), v[0].to_bits());
        assert_eq!(u[2].to_bits(), v[2].to_bits());
    }
}

#[test]
fn each_region_has_its_own_colour() {
    let red = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
    let blue = LinearRgba::new(0.0, 0.0, 1.0, 1.0);
    let topo = GroundTopology::new(&two(), GroundForm::Blocks).unwrap();
    let mesh = topo.body_mesh(&[1.0, 1.0], &[red, blue]);
    let c = colours(&mesh);
    assert_eq!(c.len(), 60);
    assert!(c[..30].iter().all(|v| *v == [1.0, 0.0, 0.0, 1.0]));
    assert!(c[30..].iter().all(|v| *v == [0.0, 0.0, 1.0, 1.0]));
}

#[test]
fn writing_in_place_equals_building_anew() {
    let red = LinearRgba::new(1.0, 0.0, 0.0, 1.0);
    let blue = LinearRgba::new(0.0, 0.0, 1.0, 1.0);
    for form in [GroundForm::Surface, GroundForm::Blocks] {
        let topo = GroundTopology::new(&two(), form).unwrap();
        let mut m = topo.body_mesh(&[1.0, 0.5], &[red, blue]);
        topo.write_body(&mut m, &[2.0, -1.0], &[blue, red]);
        let fresh = topo.body_mesh(&[2.0, -1.0], &[blue, red]);
        assert_eq!(bits3(&positions(&m)), bits3(&positions(&fresh)));
        assert_eq!(bits3(&normals(&m)), bits3(&normals(&fresh)));
        assert_eq!(bits4(&colours(&m)), bits4(&colours(&fresh)));

        let mut o = topo.outline_mesh(&[1.0, 0.5], 0.004);
        topo.write_outline(&mut o, &[2.0, -1.0], 0.004);
        let fresh = topo.outline_mesh(&[2.0, -1.0], 0.004);
        assert_eq!(bits3(&positions(&o)), bits3(&positions(&fresh)));
    }
}

#[test]
fn region_lookup_is_by_id() {
    let topo = GroundTopology::new(&two(), GroundForm::Surface).unwrap();
    assert_eq!(topo.region_index("b"), Some(1));
    assert_eq!(topo.region_index("z"), None);
    assert_eq!(topo.region_count(), 2);
    assert_eq!(topo.form(), GroundForm::Surface);
}

#[test]
fn bad_shapes_are_refused() {
    let err =
        GroundTopology::new(&[square("a", 0.0), square("a", 1.0)], GroundForm::Blocks).unwrap_err();
    assert_eq!(err, FieldError::DuplicateId("a".to_string()));

    let mut nan = square("a", 0.0);
    nan.triangles[0][0].0 = f64::NAN;
    let err = GroundTopology::new(&[nan], GroundForm::Blocks).unwrap_err();
    assert_eq!(
        err,
        FieldError::NonFinite {
            item: "a".to_string()
        }
    );

    let mut empty = square("a", 0.0);
    empty.triangles.clear();
    let err = GroundTopology::new(&[empty], GroundForm::Surface).unwrap_err();
    assert_eq!(
        err,
        FieldError::BadShape {
            region: "a".to_string(),
            problem: "no triangles"
        }
    );
}

#[test]
fn a_grid_of_3168_regions_builds_in_linear_time() {
    let mut shapes = Vec::new();
    for row in 0..44 {
        for col in 0..72 {
            let (x, y) = (f64::from(col), f64::from(row));
            let p = |dx: f64, dy: f64| -> Point { (x + dx, y + dy) };
            shapes.push(RegionShape {
                id: format!("r{row}-{col}"),
                triangles: vec![
                    [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
                    [p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
                ],
                rings: vec![vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]],
            });
        }
    }
    let topo = GroundTopology::new(&shapes, GroundForm::Surface).unwrap();
    let heights = vec![1.0_f32; 3168];
    let cols = vec![WHITE; 3168];
    let mesh = topo.body_mesh(&heights, &cols);
    assert_eq!(mesh.count_vertices(), 3168 * 18);
}
