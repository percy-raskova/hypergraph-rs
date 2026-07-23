//! Instrument II — the contradiction terrain (`topology-deck.html:338-359`,
//! Task 8). Per-agent sub-subsistence tension is IDW-interpolated into a
//! heightfield over a fixed quad grid; agents are drawn on top at the same
//! `height()` evaluated at their spectral `(x,y)`, colored by bloc.

use serde_json::json;

use crate::raster::color::heat;
use crate::raster::deck::{Agent, DeckWorld};
use crate::raster::{Face, Node3, Rgb, SceneGraph3D, Vertex3};
use crate::viz::NodeStyle;

use super::cylinder::spectral_positions;
use super::InstrumentOpts;

/// Quad grid resolution along x (`topology-deck.html:339`). `(N+1)`
/// vertices per row.
pub const N: usize = 34;
/// Quad grid resolution along z (`:339`). `(M+1)` vertices per column.
pub const M: usize = 24;
/// World extent: `x,z ∈ [-S,S]` (`:339`).
pub const S: f64 = 1.35;
/// IDW smoothing epsilon, folded into `d²` before the reciprocal
/// (`:344-345`). Scout-verified to MATERIALLY SMOOTH the field — carried
/// exactly, not a render knob (plan Task 8 Interfaces).
pub const EPS: f64 = 0.02;

/// `a.bloc===0?'#F2C56B':a.bloc===1?'#E9DFC9':'#E85A44'` (`:355`).
const BLOC_COLORS: [Rgb; 3] = [Rgb(242, 197, 107), Rgb(233, 223, 201), Rgb(232, 90, 68)];

/// `t = max(0, repression * (subsistence - wealth))` (`:340-341`) — only
/// sub-subsistence agents (`wealth < subsistence`) contribute tension.
pub fn agent_tension(a: &Agent, repression: f64) -> f64 {
    (repression * (a.subsistence - a.wealth)).max(0.0)
}

/// One IDW source: a tension value `t` sitting at world-space `(x,y)`.
#[derive(Debug, Clone, Copy)]
pub struct TensionNode {
    pub x: f64,
    pub y: f64,
    pub t: f64,
}

/// `height(x,y)` (`:344-345`): IDW-weighted mean tension at `(x,y)`,
/// normalized by `tmax`. `EPS` is folded into `d²` before the reciprocal, so
/// the field is always finite — no singularity when evaluated AT a node's
/// own position (Step 1's epsilon test).
pub fn idw_height(x: f64, y: f64, nodes: &[TensionNode], tmax: f64) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    for n in nodes {
        let d2 = (x - n.x).powi(2) + (y - n.y).powi(2) + EPS;
        num += n.t / d2;
        den += 1.0 / d2;
    }
    (num / den) / tmax
}

/// Documented seam (PHASE-R-MAP §3, plan Task 8 Interfaces): where a
/// community-link edge spans two spectral clusters (a "bridge"), babylon's
/// real per-edge Ollivier-Ricci curvature gradients raise the local
/// heightfield here. `DeckWorld` carries no bridge-curvature data and the
/// deck fixture's own cliffs already emerge from the IDW tension field
/// alone, so this is a no-op today — it exists purely as the slot-in point
/// a future task widens (no new formalism is introduced by Phase R).
pub(crate) fn bridge_cliff(_heightfield: &mut [Vec<f64>], _world: &DeckWorld) {}

/// Build Instrument II (`topology-deck.html:338-359`).
pub fn build_terrain(world: &DeckWorld, _opts: &InstrumentOpts) -> SceneGraph3D {
    let pos = spectral_positions(world);

    // Bounded: one tension node per agent (`world.agents.len()`).
    let tension_nodes: Vec<TensionNode> = world
        .agents
        .iter()
        .map(|a| {
            let (x, y) = pos[a.id];
            TensionNode {
                x,
                y,
                t: agent_tension(a, world.repression),
            }
        })
        .collect();
    let tmax = tension_nodes
        .iter()
        .map(|n| n.t)
        .fold(0.0_f64, f64::max)
        .max(0.001);

    // `(N+1)x(M+1)` vertex t-values, row-major `i in 0..=N`, `j in 0..=M`
    // (bounded: fixed N,M constants).
    let mut t_grid: Vec<Vec<f64>> = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let mut row = Vec::with_capacity(M + 1);
        for j in 0..=M {
            let x = (i as f64 / N as f64 * 2.0 - 1.0) * S;
            let z = (j as f64 / M as f64 * 2.0 - 1.0) * S;
            row.push(idw_height(x / S, z / S, &tension_nodes, tmax));
        }
        t_grid.push(row);
    }
    bridge_cliff(&mut t_grid, world);

    let vertex = |i: usize, j: usize| -> (Vertex3, f64) {
        let x = (i as f64 / N as f64 * 2.0 - 1.0) * S;
        let z = (j as f64 / M as f64 * 2.0 - 1.0) * S;
        let t = t_grid[i][j];
        (
            Vertex3 {
                x,
                y: -0.55 + t * 0.95,
                z,
            },
            t,
        )
    };

    // Bounded: N*M quads, 2 triangle faces each.
    let mut faces = Vec::with_capacity(N * M * 2);
    for i in 0..N {
        for j in 0..M {
            let (v00, t00) = vertex(i, j);
            let (v10, _) = vertex(i + 1, j);
            let (v11, t11) = vertex(i + 1, j + 1);
            let (v01, _) = vertex(i, j + 1);
            // Per-quad t = mean of the two diagonal corners (`:347`).
            let t = (t00 + t11) / 2.0;
            let fill = heat(t);
            let opacity = 0.16 + t * 0.55;
            faces.push(Face {
                verts: [v00, v10, v11],
                fill,
                opacity,
            });
            faces.push(Face {
                verts: [v00, v11, v01],
                fill,
                opacity,
            });
        }
    }

    // Bounded: one node per agent.
    let mut nodes = Vec::with_capacity(world.agents.len());
    for a in &world.agents {
        let (x, y) = pos[a.id];
        let t = idw_height(x, y, &tension_nodes, tmax);
        // `proj({x:a.x*S, y:-.53+t*.95, z:a.y*S})` (`:356`) — the height
        // field is sampled in the UNSCALED spectral frame (`height(a.x,a.y)`,
        // matching the grid's own `height(x/S, z/S)`), but the agent is
        // PLACED in the same `±S` world frame the quad grid spans. Review
        // MAJOR 1: dropping `*S` left the agents in a 1/S-shrunk frame,
        // geometrically decoupled from the terrain they annotate.
        nodes.push(Node3 {
            id: format!("agent-{}", a.id),
            pos: Vertex3 {
                x: x * S,
                y: -0.53 + t * 0.95,
                z: y * S,
            },
            radius: 0.024,
            color: BLOC_COLORS[a.bloc as usize],
            style: NodeStyle::default(),
            attrs: json!({ "agent_id": a.id, "bloc": a.bloc, "t": t }),
        });
    }

    let mut metadata = serde_json::Map::new();
    metadata.insert("grid_vertices".to_string(), json!((N + 1) * (M + 1)));
    metadata.insert("extent".to_string(), json!(S));
    metadata.insert("tmax".to_string(), json!(tmax));

    SceneGraph3D {
        nodes,
        faces,
        struts: Vec::new(),
        bounding_box: (-S, -0.55, -S, S, 0.4, S),
        metadata,
    }
}
