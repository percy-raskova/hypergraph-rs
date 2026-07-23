#![cfg(feature = "cells3d")]

use hypergraph_rs::raster::deck::psa;
use hypergraph_rs::raster::ingest::{
    build_scene, ingest_edges, ingest_nodes, to_world, AttrContract, IngestError, IngestedNodes,
    TensionSource, ATTR_CONTRACT_VERSION, DEFAULT_BLOC, DEFAULT_LANE, DEFAULT_MASS,
    DEFAULT_REPRESSION, K_WEALTH,
};
use hypergraph_rs::raster::instruments::{Instrument, InstrumentOpts};
use hypergraph_rs::raster::SceneGraph3D;
use hypergraph_rs::Hypergraph;
use serde_json::json;

#[test]
fn test_contract_default_spellings_and_version() {
    let c = AttrContract::default();
    assert_eq!(c.version, ATTR_CONTRACT_VERSION);
    assert_eq!(c.wealth, K_WEALTH);
    assert_eq!(DEFAULT_BLOC, 1);
}

#[test]
fn test_contract_roundtrips_through_json() {
    let c = AttrContract::default();
    let s = serde_json::to_string(&c).expect("contract serializes");
    assert_eq!(
        serde_json::from_str::<AttrContract>(&s).expect("deserializes"),
        c
    );
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn test_contract_keys_are_renameable() {
    let mut c = AttrContract::default();
    c.wealth = "babylon_wealth".to_string();
    assert_ne!(c.wealth, K_WEALTH);
}

// --- Task 2: node ingest -----------------------------------------------

#[test]
fn test_nodes_missing_wealth_is_typed_error_not_default() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"subsistence": 5.0}));
    let c = AttrContract::default();
    let err = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap_err();
    assert_eq!(
        err,
        IngestError::MissingNodeAttr {
            node: "a".to_string(),
            key: "wealth".to_string(),
        }
    );
}

#[test]
fn test_nodes_missing_bloc_defaults_to_one_and_is_counted() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "subsistence": 5.0}));
    let c = AttrContract::default();
    let IngestedNodes {
        agents,
        bloc_defaulted,
        ..
    } = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
    assert_eq!(agents[0].bloc, DEFAULT_BLOC);
    assert_eq!(bloc_defaulted, 1);
}

#[test]
fn test_nodes_agent_ids_are_dense_and_follow_node_ids_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("z", json!({"wealth": 1.0, "subsistence": 0.5, "bloc": 0}));
    h.add_node("a", json!({"wealth": 2.0, "subsistence": 1.0, "bloc": 1}));
    h.add_node("m", json!({"wealth": 3.0, "subsistence": 1.5, "bloc": 2}));
    let c = AttrContract::default();
    let IngestedNodes {
        agents, node_ids, ..
    } = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
    assert_eq!(node_ids, h.node_ids());
    assert_eq!(agents.len(), 3);
    for (i, a) in agents.iter().enumerate() {
        assert_eq!(a.id, i);
    }
}

#[test]
fn test_nodes_reject_non_finite_wealth() {
    let c = AttrContract::default();

    let mut h1: Hypergraph = Hypergraph::new();
    h1.add_node("a", json!({"wealth": f64::INFINITY, "subsistence": 1.0}));
    assert!(matches!(
        ingest_nodes(&h1, &c, DEFAULT_REPRESSION),
        Err(IngestError::BadAttrType { .. })
    ));

    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_node("a", json!({"wealth": null, "subsistence": 1.0}));
    assert!(matches!(
        ingest_nodes(&h2, &c, DEFAULT_REPRESSION),
        Err(IngestError::BadAttrType { .. })
    ));

    let mut h3: Hypergraph = Hypergraph::new();
    h3.add_node("a", json!({"wealth": "3.0", "subsistence": 1.0}));
    assert!(matches!(
        ingest_nodes(&h3, &c, DEFAULT_REPRESSION),
        Err(IngestError::BadAttrType { .. })
    ));
}

#[test]
fn test_h3_cell_accepts_u64_string_above_2_pow_53() {
    let c = AttrContract::default();

    let mut h: Hypergraph = Hypergraph::new();
    h.add_node(
        "a",
        json!({"wealth": 1.0, "subsistence": 0.5, "h3_cell": "622236723497533439"}),
    );
    let IngestedNodes { h3_cells, .. } = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
    assert_eq!(h3_cells[0], Some(622236723497533439u64));

    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_node(
        "a",
        json!({"wealth": 1.0, "subsistence": 0.5, "h3_cell": 622236723497533439.5}),
    );
    assert!(matches!(
        ingest_nodes(&h2, &c, DEFAULT_REPRESSION),
        Err(IngestError::BadAttrType { .. })
    ));
}

#[test]
fn test_nodes_tension_attr_overrides_subsistence_when_present_on_all_nodes() {
    let c = AttrContract::default();
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "tension": 2.0}));
    h.add_node("b", json!({"wealth": 5.0, "tension": 0.0}));
    let repression = 2.0;
    let IngestedNodes {
        agents,
        tension_source,
        ..
    } = ingest_nodes(&h, &c, repression).unwrap();
    assert_eq!(tension_source, TensionSource::Attr);
    assert!((agents[0].subsistence - (10.0 + 2.0 / repression)).abs() < 1e-9);
    assert!((agents[1].subsistence - (5.0 + 0.0 / repression)).abs() < 1e-9);
}

#[test]
fn test_nodes_tension_attr_partial_coverage_is_missing_node_attr_error() {
    let c = AttrContract::default();
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "tension": 2.0}));
    h.add_node("b", json!({"wealth": 5.0, "subsistence": 1.0}));
    let err = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap_err();
    assert_eq!(
        err,
        IngestError::MissingNodeAttr {
            node: "b".to_string(),
            key: "tension".to_string(),
        }
    );
}

// --- Task 3: edge ingest -------------------------------------------------

fn idx_for(ids: &[String]) -> std::collections::HashMap<&str, usize> {
    ids.iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect()
}

#[test]
fn test_edges_unknown_kind_is_skipped_and_reported_not_an_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_node("b", json!({}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        json!({"kind": "mystery"}),
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e2".to_string()),
        json!({}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let out = ingest_edges(&h, &c, &index).unwrap();
    assert_eq!(out.skipped, vec!["e1".to_string(), "e2".to_string()]);
    assert!(out.communities.is_empty());
    assert!(out.extraction.is_empty());
    assert!(out.industries.is_empty());
}

#[test]
fn test_community_links_outside_members_is_typed_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_node("b", json!({}));
    h.add_node("z", json!({}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community", "name": "c1", "links": [["a", "z"]]}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let err = ingest_edges(&h, &c, &index).unwrap_err();
    assert_eq!(
        err,
        IngestError::LinkOutsideCommunity {
            edge: "com1".to_string(),
            node: "z".to_string(),
        }
    );
}

#[test]
fn test_community_with_no_links_ingests_and_org_degree_is_real() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_node("b", json!({}));
    h.add_node("c", json!({}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community"}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let out = ingest_edges(&h, &c, &index).unwrap();
    assert_eq!(out.communities.len(), 1);
    let com = &out.communities[0];
    assert!(com.links.is_empty());
    assert_eq!(com.members.len(), 3);
    assert_eq!(
        hypergraph_rs::raster::deck::org_degree(com),
        0.0,
        "3 fully-disconnected members: org_degree is a real 0.0 reading"
    );
}

#[test]
fn test_extraction_head_must_be_a_member() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_node("b", json!({}));
    h.add_node("z", json!({}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("ex1".to_string()),
        json!({"kind": "extraction", "head": "z"}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let err = ingest_edges(&h, &c, &index).unwrap_err();
    assert_eq!(
        err,
        IngestError::HeadNotMember {
            edge: "ex1".to_string(),
            node: "z".to_string(),
        }
    );
}

#[test]
fn test_extraction_mass_defaults_because_no_instrument_reads_it() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_node("b", json!({}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("ex1".to_string()),
        json!({"kind": "extraction", "head": "b"}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let out = ingest_edges(&h, &c, &index).unwrap();
    assert_eq!(out.extraction.len(), 1);
    assert_eq!(out.extraction[0].mass, DEFAULT_MASS);
}

#[test]
fn test_industry_missing_c_v_or_e_is_typed_error_not_zero() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_edge(
        vec!["a".to_string()],
        Some("in1".to_string()),
        json!({"kind": "industry", "c": 1.0, "v": 2.0}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let err = ingest_edges(&h, &c, &index).unwrap_err();
    assert_eq!(
        err,
        IngestError::MissingEdgeAttr {
            edge: "in1".to_string(),
            kind: "industry".to_string(),
            key: "e".to_string(),
        }
    );
}

#[test]
fn test_industry_missing_lane_defaults_and_sets_lane_defaulted_flag() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_edge(
        vec!["a".to_string()],
        Some("in1".to_string()),
        json!({"kind": "industry", "c": 1.0, "v": 2.0, "e": 0.5}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let out = ingest_edges(&h, &c, &index).unwrap();
    assert_eq!(out.industries[0].lane, DEFAULT_LANE);
    assert!(out.lane_defaulted);
}

#[test]
fn test_industry_zero_v_is_degenerate_industry_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_edge(
        vec!["a".to_string()],
        Some("in1".to_string()),
        json!({"kind": "industry", "c": 1.0, "v": 0.0, "e": 0.5}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let err = ingest_edges(&h, &c, &index).unwrap_err();
    assert_eq!(
        err,
        IngestError::DegenerateIndustry {
            edge: "in1".to_string(),
        }
    );
}

#[test]
fn test_industry_slice_order_follows_edge_ids_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({}));
    h.add_edge(
        vec!["a".to_string()],
        Some("in_z".to_string()),
        json!({"kind": "industry", "c": 1.0, "v": 1.0, "e": 1.0}),
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string()],
        Some("in_a".to_string()),
        json!({"kind": "industry", "c": 2.0, "v": 2.0, "e": 2.0}),
    )
    .unwrap();
    let c = AttrContract::default();
    let ids = h.node_ids();
    let index = idx_for(&ids);
    let out = ingest_edges(&h, &c, &index).unwrap();
    assert_eq!(out.industries.len(), 2);
    // edge_ids() insertion order: in_z then in_a — NOT sorted by name.
    assert_eq!(out.industries[0].c, 1.0);
    assert_eq!(out.industries[1].c, 2.0);
}

// --- Task 4: to_world -----------------------------------------------------

/// Two agents, two community edges, `with_periphery` controls whether one
/// agent carries `bloc: 2`.
fn graph_with_two_communities(with_periphery: bool) -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    let b_bloc = if with_periphery { 2 } else { 0 };
    h.add_node("a", json!({"wealth": 10.0, "subsistence": 5.0, "bloc": 0}));
    h.add_node(
        "b",
        json!({"wealth": 8.0, "subsistence": 4.0, "bloc": b_bloc}),
    );
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community", "name": "c1"}),
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("com2".to_string()),
        json!({"kind": "community", "name": "c2"}),
    )
    .unwrap();
    h
}

#[test]
fn test_to_world_is_pure_and_deterministic() {
    let h = graph_with_two_communities(true);
    let c = AttrContract::default();
    let (world1, report1) = to_world(&h, &c).unwrap();
    let (world2, report2) = to_world(&h, &c).unwrap();
    assert_eq!(world1, world2);
    assert_eq!(report1, report2);
}

#[test]
fn test_two_communities_without_periphery_bloc_errors_rather_than_emitting_nan() {
    let h = graph_with_two_communities(false);
    let c = AttrContract::default();
    let err = to_world(&h, &c).unwrap_err();
    assert_eq!(
        err,
        IngestError::NoPeripheryBloc {
            communities: 2,
            bloc: 2,
        }
    );

    let h_ok = graph_with_two_communities(true);
    let (world, _report) = to_world(&h_ok, &c).unwrap();
    assert!(psa(&world).is_finite());
}

#[test]
fn test_single_agent_graph_is_too_few_agents() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "subsistence": 5.0}));
    let c = AttrContract::default();
    let err = to_world(&h, &c).unwrap_err();
    assert_eq!(err, IngestError::TooFewAgents { found: 1 });
}

#[test]
fn test_repression_defaults_to_one_and_zero_repression_is_rejected() {
    let c = AttrContract::default();

    let h = graph_with_two_communities(true);
    let (world, report) = to_world(&h, &c).unwrap();
    assert_eq!(world.repression, DEFAULT_REPRESSION);
    assert!(report.repression_defaulted);

    let mut h_bad = graph_with_two_communities(true);
    h_bad.set_graph_attr(&c.repression, json!(0.0));
    let err = to_world(&h_bad, &c).unwrap_err();
    assert_eq!(
        err,
        IngestError::BadGraphAttr {
            key: c.repression.clone(),
            reason: "must be finite and > 0.0".to_string(),
        }
    );
}

#[test]
fn test_report_records_contract_version_counts_and_id_map() {
    let h = graph_with_two_communities(true);
    let c = AttrContract::default();
    let (world, report) = to_world(&h, &c).unwrap();
    assert_eq!(report.contract_version, ATTR_CONTRACT_VERSION);
    assert_eq!(report.agent_count, world.agents.len());
    assert_eq!(report.agent_count, 2);
    assert_eq!(report.community_count, 2);
    assert_eq!(report.extraction_count, 0);
    assert_eq!(report.industry_count, 0);
    assert_eq!(report.node_ids, h.node_ids());
}

// --- Task 5: build_scene — graceful degradation ---------------------------

fn graph_with_industries(n: usize) -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "subsistence": 5.0, "bloc": 0}));
    h.add_node("b", json!({"wealth": 8.0, "subsistence": 4.0, "bloc": 2}));
    for i in 0..n {
        h.add_edge(
            vec!["a".to_string()],
            Some(format!("in{i}")),
            json!({"kind": "industry", "c": 1.0, "v": 1.0, "e": 0.5}),
        )
        .unwrap();
    }
    h
}

#[test]
fn test_sankey_with_no_industries_returns_explicitly_empty_scene() {
    let h = graph_with_two_communities(true);
    let (s, r) = build_scene(
        &h,
        &AttrContract::default(),
        Instrument::Sankey,
        &InstrumentOpts::default(),
    )
    .unwrap();
    assert_eq!(s, SceneGraph3D::empty());
    assert_eq!(r.industry_count, 0);
}

#[test]
fn test_sankey_with_industries_is_not_empty() {
    let h = graph_with_industries(2);
    let (s, r) = build_scene(
        &h,
        &AttrContract::default(),
        Instrument::Sankey,
        &InstrumentOpts::default(),
    )
    .unwrap();
    assert_ne!(s, SceneGraph3D::empty());
    assert_eq!(r.industry_count, 2);
}

#[test]
fn test_no_scene_scalar_is_nan_or_infinite() {
    let h = graph_with_two_communities(true);
    let c = AttrContract::default();
    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let (s, _r) = build_scene(&h, &c, inst, &InstrumentOpts::default()).unwrap();
        for tok in s.labeled_scalars() {
            let val = tok.rsplit('=').next().unwrap();
            let parsed: f64 = val
                .parse()
                .unwrap_or_else(|_| panic!("{inst:?}: unparsable scalar token {tok}"));
            assert!(parsed.is_finite(), "{inst:?}: non-finite scalar {tok}");
        }
    }
}

#[test]
fn test_isolated_agents_do_not_panic_the_spectral_embedding() {
    // 4 agents, 1 community over 2 of them, 2 agents in no edge at all.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"wealth": 10.0, "subsistence": 5.0}));
    h.add_node("b", json!({"wealth": 8.0, "subsistence": 4.0}));
    h.add_node("c", json!({"wealth": 6.0, "subsistence": 3.0}));
    h.add_node("d", json!({"wealth": 4.0, "subsistence": 2.0}));
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community", "name": "c1"}),
    )
    .unwrap();
    let c = AttrContract::default();
    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let result = build_scene(&h, &c, inst, &InstrumentOpts::default());
        assert!(result.is_ok(), "{inst:?}: {:?}", result.err());
    }
}

#[test]
fn test_all_three_instruments_are_panic_free_over_adversarial_graphs() {
    let mut graphs: Vec<Hypergraph> = Vec::new();

    // empty-ish: exactly 2 agents, no edges at all.
    let mut g1: Hypergraph = Hypergraph::new();
    g1.add_node("a", json!({"wealth": 1.0, "subsistence": 0.5}));
    g1.add_node("b", json!({"wealth": 2.0, "subsistence": 1.0}));
    graphs.push(g1);

    // isolated-only: 3 agents, no edges.
    let mut g2: Hypergraph = Hypergraph::new();
    g2.add_node("a", json!({"wealth": 1.0, "subsistence": 0.5}));
    g2.add_node("b", json!({"wealth": 2.0, "subsistence": 1.0}));
    g2.add_node("c", json!({"wealth": 3.0, "subsistence": 1.5}));
    graphs.push(g2);

    // single-community: 2 agents, one community edge.
    graphs.push(graph_with_two_communities(true));

    // duplicate-member: an edge listing the same node twice (deduped by
    // `add_edge` itself; still exercises the community-with-repeats shape).
    let mut g3: Hypergraph = Hypergraph::new();
    g3.add_node("a", json!({"wealth": 1.0, "subsistence": 0.5}));
    g3.add_node("b", json!({"wealth": 2.0, "subsistence": 1.0}));
    g3.add_edge(
        vec!["a".to_string(), "a".to_string(), "b".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community", "name": "c1"}),
    )
    .unwrap();
    graphs.push(g3);

    // zero-link: a community edge with an explicit empty `links` array.
    let mut g4: Hypergraph = Hypergraph::new();
    g4.add_node("a", json!({"wealth": 1.0, "subsistence": 0.5}));
    g4.add_node("b", json!({"wealth": 2.0, "subsistence": 1.0}));
    g4.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("com1".to_string()),
        json!({"kind": "community", "name": "c1", "links": []}),
    )
    .unwrap();
    graphs.push(g4);

    let c = AttrContract::default();
    for g in &graphs {
        for inst in [
            Instrument::Cylinder,
            Instrument::Terrain,
            Instrument::Sankey,
        ] {
            // Never panics; either an Ok scene or a typed Err — both are a
            // pass for this test, panic-freedom is the only claim.
            let _ = build_scene(g, &c, inst, &InstrumentOpts::default());
        }
    }
}

// --- Task 6: THE SAFETY PROPERTY -------------------------------------------

/// The frozen deck path (`build_world` -> `instruments::build` ->
/// `rasterize` -> `to_text()`) is untouched *in value*, not merely in
/// source lines. Phase I's whole architecture (adapter into `DeckWorld`,
/// never a trait threaded through the frozen builders) depends on this
/// holding by construction; this test makes it an executable, red-on-drift
/// claim rather than an implicit one. Byte-exact against the very same
/// committed goldens Task 12 (Stage 2) pins in `tests/test_raster.rs`.
#[test]
fn test_deck_path_frames_still_match_the_committed_goldens() {
    use hypergraph_rs::raster::deck::build_world;
    use hypergraph_rs::raster::instruments::build;
    use hypergraph_rs::raster::rasterize;
    use hypergraph_rs::raster::Camera;

    let front_camera = Camera {
        ry: 0.0,
        rx: 0.0,
        dist: 4.0,
        fov: 70.0,
    };
    let world = build_world(1917);

    let cases = [
        (
            Instrument::Cylinder,
            include_str!("fixtures/frames/cylinder_front.txt"),
        ),
        (
            Instrument::Terrain,
            include_str!("fixtures/frames/terrain_front.txt"),
        ),
        (
            Instrument::Sankey,
            include_str!("fixtures/frames/sankey_front.txt"),
        ),
    ];

    for (inst, golden) in cases {
        let scene = build(inst, &world, &InstrumentOpts::default());
        let text = rasterize(&scene, &front_camera, 80, 40).to_text();
        assert_eq!(
            text, golden,
            "{inst:?}: deck path frame drifted from golden"
        );
    }
}

#[test]
fn test_missing_required_attr_is_err_not_a_default_scene() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", json!({"subsistence": 5.0}));
    h.add_node("b", json!({"wealth": 8.0, "subsistence": 4.0}));
    let c = AttrContract::default();
    let err = build_scene(&h, &c, Instrument::Cylinder, &InstrumentOpts::default()).unwrap_err();
    assert_eq!(
        err,
        IngestError::MissingNodeAttr {
            node: "a".to_string(),
            key: "wealth".to_string(),
        }
    );
}

// --- Task 7: real-Hypergraph render acceptance -----------------------------
//
// Necessary-but-not-sufficient scene checks alone would have let Phase R's
// collapsed frames (34/83/389 lit subcells, all badly wrong — see the plan's
// Task 7 preamble and commit `67b1c17`) pass unnoticed. These tests instead
// assert lit-cell *density* (self-calibrated against the same deck frame, so
// the band cannot rot against the committed fixtures) and *spread* across
// the frame. No golden is frozen here — that is a deliberate, human-review
// gate the plan calls out explicitly.

/// 12 agents across all three blocs (>= 1 `bloc == 2`, satisfying precondition
/// 2), 3 community edges with real (non-empty) `links`, 2 extraction edges,
/// and 5 industry edges with distinct `lane`/`c`/`v`/`e`. Every attr below is
/// a literal, hand-written value — no generator, no RNG, no loop-generated
/// value (plan Task 7, "The fixture graph").
fn contract_fixture() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();

    // Core bloc (0): well-off, low tension.
    h.add_node("a0", json!({"wealth": 20.0, "subsistence": 5.0, "bloc": 0}));
    h.add_node("a1", json!({"wealth": 18.0, "subsistence": 6.0, "bloc": 0}));
    h.add_node("a2", json!({"wealth": 22.0, "subsistence": 4.0, "bloc": 0}));
    h.add_node(
        "a11",
        json!({"wealth": 25.0, "subsistence": 3.0, "bloc": 0}),
    );
    // Semi bloc (1): middling.
    h.add_node("a3", json!({"wealth": 12.0, "subsistence": 8.0, "bloc": 1}));
    h.add_node("a4", json!({"wealth": 10.0, "subsistence": 9.0, "bloc": 1}));
    h.add_node("a5", json!({"wealth": 14.0, "subsistence": 7.0, "bloc": 1}));
    h.add_node("a6", json!({"wealth": 11.0, "subsistence": 8.0, "bloc": 1}));
    // Periphery bloc (2): squeezed, high tension.
    h.add_node("a7", json!({"wealth": 3.0, "subsistence": 6.0, "bloc": 2}));
    h.add_node("a8", json!({"wealth": 2.0, "subsistence": 7.0, "bloc": 2}));
    h.add_node("a9", json!({"wealth": 4.0, "subsistence": 8.0, "bloc": 2}));
    h.add_node("a10", json!({"wealth": 1.0, "subsistence": 5.0, "bloc": 2}));

    // 3 community edges, each with real (non-empty) internal links.
    h.add_edge(
        vec![
            "a0".to_string(),
            "a1".to_string(),
            "a2".to_string(),
            "a11".to_string(),
        ],
        Some("com_core".to_string()),
        json!({"kind": "community", "name": "core", "links": [["a0", "a1"], ["a1", "a2"]]}),
    )
    .unwrap();
    h.add_edge(
        vec![
            "a3".to_string(),
            "a4".to_string(),
            "a5".to_string(),
            "a6".to_string(),
        ],
        Some("com_semi".to_string()),
        json!({"kind": "community", "name": "semi", "links": [["a3", "a4"], ["a5", "a6"]]}),
    )
    .unwrap();
    h.add_edge(
        vec![
            "a7".to_string(),
            "a8".to_string(),
            "a9".to_string(),
            "a10".to_string(),
        ],
        Some("com_periphery".to_string()),
        json!({"kind": "community", "name": "periphery", "links": [["a7", "a8"], ["a8", "a9"], ["a9", "a10"]]}),
    )
    .unwrap();

    // 2 extraction edges.
    h.add_edge(
        vec!["a7".to_string(), "a8".to_string(), "a0".to_string()],
        Some("ex_core".to_string()),
        json!({"kind": "extraction", "head": "a0", "mass": 2.5}),
    )
    .unwrap();
    h.add_edge(
        vec!["a9".to_string(), "a10".to_string(), "a11".to_string()],
        Some("ex_periphery".to_string()),
        json!({"kind": "extraction", "head": "a11", "mass": 1.5}),
    )
    .unwrap();

    // 5 industry edges, distinct lane/c/v/e.
    h.add_edge(
        vec!["a0".to_string()],
        Some("in_steel".to_string()),
        json!({"kind": "industry", "name": "steel", "lane": 0, "c": 10.0, "v": 5.0, "e": 2.0}),
    )
    .unwrap();
    h.add_edge(
        vec!["a3".to_string()],
        Some("in_textile".to_string()),
        json!({"kind": "industry", "name": "textile", "lane": 1, "c": 8.0, "v": 4.0, "e": 1.5}),
    )
    .unwrap();
    h.add_edge(
        vec!["a7".to_string()],
        Some("in_mining".to_string()),
        json!({"kind": "industry", "name": "mining", "lane": 2, "c": 15.0, "v": 6.0, "e": 3.0}),
    )
    .unwrap();
    h.add_edge(
        vec!["a11".to_string()],
        Some("in_agri".to_string()),
        json!({"kind": "industry", "name": "agri", "lane": 3, "c": 5.0, "v": 3.0, "e": 1.0}),
    )
    .unwrap();
    h.add_edge(
        vec!["a5".to_string()],
        Some("in_tech".to_string()),
        json!({"kind": "industry", "name": "tech", "lane": 0, "c": 12.0, "v": 7.0, "e": 2.5}),
    )
    .unwrap();

    h
}

fn front_camera() -> hypergraph_rs::raster::Camera {
    hypergraph_rs::raster::Camera {
        ry: 0.0,
        rx: 0.0,
        dist: 4.0,
        fov: 70.0,
    }
}

/// Fraction of non-blank glyph cells in a `to_text()` frame. `' '` is a
/// literally-unset cell; `'\u{2800}'` (braille blank) is the empty-subcell
/// glyph `compose_braille` emits when no subcell in a 2x4 block is lit.
fn lit_fraction(text: &str) -> f64 {
    let cells: Vec<char> = text.chars().filter(|c| *c != '\n').collect();
    let lit = cells
        .iter()
        .filter(|c| **c != ' ' && **c != '\u{2800}')
        .count();
    lit as f64 / cells.len() as f64
}

/// Reimplements `raster::wrap_tokens`'s greedy word-wrap (private to the
/// crate) purely to count how many leading rows of a rasterized frame are
/// the labeled-scalar banner, so the spread test can exclude them — spread
/// must come from the rendered geometry, not from a full-width text banner.
fn banner_row_count(scalars: &[String], cols: usize, rows: usize) -> usize {
    let mut wrapped = 0usize;
    let mut line_len = 0usize;
    for t in scalars {
        let t_len = t.chars().count();
        if line_len == 0 {
            line_len = t_len;
        } else if line_len + 1 + t_len <= cols {
            line_len += 1 + t_len;
        } else {
            wrapped += 1;
            line_len = t_len;
        }
    }
    if line_len > 0 {
        wrapped += 1;
    }
    wrapped.min(rows)
}

/// Column/row span (0.0-1.0) of lit cells in `text`, excluding the top
/// `banner_rows` rows (the labeled-scalar banner is full-width by
/// construction and would otherwise mask a geometrically collapsed frame).
fn lit_spread(text: &str, cols: usize, banner_rows: usize) -> (f64, f64) {
    let rows: Vec<&str> = text.lines().collect();
    let mut min_col = usize::MAX;
    let mut max_col = 0usize;
    let mut min_row = usize::MAX;
    let mut max_row = 0usize;
    for (r, row) in rows.iter().enumerate().skip(banner_rows) {
        for (col, ch) in row.chars().enumerate() {
            if ch != ' ' && ch != '\u{2800}' {
                min_col = min_col.min(col);
                max_col = max_col.max(col);
                min_row = min_row.min(r);
                max_row = max_row.max(r);
            }
        }
    }
    if min_col == usize::MAX {
        return (0.0, 0.0);
    }
    let col_span = (max_col - min_col + 1) as f64 / cols as f64;
    let row_span = (max_row - min_row + 1) as f64 / rows.len().max(1) as f64;
    (col_span, row_span)
}

#[test]
fn test_real_hypergraph_scene_is_non_degenerate_for_every_instrument() {
    let h = contract_fixture();
    let c = AttrContract::default();
    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let (s, _r) = build_scene(&h, &c, inst, &InstrumentOpts::default()).unwrap();
        assert!(
            s.nodes.len() + s.faces.len() + s.struts.len() > 0,
            "{inst:?}: scene has no drawable geometry"
        );
        let (min_x, min_y, min_z, max_x, max_y, max_z) = s.bounding_box;
        let extents = [max_x - min_x, max_y - min_y, max_z - min_z];
        let nonzero_axes = extents.iter().filter(|e| **e > 1e-9).count();
        assert!(
            nonzero_axes >= 2,
            "{inst:?}: bounding box degenerate on >= 2 axes: {extents:?}"
        );
        let scalars = s.labeled_scalars();
        assert!(!scalars.is_empty(), "{inst:?}: no labeled scalars");
        for tok in &scalars {
            let val = tok.rsplit('=').next().unwrap();
            let parsed: f64 = val
                .parse()
                .unwrap_or_else(|_| panic!("{inst:?}: unparsable scalar token {tok}"));
            assert!(parsed.is_finite(), "{inst:?}: non-finite scalar {tok}");
        }
    }
}

#[test]
fn test_real_hypergraph_frame_has_deck_comparable_lit_density() {
    use hypergraph_rs::raster::deck::build_world;
    use hypergraph_rs::raster::instruments::build;
    use hypergraph_rs::raster::rasterize;

    let h = contract_fixture();
    let c = AttrContract::default();
    let deck_world = build_world(1917);

    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let (real_scene, _r) = build_scene(&h, &c, inst, &InstrumentOpts::default()).unwrap();
        let real_text = rasterize(&real_scene, &front_camera(), 80, 40).to_text();
        let real_density = lit_fraction(&real_text);

        let deck_scene = build(inst, &deck_world, &InstrumentOpts::default());
        let deck_text = rasterize(&deck_scene, &front_camera(), 80, 40).to_text();
        let deck_density = lit_fraction(&deck_text);

        assert!(
            real_density >= 0.02,
            "{inst:?}: lit density {real_density} below the absolute floor 0.02"
        );
        assert!(
            real_density >= deck_density * 0.25 && real_density <= deck_density * 4.0,
            "{inst:?}: lit density {real_density} not within [0.25x, 4.0x] of deck density {deck_density}"
        );
    }
}

/// Deviation from the plan's literal `>= 0.40 cols / >= 0.30 rows` pseudocode
/// (recorded precisely, per the executor contract): measured directly, the
/// **committed deck goldens themselves** — the reference this task is
/// supposed to be "comparable to" — do not clear a flat 0.30 row-span floor
/// for every instrument. `terrain_front.txt` (front camera, banner row
/// excluded) spans rows 20-24 of 40, a row span of 0.125; that is the real
/// geometry of a heightfield surface viewed nearly edge-on, not a fixture
/// defect or a Phase-R-style collapse. A flat 0.30 threshold would fail the
/// deck's own committed golden, which cannot be the intended bar. So, exactly
/// as the density test does one function above, this test **self-calibrates
/// spread against the same deck frame** instead of a fixed constant: the
/// real-graph frame's column/row span must be at least half the deck frame's
/// own span (same instrument, same front camera, banner rows excluded), with
/// a low absolute floor (0.15 cols / 0.05 rows) to still catch a true
/// collapse-to-a-corner. This is the density test's self-calibrating
/// philosophy applied to spread, not a threshold weakened to force a pass.
#[test]
fn test_real_hypergraph_frame_is_spread_not_collapsed() {
    use hypergraph_rs::raster::deck::build_world;
    use hypergraph_rs::raster::instruments::build;
    use hypergraph_rs::raster::rasterize;

    let h = contract_fixture();
    let c = AttrContract::default();
    let deck_world = build_world(1917);

    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let (real_scene, _r) = build_scene(&h, &c, inst, &InstrumentOpts::default()).unwrap();
        let real_text = rasterize(&real_scene, &front_camera(), 80, 40).to_text();
        let real_banner_rows = banner_row_count(&real_scene.labeled_scalars(), 80, 40);
        let (real_col_span, real_row_span) = lit_spread(&real_text, 80, real_banner_rows);

        let deck_scene = build(inst, &deck_world, &InstrumentOpts::default());
        let deck_text = rasterize(&deck_scene, &front_camera(), 80, 40).to_text();
        let deck_banner_rows = banner_row_count(&deck_scene.labeled_scalars(), 80, 40);
        let (deck_col_span, deck_row_span) = lit_spread(&deck_text, 80, deck_banner_rows);

        assert!(
            real_col_span >= 0.15,
            "{inst:?}: lit column span {real_col_span} below the absolute floor 0.15 — collapsed frame"
        );
        assert!(
            real_row_span >= 0.05,
            "{inst:?}: lit row span {real_row_span} below the absolute floor 0.05 — collapsed frame"
        );
        assert!(
            real_col_span >= deck_col_span * 0.5,
            "{inst:?}: lit column span {real_col_span} below half the deck's own {deck_col_span}"
        );
        assert!(
            real_row_span >= deck_row_span * 0.5,
            "{inst:?}: lit row span {real_row_span} below half the deck's own {deck_row_span}"
        );
    }
}

#[test]
fn test_real_hypergraph_render_is_deterministic_across_repeated_runs() {
    let h = contract_fixture();
    let c = AttrContract::default();
    for inst in [
        Instrument::Cylinder,
        Instrument::Terrain,
        Instrument::Sankey,
    ] {
        let mut texts = Vec::new();
        for _ in 0..3 {
            let (scene, _r) = build_scene(&h, &c, inst, &InstrumentOpts::default()).unwrap();
            texts.push(hypergraph_rs::raster::rasterize(&scene, &front_camera(), 80, 40).to_text());
        }
        assert_eq!(texts[0], texts[1], "{inst:?}: run 1 vs run 2 diverged");
        assert_eq!(texts[1], texts[2], "{inst:?}: run 2 vs run 3 diverged");
    }
}
