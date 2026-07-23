#![cfg(feature = "cells3d")]

use hypergraph_rs::raster::deck::build_world;
use hypergraph_rs::raster::instruments::{build, Instrument, InstrumentOpts};

const H: f64 = 1.55;

#[test]
fn test_cylinder_community_height_equals_org_degree() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());

    // capital δ (o=1) sits at top (+H/2); the three o=0.2 communities near
    // the bottom (-H/2 + 0.2*H). Read the height back off the `communities`
    // metadata this instrument writes, and cross-check against every hull
    // face's y (all verts of a community's faces share its height exactly).
    let comms = s.metadata.get("communities").unwrap().as_array().unwrap();
    assert_eq!(comms.len(), 4);
    let mut saw_top = false;
    for c in comms {
        let name = c["name"].as_str().unwrap();
        let o = c["o"].as_f64().unwrap();
        let want_y = -H / 2.0 + o * H;
        if name == "capital \u{3b4}" {
            assert!((o - 1.0).abs() < 1e-12);
            saw_top = true;
        } else {
            assert!((o - 0.2).abs() < 1e-12, "{name}: o={o}");
        }
        // At least one hull face exists at exactly this community's height.
        let has_face_at_height = s
            .faces
            .iter()
            .any(|f| f.verts.iter().all(|v| (v.y - want_y).abs() < 1e-9));
        assert!(has_face_at_height, "{name}: no face at y={want_y}");
    }
    assert!(saw_top, "capital δ must be present at o=1");
}

#[test]
fn test_cylinder_every_agent_on_bottom_disc() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());

    let bottom_agents: Vec<_> = s
        .nodes
        .iter()
        .filter(|n| n.id.starts_with("agent-") && !n.id.contains('@'))
        .collect();
    assert_eq!(
        bottom_agents.len(),
        28,
        "all 28 agents get a bottom-disc point"
    );
    for n in &bottom_agents {
        assert!(
            (n.pos.y - (-H / 2.0)).abs() < 1e-12,
            "{}: y={}",
            n.id,
            n.pos.y
        );
    }
}

#[test]
fn test_cylinder_scene_is_camera_independent_and_deterministic() {
    let w = build_world(1917);
    assert_eq!(
        build(Instrument::Cylinder, &w, &InstrumentOpts::default()),
        build(Instrument::Cylinder, &w, &InstrumentOpts::default())
    );
}

#[test]
fn test_terrain_grid_dimensions_and_extent() {
    use hypergraph_rs::raster::instruments::terrain::{M, N, S};

    let w = build_world(1917);
    let s = build(Instrument::Terrain, &w, &InstrumentOpts::default());

    let grid_vertices = s.metadata["grid_vertices"].as_u64().unwrap();
    assert_eq!(grid_vertices, ((N + 1) * (M + 1)) as u64);
    assert_eq!(grid_vertices, 35 * 25);

    let extent = s.metadata["extent"].as_f64().unwrap();
    assert!((extent - S).abs() < 1e-12);
    assert!((S - 1.35).abs() < 1e-12);

    // N*M quads, 2 triangle faces each; every vertex stays within [-S,S].
    assert_eq!(s.faces.len(), N * M * 2);
    for f in &s.faces {
        for v in &f.verts {
            assert!(
                v.x >= -S - 1e-9 && v.x <= S + 1e-9,
                "x={} out of extent",
                v.x
            );
            assert!(
                v.z >= -S - 1e-9 && v.z <= S + 1e-9,
                "z={} out of extent",
                v.z
            );
        }
    }
}

#[test]
fn test_terrain_tension_only_below_subsistence() {
    use hypergraph_rs::raster::deck::Agent;
    use hypergraph_rs::raster::instruments::terrain::agent_tension;

    let above = Agent {
        id: 0,
        bloc: 0,
        wealth: 8.5,
        subsistence: 3.0,
    };
    assert_eq!(agent_tension(&above, 2.6), 0.0);

    let at = Agent {
        id: 1,
        bloc: 0,
        wealth: 3.0,
        subsistence: 3.0,
    };
    assert_eq!(agent_tension(&at, 2.6), 0.0);

    let below = Agent {
        id: 2,
        bloc: 2,
        wealth: 1.0,
        subsistence: 3.0,
    };
    assert!((agent_tension(&below, 2.6) - 2.6 * 2.0).abs() < 1e-12);
}

#[test]
fn test_terrain_idw_epsilon_is_exact_0_02() {
    use hypergraph_rs::raster::instruments::terrain::{idw_height, TensionNode, EPS};

    assert!((EPS - 0.02).abs() < 1e-15);

    // A single tension source evaluated AT its own (x,y): d2 = 0 + EPS, so
    // the field is finite (no singularity) and equals exactly 1/EPS/tmax
    // scaled by t — smoothed, not blown up.
    let nodes = vec![TensionNode {
        x: 0.3,
        y: 0.3,
        t: 1.0,
    }];
    let v = idw_height(0.3, 0.3, &nodes, 1.0);
    assert!(v.is_finite());
    assert!(
        (v - 1.0).abs() < 1e-12,
        "single-node field normalizes to 1.0, got {v}"
    );
}

#[test]
fn test_terrain_deterministic() {
    let w = build_world(1917);
    assert_eq!(
        build(Instrument::Terrain, &w, &InstrumentOpts::default()),
        build(Instrument::Terrain, &w, &InstrumentOpts::default())
    );
}

// ---------- Task 9: Instrument III — OCC hyper-Sankey over u64 H3 ----------

use hypergraph_rs::raster::algebra::deck_industries;
use hypergraph_rs::raster::instruments::sankey::{build_sankey, Zoom};

#[test]
fn test_occ_altitude_finance_is_highest() {
    let scene = build_sankey(&deck_industries(), Zoom::Fine);
    let finance = scene
        .entities
        .iter()
        .find(|e| e.name == "finance")
        .expect("finance industry present");
    let carceral = scene
        .entities
        .iter()
        .find(|e| e.name == "carceral labor")
        .expect("carceral labor industry present");

    // finance: c=60,v=5 => q=12 => occY clamps to +0.75 (top of the range).
    let finance_y = hypergraph_rs::raster::instruments::sankey::occ_y(finance.c, finance.v);
    assert!(
        (finance_y - 0.75).abs() < 1e-9,
        "finance occY={finance_y}, want 0.75"
    );

    // carceral labor: c=4,v=5 => q=0.8 => occY ≈ -0.7542.
    let carceral_y = hypergraph_rs::raster::instruments::sankey::occ_y(carceral.c, carceral.v);
    assert!(
        (carceral_y - (-0.754_181_732)).abs() < 1e-6,
        "carceral labor occY={carceral_y}, want ≈ -0.7542"
    );

    // finance sits strictly above every other entity's altitude.
    for e in &scene.entities {
        if e.name != "finance" {
            let y = hypergraph_rs::raster::instruments::sankey::occ_y(e.c, e.v);
            assert!(finance_y >= y, "finance ({finance_y}) not >= {}: {y}", e.name);
        }
    }
}

#[test]
fn test_ribbon_flows_conserve_to_phi_unrendered() {
    let scene = build_sankey(&deck_industries(), Zoom::Fine);

    // Total flow across the FULL allocation (no 0.4 render threshold)
    // equals Φ (Deviation 5).
    let total: f64 = scene.ribbons.iter().map(|r| r.flow).sum();
    assert!(
        (total - scene.phi).abs() < 1e-9,
        "total flow {total} != phi {}",
        scene.phi
    );

    // Absorber-side conservation: every absorber's incoming flow sums back
    // to exactly its own T (both directions of the bipartite allocation
    // agree on the same Φ).
    for j in scene.entities.iter().filter(|e| e.t > 0.0) {
        let received: f64 = scene
            .ribbons
            .iter()
            .filter(|r| r.absorber == j.name)
            .map(|r| r.flow)
            .sum();
        assert!(
            (received - j.t).abs() < 1e-9,
            "{}: received {received} != T {}",
            j.name,
            j.t
        );
    }
}

#[test]
fn test_net_t_zero_at_both_zooms() {
    let fine = build_sankey(&deck_industries(), Zoom::Fine);
    let coarse = build_sankey(&deck_industries(), Zoom::Coarse);
    assert!(fine.net_t().abs() < 1e-9, "fine net_t={}", fine.net_t());
    assert!(
        coarse.net_t().abs() < 1e-9,
        "coarse net_t={}",
        coarse.net_t()
    ); // partition-independent (Deviation 7)
}

#[test]
fn test_r_invariant_under_aggregation() {
    let fine = build_sankey(&deck_industries(), Zoom::Fine);
    let coarse = build_sankey(&deck_industries(), Zoom::Coarse);
    assert!(
        (fine.r - coarse.r).abs() < 1e-12,
        "fine.r={} coarse.r={}",
        fine.r,
        coarse.r
    );
}

#[test]
fn test_h3_parent_is_u64_bit_op_never_string() {
    use h3o::Resolution;
    use hypergraph_rs::raster::h3::{industry_cell, origin_cell};

    let origin = origin_cell(Resolution::Eight);
    let cell = industry_cell(origin, -6, 0).expect("fixture offset is pentagon-free");

    // u64 round-trip — no String in the hot path.
    let raw: u64 = cell.into();
    assert_ne!(raw, 0);
    let back = h3o::CellIndex::try_from(raw).expect("valid u64 round-trips to a CellIndex");
    assert_eq!(back, cell);

    // parent() is a bit-op that reduces resolution.
    let parent = cell.parent(Resolution::Seven).expect("Seven <= Eight");
    assert_eq!(parent.resolution(), Resolution::Seven);
    assert_ne!(u64::from(parent), u64::from(cell));
}

// ---------- Task 11: Stage 1 — SceneGraph3D JSON golden fixtures ----------

#[test]
fn test_scene3d_golden_cylinder() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    let json = serde_json::to_string_pretty(&s).unwrap();
    assert_eq!(json, include_str!("fixtures/scene3d/cylinder.json"));
}

#[test]
fn test_scene3d_golden_terrain() {
    let w = build_world(1917);
    let s = build(Instrument::Terrain, &w, &InstrumentOpts::default());
    let json = serde_json::to_string_pretty(&s).unwrap();
    assert_eq!(json, include_str!("fixtures/scene3d/terrain.json"));
}

#[test]
fn test_scene3d_golden_sankey_fine() {
    let s = build_sankey(&deck_industries(), Zoom::Fine).scene;
    let json = serde_json::to_string_pretty(&s).unwrap();
    assert_eq!(json, include_str!("fixtures/scene3d/sankey_fine.json"));
}

#[test]
fn test_scene3d_golden_sankey_coarse() {
    let s = build_sankey(&deck_industries(), Zoom::Coarse).scene;
    let json = serde_json::to_string_pretty(&s).unwrap();
    assert_eq!(json, include_str!("fixtures/scene3d/sankey_coarse.json"));
}

// ---------- Task 13: Stage 3 — property tests ----------

/// Property 2 (zoom-invariance of extensives vs intensives): every
/// extensive (`c, v, s, T`) sums identically fine vs coarse — the h3o
/// `parent()` aggregation only regroups rows, it never creates or destroys
/// value (Deviation 7). The ratio-of-sums intensive `r` is likewise
/// identical (already covered structurally by `transform`'s `r=S/CV`, and
/// re-asserted here alongside the extensive sums as one grouped contract).
/// A *per-entity* intensive (`e = s/v`) is explicitly NOT required to
/// survive aggregation 1:1 — FINE has one row per industry (13), COARSE has
/// strictly fewer (h3o `parent()` merges several industries per cell), so
/// the two `e()` sequences cannot even be the same length, let alone equal
/// element-wise; `e` is recomputed post-aggregation from summed extensives.
#[test]
fn test_property_zoom_invariance_extensive_vs_intensive_split() {
    let fine = build_sankey(&deck_industries(), Zoom::Fine);
    let coarse = build_sankey(&deck_industries(), Zoom::Coarse);

    let fine_c: f64 = fine.entities.iter().map(|e| e.c).sum();
    let coarse_c: f64 = coarse.entities.iter().map(|e| e.c).sum();
    let fine_v: f64 = fine.entities.iter().map(|e| e.v).sum();
    let coarse_v: f64 = coarse.entities.iter().map(|e| e.v).sum();
    let fine_s: f64 = fine.entities.iter().map(|e| e.s).sum();
    let coarse_s: f64 = coarse.entities.iter().map(|e| e.s).sum();
    let fine_t: f64 = fine.entities.iter().map(|e| e.t).sum();
    let coarse_t: f64 = coarse.entities.iter().map(|e| e.t).sum();
    for (name, f, c) in [
        ("c", fine_c, coarse_c),
        ("v", fine_v, coarse_v),
        ("s", fine_s, coarse_s),
        ("t", fine_t, coarse_t),
    ] {
        assert!(
            (f - c).abs() < 1e-9,
            "extensive {name}: fine Σ={f} coarse Σ={c}"
        );
    }

    // Intensive r: ratio of sums, invariant by construction.
    assert!((fine.r - coarse.r).abs() < 1e-12);

    // Explicit split: per-entity intensive e is NOT preserved 1:1 —
    // aggregation strictly reduces row count (fine has 13 industries).
    assert_eq!(fine.entities.len(), 13);
    assert!(
        coarse.entities.len() < fine.entities.len(),
        "coarse aggregation must reduce entity count: fine={} coarse={}",
        fine.entities.len(),
        coarse.entities.len()
    );
}

/// Property 3 (glyph-floor information parity): every labeled scalar an
/// instrument computes appears as a literal substring of a `CellGrid`'s
/// `to_text()` — no quantity is pixel-only. Uses the Task 13 hooks
/// (`SceneGraph3D::labeled_scalars`, `CellGrid::from_banner_text`), which
/// are independent of the pixel `rasterize()` path (they never touch a
/// frame golden). Covers the sankey `Φ`/`r` conservation banner, every
/// cylinder community's `o(C)`, and the deck-level `psa`/`psr` survival
/// probabilities (Global Constraints; PHASE-R-MAP §4's `PHI=… r=…` banner
/// is the model).
#[test]
fn test_property_glyph_floor_parity_sankey_conservation() {
    let sankey = build_sankey(&deck_industries(), Zoom::Fine);
    let labels = sankey.scene.labeled_scalars();
    let banner = hypergraph_rs::raster::CellGrid::from_banner_text(&labels.join(" "));
    let text = banner.to_text();

    use hypergraph_rs::raster::fmt::fmt6;
    let want_phi = format!("conservation.phi={}", fmt6(sankey.phi));
    let want_r = format!("conservation.r={}", fmt6(sankey.r));
    assert!(labels.contains(&want_phi), "missing {want_phi} in {labels:?}");
    assert!(labels.contains(&want_r), "missing {want_r} in {labels:?}");
    assert!(text.contains(&want_phi), "banner text lost {want_phi}");
    assert!(text.contains(&want_r), "banner text lost {want_r}");
}

#[test]
fn test_property_glyph_floor_parity_cylinder_org_degrees() {
    let w = build_world(1917);
    let s = build(Instrument::Cylinder, &w, &InstrumentOpts::default());
    let labels = s.labeled_scalars();
    let banner = hypergraph_rs::raster::CellGrid::from_banner_text(&labels.join(" "));
    let text = banner.to_text();

    use hypergraph_rs::raster::fmt::fmt6;
    let comms = s.metadata.get("communities").unwrap().as_array().unwrap();
    assert_eq!(comms.len(), 4, "sanity: 4 communities in the fixture");
    for (i, c) in comms.iter().enumerate() {
        let name = c["name"].as_str().unwrap();
        let o = c["o"].as_f64().unwrap();
        let want = format!("communities[{i}].o={}", fmt6(o));
        assert!(labels.contains(&want), "missing {want} in {labels:?} (community {name})");
        assert!(text.contains(&want), "banner text lost {want}");
    }
}

#[test]
fn test_property_glyph_floor_parity_psa_psr() {
    use hypergraph_rs::raster::deck::{psa, psr};
    use hypergraph_rs::raster::fmt::fmt6;

    let w = build_world(1917);
    let psa_v = psa(&w);
    let psr_v = psr(&w);
    let banner_text = format!("psa={} psr={}", fmt6(psa_v), fmt6(psr_v));
    let banner = hypergraph_rs::raster::CellGrid::from_banner_text(&banner_text);
    let text = banner.to_text();
    assert!(text.contains(&format!("psa={}", fmt6(psa_v))));
    assert!(text.contains(&format!("psr={}", fmt6(psr_v))));
}

/// Property 5 (mutation test, Deviation 6): `Φ == rent_imperial +
/// rent_internal` holds at the seed-1917 fixture only because every lane-2/3
/// industry is currently a net absorber. Perturb one lane-3 industry
/// (finance) so it becomes a donor (`T < 0`) by inflating its exploitation
/// rate `e` — the fixture property must then FAIL, proving the baseline
/// equality (`test_phi_equals_rent_sum_at_seed_1917_fixture`,
/// `tests/test_deck_fixture.rs`) is empirical, not structural, and that a
/// regression hard-coding the equality would be caught.
#[test]
fn test_property_mutation_phi_rent_equality_is_not_structural() {
    use hypergraph_rs::raster::algebra::transform;

    let mut inds = deck_industries();
    let finance = inds
        .iter_mut()
        .find(|i| i.name == "finance")
        .expect("finance industry present");
    assert_eq!(finance.lane, 3, "finance must be lane 2/3 for this mutation");
    finance.e *= 50.0; // huge s = v*e -> strongly negative T -> finance becomes a donor

    let e = transform(&inds);
    let mutated_finance_t = e.t[inds.iter().position(|i| i.name == "finance").unwrap()];
    assert!(
        mutated_finance_t < 0.0,
        "mutation must flip finance to a donor (T<0), got T={mutated_finance_t}"
    );

    let baseline_holds = (e.phi - (e.rent_imperial + e.rent_internal)).abs() < 1e-9;
    assert!(
        !baseline_holds,
        "Φ==rent_imperial+rent_internal must FAIL once a lane-3 industry becomes a donor \
         (Deviation 6) — got Φ={} rent_imperial+rent_internal={}",
        e.phi,
        e.rent_imperial + e.rent_internal
    );
}
