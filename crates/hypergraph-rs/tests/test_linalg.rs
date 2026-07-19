use hypergraph_rs::linalg::{incidence_matrix, incidence_matrix_sparse};
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
