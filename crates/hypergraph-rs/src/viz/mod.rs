//! Scene-graph serializer: the matplotlib-free rendering contract (spec §5.3).
//!
//! A `SceneGraph` is a `LayoutResult` enriched with attrs + style, ready for
//! JSON export, SVG emit, or consumption by a WASM/React renderer.

use serde::{Deserialize, Serialize};

use crate::core::hypergraph::Hypergraph;
use crate::layout::{compute_layout, LayoutStrategy};

pub mod svg;

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

/// Default rendered node radius (layout units).
pub const DEFAULT_NODE_RADIUS: f64 = 0.02;

impl<M> Hypergraph<serde_json::Value, serde_json::Value, M> {
    /// Build a `SceneGraph` from a layout strategy (spec §5.3). Concrete impl:
    /// clones `serde_json::Value` node/edge attrs into the glyphs.
    pub fn render(&self, strategy: LayoutStrategy) -> SceneGraph {
        let layout = compute_layout(self, strategy);

        let nodes: Vec<NodeGlyph> = layout
            .node_positions
            .iter()
            .map(|(id, (x, y))| {
                let attrs = self
                    .node_attrs(id)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                NodeGlyph {
                    id: id.clone(),
                    x: *x,
                    y: *y,
                    radius: DEFAULT_NODE_RADIUS,
                    attrs,
                    style: NodeStyle::default(),
                }
            })
            .collect();

        let hyperedges: Vec<HyperedgeGlyph> = layout
            .edge_geometries
            .iter()
            .map(|(eid, geom)| {
                let attrs = self
                    .edge_attrs(eid)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let member_ids = self.members(eid).unwrap_or_default();
                HyperedgeGlyph {
                    id: eid.clone(),
                    member_ids,
                    geometry: geom.clone(),
                    attrs,
                    style: HyperedgeStyle::default(),
                }
            })
            .collect();

        SceneGraph {
            nodes,
            hyperedges,
            bounding_box: layout.bounding_box,
            metadata: serde_json::Map::new(),
        }
    }

    /// Serialize the rendered scene-graph to JSON (spec §5.3).
    pub fn render_to_json(&self, strategy: LayoutStrategy) -> String {
        let scene = self.render(strategy);
        serde_json::to_string(&scene).unwrap_or_else(|_| "{}".to_string())
    }

    /// Render the scene-graph to a self-contained SVG string (spec §5.3).
    pub fn render_to_svg(&self, strategy: LayoutStrategy) -> String {
        let scene = self.render(strategy);
        svg::scene_to_svg(&scene)
    }
}
