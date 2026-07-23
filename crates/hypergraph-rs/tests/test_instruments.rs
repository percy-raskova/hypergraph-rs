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
