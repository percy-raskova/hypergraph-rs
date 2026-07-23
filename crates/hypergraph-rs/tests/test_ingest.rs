#![cfg(feature = "cells3d")]

use hypergraph_rs::raster::ingest::{AttrContract, ATTR_CONTRACT_VERSION, DEFAULT_BLOC, K_WEALTH};

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
