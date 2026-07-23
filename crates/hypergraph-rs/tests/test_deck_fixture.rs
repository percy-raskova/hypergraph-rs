#![cfg(feature = "raster")]

use hypergraph_rs::raster::algebra::{deck_industries, transform};
use hypergraph_rs::raster::deck::{build_world, global_pi0, org_degree, Mulberry32};

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

#[test]
fn test_mulberry32_golden_vector_seed_1917() {
    let mut r = Mulberry32::new(1917);
    let got: Vec<f64> = (0..8).map(|_| r.next()).collect();
    let want = [
        0.9034266413655132,
        0.5877873066347092,
        0.40103743225336075,
        0.3932968541048467,
        0.879000983433798,
        0.6506219764705747,
        0.570820358581841,
        0.25069746002554893,
    ];
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-15, "{g} != {w}");
    }
}

#[test]
fn test_mulberry32_spectral_seed_derivation() {
    // spectral() uses mulberry32(W.seed ^ 0x5EED) = mulberry32(1917 ^ 24301) = mulberry32(22928)
    assert_eq!(1917u32 ^ 0x5EED, 22928);
    let mut r = Mulberry32::new(22928);
    let want = [
        0.17359042004682124,
        0.14775428897701204,
        0.5368493683636189,
        0.9822371983900666,
    ];
    for w in want {
        assert!((r.next() - w).abs() < 1e-15);
    }
}

#[test]
fn test_org_degrees_at_seed_1917() {
    let w = build_world(1917);
    let orgs: Vec<f64> = w.communities.iter().map(org_degree).collect();
    // cell α, cell β, union γ = 0.2 (6 members, 1 link, 5 components); capital δ = 1 (chain).
    assert!((orgs[0] - 0.2).abs() < 1e-12);
    assert!((orgs[1] - 0.2).abs() < 1e-12);
    assert!((orgs[2] - 0.2).abs() < 1e-12);
    assert!((orgs[3] - 1.0).abs() < 1e-12);
}

#[test]
fn test_global_pi0_is_13() {
    assert_eq!(global_pi0(&build_world(1917)), 13);
}
