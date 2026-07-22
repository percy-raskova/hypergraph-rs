//! Hand-rolled minimal SVG emitter for a `SceneGraph` (no external SVG crate).
//!
//! Coordinates are the layout's own units mapped through a `viewBox` with a
//! small margin. Note: SVG's y-axis grows downward while layout y grows upward;
//! consumers that need a "math" orientation can flip y — we emit layout coords
//! directly for a lossless round-trip with the scene-graph JSON.

use super::{HyperedgeStyle, SceneGraph};
use crate::layout::HyperedgeGeometry;

/// Emit a self-contained SVG document for a scene-graph.
pub fn scene_to_svg(scene: &SceneGraph) -> String {
    let (min_x, min_y, max_x, max_y) = scene.bounding_box;
    let span = (max_x - min_x).max(max_y - min_y).max(1.0);
    let margin = 0.1 * span;
    let vx = min_x - margin;
    let vy = min_y - margin;
    let vw = ((max_x - min_x) + 2.0 * margin).max(1.0);
    let vh = ((max_y - min_y) + 2.0 * margin).max(1.0);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">\n",
        fmt(vx),
        fmt(vy),
        fmt(vw),
        fmt(vh)
    ));

    // Hyperedges first (drawn under nodes).
    for he in &scene.hyperedges {
        let g = geometry_to_svg(&he.geometry, &he.style);
        if !g.is_empty() {
            s.push_str(&g);
            s.push('\n');
        }
    }
    // Nodes on top.
    for n in &scene.nodes {
        s.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />\n",
            fmt(n.x),
            fmt(n.y),
            fmt(n.radius),
            escape(&n.style.fill),
            escape(&n.style.stroke),
            fmt(n.style.stroke_width)
        ));
    }
    s.push_str("</svg>\n");
    s
}

fn geometry_to_svg(geom: &HyperedgeGeometry, style: &HyperedgeStyle) -> String {
    match geom {
        HyperedgeGeometry::ConvexHull { points }
        | HyperedgeGeometry::Spline { control_points: points } => {
            if points.is_empty() {
                return String::new();
            }
            let pts: String = points
                .iter()
                .map(|(x, y)| format!("{},{}", fmt(*x), fmt(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "  <polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" fill-opacity=\"{}\" />",
                pts,
                escape(&style.fill),
                escape(&style.stroke),
                fmt(style.opacity)
            )
        }
        HyperedgeGeometry::Stripe { x, y_min, y_max } => format!(
            "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-opacity=\"{}\" />",
            fmt(*x),
            fmt(*y_min),
            fmt(*x),
            fmt(*y_max),
            escape(&style.stroke),
            fmt(style.opacity)
        ),
        HyperedgeGeometry::Arc { center, radius, angle_range } => {
            let (a0, a1) = *angle_range;
            let (cx, cy) = *center;
            let x0 = cx + radius * a0.cos();
            let y0 = cy + radius * a0.sin();
            let x1 = cx + radius * a1.cos();
            let y1 = cy + radius * a1.sin();
            let large = if (a1 - a0).abs() > std::f64::consts::PI { 1 } else { 0 };
            format!(
                "  <path d=\"M {} {} A {} {} 0 {} 1 {} {}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{}\" />",
                fmt(x0),
                fmt(y0),
                fmt(*radius),
                fmt(*radius),
                large,
                fmt(x1),
                fmt(y1),
                escape(&style.stroke),
                fmt(style.opacity)
            )
        }
    }
}

/// Deterministic fixed-precision formatting (6 decimals, trailing zeros
/// trimmed, `-0` normalised to `0`).
fn fmt(v: f64) -> String {
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s
    }
}

/// Escape XML special characters in attribute values.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
