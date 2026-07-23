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

// ---------- Task 4: color under constraint ----------

use hypergraph_rs::raster::{heat, to_ansi256};

#[test]
fn test_heat_endpoints_match_deck_canon() {
    assert_eq!(heat(0.0), Rgb(217, 164, 65)); // gold
    assert_eq!(heat(0.5), Rgb(233, 223, 201)); // bone
    assert_eq!(heat(1.0), Rgb(196, 58, 46)); // crimson
}

#[test]
fn test_heat_clamps_out_of_range() {
    assert_eq!(heat(-1.0), heat(0.0));
    assert_eq!(heat(2.0), heat(1.0));
}

#[test]
fn test_ansi256_quantization_deterministic_and_pinned() {
    assert_eq!(to_ansi256(Rgb(196, 58, 46)), to_ansi256(Rgb(196, 58, 46)));
    // Pin the exact cube index for crimson (computed once, frozen as golden).
    assert_eq!(to_ansi256(Rgb(196, 58, 46)), 167);
    // Grayscale ramp: mid-gray maps into 232..=255.
    assert!((232..=255).contains(&to_ansi256(Rgb(128, 128, 128))));
}

/// A single-cell (1x1, 2x4 subcells), depth-decoupled-from-screen-position
/// camera rig: with `ry=rx=0.0, dist=1.0, fov=90.0` (`f=1`, `aspect=2`),
/// `project()` reduces to `ndc_x = v.x/D*2`, `ndc_y = v.y/D`, `depth=D` for
/// `v = (ndc_x*D/2, ndc_y*D, 1.0-D)` — letting a test pick screen subcell
/// AND depth independently.
fn subcell_cam() -> Camera {
    Camera {
        ry: 0.0,
        rx: 0.0,
        dist: 1.0,
        fov: 90.0,
    }
}

/// `ndc_x` for braille subcell column 0/1 in a 1-wide grid (`w=2`).
fn col_ndc(sub_col: usize) -> f64 {
    if sub_col == 0 {
        -1.0
    } else {
        1.0
    }
}

/// `ndc_y` for braille subcell row 0..3 in a 1-tall grid (`h=4`).
fn row_ndc(sub_row: usize) -> f64 {
    match sub_row {
        0 => 1.0,
        1 => 1.0 / 3.0,
        2 => -1.0 / 3.0,
        _ => -1.0,
    }
}

/// A degenerate (`a==b`) strut at exactly subcell `(sub_col,sub_row)` at
/// depth `d`, colored `color` — writes exactly one subcell.
fn subcell_strut(sub_col: usize, sub_row: usize, d: f64, color: Rgb) -> Strut {
    let ndc_x = col_ndc(sub_col);
    let ndc_y = row_ndc(sub_row);
    let v = Vertex3 {
        x: ndc_x * d / 2.0,
        y: ndc_y * d,
        z: 1.0 - d,
    };
    Strut { a: v, b: v, color }
}

#[test]
fn test_cell_fg_bg_pick_deterministic_tiebreak() {
    // Case A: clean majority (5 vs 3, same depth) -> fg = majority color.
    let a = Rgb(255, 0, 0);
    let b = Rgb(0, 0, 255);
    let mut scene = SceneGraph3D::empty();
    let majority_subcells = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1)]; // 5
    let minority_subcells = [(0, 3), (1, 2), (1, 3)]; // 3
    for &(c, r) in &majority_subcells {
        scene.struts.push(subcell_strut(c, r, 1.0, a));
    }
    for &(c, r) in &minority_subcells {
        scene.struts.push(subcell_strut(c, r, 1.0, b));
    }
    let grid = rasterize(&scene, &subcell_cam(), 1, 1);
    assert_eq!(grid.cells[0].fg, a);
    assert_eq!(grid.cells[0].bg, b);

    // Case B: 4-vs-4 count tie, broken by nearest depth (smaller depth wins).
    let mut scene = SceneGraph3D::empty();
    for r in 0..4usize {
        scene.struts.push(subcell_strut(0, r, 2.0, a)); // farther
        scene.struts.push(subcell_strut(1, r, 1.0, b)); // nearer
    }
    let grid = rasterize(&scene, &subcell_cam(), 1, 1);
    assert_eq!(grid.cells[0].fg, b);
    assert_eq!(grid.cells[0].bg, a);

    // Case C: 4-vs-4 count AND depth tie, broken by fixed scan order
    // ((sub_col,sub_row) in the order (0,0),(1,0),(0,1),(1,1),(0,2),(1,2),
    // (0,3),(1,3)) -> the color occupying the lowest scan index wins.
    let mut scene = SceneGraph3D::empty();
    for r in 0..4usize {
        scene.struts.push(subcell_strut(0, r, 1.0, a)); // scan indices 0,2,4,6
        scene.struts.push(subcell_strut(1, r, 1.0, b)); // scan indices 1,3,5,7
    }
    let grid = rasterize(&scene, &subcell_cam(), 1, 1);
    assert_eq!(grid.cells[0].fg, a);
    assert_eq!(grid.cells[0].bg, b);
}

// ---------- Task 12: Stage 2 — CellGrid text golden frames ----------
//
// Matrix: 3 instruments x 2 cameras (front, 3/4) x 1 size (80x40) x
// TrueColor glyph-floor text (`to_text()` is palette-independent — glyphs
// only — so palette is asserted separately via a small `to_ansi()` golden).
// `cells3d` is off by default, so every combo built from a `cells3d`-gated
// instrument builder (`raster::deck`/`raster::instruments`) is wrapped in a
// per-test `#[cfg(feature = "cells3d")]`; this file stays ungated at the
// crate level so Tasks 1-4's camera/grid/color tests keep compiling and
// running at default features (raster on, cells3d off).

#[cfg(feature = "cells3d")]
fn frame_front_camera() -> Camera {
    Camera {
        ry: 0.0,
        rx: 0.0,
        dist: 4.0,
        fov: 70.0,
    }
}

#[cfg(feature = "cells3d")]
fn frame_three_quarter_camera() -> Camera {
    Camera {
        ry: 0.6,
        rx: 0.3,
        dist: 4.0,
        fov: 70.0,
    }
}

#[cfg(feature = "cells3d")]
fn frame_scene(
    inst: hypergraph_rs::raster::instruments::Instrument,
) -> hypergraph_rs::raster::SceneGraph3D {
    use hypergraph_rs::raster::deck::build_world;
    use hypergraph_rs::raster::instruments::{build, InstrumentOpts};
    let world = build_world(1917);
    build(inst, &world, &InstrumentOpts::default())
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_cylinder_front() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Cylinder);
    assert_eq!(
        rasterize(&scene, &frame_front_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/cylinder_front.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_cylinder_three_quarter() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Cylinder);
    assert_eq!(
        rasterize(&scene, &frame_three_quarter_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/cylinder_3q.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_terrain_front() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Terrain);
    assert_eq!(
        rasterize(&scene, &frame_front_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/terrain_front.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_terrain_three_quarter() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Terrain);
    assert_eq!(
        rasterize(&scene, &frame_three_quarter_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/terrain_3q.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_sankey_front() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Sankey);
    assert_eq!(
        rasterize(&scene, &frame_front_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/sankey_front.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_sankey_three_quarter() {
    use hypergraph_rs::raster::instruments::Instrument;
    let scene = frame_scene(Instrument::Sankey);
    assert_eq!(
        rasterize(&scene, &frame_three_quarter_camera(), 80, 40).to_text(),
        include_str!("fixtures/frames/sankey_3q.txt")
    );
}

#[cfg(feature = "cells3d")]
#[test]
fn test_frame_cylinder_front_ansi_truecolor() {
    use hypergraph_rs::raster::instruments::Instrument;
    use hypergraph_rs::raster::Palette;
    let scene = frame_scene(Instrument::Cylinder);
    assert_eq!(
        rasterize(&scene, &frame_front_camera(), 80, 40).to_ansi(Palette::TrueColor),
        include_str!("fixtures/frames/cylinder_front.ansi.txt")
    );
}
