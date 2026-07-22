#![cfg(feature = "dynamics")]

use hypergraph_rs::dynamics::compute_kuramoto_order_parameter;

#[test]
fn test_order_param_fully_synchronized_is_one() {
    // All phases equal -> perfect synchrony -> r = 1.
    let theta_time = vec![vec![0.5, 0.5, 0.5], vec![1.2, 1.2, 1.2]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert_eq!(r.len(), 2);
    assert!((r[0] - 1.0).abs() < 1e-12);
    assert!((r[1] - 1.0).abs() < 1e-12);
}

#[test]
fn test_order_param_antiphase_is_zero() {
    use std::f64::consts::PI;
    // Two nodes exactly antiphase -> r = 0.
    let theta_time = vec![vec![0.0, PI]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert!(r[0].abs() < 1e-12);
}

#[test]
fn test_order_param_length_matches_timesteps() {
    let theta_time = vec![vec![0.0, 0.1], vec![0.2, 0.3], vec![0.4, 0.5]];
    let r = compute_kuramoto_order_parameter(&theta_time);
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|&x| (0.0..=1.0 + 1e-12).contains(&x)));
}

use hypergraph_rs::dynamics::simulate_kuramoto;
use hypergraph_rs::Hypergraph;

fn two_edge_hg() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("link".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("tri".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_simulate_uniform_phase_stays_uniform() {
    // Identical phases + zero natural frequency => zero coupling drive => phases frozen.
    let h = two_edge_hg();
    let omega = vec![0.0, 0.0, 0.0];
    let theta = vec![0.5, 0.5, 0.5];
    let (theta_time, _times) =
        simulate_kuramoto(&h, 1.0, 1.0, Some(omega), Some(theta), 5, 0.01, None);
    for row in &theta_time {
        for &x in row {
            assert!((x - 0.5).abs() < 1e-9);
        }
    }
}

#[test]
fn test_simulate_shape() {
    let h = two_edge_hg();
    let (theta_time, times) = simulate_kuramoto(
        &h, 2.0, 3.0, Some(vec![0.1, 0.2, 0.3]), Some(vec![0.0, 1.0, 2.0]), 7, 0.002, None,
    );
    assert_eq!(theta_time.len(), 7);
    assert!(theta_time.iter().all(|r| r.len() == 3));
    assert_eq!(times.len(), 7);
    assert!((times[1] - 0.002).abs() < 1e-12);
}

#[test]
fn test_simulate_deterministic_with_seed() {
    // Defaulted omega/theta drawn from the seeded PCG => identical runs for the same seed.
    let h = two_edge_hg();
    let (a, _) = simulate_kuramoto(&h, 1.0, 1.0, None, None, 4, 0.01, Some(42));
    let (b, _) = simulate_kuramoto(&h, 1.0, 1.0, None, None, 4, 0.01, Some(42));
    assert_eq!(a, b);
}
