//! Deterministic 3D-scene → terminal-text rasterizer (Phase R, PHASE-R-MAP §3/§4).
//!
//! Pipeline: `state → SceneGraph3D → CellGrid` (glyph text). Every public
//! type here is `Debug + Clone + PartialEq + Serialize + Deserialize` (the
//! JSON golden-artifact contract) and every `Cell` is Tier-0 fully
//! informative (`ch + fg + bg` unconditionally — no `Option`).

use serde::{Deserialize, Serialize};

use crate::viz::NodeStyle;

pub mod fmt;

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

    /// Glyphs only, newline-joined — the diffable byte-exact golden form.
    pub fn to_text(&self) -> String {
        let cols = self.cols as usize;
        self.cells
            .chunks(cols.max(1))
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Glyphs with truecolor fg/bg ANSI escapes. Task 4 fills in the real
    /// escape sequences; this is a plain glyph fallback until then.
    pub fn to_ansi(&self) -> String {
        self.to_text()
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
}
