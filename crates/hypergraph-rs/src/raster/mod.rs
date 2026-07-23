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
pub mod ingest;
#[cfg(feature = "cells3d")]
pub mod instruments;
#[cfg(feature = "raster-png")]
pub mod png;

pub use camera::{
    depth_sort, depth_sort_nodes, face_is_clipped, node_is_clipped, project, strut_is_clipped,
    Camera, Projected,
};
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

    /// Overwrite cells with the literal glyphs of `text`, starting at
    /// `(row, col)` and wrapping onto the following rows, clipped at the
    /// grid's last row (review MAJOR 5: the glyph-floor information-parity
    /// constraint is met by stamping labeled scalars into the REAL frame,
    /// not by building a separate banner grid nothing renders). Every
    /// stamped cell still carries `ch+fg+bg` unconditionally. Bounded:
    /// `text.chars().count()`, and never writes past `cells.len()`.
    pub fn stamp_text(&mut self, row: u16, col: u16, text: &str, fg: Rgb, bg: Rgb) {
        let cols = self.cols as usize;
        if cols == 0 {
            return;
        }
        let mut i = row as usize * cols + col as usize;
        for ch in text.chars() {
            if i >= self.cells.len() {
                return;
            }
            self.cells[i] = Cell { ch, fg, bg };
            i += 1;
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

/// The banner's foreground: the deck's own readout ink
/// (`rgba(138,122,97,.9)`, `topology-deck.html:430-431`).
const BANNER_FG: Rgb = Rgb(138, 122, 97);
/// The banner's background — the frame's clear color.
const BANNER_BG: Rgb = Rgb(0, 0, 0);

/// Greedy word-wrap of the labeled-scalar tokens into `cols`-wide rows, one
/// space between tokens, never splitting a token that fits on its own row
/// (an over-long token gets its own row and is clipped by `stamp_text`).
/// Bounded: one pass over `tokens`.
fn wrap_tokens(tokens: &[String], cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for t in tokens {
        if line.is_empty() {
            line.push_str(t);
        } else if line.chars().count() + 1 + t.chars().count() <= cols {
            line.push(' ');
            line.push_str(t);
        } else {
            out.push(core::mem::take(&mut line));
            line.push_str(t);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// Rasterize `scene` under `camera` into a `cols x rows` `CellGrid`
/// (Task 3): near-plane-clip, painter's depth-sort, rasterize faces,
/// struts and node discs into a braille 2x4 subcell buffer via the shared
/// `grid::draw_scene` core, compose glyphs, then stamp the scene's labeled
/// scalars across the top rows as literal glyph cells.
///
/// The banner stamp is the Global Constraint (plan §Global Constraints,
/// PHASE-R-MAP §4): *every labeled scalar an instrument computes must
/// appear as literal text somewhere in the `CellGrid`* — the deck's own
/// `PHI = … r = … — invariant under zoom` conservation banner
/// (`topology-deck.html:430-433`) is the canonical case. A scene with empty
/// `metadata` stamps nothing, so the pure-geometry tests are unaffected.
///
/// Pure function of `(scene, camera, cols, rows)` — byte-exact
/// golden-testable.
pub fn rasterize(scene: &SceneGraph3D, camera: &Camera, cols: u16, rows: u16) -> CellGrid {
    let mut buf = grid::SubBuffer::new(cols as usize * 2, rows as usize * 4);
    grid::draw_scene(
        &mut buf,
        scene,
        camera,
        cols as f64 * 2.0,
        rows as f64 * 4.0,
    );
    let mut out = grid::compose_braille(&buf, cols, rows);

    for (i, line) in wrap_tokens(&scene.labeled_scalars(), cols as usize)
        .iter()
        .enumerate()
    {
        if i >= rows as usize {
            break;
        }
        out.stamp_text(i as u16, 0, line, BANNER_FG, BANNER_BG);
    }
    out
}
