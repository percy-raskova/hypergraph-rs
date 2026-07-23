#![cfg(feature = "raster")]

use hypergraph_rs::raster::{Cell, CellGrid, Rgb, SceneGraph3D};

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
