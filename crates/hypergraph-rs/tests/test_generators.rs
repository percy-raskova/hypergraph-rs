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

use hypergraph_rs::generators::{
    complement, complete_hypergraph, empty_hypergraph, trivial_hypergraph,
};
use hypergraph_rs::Hypergraph;

#[test]
fn test_empty_hypergraph() {
    let h = empty_hypergraph();
    assert_eq!(h.num_nodes(), 0);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_trivial_hypergraph() {
    let h = trivial_hypergraph(5);
    assert_eq!(h.num_nodes(), 5);
    assert_eq!(h.num_edges(), 0);
    assert!(h.has_node("0"));
    assert!(h.has_node("4"));
}

#[test]
fn test_complete_hypergraph_single_order() {
    // order=2 -> all 3-subsets of 4 nodes = comb(4,3) = 4 edges
    let h = complete_hypergraph(4, Some(2), None, false).unwrap();
    assert_eq!(h.num_nodes(), 4);
    assert_eq!(h.num_edges(), 4);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_complete_hypergraph_max_order_no_singletons() {
    // max_order=2, no singletons -> subsets of size 2 and 3 of 4 nodes
    // = comb(4,2)+comb(4,3) = 6 + 4 = 10 edges
    let h = complete_hypergraph(4, None, Some(2), false).unwrap();
    assert_eq!(h.num_edges(), 10);
}

#[test]
fn test_complete_hypergraph_max_order_with_singletons() {
    // sizes 1,2,3 of 4 nodes = 4 + 6 + 4 = 14
    let h = complete_hypergraph(4, None, Some(2), true).unwrap();
    assert_eq!(h.num_edges(), 14);
}

#[test]
fn test_complete_hypergraph_rejects_both_none() {
    assert!(complete_hypergraph(4, None, None, false).is_err());
}

#[test]
fn test_complete_hypergraph_rejects_both_some() {
    assert!(complete_hypergraph(4, Some(2), Some(3), false).is_err());
}

#[test]
fn test_complement_disjoint_edge_sets() {
    // H on 4 nodes with one 3-edge {0,1,2}; complement (max size 3) contains
    // every subset of size 1..=3 except {0,1,2}.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_nodes_from((0..4).map(|i| (i.to_string(), serde_json::Value::Null)));
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let hc = complement(&h);
    assert_eq!(hc.num_nodes(), 4);
    // no edge in hc has the same member-set as {0,1,2}
    let has_012 = hc.edge_ids().iter().any(|e| {
        let mut m = hc.members(e).unwrap();
        m.sort();
        m == vec!["0".to_string(), "1".to_string(), "2".to_string()]
    });
    assert!(!has_012, "complement must exclude the original edge");
    assert!(hc.num_edges() > 0);
}

use hypergraph_rs::generators::ring_lattice;

#[test]
fn test_ring_lattice_node_count_and_uniformity() {
    // n=10, d=3, k=4, l=1
    let h = ring_lattice(10, 3, 4, 1).unwrap();
    assert_eq!(h.num_nodes(), 10);
    // each of n nodes contributes k/2 edges -> 10 * 2 = 20 edges
    assert_eq!(h.num_edges(), 20);
    // d-uniform: every edge has <= d members (dedup can shrink)
    for e in h.edge_ids() {
        let sz = h.members(&e).unwrap().len();
        assert!(sz >= 1 && sz <= 3);
    }
}

#[test]
fn test_ring_lattice_is_deterministic() {
    // ring_lattice has no randomness; two builds are structurally identical
    let a = ring_lattice(8, 2, 4, 0).unwrap();
    let b = ring_lattice(8, 2, 4, 0).unwrap();
    assert_eq!(a.num_nodes(), b.num_nodes());
    assert_eq!(a.num_edges(), b.num_edges());
}
