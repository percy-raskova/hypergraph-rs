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
