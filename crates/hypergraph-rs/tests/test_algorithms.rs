use hypergraph_rs::algorithms::properties::{
    degree_counts, density, incidence_density, is_uniform, max_edge_order, num_edges_order,
    unique_edge_sizes,
};
use hypergraph_rs::Hypergraph;

#[test]
fn test_is_uniform_and_max_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(is_uniform(&h), Some(2)); // all edges order 2
    assert_eq!(max_edge_order(&h), Some(2));
}

#[test]
fn test_is_uniform_false_when_mixed() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(is_uniform(&h), None);
}

#[test]
fn test_unique_edge_sizes_and_num_edges_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(unique_edge_sizes(&h), vec![2, 3]);
    assert_eq!(num_edges_order(&h, Some(1)), 1); // size-2 edges
    assert_eq!(num_edges_order(&h, None), 2);
}

#[test]
fn test_degree_counts() {
    // XGI docstring: edges [[1,2],[2,3,4]] => degree_counts == [0, 3, 1]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["2".into(), "3".into(), "4".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(degree_counts(&h, None), vec![0, 3, 1]);
}

#[test]
fn test_density_and_incidence_density_nonempty() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    // 1 edge, n=2 => density = 1/(2^2 - 1) = 1/3
    assert!((density(&h, None, None, false) - (1.0 / 3.0)).abs() < 1e-12);
    // incidence density: nnz=2, denom = n*num_edges = 2*1 = 2 => 1.0
    assert!((incidence_density(&h, None, None, false) - 1.0).abs() < 1e-12);
}

use hypergraph_rs::algorithms::connected::{
    connected_components, is_connected, largest_connected_component, node_connected_component,
    number_connected_components,
};

fn two_components() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into(), "e".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_number_and_is_connected() {
    let h = two_components();
    assert_eq!(number_connected_components(&h), 2);
    assert!(!is_connected(&h));

    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    assert!(is_connected(&h2));
    assert_eq!(number_connected_components(&h2), 1);
}

#[test]
fn test_connected_components_deterministic() {
    let h = two_components();
    let comps = connected_components(&h);
    assert_eq!(comps.len(), 2);
    assert_eq!(comps[0], vec!["a", "b"]);
    assert_eq!(comps[1], vec!["c", "d", "e"]);
}

#[test]
fn test_largest_and_node_component() {
    let h = two_components();
    assert_eq!(largest_connected_component(&h), vec!["c", "d", "e"]);
    assert_eq!(node_connected_component(&h, "a").unwrap(), vec!["a", "b"]);
    assert!(node_connected_component(&h, "zzz").is_none());
}

#[test]
fn test_rustworkx_delegated_count_matches_handrolled() {
    let h = two_components();
    // number_connected_components delegates to rustworkx_core on the skeleton;
    // it must agree with the hand-rolled component extraction.
    assert_eq!(number_connected_components(&h), connected_components(&h).len());
}

use hypergraph_rs::algorithms::shortest_path::{
    shortest_path_length, single_source_shortest_path_length,
};

#[test]
fn test_single_source_distances() {
    // Path-ish: e0={a,b}, e1={b,c}. a->b=1, a->c=2 (via shared node b).
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let d = single_source_shortest_path_length(&h, "a").unwrap();
    assert_eq!(d["a"], 0.0);
    assert_eq!(d["b"], 1.0);
    assert_eq!(d["c"], 2.0);
}

#[test]
fn test_unreachable_is_infinity() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let d = single_source_shortest_path_length(&h, "a").unwrap();
    assert_eq!(d["b"], 1.0);
    assert!(d["c"].is_infinite());
}

#[test]
fn test_missing_source_none() {
    let h: Hypergraph = Hypergraph::new();
    assert!(single_source_shortest_path_length(&h, "nope").is_none());
}

#[test]
fn test_shortest_path_length_all_sources() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let all = shortest_path_length(&h);
    assert_eq!(all.len(), 2); // one per node
    let (src, dists) = &all[0];
    assert_eq!(src, "a");
    assert_eq!(dists["b"], 1.0);
}

use hypergraph_rs::algorithms::clustering::{
    clustering_coefficient, local_clustering_coefficient, two_node_clustering_coefficient,
};

fn triangle() -> Hypergraph {
    // Three size-2 edges over {0,1,2} form a closed triangle, plus the size-3
    // edge {0,1,2} itself — matches XGI's `random_hypergraph(3,[1,1])`
    // docstring fixture (the source of the 0.5 two-node-CC golden below).
    // Without the triple edge, `_uv_cc`(union) evaluates to 1/3, not 0.5.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "2".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("e3".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_clustering_coefficient_triangle_is_one() {
    let h = triangle();
    let cc = clustering_coefficient(&h);
    for n in ["0", "1", "2"] {
        assert!((cc[n] - 1.0).abs() < 1e-9);
    }
}

#[test]
fn test_clustering_coefficient_no_neighbors_is_zero() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("lonely", serde_json::Value::Null);
    let cc = clustering_coefficient(&h);
    assert_eq!(cc["lonely"], 0.0);
}

#[test]
fn test_two_node_clustering_coefficient_triangle() {
    let h = triangle();
    let cc = two_node_clustering_coefficient(&h, "union");
    // XGI docstring golden (`random_hypergraph(3,[1,1])`, which also contains
    // the triple {0,1,2}): 0.5 each. Without the triple edge, union-CC is 1/3.
    for n in ["0", "1", "2"] {
        assert!((cc[n] - 0.5).abs() < 1e-9);
    }
}

#[test]
fn test_local_clustering_coefficient_defined() {
    let h = triangle();
    let cc = local_clustering_coefficient(&h);
    assert_eq!(cc.len(), 3);
    // each node is in 3 edges here -> value present & finite
    for n in ["0", "1", "2"] {
        assert!(cc[n].is_finite());
    }
}

use hypergraph_rs::algorithms::assortativity::{
    degree_assortativity_exact, dynamical_assortativity, AssortError, AssortKind,
};

#[test]
fn test_dynamical_requires_uniform() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    assert_eq!(dynamical_assortativity(&h), Err(AssortError::NotUniform));
}

#[test]
fn test_dynamical_uniform_regular_is_near_zero() {
    // 3-uniform (edge order 2): two copies of the same triple {0,1,2}, so all
    // node degrees are equal (2) => dynamical assortativity is 0.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let val = dynamical_assortativity(&h).unwrap();
    assert!(val.abs() < 1e-9);
}

#[test]
fn test_degree_assortativity_exact_runs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let rho = degree_assortativity_exact(&h, AssortKind::Uniform).unwrap();
    assert!(rho.is_finite());
    assert!((-1.0..=1.0).contains(&rho));
}

#[test]
fn test_assortativity_empty_errors() {
    let h: Hypergraph = Hypergraph::new();
    assert_eq!(dynamical_assortativity(&h), Err(AssortError::NoNodes));
}

use hypergraph_rs::algorithms::centrality::{clique_eigenvector_centrality, katz_centrality};

fn connected_triangle() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["0".into(), "2".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_katz_l1_normalized_and_symmetric() {
    let h = connected_triangle();
    let c = katz_centrality(&h, 100);
    let sum: f64 = c.values().map(|v| v.abs()).sum();
    assert!((sum - 1.0).abs() < 1e-9); // L1-normalized
    // symmetric triangle => equal centralities
    assert!((c["0"] - c["1"]).abs() < 1e-9);
    assert!((c["1"] - c["2"]).abs() < 1e-9);
}

#[test]
fn test_katz_empty_no_edges_is_zeros() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    let c = katz_centrality(&h, 100);
    assert_eq!(c["a"], 0.0);
}

#[test]
fn test_clique_eigenvector_positive_normalized() {
    let h = connected_triangle();
    let c = clique_eigenvector_centrality(&h, 1e-6, 1000);
    let sum: f64 = c.values().map(|v| v.abs()).sum();
    assert!((sum - 1.0).abs() < 1e-6);
    for v in c.values() {
        assert!(*v > 0.0);
    }
    assert!((c["0"] - c["1"]).abs() < 1e-6);
}

#[test]
fn test_clique_eigenvector_disconnected_is_nan() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let c = clique_eigenvector_centrality(&h, 1e-6, 1000);
    assert!(c["a"].is_nan());
}

use hypergraph_rs::algorithms::simpliciality::{
    edit_simpliciality, face_edit_simpliciality, mean_face_edit_distance,
    simplicial_edit_distance, simplicial_fraction,
};

fn full_simplex() -> Hypergraph {
    // Down-closed on {a,b,c} for sizes >= 2: {a,b},{a,c},{b,c},{a,b,c}
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e3".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_simplicial_fraction_full_simplex_is_one() {
    let h = full_simplex();
    let sf = simplicial_fraction(&h, 2, true);
    assert!((sf - 1.0).abs() < 1e-9);
}

#[test]
fn test_edit_distance_full_simplex_is_zero() {
    let h = full_simplex();
    let d = simplicial_edit_distance(&h, 2, true, true);
    assert!(d.abs() < 1e-9);
    assert!((edit_simpliciality(&h, 2, true) - 1.0).abs() < 1e-9);
}

#[test]
fn test_edit_distance_missing_face_positive() {
    // {a,b,c} present but {b,c} missing => not a full simplex.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let d = simplicial_edit_distance(&h, 2, true, true);
    assert!(d > 0.0);
}

#[test]
fn test_face_edit_simpliciality_full_simplex_is_one() {
    let h = full_simplex();
    assert!((face_edit_simpliciality(&h, 2, true) - 1.0).abs() < 1e-9);
    assert!(mean_face_edit_distance(&h, 2, true, true).abs() < 1e-9);
}
