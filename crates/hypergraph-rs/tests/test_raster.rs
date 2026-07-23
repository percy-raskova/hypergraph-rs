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

// --- Task 3: cell rasterizer -------------------------------------------

use hypergraph_rs::raster::rasterize;

#[test]
fn test_single_subdot_maps_to_correct_braille() {
    // cols=1,rows=1 => 2x4 subcell buffer (one terminal cell). Under `cam()`
    // (ry=0,rx=0,dist=4,fov=70) a vertex at (-2.0, 2.0, 0.0) projects to
    // sx≈-0.214 (clamps/rounds to subcell col0) and sy≈0.429 (rounds to
    // subcell row0) — see derivation in the task notes. A degenerate strut
    // (a==b) lights only that one subcell => bit 0x01 => glyph U+2801 '⠁'.
    let v = Vertex3 {
        x: -2.0,
        y: 2.0,
        z: 0.0,
    };
    let mut scene = SceneGraph3D::empty();
    scene.struts.push(Strut {
        a: v,
        b: v,
        color: Rgb(255, 255, 255),
    });
    let grid = rasterize(&scene, &cam(), 1, 1);
    assert_eq!(grid.cells.len(), 1);
    assert_eq!(grid.cells[0].ch, '\u{2801}');
    assert_eq!(grid.cells[0].fg, Rgb(255, 255, 255));
}

#[test]
fn test_bresenham_line_is_integer_deterministic() {
    let mut scene = SceneGraph3D::empty();
    scene.struts.push(Strut {
        a: Vertex3 {
            x: -1.0,
            y: -1.0,
            z: 0.0,
        },
        b: Vertex3 {
            x: 1.0,
            y: 1.0,
            z: 0.0,
        },
        color: Rgb(10, 20, 30),
    });
    let run1 = rasterize(&scene, &cam(), 20, 10).to_text();
    let run2 = rasterize(&scene, &cam(), 20, 10).to_text();
    assert_eq!(run1, run2);
    // The diagonal strut must have lit at least one subcell (non-blank grid).
    assert!(run1.chars().any(|c| c != '\u{2800}'));
}

#[test]
fn test_triangle_fill_top_left_rule_no_double_shade() {
    // Two triangles sharing an edge (the diagonal of a screen-filling quad)
    // together cover every subcell of a 1x1-cell (2x4 subcell) grid, under
    // `cam()`. If the top-left rule leaves a gap along the shared edge, the
    // resulting glyph will be missing bits from the full block U+28FF '⣿'.
    //
    // Corners are solved analytically (not decimal-approximated) from the
    // exact `project()` formula (camera.rs: `f = 1/tan(fov*pi/360)`,
    // `aspect = height/width`, `dist=4`, `z=dist` at world z=0) so the
    // projected subcell coords land within f64-rounding (~1e-15) of the
    // exact integer corners 0/1 (x) and 0/3 (y) — not ~1e-4 off, which
    // would spuriously push a corner sample outside the triangle.
    let f = 1.0 / (70.0_f64 * core::f64::consts::PI / 360.0).tan();
    let aspect = 4.0 / 2.0; // height/width for cols=1,rows=1 (h=4,w=2)
    let z = 4.0; // dist - world_z, world_z=0, dist=4
    let x_mag = 2.0 * z / (f * aspect); // ndc_x = +-1 => sx = 1/0
    let y_mag = 2.0 * z / f; // ndc_y = +-1 => sy = 0/3
    let tl = Vertex3 {
        x: -x_mag,
        y: y_mag,
        z: 0.0,
    };
    let tr = Vertex3 {
        x: x_mag,
        y: y_mag,
        z: 0.0,
    };
    let bl = Vertex3 {
        x: -x_mag,
        y: -y_mag,
        z: 0.0,
    };
    let br = Vertex3 {
        x: x_mag,
        y: -y_mag,
        z: 0.0,
    };
    let mut scene = SceneGraph3D::empty();
    scene.faces.push(Face {
        verts: [tl, tr, bl],
        fill: Rgb(200, 200, 200),
        opacity: 1.0,
    });
    scene.faces.push(Face {
        verts: [tr, br, bl],
        fill: Rgb(200, 200, 200),
        opacity: 1.0,
    });
    let grid = rasterize(&scene, &cam(), 1, 1);
    assert_eq!(grid.cells[0].ch, '\u{28FF}', "expected full block, got glyph missing subcells along the shared diagonal");
}

#[test]
fn test_depth_buffer_occludes_farther_primitive() {
    // Two full-grid-covering triangles at different depths, same footprint;
    // the nearer (z=+1, smaller camera-space depth) must win every subcell
    // regardless of scene insertion order.
    let tl = Vertex3 {
        x: -14.004,
        y: 28.009,
        z: -10.0,
    };
    let tr = Vertex3 {
        x: 14.004,
        y: 28.009,
        z: -10.0,
    };
    let apex = Vertex3 {
        x: 0.0,
        y: -28.009,
        z: -10.0,
    };
    let far = Face {
        verts: [tl, tr, apex],
        fill: Rgb(0, 255, 0),
        opacity: 1.0,
    };
    let near = Face {
        verts: [
            Vertex3 { z: 1.0, ..tl },
            Vertex3 { z: 1.0, ..tr },
            Vertex3 { z: 1.0, ..apex },
        ],
        fill: Rgb(255, 0, 0),
        opacity: 1.0,
    };
    let mut scene = SceneGraph3D::empty();
    scene.faces.push(far.clone());
    scene.faces.push(near.clone());
    let grid = rasterize(&scene, &cam(), 1, 1);
    assert_eq!(grid.cells[0].fg, Rgb(255, 0, 0));

    // Insertion order reversed must give the same occlusion result.
    let mut scene2 = SceneGraph3D::empty();
    scene2.faces.push(near);
    scene2.faces.push(far);
    let grid2 = rasterize(&scene2, &cam(), 1, 1);
    assert_eq!(grid2.cells[0].fg, Rgb(255, 0, 0));
}

#[test]
fn test_frame_is_pure_function_byte_exact() {
    let mut scene = SceneGraph3D::empty();
    scene.struts.push(Strut {
        a: Vertex3 {
            x: -1.0,
            y: 0.5,
            z: 0.0,
        },
        b: Vertex3 {
            x: 1.0,
            y: -0.5,
            z: 0.0,
        },
        color: Rgb(5, 6, 7),
    });
    let c = cam();
    assert_eq!(
        rasterize(&scene, &c, 40, 20).to_text(),
        rasterize(&scene, &c, 40, 20).to_text()
    );
}
