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
fn test_comb_100_15_matches_oracle() {
    // FINDING 2 oracle: python -c "from math import comb; print(comb(100,15))"
    assert_eq!(comb(100, 15), 253_338_471_349_988_640);
}

#[test]
fn test_comb_large_but_legal_does_not_panic_and_matches_oracle() {
    // FINDING 2: comb(100, 16) and comb(100, 17) both FIT in u64 (oracle
    // values below), but the old u64-accumulator multiplicative form
    // (`acc * (n - i)` before dividing by `(i + 1)`) overflows u64 mid-loop
    // even though the final result fits. Oracle: `python -c "from math import
    // comb; print(comb(100,16)); print(comb(100,17))"`.
    assert_eq!(comb(100, 16), 1_345_860_629_046_814_650);
    assert_eq!(comb(100, 17), 6_650_134_872_937_201_800);
}

#[test]
fn test_comb_beyond_u64_saturates_without_panicking() {
    // comb(100, 18) == 30_664_510_802_988_208_300 (oracle), which exceeds
    // u64::MAX (18_446_744_073_709_551_615). Must saturate, not panic/wrap.
    assert_eq!(comb(100, 18), u64::MAX);
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
        assert!((1..=3).contains(&sz));
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

use hypergraph_rs::generators::{star_clique, sunflower};

#[test]
fn test_star_clique_node_count() {
    // n_star=6, n_clique=7 -> 13 nodes
    let h = star_clique(6, 7, 2).unwrap();
    assert_eq!(h.num_nodes(), 13);
    assert!(h.num_edges() > 0);
}

#[test]
fn test_star_clique_rejects_bad_dmax() {
    // d_max must be <= n_clique - 1
    assert!(star_clique(6, 7, 7).is_err());
    assert!(star_clique(0, 7, 2).is_err());
}

#[test]
fn test_sunflower_is_m_uniform() {
    // l=3 petals, c=2 core, m=4 edge size
    let h = sunflower(3, 2, 4).unwrap();
    assert_eq!(h.num_edges(), 3);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 4);
    }
}

#[test]
fn test_sunflower_rejects_m_less_than_c() {
    assert!(sunflower(3, 5, 4).is_err());
}

use hypergraph_rs::generators::{fast_random_hypergraph, random_hypergraph};

#[test]
fn test_fast_random_hypergraph_node_count() {
    let h = fast_random_hypergraph(50, &[0.1, 0.01], None, Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 50);
}

#[test]
fn test_fast_random_hypergraph_is_deterministic() {
    let a = fast_random_hypergraph(30, &[0.2, 0.05], None, Some(99)).unwrap();
    let b = fast_random_hypergraph(30, &[0.2, 0.05], None, Some(99)).unwrap();
    assert_eq!(a.num_edges(), b.num_edges());
    assert_eq!(a.edge_ids().len(), b.edge_ids().len());
    // exact structural equality (same seed -> same graph)
    assert!(a == b);
}

#[test]
fn test_fast_random_hypergraph_p_one_is_complete_at_order() {
    // order=[1], ps=[1.0] -> every 2-subset of n nodes = comb(n,2)
    let h = fast_random_hypergraph(5, &[1.0], Some(&[1]), Some(3)).unwrap();
    assert_eq!(h.num_edges(), 10); // comb(5,2)
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 2);
    }
}

#[test]
fn test_fast_random_edge_orders_match_requested() {
    // Only 3-node edges requested (order=[2]); every edge has 3 members.
    let h = fast_random_hypergraph(20, &[0.5], Some(&[2]), Some(5)).unwrap();
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_fast_random_hypergraph_order_exceeding_n_yields_no_edges_of_that_order() {
    // n=3, order=[10] -> requested edge size is 11 > n=3, so comb(3, 11) == 0.
    // XGI's max_index is comb(n, d+1, exact=True) - 1 == -1, so the geometric-skip
    // loop (index starts at >= 0) never runs: zero edges of that order, no panic.
    // FINDING 1: previously `comb(n, d+1).saturating_sub(1)` produced max_index=0
    // instead of -1, driving `index_to_edge_comb` past `n` and overflowing `comb`'s
    // u64 accumulator.
    let h = fast_random_hypergraph(3, &[0.9], Some(&[10]), Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 3);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_random_hypergraph_node_count_and_determinism() {
    let a = random_hypergraph(15, &[0.3, 0.05], None, Some(7)).unwrap();
    let b = random_hypergraph(15, &[0.3, 0.05], None, Some(7)).unwrap();
    assert_eq!(a.num_nodes(), 15);
    assert!(a == b);
}

#[test]
fn test_random_hypergraph_rejects_bad_probability() {
    assert!(random_hypergraph(10, &[1.5], Some(&[1]), Some(1)).is_err());
}

use hypergraph_rs::generators::{chung_lu_hypergraph, watts_strogatz_hypergraph};
use indexmap::IndexMap;

#[test]
fn test_chung_lu_node_count_and_determinism() {
    let mut k1: IndexMap<String, f64> = IndexMap::new();
    let mut k2: IndexMap<String, f64> = IndexMap::new();
    for i in 0..20 {
        k1.insert(i.to_string(), (1 + (i % 5)) as f64);
        k2.insert(i.to_string(), (1 + (i % 5)) as f64);
    }
    let a = chung_lu_hypergraph(&k1, &k2, Some(4));
    let b = chung_lu_hypergraph(&k1, &k2, Some(4));
    assert_eq!(a.num_nodes(), 20);
    assert!(a == b);
}

#[test]
fn test_watts_strogatz_preserves_node_count() {
    let h = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    assert_eq!(h.num_nodes(), 12);
    assert!(h.num_edges() > 0);
}

#[test]
fn test_watts_strogatz_is_deterministic() {
    let a = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    let b = watts_strogatz_hypergraph(12, 3, 4, 1, 0.3, Some(11)).unwrap();
    assert!(a == b);
}

use hypergraph_rs::generators::uniform::PType;
use hypergraph_rs::generators::{
    uniform_erdos_renyi_hypergraph, uniform_hypergraph_configuration_model,
};

#[test]
fn test_uniform_er_is_m_uniform() {
    let h = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(2)).unwrap();
    assert_eq!(h.num_nodes(), 30);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_uniform_er_p_zero_is_empty_edges() {
    let h = uniform_erdos_renyi_hypergraph(10, 2, 0.0, PType::Prob, false, Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 10);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_uniform_er_p_one_is_complete() {
    // q==1, no multiedges -> complete_hypergraph(n, order=m-1) = comb(n, m) edges
    let h = uniform_erdos_renyi_hypergraph(5, 2, 1.0, PType::Prob, false, Some(1)).unwrap();
    assert_eq!(h.num_edges(), 10); // comb(5,2)
}

#[test]
fn test_uniform_er_is_deterministic() {
    let a = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(9)).unwrap();
    let b = uniform_erdos_renyi_hypergraph(30, 3, 0.05, PType::Prob, false, Some(9)).unwrap();
    assert!(a == b);
}

#[test]
fn test_uniform_er_rejects_bad_probability() {
    assert!(uniform_erdos_renyi_hypergraph(10, 2, 2.0, PType::Prob, false, Some(1)).is_err());
}

#[test]
fn test_uniform_er_edge_size_exceeding_n_yields_no_edges() {
    // n=3, m=5 (edge size) > n=3, so comb(3, 5) == 0. Same FINDING 1 class as
    // fast_random_hypergraph: must not panic, must yield zero edges.
    let h = uniform_erdos_renyi_hypergraph(3, 5, 0.9, PType::Prob, false, Some(1)).unwrap();
    assert_eq!(h.num_nodes(), 3);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_configuration_model_is_m_uniform() {
    let mut k: IndexMap<String, usize> = IndexMap::new();
    for i in 0..30 {
        k.insert(i.to_string(), 3);
    }
    let h = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    assert_eq!(h.num_nodes(), 30);
    for e in h.edge_ids() {
        assert_eq!(h.members(&e).unwrap().len(), 3);
    }
}

#[test]
fn test_configuration_model_is_deterministic() {
    let mut k: IndexMap<String, usize> = IndexMap::new();
    for i in 0..30 {
        k.insert(i.to_string(), 3);
    }
    let a = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    let b = uniform_hypergraph_configuration_model(&k, 3, Some(5));
    assert!(a == b);
}

use hypergraph_rs::generators::random_simplicial_complex;

#[test]
fn test_random_simplicial_complex_node_count() {
    let s = random_simplicial_complex(20, &[0.1, 0.01], Some(3)).unwrap();
    assert_eq!(s.num_nodes(), 20);
}

#[test]
fn test_random_simplicial_complex_is_deterministic() {
    let a = random_simplicial_complex(15, &[0.2, 0.05], Some(8)).unwrap();
    let b = random_simplicial_complex(15, &[0.2, 0.05], Some(8)).unwrap();
    assert_eq!(a.num_nodes(), b.num_nodes());
    assert_eq!(a.num_edges(), b.num_edges());
}

#[test]
fn test_random_simplicial_complex_rejects_bad_probability() {
    assert!(random_simplicial_complex(10, &[1.5], Some(1)).is_err());
}
