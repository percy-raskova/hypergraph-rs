#![cfg(feature = "communities")]

use hypergraph_rs::communities::{kmeans, normalized_laplacian};
use hypergraph_rs::Hypergraph;
use ndarray::Array2;

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

use hypergraph_rs::communities::smallest_k_eigenvectors;

#[test]
fn test_eigen_returns_k_columns() {
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let x = smallest_k_eigenvectors(&l, 2, Some(1));
    assert_eq!(x.nrows(), 6);
    assert_eq!(x.ncols(), 2);
}

#[test]
fn test_eigen_nullspace_separates_two_components() {
    // Two disconnected triangles => the 2-dim smallest-eigenvalue subspace separates
    // components: rows a,b,c share coordinates distinct from d,e,f.
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let x = smallest_k_eigenvectors(&l, 2, Some(1));
    // Distance within a component is ~0; across components is not.
    let dist = |i: usize, j: usize| -> f64 {
        ((x[[i, 0]] - x[[j, 0]]).powi(2) + (x[[i, 1]] - x[[j, 1]]).powi(2)).sqrt()
    };
    // Rows 0,1,2 = a,b,c ; rows 3,4,5 = d,e,f (node insertion order).
    assert!(dist(0, 1) < 1e-6, "a,b within component should coincide");
    assert!(dist(0, 2) < 1e-6, "a,c within component should coincide");
    assert!(dist(3, 4) < 1e-6, "d,e within component should coincide");
    assert!(dist(0, 3) > 1e-3, "components should be separated");
}

#[test]
fn test_eigen_deterministic_with_seed() {
    let h = two_triangles();
    let (l, _) = normalized_laplacian(&h).unwrap();
    let a = smallest_k_eigenvectors(&l, 2, Some(7));
    let b = smallest_k_eigenvectors(&l, 2, Some(7));
    assert_eq!(a, b);
}

#[test]
fn test_kmeans_separates_two_clusters() {
    // Two well-separated 1-D point clusters near 0 and near 10.
    let mut x = Array2::<f64>::zeros((6, 1));
    for (i, &v) in [0.0, 0.1, 0.2, 10.0, 10.1, 10.2].iter().enumerate() {
        x[[i, 0]] = v;
    }
    let labels = kmeans(&x, 2, 100, Some(42));
    assert_eq!(labels.len(), 6);
    // The first three share a label, the last three share the other label.
    assert_eq!(labels[0], labels[1]);
    assert_eq!(labels[1], labels[2]);
    assert_eq!(labels[3], labels[4]);
    assert_eq!(labels[4], labels[5]);
    assert_ne!(labels[0], labels[3]);
}

#[test]
fn test_kmeans_k1_all_zero() {
    let x = Array2::<f64>::zeros((4, 2));
    let labels = kmeans(&x, 1, 100, Some(1));
    assert_eq!(labels, vec![0, 0, 0, 0]);
}

#[test]
fn test_kmeans_deterministic_with_seed() {
    let mut x = Array2::<f64>::zeros((6, 1));
    for (i, &v) in [0.0, 0.1, 0.2, 10.0, 10.1, 10.2].iter().enumerate() {
        x[[i, 0]] = v;
    }
    let a = kmeans(&x, 2, 100, Some(5));
    let b = kmeans(&x, 2, 100, Some(5));
    assert_eq!(a, b);
}
