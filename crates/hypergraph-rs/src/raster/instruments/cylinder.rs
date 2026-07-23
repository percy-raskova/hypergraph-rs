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
use crate::raster::deck::{deck_hypergraph, org_degree, psa, psr, Community, DeckWorld};
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

/// Per-vector max-abs normalization inset, `a.x = X[i]*.92`
/// (`topology-deck.html:238-240`). Applied to BOTH coordinate columns
/// before any instrument's own draw scale.
const SPECTRAL_INSET: f64 = 0.92;

/// The deck's `sc(v) = {const mx = Math.max(...v.map(Math.abs)) || 1;
/// return v.map(x => x/mx)}` (`topology-deck.html:238`) applied to one
/// coordinate column, followed by the `*.92` inset (`:240`). The `|| 1`
/// all-zero guard is carried verbatim: a column of exact zeros divides by
/// `1.0`, never by `0.0`. Bounded: one pass over `col`.
fn scale_column(col: &mut [f64]) {
    let mut mx = 0.0_f64;
    for v in col.iter() {
        let a = v.abs();
        if a > mx {
            mx = a;
        }
    }
    // JS `|| 1`: substitutes 1 for a falsy (exactly 0 / NaN) maximum only.
    if mx.is_nan() || mx <= 0.0 {
        mx = 1.0;
    }
    for v in col.iter_mut() {
        *v = *v / mx * SPECTRAL_INSET;
    }
}

/// Every agent's spectral `(x,y)` disc position, indexed by `Agent.id`,
/// normalized exactly as the deck normalizes it: per-coordinate-column
/// max-abs scaling then the `0.92` inset (`topology-deck.html:238-240`), so
/// the returned components span `[-0.92, 0.92]` — the frame every
/// instrument's own draw scale (`R*0.85` on the cylinder, `S` on the
/// terrain) is written against. Review MAJOR 4: returning raw eigenvector
/// components instead left the cylinder disc spanning ~38% of its radius.
///
/// Reuses the shared `deck_hypergraph` adapter (Finding 6) + the crate's
/// pinned-seed eigensolver (Deviation 2: structural-parity, not deck-byte-
/// exact). `pub(crate)` so Task 8's terrain builder (which needs the same
/// per-agent `(x,y)` the deck's `drawTerrain` reads off `a.x`/`a.y`) reuses
/// it rather than re-deriving spectral positions a second time (DRY) — the
/// deck likewise normalizes ONCE in `spectral()` and shares the result.
pub(crate) fn spectral_positions(world: &DeckWorld) -> Vec<(f64, f64)> {
    let hg = deck_hypergraph(world);
    let (l, ids) = normalized_laplacian(&hg)
        .expect("deck_hypergraph guarantees every agent has degree >= 1 (Task 7 deviation)");
    let ev = smallest_k_eigenvectors(&l, 2, Some(SPECTRAL_SEED));

    let mut row_of: HashMap<String, usize> = HashMap::new();
    for (row, id) in ids.iter().enumerate() {
        row_of.insert(id.clone(), row);
    }

    let mut xs: Vec<f64> = Vec::with_capacity(world.agents.len());
    let mut ys: Vec<f64> = Vec::with_capacity(world.agents.len());
    for a in &world.agents {
        let row = row_of[&a.id.to_string()];
        xs.push(ev[[row, 0]]);
        ys.push(ev[[row, 1]]);
    }
    scale_column(&mut xs);
    scale_column(&mut ys);

    xs.into_iter().zip(ys).collect()
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

/// Segments per ring (`topology-deck.html:297`, `i<=64`).
const RING_SEGMENTS: usize = 64;
/// Vertical body struts around the barrel (`:302`, `i<12`).
const BODY_STRUTS: usize = 12;
/// Bottom ring ink, `rgba(138,122,97,.55)` (`:301`).
const RING_BOTTOM_COLOR: Rgb = Rgb(138, 122, 97);
/// Top ring ink, `rgba(217,164,65,.65)` (`:301`).
const RING_TOP_COLOR: Rgb = Rgb(217, 164, 65);
/// Body strut ink, `rgba(74,62,44,.5)` (`:304`).
const BODY_STRUT_COLOR: Rgb = Rgb(74, 62, 44);

/// One 64-segment circle of radius `R` at height `y`, as 64 chord `Strut`s
/// (`topology-deck.html:296-300`). Bounded: `RING_SEGMENTS`.
fn ring(y: f64, color: Rgb) -> Vec<Strut> {
    let point = |i: usize| -> Vertex3 {
        let a = i as f64 / RING_SEGMENTS as f64 * std::f64::consts::TAU;
        Vertex3 {
            x: a.cos() * R,
            y,
            z: a.sin() * R,
        }
    };
    (0..RING_SEGMENTS)
        .map(|i| Strut {
            a: point(i),
            b: point(i + 1),
            color,
        })
        .collect()
}

/// Build Instrument I (`topology-deck.html:294-336`).
pub fn build_cylinder(world: &DeckWorld, opts: &InstrumentOpts) -> SceneGraph3D {
    let pos = spectral_positions(world);

    let mut nodes = Vec::with_capacity(world.agents.len());
    let mut faces = Vec::new();
    let mut struts = Vec::new();
    let mut community_labels = Vec::with_capacity(world.communities.len());

    // The barrel itself: two rings + 12 vertical body struts (`:301-305`).
    // Without these the instrument has no cylinder silhouette at all
    // (review MAJOR 4 / FIX B3).
    struts.extend(ring(-H / 2.0, RING_BOTTOM_COLOR));
    struts.extend(ring(H / 2.0, RING_TOP_COLOR));
    for i in 0..BODY_STRUTS {
        let a = i as f64 / BODY_STRUTS as f64 * std::f64::consts::TAU;
        let (x, z) = (a.cos() * R, a.sin() * R);
        struts.push(Strut {
            a: Vertex3 { x, y: -H / 2.0, z },
            b: Vertex3 { x, y: H / 2.0, z },
            color: BODY_STRUT_COLOR,
        });
    }

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
    // The survival calculus (`topology-deck.html:443-444`) is read off the
    // SAME organization degrees this instrument uses for its hull heights
    // (`psr` is literally `mean(org(cell α), org(cell β))` over a repression
    // denominator), so the cylinder is where those two labeled scalars
    // belong. Carrying them in `metadata` is what makes them appear as
    // literal glyph text in every rendered frame — the Global Constraint
    // that names `psa`/`psr` explicitly (review MAJOR 5).
    //
    // Tolerance caveat (extends the plan's Task 11 Step 3 note): `psa`
    // routes through `sigmoid`'s `exp`, a libm transcendental, so the
    // cylinder's JSON golden is now **platform-pinned** on this one field
    // exactly as the Sankey's `ln`-derived altitudes already are. Every
    // other cylinder field remains basic-IEEE-754 byte-exact, and the
    // `fmt6`-rounded banner text is stable to 6 significant digits.
    if world.communities.len() >= 2 {
        metadata.insert("psa".to_string(), json!(psa(world)));
        metadata.insert("psr".to_string(), json!(psr(world)));
    }

    SceneGraph3D {
        nodes,
        faces,
        struts,
        bounding_box: (-R, -H / 2.0, -R, R, H / 2.0, R),
        metadata,
    }
}
