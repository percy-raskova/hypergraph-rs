#![cfg(feature = "raster")]

use hypergraph_rs::raster::{Cell, CellGrid, Face, Rgb, SceneGraph3D, Strut, Vertex3};

#[test]
fn test_cell_glyph_floor_always_complete() {
    // Glyph floor: a Cell ALWAYS carries ch+fg+bg (no Option).
    let c = Cell {
        ch: '⠿',
        fg: Rgb(196, 58, 46),
        bg: Rgb(0, 0, 0),
    };
    assert_eq!(c.ch, '⠿');
    assert_eq!(c.fg, Rgb(196, 58, 46));
}

#[test]
fn test_cellgrid_row_major_len_and_text() {
    let g = CellGrid::filled(
        3,
        2,
        Cell {
            ch: ' ',
            fg: Rgb(0, 0, 0),
            bg: Rgb(0, 0, 0),
        },
    );
    assert_eq!(g.cells.len(), 6);
    assert_eq!(g.to_text(), "   \n   "); // 3 cols, 2 rows, glyphs only
}

#[test]
fn test_scenegraph3d_serde_round_trip() {
    let s = SceneGraph3D::empty();
    let j = serde_json::to_string(&s).unwrap();
    let back: SceneGraph3D = serde_json::from_str(&j).unwrap();
    assert_eq!(s, back);
}

#[test]
fn test_fmt6_matches_phase5_formatter() {
    use hypergraph_rs::raster::fmt::fmt6;
    assert_eq!(fmt6(0.4608359187088229), "0.460836");
    assert_eq!(fmt6(-0.0), "0");
    assert_eq!(fmt6(198.3), "198.3");
}

use hypergraph_rs::raster::{depth_sort, project, Camera};

fn cam() -> Camera {
    Camera {
        ry: 0.0,
        rx: 0.0,
        dist: 4.0,
        fov: 70.0,
    }
}

#[test]
fn test_project_center_is_center() {
    let p = project(
        &Vertex3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        &cam(),
        80,
        48,
    );
    // Origin projects to the screen midpoint (subcell space: cols*2 wide, rows*4 tall for braille).
    assert!((p.sx - (80.0 * 2.0 - 1.0) / 2.0).abs() < 1e-9);
    assert!((p.sy - (48.0 * 4.0 - 1.0) / 2.0).abs() < 1e-9);
}

#[test]
fn test_project_nearer_z_has_smaller_depth() {
    let near = project(
        &Vertex3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        &cam(),
        80,
        48,
    );
    let far = project(
        &Vertex3 {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        &cam(),
        80,
        48,
    );
    assert!(near.depth < far.depth);
}

#[test]
fn test_rotation_determinism_bit_identical() {
    let c = Camera {
        ry: 0.7,
        rx: 0.3,
        dist: 4.0,
        fov: 70.0,
    };
    let v = Vertex3 {
        x: 0.5,
        y: -0.2,
        z: 0.9,
    };
    assert_eq!(project(&v, &c, 80, 48).sx, project(&v, &c, 80, 48).sx);
}

#[test]
fn test_depth_sort_stable_on_ties() {
    // Two faces at identical depth keep insertion order (stable sort).
    let v0 = Vertex3 {
        x: -1.0,
        y: -1.0,
        z: 0.0,
    };
    let v1 = Vertex3 {
        x: 1.0,
        y: -1.0,
        z: 0.0,
    };
    let v2 = Vertex3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    let face_a = Face {
        verts: [v0, v1, v2],
        fill: Rgb(1, 0, 0),
        opacity: 1.0,
    };
    let face_b = Face {
        verts: [v0, v1, v2],
        fill: Rgb(2, 0, 0),
        opacity: 1.0,
    };
    let mut faces = vec![face_a.clone(), face_b.clone()];
    let mut struts: Vec<Strut> = Vec::new();
    depth_sort(&mut faces, &mut struts, &cam());
    assert_eq!(faces[0].fill, face_a.fill);
    assert_eq!(faces[1].fill, face_b.fill);
}
