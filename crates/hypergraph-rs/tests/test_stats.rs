use hypergraph_rs::stats::nodestats::{node_attrs, node_degree};
use hypergraph_rs::Hypergraph;

fn triangle_plus() -> Hypergraph {
    // XGI docstring fixture: [[1,2,3],[2,3,4,5],[3,4,5]]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into(), "4".into(), "5".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "5".into()],
        Some("2".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_node_degree_matches_xgi() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // XGI: {1:1, 2:2, 3:3, 4:2, 5:2}
    assert_eq!(*deg.get("1").unwrap(), 1);
    assert_eq!(*deg.get("2").unwrap(), 2);
    assert_eq!(*deg.get("3").unwrap(), 3);
    assert_eq!(*deg.get("4").unwrap(), 2);
    assert_eq!(*deg.get("5").unwrap(), 2);
}

#[test]
fn test_node_degree_by_order() {
    let h = triangle_plus();
    // order=2 -> only size-3 edges (edges 0 and 2)
    let deg = node_degree(&h, Some(2));
    assert_eq!(*deg.get("3").unwrap(), 2); // 3 is in both size-3 edges
    assert_eq!(*deg.get("2").unwrap(), 1); // 2 only in edge 0 (size 3)
}

#[test]
fn test_node_degree_aslist_order() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // node insertion order is 1,2,3,4,5
    assert_eq!(deg.aslist(), vec![1, 2, 3, 2, 2]);
}

#[test]
fn test_node_degree_filterby() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    let mut two = deg.filterby(&2);
    two.sort();
    assert_eq!(two, vec!["2".to_string(), "4".to_string(), "5".to_string()]);
}

#[test]
fn test_node_attrs_single() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"color": "red"}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let colors = node_attrs(&h, Some("color"));
    assert_eq!(colors.get("a").unwrap(), &serde_json::json!("red"));
    assert_eq!(colors.get("b").unwrap(), &serde_json::json!("blue"));
}

#[test]
fn test_node_attrs_missing_imputes_null() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"age": 20}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let ages = node_attrs(&h, Some("age"));
    assert_eq!(ages.get("a").unwrap(), &serde_json::json!(20));
    assert_eq!(ages.get("b").unwrap(), &serde_json::Value::Null);
}
