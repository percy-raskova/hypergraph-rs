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
