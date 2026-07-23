#![cfg(feature = "raster")]

use hypergraph_rs::raster::algebra::{deck_industries, transform};

#[test]
fn test_raw_buildworld_r_before_any_tick() {
    let e = transform(&deck_industries());
    assert!((e.r - 0.47102137767220903).abs() < 1e-12); // raw buildWorld() r
    assert!((e.big_s - 198.3).abs() < 1e-9); // S invariant
    assert!((e.cv - 421.0).abs() < 1e-9); // ΣCV
}

#[test]
fn test_net_transfer_conservation_is_zero() {
    let e = transform(&deck_industries());
    let net: f64 = e.t.iter().sum();
    assert!(net.abs() < 1e-12, "ΣT must be 0 (zero-sum), got {net}");
}

#[test]
fn test_phi_equals_rent_sum_at_seed_1917_fixture() {
    // Deviation 6: empirical at these parameters, NOT a structural law.
    let e = transform(&deck_industries());
    assert!((e.phi - (e.rent_imperial + e.rent_internal)).abs() < 1e-9);
}
