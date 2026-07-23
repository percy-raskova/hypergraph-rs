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
