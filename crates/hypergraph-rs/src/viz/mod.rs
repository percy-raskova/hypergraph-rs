//! Scene-graph serializer: the matplotlib-free rendering contract (spec §5.3).
//!
//! A `SceneGraph` is a `LayoutResult` enriched with attrs + style, ready for
//! JSON export, SVG emit, or consumption by a WASM/React renderer.

use serde::{Deserialize, Serialize};

/// Re-export so `viz::HyperedgeGeometry` resolves (spec §5.3 places it here).
pub use crate::layout::HyperedgeGeometry;

/// Visual style for a node glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeStyle {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill: "#1f77b4".to_string(),
            stroke: "#000000".to_string(),
            stroke_width: 1.0,
        }
    }
}

/// Visual style for a hyperedge glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperedgeStyle {
    pub fill: String,
    pub stroke: String,
    pub opacity: f64,
}

impl Default for HyperedgeStyle {
    fn default() -> Self {
        Self {
            fill: "#ff7f0e".to_string(),
            stroke: "#ff7f0e".to_string(),
            opacity: 0.4,
        }
    }
}

/// A rendered node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeGlyph {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub attrs: serde_json::Value,
    pub style: NodeStyle,
}

/// A rendered hyperedge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperedgeGlyph {
    pub id: String,
    pub member_ids: Vec<String>,
    pub geometry: HyperedgeGeometry,
    pub attrs: serde_json::Value,
    pub style: HyperedgeStyle,
}

/// The scene-graph: the contract between Rust and every renderer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneGraph {
    pub nodes: Vec<NodeGlyph>,
    pub hyperedges: Vec<HyperedgeGlyph>,
    pub bounding_box: (f64, f64, f64, f64),
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
