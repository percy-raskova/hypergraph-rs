use hypergraph_rs::linalg::{
    adjacency_matrix, clique_motif_matrix, incidence_matrix, incidence_matrix_sparse,
};
use hypergraph_rs::Hypergraph;
use ndarray::array;

fn sample() -> Hypergraph {
    // Nodes added first (1..=7) to pin row order; XGI docstring example.
    let mut h: Hypergraph = Hypergraph::new();
    for n in ["1", "2", "3", "4", "5", "6", "7"] {
        h.add_node(n, serde_json::Value::Null);
    }
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into(), "7".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(vec!["4".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(
        vec!["5".into(), "6".into(), "7".into()],
        Some("e2".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_incidence_matrix_values_and_order() {
    let h = sample();
    let (i, rows, cols) = incidence_matrix(&h, None);
    assert_eq!(rows, vec!["1", "2", "3", "4", "5", "6", "7"]);
    assert_eq!(cols, vec!["e0", "e1", "e2"]);
    // XGI docstring golden (hypergraph_matrix.py:37-43).
    let expected = array![
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
    ];
    assert_eq!(i, expected);
}

#[test]
fn test_incidence_matrix_order_filter() {
    let h = sample();
    // order=2 keeps only the size-3 edge e2.
    let (i, _rows, cols) = incidence_matrix(&h, Some(2));
    assert_eq!(cols, vec!["e2"]);
    assert_eq!(i.shape(), &[7, 1]);
    assert_eq!(i.column(0).sum(), 3.0);
}

#[test]
fn test_incidence_matrix_empty() {
    let h: Hypergraph = Hypergraph::new();
    let (i, rows, cols) = incidence_matrix(&h, None);
    assert_eq!(i.shape(), &[0, 0]);
    assert!(rows.is_empty());
    assert!(cols.is_empty());
}

#[test]
fn test_incidence_matrix_sparse_matches_dense() {
    let h = sample();
    let (dense, _, _) = incidence_matrix(&h, None);
    let (sparse, _, _) = incidence_matrix_sparse(&h, None);
    assert_eq!(sparse.to_dense(), dense);
}

#[test]
fn test_adjacency_binary() {
    let h = sample();
    let (a, rows) = adjacency_matrix(&h, None, 1, false);
    assert_eq!(rows, vec!["1", "2", "3", "4", "5", "6", "7"]);
    // node 7 (index 6) shares edges with 1,2,3 (via e0) and 5,6 (via e2).
    assert_eq!(a[[6, 0]], 1.0); // 7-1 connected
    assert_eq!(a[[6, 4]], 1.0); // 7-5 connected
    assert_eq!(a[[0, 0]], 0.0); // diagonal zeroed
    assert_eq!(a[[3, 0]], 0.0); // node 4 isolated from node 1
}

#[test]
fn test_adjacency_weighted_counts_shared_edges() {
    // Two edges both containing {a, b} => weight 2 between them.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let (a, rows) = adjacency_matrix(&h, None, 1, true);
    let ia = rows.iter().position(|x| x == "a").unwrap();
    let ib = rows.iter().position(|x| x == "b").unwrap();
    assert_eq!(a[[ia, ib]], 2.0);
}

#[test]
fn test_adjacency_s_threshold() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    // s=2: only pairs sharing >= 2 edges remain. a-b share 2; a-c share 1.
    let (a, rows) = adjacency_matrix(&h, None, 2, false);
    let ia = rows.iter().position(|x| x == "a").unwrap();
    let ib = rows.iter().position(|x| x == "b").unwrap();
    let ic = rows.iter().position(|x| x == "c").unwrap();
    assert_eq!(a[[ia, ib]], 1.0);
    assert_eq!(a[[ia, ic]], 0.0);
}

#[test]
fn test_adjacency_no_edges_returns_zeros_n_by_n() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);
    let (a, rows) = adjacency_matrix(&h, None, 1, false);
    assert_eq!(a.shape(), &[2, 2]);
    assert_eq!(rows, vec!["a", "b"]);
    assert_eq!(a.sum(), 0.0);
}

#[test]
fn test_clique_motif_equals_weighted_adjacency() {
    let h = sample();
    let (w, _) = clique_motif_matrix(&h);
    let (a, _) = adjacency_matrix(&h, None, 1, true);
    assert_eq!(w, a);
}
