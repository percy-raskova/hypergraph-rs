#![cfg(feature = "cells3d")]

use hypergraph_rs::raster::ingest::{
    ingest_nodes, AttrContract, IngestError, TensionSource, ATTR_CONTRACT_VERSION, DEFAULT_BLOC,
    DEFAULT_REPRESSION, K_WEALTH,
};
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
