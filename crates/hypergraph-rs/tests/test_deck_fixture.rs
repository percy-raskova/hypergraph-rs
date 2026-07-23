#![cfg(feature = "raster")]

use hypergraph_rs::raster::algebra::{deck_industries, transform};
use hypergraph_rs::raster::deck::{
    build_world, do_tick, global_pi0, org_degree, periphery_mean_wealth, psa, psr, Mulberry32,
};

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

// ---------- Task 10: Stage 0 — deck-world reproduction ----------

#[test]
fn test_displayed_r_after_one_startup_tick() {
    let mut world = build_world(1917);
    do_tick(&mut world); // the deck runs doTick() once before first paint (:499)
    assert!((transform(&world.industries).r - 0.4608359187088229).abs() < 1e-12);
}

#[test]
fn test_falling_r_sequence_six_ticks() {
    let want = [
        0.4608359187088229,
        0.45077630191919216,
        0.44084464189389844,
        0.43104290735742473,
        0.4213729206846403,
        0.4118363577727333,
    ];
    let mut world = build_world(1917);
    for w in want {
        do_tick(&mut world);
        assert!((transform(&world.industries).r - w).abs() < 1e-12);
    }
}

#[test]
fn test_s_invariant_under_ticks() {
    let mut world = build_world(1917);
    for _ in 0..6 {
        do_tick(&mut world);
        assert!((transform(&world.industries).big_s - 198.3).abs() < 1e-9);
    }
}

#[test]
fn test_net_t_zero_every_tick_and_both_zooms() {
    // ΣT == 0 at tick1..6 (fine) — r=S/CV construction makes this hold for
    // any industry state, ticked or not. The COARSE-zoom half of this
    // invariant (partition-independent, Deviation 7) is already asserted
    // over the raw fixture by Task 9's `test_net_t_zero_at_both_zooms`
    // (`tests/test_instruments.rs`, `cells3d`-gated); nothing about
    // aggregation depends on whether the industries were ticked.
    let mut world = build_world(1917);
    for _ in 0..6 {
        do_tick(&mut world);
        let net: f64 = transform(&world.industries).t.iter().sum();
        assert!(net.abs() < 1e-9, "ΣT must be 0 at every tick, got {net}");
    }
}

#[test]
fn test_org_and_pi0_and_psa_golden() {
    // Built on build_world(1917) then one do_tick (Finding 1: the wealth
    // extraction step happens BEFORE the readouts are ever displayed).
    let mut world = build_world(1917);
    do_tick(&mut world);

    assert_eq!(global_pi0(&world), 13);

    let orgs: Vec<f64> = world.communities.iter().map(org_degree).collect();
    assert!((orgs[0] - 0.2).abs() < 1e-12);
    assert!((orgs[1] - 0.2).abs() < 1e-12);
    assert!((orgs[2] - 0.2).abs() < 1e-12);
    assert!((orgs[3] - 1.0).abs() < 1e-12);
    let mean_org: f64 = orgs.iter().sum::<f64>() / orgs.len() as f64;
    assert!((mean_org - 0.4).abs() < 1e-12);

    let pw = periphery_mean_wealth(&world);
    assert!((pw - 1.8250060787286078).abs() < 1e-12); // POST-tick, byte-exact (pure arithmetic)

    let psr_v = psr(&world);
    assert!((psr_v - 0.22222222222222215).abs() < 1e-12); // pure division, byte-exact

    // psa = sigmoid(pw-3.0) routes through exp() => tolerance-bounded, NOT
    // byte-exact (user CLAUDE.md §4 / Finding 3): a faithful Python
    // re-derivation gives 0.23595349607058722 — last-ULP drift.
    let psa_v = psa(&world);
    assert!((psa_v - 0.23595349607058727).abs() < 1e-12);

    // rupture NOT armed (psr<=psa).
    assert!(psr_v <= psa_v);
}
