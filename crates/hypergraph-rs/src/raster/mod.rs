//! Deterministic 3D-scene → terminal-text rasterizer (Phase R, PHASE-R-MAP §3/§4).
//!
//! Pipeline: `state → SceneGraph3D → CellGrid` (glyph text). Every public
//! type here is `Debug + Clone + PartialEq + Serialize + Deserialize` (the
//! JSON golden-artifact contract) and every `Cell` is Tier-0 fully
//! informative (`ch + fg + bg` unconditionally — no `Option`).

use serde::{Deserialize, Serialize};

use crate::viz::NodeStyle;

pub mod algebra;
pub mod camera;
pub mod color;
pub mod deck;
pub mod fmt;
mod grid;
#[cfg(feature = "cells3d")]
pub mod h3;
#[cfg(feature = "cells3d")]
pub mod instruments;
#[cfg(feature = "raster-png")]
pub mod png;

pub use camera::{depth_sort, face_is_clipped, project, strut_is_clipped, Camera, Projected};
pub use color::{heat, to_ansi256, Palette};

/// A truecolor RGB triple. No alpha — the glyph floor composes fg/bg
/// unconditionally (Task 4 fills the quantization/heat-ramp logic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A single terminal cell: glyph + foreground + background, always all
/// three present (the glyph floor — PHASE-R-MAP §4 Tier-0 contract).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
}

/// A row-major grid of `Cell`s (`cells.len() == cols*rows`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellGrid {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
}

impl CellGrid {
    /// A grid of `cols * rows` cells, all set to `fill`.
    pub fn filled(cols: u16, rows: u16, fill: Cell) -> Self {
        let n = cols as usize * rows as usize;
        Self {
            cols,
            rows,
            cells: vec![fill; n],
        }
    }

    /// Render `text` as a single-row banner of literal glyph `Cell`s (Task
    /// 13 Property 3 hook: the labeled-scalar text path, independent of the
    /// pixel `rasterize()` path — it never touches a frame golden). Every
    /// cell still carries `ch+fg+bg` unconditionally (glyph floor). Bounded:
    /// `text.chars().count()`.
    pub fn from_banner_text(text: &str) -> Self {
        let cells: Vec<Cell> = text
            .chars()
            .map(|ch| Cell {
                ch,
                fg: Rgb(255, 255, 255),
                bg: Rgb(0, 0, 0),
            })
            .collect();
        let cols = cells.len() as u16;
        Self {
            cols,
            rows: u16::from(!cells.is_empty()),
            cells,
        }
    }

    /// Glyphs only, newline-joined — the diffable byte-exact golden form.
    pub fn to_text(&self) -> String {
        let cols = self.cols as usize;
        self.cells
            .chunks(cols.max(1))
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Glyphs with fg/bg ANSI escapes under `palette` (Task 4): `\x1b[0m`
    /// reset, then per-cell `\x1b[38;2;r;g;bm`/`\x1b[48;2;r;g;bm` (truecolor)
    /// or `\x1b[38;5;{n}m`/`\x1b[48;5;{n}m` (256, via `color::to_ansi256`,
    /// quantized after the fg/bg pick — pick in truecolor, degrade last),
    /// followed by the glyph.
    pub fn to_ansi(&self, palette: Palette) -> String {
        let cols = self.cols as usize;
        let mut out = String::from("\x1b[0m");
        for (i, cell) in self.cells.iter().enumerate() {
            if i > 0 && i % cols.max(1) == 0 {
                out.push('\n');
            }
            match palette {
                Palette::TrueColor => {
                    out.push_str(&format!(
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
                        cell.fg.0, cell.fg.1, cell.fg.2, cell.bg.0, cell.bg.1, cell.bg.2
                    ));
                }
                Palette::Ansi256 => {
                    out.push_str(&format!(
                        "\x1b[38;5;{}m\x1b[48;5;{}m",
                        to_ansi256(cell.fg),
                        to_ansi256(cell.bg)
                    ));
                }
            }
            out.push(cell.ch);
        }
        out
    }
}

/// A point in the 3D scene, prior to camera projection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vertex3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A rendered node lifted into 3D (mirrors `viz::NodeGlyph`, `viz/mod.rs:52-61`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node3 {
    pub id: String,
    pub pos: Vertex3,
    pub radius: f64,
    pub color: Rgb,
    pub style: NodeStyle,
    pub attrs: serde_json::Value,
}

/// A triangle face for the scanline fill (Task 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Face {
    pub verts: [Vertex3; 3],
    pub fill: Rgb,
    pub opacity: f64,
}

/// A line strut for the Bresenham line walk (Task 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Strut {
    pub a: Vertex3,
    pub b: Vertex3,
    pub color: Rgb,
}

/// The 3D scene graph — mirrors `viz::SceneGraph` (`viz/mod.rs:74-80`) lifted
/// to `(x,y,z)` with triangle faces and line struts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneGraph3D {
    pub nodes: Vec<Node3>,
    pub faces: Vec<Face>,
    pub struts: Vec<Strut>,
    pub bounding_box: (f64, f64, f64, f64, f64, f64),
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl SceneGraph3D {
    /// An empty scene graph (all collections empty, zero bbox).
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            faces: Vec::new(),
            struts: Vec::new(),
            bounding_box: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            metadata: serde_json::Map::new(),
        }
    }

    /// Every scalar (number) value in `metadata`, walked recursively through
    /// nested objects/arrays, formatted as `"dotted.path=value"` (floats via
    /// `fmt::fmt6`, the golden-fixture float formatter — Task 13 Property 3
    /// hook: "SceneGraph3D labeled-scalar accessor"). `metadata` is a
    /// `serde_json::Map` (BTreeMap-backed, no `preserve_order` feature) so
    /// iteration order is key-sorted and deterministic, never `HashMap`
    /// order. Bounded: total scalar count in `metadata`, a small fixed tree
    /// built by the instrument builders.
    pub fn labeled_scalars(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, v) in &self.metadata {
            collect_scalars(v, k, &mut out);
        }
        out
    }
}

/// Recursive walk backing [`SceneGraph3D::labeled_scalars`]. Bounded by the
/// metadata JSON tree's own (small, fixed) size.
fn collect_scalars(v: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Number(n) => {
            let text = match n.as_f64() {
                Some(f) if !n.is_i64() && !n.is_u64() => fmt::fmt6(f),
                _ => n.to_string(),
            };
            out.push(format!("{prefix}={text}"));
        }
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                collect_scalars(val, &format!("{prefix}.{k}"), out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                collect_scalars(val, &format!("{prefix}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// Rasterize `scene` under `camera` into a `cols x rows` `CellGrid`
/// (Task 3): near-plane-clip, painter's depth-sort, rasterize into a
/// braille 2x4 subcell buffer, then compose glyphs. Pure function of
/// `(scene, camera, cols, rows)` — byte-exact golden-testable.
pub fn rasterize(scene: &SceneGraph3D, camera: &Camera, cols: u16, rows: u16) -> CellGrid {
    let mut faces: Vec<Face> = scene
        .faces
        .iter()
        .filter(|f| !face_is_clipped(f, camera))
        .cloned()
        .collect();
    let mut struts: Vec<Strut> = scene
        .struts
        .iter()
        .filter(|s| !strut_is_clipped(s, camera))
        .cloned()
        .collect();
    depth_sort(&mut faces, &mut struts, camera);

    let w = cols as usize * 2;
    let h = rows as usize * 4;
    let mut buf = grid::SubBuffer::new(w, h);

    for f in &faces {
        let p0 = project(&f.verts[0], camera, cols, rows);
        let p1 = project(&f.verts[1], camera, cols, rows);
        let p2 = project(&f.verts[2], camera, cols, rows);
        grid::fill_triangle(&mut buf, [p0, p1, p2], f.fill);
    }
    for s in &struts {
        let a = project(&s.a, camera, cols, rows);
        let b = project(&s.b, camera, cols, rows);
        grid::line_bresenham(&mut buf, a, b, s.color);
    }

    grid::compose_braille(&buf, cols, rows)
}
