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
