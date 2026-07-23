//! Instrument I — the adjoint cylinder (`topology-deck.html:294-336`, Task
//! 7). Bottom disc: every agent as a lone point (the atomized class
//! in-itself) at its spectral `(x,y)` position. Each community's hull
//! floats at height `-H/2 + o(C)*H`; ghost-edge struts run from every
//! member's bottom-disc point to its community-height point.

use std::collections::HashMap;

use serde_json::json;

use crate::communities::{normalized_laplacian, smallest_k_eigenvectors};
use crate::layout::convex_hull;
use crate::raster::color::heat;
use crate::raster::deck::{deck_hypergraph, org_degree, Community, DeckWorld};
use crate::raster::{Face, Node3, Rgb, SceneGraph3D, Strut, Vertex3};
use crate::viz::NodeStyle;

use super::{CylinderHeight, InstrumentOpts};

/// The cylinder's radius (`topology-deck.html:295`).
pub const R: f64 = 1.05;
/// The cylinder's full height, bottom disc to top disc (`:295`).
pub const H: f64 = 1.55;
/// Pinned seed for `smallest_k_eigenvectors` (Finding 6 — never `None`).
/// Derived the same way the deck derives its own spectral-init seed
/// (`W.seed ^ 0x5EED` = `1917 ^ 24301` = `22928`, `topology-deck.html:213`)
/// for continuity of intent; Deviation 2 means this does NOT reproduce the
/// deck's spectral positions bit-for-bit (different generator/algorithm),
/// only that the Rust output is itself deterministic and self-golden.
pub const SPECTRAL_SEED: u64 = 22_928;

const AGENT_RADIUS: f64 = 0.03;
const COMMUNITY_NODE_RADIUS: f64 = 0.04;
const GHOST_STRUT_COLOR: Rgb = Rgb(217, 164, 65);

/// `o(C)` under `opts.cylinder_height` (D-R1).
fn community_height_scalar(c: &Community, opts: &InstrumentOpts) -> f64 {
    match opts.cylinder_height {
        CylinderHeight::OrgDegree => org_degree(c),
        CylinderHeight::HullFill => {
            let n = c.members.len();
            if n < 2 {
                1.0
            } else {
                let max_links = (n * (n - 1) / 2) as f64;
                (c.links.len() as f64 / max_links).clamp(0.0, 1.0)
            }
        }
    }
}

/// Every agent's spectral `(x,y)` disc position, indexed by `Agent.id`.
/// Reuses the shared `deck_hypergraph` adapter (Finding 6) + the crate's
/// pinned-seed eigensolver (Deviation 2: structural-parity, not deck-byte-
/// exact). `pub(crate)` so Task 8's terrain builder (which needs the same
/// per-agent `(x,y)` the deck's `drawTerrain` reads off `a.x`/`a.y`) reuses
/// it rather than re-deriving spectral positions a second time (DRY).
pub(crate) fn spectral_positions(world: &DeckWorld) -> Vec<(f64, f64)> {
    let hg = deck_hypergraph(world);
    let (l, ids) = normalized_laplacian(&hg)
        .expect("deck_hypergraph guarantees every agent has degree >= 1 (Task 7 deviation)");
    let ev = smallest_k_eigenvectors(&l, 2, Some(SPECTRAL_SEED));

    let mut row_of: HashMap<String, usize> = HashMap::new();
    for (row, id) in ids.iter().enumerate() {
        row_of.insert(id.clone(), row);
    }

    world
        .agents
        .iter()
        .map(|a| {
            let row = row_of[&a.id.to_string()];
            (ev[[row, 0]], ev[[row, 1]])
        })
        .collect()
}

/// Fan-triangulate a (possibly non-triangular) convex hull into `Face`s at
/// world-space height `y`. Bounded by `hull.len()` (<= community size).
fn hull_faces(hull: &[(f64, f64)], y: f64, fill: Rgb, opacity: f64) -> Vec<Face> {
    if hull.len() < 3 {
        return Vec::new();
    }
    let v0 = Vertex3 {
        x: hull[0].0,
        y,
        z: hull[0].1,
    };
    let mut faces = Vec::with_capacity(hull.len() - 2);
    for i in 1..hull.len() - 1 {
        let v1 = Vertex3 {
            x: hull[i].0,
            y,
            z: hull[i].1,
        };
        let v2 = Vertex3 {
            x: hull[i + 1].0,
            y,
            z: hull[i + 1].1,
        };
        faces.push(Face {
            verts: [v0, v1, v2],
            fill,
            opacity,
        });
    }
    faces
}

/// Build Instrument I (`topology-deck.html:294-336`).
pub fn build_cylinder(world: &DeckWorld, opts: &InstrumentOpts) -> SceneGraph3D {
    let pos = spectral_positions(world);

    let mut nodes = Vec::with_capacity(world.agents.len());
    let mut faces = Vec::new();
    let mut struts = Vec::new();
    let mut community_labels = Vec::with_capacity(world.communities.len());

    // Bottom disc: every agent as a lone point (Power-of-10: bounded by
    // agents.len()).
    for a in &world.agents {
        let (sx, sy) = pos[a.id];
        nodes.push(Node3 {
            id: format!("agent-{}", a.id),
            pos: Vertex3 {
                x: sx * R * 0.85,
                y: -H / 2.0,
                z: sy * R * 0.85,
            },
            radius: AGENT_RADIUS,
            color: Rgb(233, 223, 201),
            style: NodeStyle::default(),
            attrs: json!({ "agent_id": a.id, "bloc": a.bloc }),
        });
    }

    // Each community: hull at its height scalar + ghost-edge struts
    // (bounded by communities.len() * max member count).
    for c in &world.communities {
        let o = community_height_scalar(c, opts);
        let y = -H / 2.0 + o * H;
        let col = heat(1.0 - o);

        let member_pts: Vec<(f64, f64)> = c
            .members
            .iter()
            .map(|&m| {
                let (sx, sy) = pos[m];
                (sx * R * 0.85, sy * R * 0.85)
            })
            .collect();
        // D-R6 default: world-space hull (camera-independent SceneGraph3D),
        // not the deck's post-projection hull.
        let hull = convex_hull(&member_pts);
        faces.extend(hull_faces(&hull, y, col, 0.12 + 0.3 * o));

        for &m in &c.members {
            let (sx, sy) = pos[m];
            let bottom = Vertex3 {
                x: sx * R * 0.85,
                y: -H / 2.0,
                z: sy * R * 0.85,
            };
            let top = Vertex3 {
                x: sx * R * 0.85,
                y,
                z: sy * R * 0.85,
            };
            struts.push(Strut {
                a: bottom,
                b: top,
                color: GHOST_STRUT_COLOR,
            });
            nodes.push(Node3 {
                id: format!("agent-{}@{}", m, c.name),
                pos: top,
                radius: COMMUNITY_NODE_RADIUS,
                color: col,
                style: NodeStyle::default(),
                attrs: json!({ "agent_id": m, "community": c.name }),
            });
        }

        community_labels.push(json!({ "name": c.name, "o": o }));
    }

    let mut metadata = serde_json::Map::new();
    metadata.insert("communities".to_string(), json!(community_labels));

    SceneGraph3D {
        nodes,
        faces,
        struts,
        bounding_box: (-R, -H / 2.0, -R, R, H / 2.0, R),
        metadata,
    }
}
