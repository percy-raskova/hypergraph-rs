//! Instrument III — the OCC hyper-Sankey over the invariant u64 H3 grid
//! (`topology-deck.html:361-435`, Task 9). Every industry snaps to a real
//! `h3o::CellIndex`; ribbons are the transformation-algebra `T` transfers
//! (Task 5) reified as a proportional bipartite (multi-donor -> absorber)
//! allocation — a directed hyperedge, PHASE-R-MAP §3. FINE/COARSE zoom is
//! the coarse-graining functor `GROUP BY parent(h3_index)` (`h3::
//! aggregate_by_parent`): extensives sum, `Φ` and `r` are invariant.

use h3o::{CellIndex, Resolution};

use crate::raster::algebra::{transform, Industry};
use crate::raster::color::heat;
use crate::raster::h3::{aggregate_by_parent, industry_cell, origin_cell, Extensives};
use crate::raster::{Face, Node3, Rgb, SceneGraph3D, Strut, Vertex3};
use crate::viz::NodeStyle;

/// The fixed resolution industries snap to (Task 9 Interfaces: "pick a
/// res-8 origin").
const FINE_RES: Resolution = Resolution::Eight;
/// One level coarser — the `parent()` target for `Zoom::Coarse`.
const COARSE_RES: Resolution = Resolution::Seven;

/// Ground-plane hex scale (`topology-deck.html:373`, `S=.15`) — shared by
/// `hexXZ` position AND the fine/coarse `hexR` outline radius below.
pub const S: f64 = 0.15;
/// Fine-level hex outline radius: `S*.92` (`:374`).
pub const HEX_R_FINE: f64 = S * 0.92;
/// Coarse-level hex outline radius: `S*1.9` (`:374`).
pub const HEX_R_COARSE: f64 = S * 1.9;
/// The ground plane's world-space `y` (`:373`, `GROUNDY=-.95`).
pub const GROUND_Y: f64 = -0.95;
/// Bezier ribbon midpoint raise (`:427`, `mid.y = max(a.y,b.y)+.32`).
pub const RIBBON_LIFT: f64 = 0.32;
/// Sub-threshold flows are skipped from rendering only (Deviation 5).
pub const RENDER_THRESHOLD: f64 = 0.4;
/// Node radius `∝ √(c+v)` (world-space scale; the deck's own `.62` factor
/// additionally divides by a camera-dependent `top.s/300` foreshortening
/// term that has no meaning in a camera-independent `SceneGraph3D`,
/// mirroring the D-R6 world-space-hull precedent in `cylinder.rs`).
const NODE_RADIUS_SCALE: f64 = 0.05;

/// `#F2C56B` / `#C43A2E` — the deck's literal outline hex strings
/// (`:417`), distinct from `color::heat`'s own gold/crimson RGB endpoints.
const OUTLINE_GOLD: &str = "#F2C56B";
const OUTLINE_CRIMSON: &str = "#C43A2E";
const GROUND_HEX_COLOR: Rgb = Rgb(217, 164, 65);
const RIBBON_COLOR: Rgb = Rgb(196, 58, 46);

/// FINE = 13 industries; COARSE = `h3o::CellIndex::parent()` aggregation
/// (`topology-deck.html:373`, `W.zoom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoom {
    Fine,
    Coarse,
}

/// One rendered grid entity — an industry (FINE) or an aggregated H3 parent
/// cell (COARSE). `s = v·e` (recomputed as `s/v` from summed extensives on
/// aggregation, mirroring `aggregateCells`'s `e=gr.s/gr.v`).
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyEntity {
    pub name: String,
    pub cell: CellIndex,
    pub c: f64,
    pub v: f64,
    pub s: f64,
    pub t: f64,
    pub n: u32,
    pub hx: f64,
    pub hz: f64,
}

impl SankeyEntity {
    /// `e = s/v` (the industry/cell's rate of exploitation).
    pub fn e(&self) -> f64 {
        self.s / self.v
    }
}

/// One donor->absorber value transfer in the FULL (unrendered) allocation
/// (Deviation 5). `rendered` marks whether it cleared `RENDER_THRESHOLD`
/// and therefore has ribbon `Strut`s in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonFlow {
    pub donor: String,
    pub absorber: String,
    pub flow: f64,
    pub rendered: bool,
}

/// The built instrument: the camera-independent `SceneGraph3D` plus the
/// entities/ribbons/`(Φ,r)` the scene was derived from — golden-testable
/// without re-deriving the algebra from the scene geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyScene {
    pub scene: SceneGraph3D,
    pub entities: Vec<SankeyEntity>,
    pub ribbons: Vec<RibbonFlow>,
    pub phi: f64,
    pub r: f64,
}

impl SankeyScene {
    /// `Σ T_i` over rendered entities — `0` by construction of `r=S/CV`
    /// (`r·CV − S = 0`), partition-independent (Deviation 7).
    pub fn net_t(&self) -> f64 {
        self.entities.iter().map(|e| e.t).sum()
    }
}

/// `hexXZ(q,r,s) = {x: s·√3·(q+r/2), z: s·1.5·r}` (`:361`).
pub fn hex_xz(q: f64, r: f64, s: f64) -> (f64, f64) {
    (s * 1.732_050_8 * (q + r / 2.0), s * 1.5 * r)
}

/// `occY(c,v) = -0.8 + 1.55·clamp((ln(c/v)+0.3)/2.6, 0, 1)` (`:362`).
pub fn occ_y(c: f64, v: f64) -> f64 {
    let q = (c / v).ln();
    -0.8 + 1.55 * ((q + 0.3) / 2.6).clamp(0.0, 1.0)
}

/// The deck's `CLUSTER`/`OFF` axial-offset snap (`:169-177`): each
/// industry, in `inds` order, gets `(hq,hr) = CLUSTER[lane] + OFF[k]` where
/// `k` is that lane's running count mod 4. Bounded: `inds.len()`.
fn industry_hex_offsets(inds: &[Industry]) -> Vec<(i32, i32)> {
    const CLUSTER: [(i32, i32); 4] = [(-6, 0), (-2, 0), (1, -1), (4, 0)];
    const OFF: [(i32, i32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];
    let mut cnt = [0usize; 4];
    inds.iter()
        .map(|i| {
            let lane = i.lane as usize;
            let k = cnt[lane] % OFF.len();
            cnt[lane] += 1;
            (CLUSTER[lane].0 + OFF[k].0, CLUSTER[lane].1 + OFF[k].1)
        })
        .collect()
}

/// The 13 fine-level entities, each snapped to a real `h3o::CellIndex`
/// (Deviation 7). Bounded: `inds.len()`.
fn fine_entities(inds: &[Industry]) -> Vec<(CellIndex, SankeyEntity)> {
    let economy = transform(inds);
    let offsets = industry_hex_offsets(inds);
    let origin = origin_cell(FINE_RES);

    inds.iter()
        .zip(offsets.iter())
        .zip(economy.t.iter())
        .map(|((ind, &(hq, hr)), &t)| {
            let cell = industry_cell(origin, hq, hr).unwrap_or_else(|e| {
                panic!(
                    "industry hex ({hq},{hr}) hit h3o::LocalIjError::{e:?} — the deck fixture's \
                     fixed axial offsets are pentagon-free at FINE_RES from the (0,0) origin"
                )
            });
            let (hx, hz) = hex_xz(hq as f64, hr as f64, S);
            let s = ind.v * ind.e;
            let entity = SankeyEntity {
                name: ind.name.clone(),
                cell,
                c: ind.c,
                v: ind.v,
                s,
                t,
                n: 1,
                hx,
                hz,
            };
            (cell, entity)
        })
        .collect()
}

/// The COARSE-level entities: `fine_entities` grouped `BY parent(h3_index)`
/// (`h3::aggregate_by_parent`) — extensives summed, position averaged, `e`
/// recomputed (Deviation 7). Bounded: `inds.len()`.
fn coarse_entities(inds: &[Industry]) -> Vec<(CellIndex, SankeyEntity)> {
    let fine = fine_entities(inds);
    let extensive_pairs: Vec<(CellIndex, Extensives)> = fine
        .iter()
        .map(|(cell, e)| {
            (
                *cell,
                Extensives {
                    c: e.c,
                    v: e.v,
                    s: e.s,
                    t: e.t,
                    n: e.n,
                    hx: e.hx,
                    hz: e.hz,
                },
            )
        })
        .collect();

    aggregate_by_parent(&extensive_pairs, COARSE_RES)
        .into_iter()
        .map(|(parent, ext)| {
            let n = ext.n.max(1) as f64;
            let entity = SankeyEntity {
                name: format!("cell {:x}", u64::from(parent)),
                cell: parent,
                c: ext.c,
                v: ext.v,
                s: ext.s,
                t: ext.t,
                n: ext.n,
                hx: ext.hx / n,
                hz: ext.hz / n,
            };
            (parent, entity)
        })
        .collect()
}

/// The FULL (unrendered) donor->absorber allocation (Deviation 5): donors
/// (`T<0`) sorted ascending, `negSum = Σ(−T_donor)`; for each absorber `j`
/// and donor `i`, `flow = j.T·(−i.T)/negSum` (`:429-431`). Conserves to `Φ`
/// regardless of the `RENDER_THRESHOLD` cutoff. Bounded:
/// `donors.len() * absorbers.len()`.
fn allocate_ribbons(entities: &[SankeyEntity]) -> Vec<RibbonFlow> {
    let mut donors: Vec<&SankeyEntity> = entities.iter().filter(|e| e.t < 0.0).collect();
    // NaN-safe comparator (review MINOR 6): the plan forbids `.unwrap()` on
    // `partial_cmp` absolutely — same pattern as `camera.rs`'s depth sort.
    donors.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(core::cmp::Ordering::Equal));
    // The deck's `negSum = donors.reduce(...) || 1` (`:429`) substitutes 1
    // for EXACTLY zero — JS `||` tests falsiness, not magnitude. Review
    // MAJOR 2: `.max(1.0)` also substituted 1 for any `0 < Σ(−T) < 1`,
    // silently breaking `Σflow == Φ` on any scaled-down industry table.
    let raw_neg_sum: f64 = donors.iter().map(|e| -e.t).sum();
    let neg_sum = if raw_neg_sum == 0.0 { 1.0 } else { raw_neg_sum };

    let mut ribbons = Vec::with_capacity(donors.len() * entities.len());
    for j in entities.iter().filter(|e| e.t > 0.0) {
        for i in &donors {
            let flow = j.t * (-i.t) / neg_sum;
            ribbons.push(RibbonFlow {
                donor: i.name.clone(),
                absorber: j.name.clone(),
                flow,
                rendered: flow >= RENDER_THRESHOLD,
            });
        }
    }
    ribbons
}

/// Fan-triangulate a regular hexagon (7 corners, closed) at ground `y` into
/// filled `Face`s, plus the 6 perimeter edges as outline `Struts`. Bounded:
/// fixed 6-corner hex.
fn ground_hex(cx: f64, cz: f64, r: f64, fill: Rgb) -> (Vec<Face>, Vec<Strut>) {
    let mut corners = Vec::with_capacity(7);
    for k in 0..=6 {
        let a = std::f64::consts::PI / 180.0 * (60.0 * k as f64 + 30.0);
        corners.push(Vertex3 {
            x: cx + r * a.cos(),
            y: GROUND_Y,
            z: cz + r * a.sin(),
        });
    }
    let center = Vertex3 {
        x: cx,
        y: GROUND_Y,
        z: cz,
    };
    let mut faces = Vec::with_capacity(6);
    let mut struts = Vec::with_capacity(6);
    for w in corners.windows(2) {
        faces.push(Face {
            verts: [center, w[0], w[1]],
            fill,
            opacity: 0.05,
        });
        struts.push(Strut {
            a: w[0],
            b: w[1],
            color: fill,
        });
    }
    (faces, struts)
}

/// Build Instrument III (`topology-deck.html:363-433`).
pub fn build_sankey(inds: &[Industry], zoom: Zoom) -> SankeyScene {
    let economy = transform(inds);
    let (entity_pairs, hex_r) = match zoom {
        Zoom::Fine => (fine_entities(inds), HEX_R_FINE),
        Zoom::Coarse => (coarse_entities(inds), HEX_R_COARSE),
    };
    let entities: Vec<SankeyEntity> = entity_pairs.into_iter().map(|(_, e)| e).collect();
    let ribbons = allocate_ribbons(&entities);

    let mut faces = Vec::new();
    let mut struts = Vec::new();
    let mut nodes = Vec::with_capacity(entities.len());
    let mut labels = Vec::with_capacity(entities.len());

    // Ground hexes + stems + OCC-altitude node per entity (bounded:
    // entities.len()).
    for e in &entities {
        let (hex_faces, hex_struts) = ground_hex(e.hx, e.hz, hex_r, GROUND_HEX_COLOR);
        faces.extend(hex_faces);
        struts.extend(hex_struts);

        let y = occ_y(e.c, e.v);
        let base = Vertex3 {
            x: e.hx,
            y: GROUND_Y,
            z: e.hz,
        };
        let top = Vertex3 {
            x: e.hx,
            y,
            z: e.hz,
        };
        struts.push(Strut {
            a: base,
            b: top,
            color: GROUND_HEX_COLOR,
        });

        let style = NodeStyle {
            stroke: if e.t > 0.0 {
                OUTLINE_GOLD.to_string()
            } else {
                OUTLINE_CRIMSON.to_string()
            },
            ..NodeStyle::default()
        };

        let q = e.c / e.v;
        let label = format!(
            "{} q={:.1} {}{:.1}",
            e.name,
            q,
            if e.t >= 0.0 { "+" } else { "" },
            e.t
        );
        nodes.push(Node3 {
            id: format!("sankey-{}", e.name),
            pos: top,
            radius: NODE_RADIUS_SCALE * (e.c + e.v).sqrt(),
            color: heat((e.e() / 4.0).min(1.0)),
            style,
            attrs: serde_json::json!({
                "c": e.c, "v": e.v, "s": e.s, "t": e.t, "n": e.n,
                "q": q, "e": e.e(), "label": label,
            }),
        });
        labels.push(serde_json::json!({ "name": e.name, "q": q, "t": e.t }));
    }

    // Ribbon Struts, rendered allocation only (Deviation 5). Bounded:
    // ribbons.len().
    for f in ribbons.iter().filter(|f| f.rendered) {
        let a_entity = entities
            .iter()
            .find(|e| e.name == f.donor)
            .expect("donor entity exists");
        let b_entity = entities
            .iter()
            .find(|e| e.name == f.absorber)
            .expect("absorber entity exists");
        let a = Vertex3 {
            x: a_entity.hx,
            y: occ_y(a_entity.c, a_entity.v),
            z: a_entity.hz,
        };
        let b = Vertex3 {
            x: b_entity.hx,
            y: occ_y(b_entity.c, b_entity.v),
            z: b_entity.hz,
        };
        let mid = Vertex3 {
            x: (a.x + b.x) / 2.0,
            y: a.y.max(b.y) + RIBBON_LIFT,
            z: (a.z + b.z) / 2.0,
        };
        struts.push(Strut {
            a,
            b: mid,
            color: RIBBON_COLOR,
        });
        struts.push(Strut {
            a: mid,
            b,
            color: RIBBON_COLOR,
        });
    }

    let bbox = bounding_box(&faces, &struts, &nodes);

    let mut metadata = serde_json::Map::new();
    metadata.insert("entities".to_string(), serde_json::json!(labels));
    metadata.insert(
        "conservation".to_string(),
        serde_json::json!({
            "phi": economy.phi,
            "r": economy.r,
            "zoom": match zoom { Zoom::Fine => "fine", Zoom::Coarse => "coarse" },
        }),
    );

    SankeyScene {
        scene: SceneGraph3D {
            nodes,
            faces,
            struts,
            bounding_box: bbox,
            metadata,
        },
        entities,
        ribbons,
        phi: economy.phi,
        r: economy.r,
    }
}

/// Min/max over every vertex the scene actually produced. Bounded:
/// `faces.len()*3 + struts.len()*2 + nodes.len()`.
fn bounding_box(
    faces: &[Face],
    struts: &[Strut],
    nodes: &[Node3],
) -> (f64, f64, f64, f64, f64, f64) {
    let mut min = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut fold = |v: &Vertex3| {
        min.0 = min.0.min(v.x);
        min.1 = min.1.min(v.y);
        min.2 = min.2.min(v.z);
        max.0 = max.0.max(v.x);
        max.1 = max.1.max(v.y);
        max.2 = max.2.max(v.z);
    };
    for f in faces {
        for v in &f.verts {
            fold(v);
        }
    }
    for s in struts {
        fold(&s.a);
        fold(&s.b);
    }
    for n in nodes {
        fold(&n.pos);
    }
    if !min.0.is_finite() {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    (min.0, min.1, min.2, max.0, max.1, max.2)
}
