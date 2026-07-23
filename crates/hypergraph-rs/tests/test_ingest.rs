#![cfg(feature = "cells3d")]

use hypergraph_rs::raster::deck::psa;
use hypergraph_rs::raster::ingest::{
    build_scene, ingest_edges, ingest_nodes, to_world, AttrContract, IngestError, TensionSource,
    ATTR_CONTRACT_VERSION, DEFAULT_BLOC, DEFAULT_LANE, DEFAULT_MASS, DEFAULT_REPRESSION, K_WEALTH,
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
    let (agents, _ids, _h3, bloc_defaulted, _tsrc) =
        ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
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
    let (agents, ids, _h3, _bd, _tsrc) = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
    assert_eq!(ids, h.node_ids());
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
    let (_agents, _ids, h3_cells, _bd, _tsrc) = ingest_nodes(&h, &c, DEFAULT_REPRESSION).unwrap();
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
    let (agents, _ids, _h3, _bd, tsrc) = ingest_nodes(&h, &c, repression).unwrap();
    assert_eq!(tsrc, TensionSource::Attr);
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
