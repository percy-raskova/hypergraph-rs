#![cfg(feature = "communities")]

use hypergraph_rs::communities::normalized_laplacian;
use hypergraph_rs::Hypergraph;

fn two_triangles() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("t1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["d".into(), "e".into(), "f".into()], Some("t2".into()), serde_json::Value::Null).unwrap();
    h
}

#[test]
fn test_laplacian_dims_and_rowdict() {
    let h = two_triangles();
    let (l, rowdict) = normalized_laplacian(&h).unwrap();
    assert_eq!(l.nrows(), 6);
    assert_eq!(l.ncols(), 6);
    assert_eq!(rowdict.len(), 6);
    assert_eq!(rowdict[0], "a");
}

#[test]
fn test_laplacian_diagonal_is_identity_minus_selfterm() {
    // For a single simplex-triangle, each node has degree 1; the diagonal of L is
    // 1 - (1/edge_size) = 1 - 1/3 = 2/3.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("t".into()), serde_json::Value::Null).unwrap();
    let (l, _) = normalized_laplacian(&h).unwrap();
    for i in 0..3 {
        assert!((l[[i, i]] - (2.0 / 3.0)).abs() < 1e-12);
    }
}

#[test]
fn test_laplacian_isolated_node_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e".into()), serde_json::Value::Null).unwrap();
    h.add_node("lonely", serde_json::Value::Null); // degree 0
    assert!(normalized_laplacian(&h).is_err());
}
