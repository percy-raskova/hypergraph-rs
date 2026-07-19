use hypergraph_rs::generators::rng::{
    choice_without_replacement, comb, geometric, index_to_edge_comb, index_to_edge_prod,
    random_below, seeded_rng, uniform_f64,
};

#[test]
fn test_seeded_rng_is_deterministic() {
    let mut a = seeded_rng(Some(42));
    let mut b = seeded_rng(Some(42));
    let seq_a: Vec<f64> = (0..10).map(|_| uniform_f64(&mut a)).collect();
    let seq_b: Vec<f64> = (0..10).map(|_| uniform_f64(&mut b)).collect();
    assert_eq!(seq_a, seq_b);
}

#[test]
fn test_different_seeds_diverge() {
    let mut a = seeded_rng(Some(1));
    let mut b = seeded_rng(Some(2));
    let va: Vec<f64> = (0..10).map(|_| uniform_f64(&mut a)).collect();
    let vb: Vec<f64> = (0..10).map(|_| uniform_f64(&mut b)).collect();
    assert!(va != vb);
}

#[test]
fn test_uniform_f64_in_unit_interval() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        let x = uniform_f64(&mut r);
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn test_random_below_in_range() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        let x = random_below(&mut r, 5);
        assert!(x < 5);
    }
}

#[test]
fn test_choice_without_replacement_distinct() {
    let mut r = seeded_rng(Some(7));
    let picks = choice_without_replacement(&mut r, 10, 4);
    assert_eq!(picks.len(), 4);
    let mut sorted = picks.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 4, "picks must be distinct");
    assert!(picks.iter().all(|&i| i < 10));
}

#[test]
fn test_geometric_is_at_least_one() {
    let mut r = seeded_rng(Some(7));
    for _ in 0..1000 {
        assert!(geometric(0.3, &mut r) >= 1);
    }
    // p == 1 always yields exactly 1
    assert_eq!(geometric(1.0, &mut r), 1);
}

#[test]
fn test_comb_values() {
    assert_eq!(comb(4, 3), 4);
    assert_eq!(comb(5, 2), 10);
    assert_eq!(comb(10, 0), 1);
    assert_eq!(comb(3, 5), 0);
    assert_eq!(comb(52, 5), 2_598_960);
}

#[test]
fn test_index_to_edge_comb_is_bijective_and_sorted() {
    // For n=4, m=3 there are comb(4,3)=4 combinations, indices 0..4.
    let all: Vec<Vec<usize>> = (0..4).map(|i| index_to_edge_comb(i, 4, 3)).collect();
    // each is strictly increasing (a valid combination)
    for e in &all {
        assert_eq!(e.len(), 3);
        assert!(e.windows(2).all(|w| w[0] < w[1]));
        assert!(e.iter().all(|&x| x < 4));
    }
    // all distinct
    let mut set = all.clone();
    set.sort();
    set.dedup();
    assert_eq!(set.len(), 4);
}

#[test]
fn test_index_to_edge_prod_shape() {
    // n=4, m=3: index 3 -> (0,0,3) per the XGI docstring example.
    assert_eq!(index_to_edge_prod(3, 4, 3), vec![0, 0, 3]);
    assert_eq!(index_to_edge_prod(0, 4, 3), vec![0, 0, 0]);
}
