use hypergraph_rs::{DiHypergraph, EdgeError};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_dihypergraph_new_is_empty() {
    let dh: DiHypergraph = DiHypergraph::new();
    assert_eq!(dh.num_nodes(), 0);
    assert_eq!(dh.num_edges(), 0);
}

#[test]
fn test_dihypergraph_add_node() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    assert!(dh.add_node("a", v()));
    assert!(!dh.add_node("a", v()));
    assert!(dh.has_node("a"));
    assert_eq!(dh.num_nodes(), 1);
}

#[test]
fn test_dihypergraph_add_edge_creates_members() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let id = dh
        .add_edge(vec!["1".into(), "2".into()], vec!["3".into()], Some("e0".into()), v())
        .unwrap();
    assert_eq!(id, "e0");
    assert_eq!(dh.num_edges(), 1);
    assert_eq!(dh.num_nodes(), 3);
    assert!(dh.has_edge("e0"));
}

#[test]
fn test_dihypergraph_add_edge_auto_id() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let a = dh.add_edge(vec!["1".into()], vec!["2".into()], None, v()).unwrap();
    let b = dh.add_edge(vec!["3".into()], vec!["4".into()], None, v()).unwrap();
    assert_eq!(a, "0");
    assert_eq!(b, "1");
}

#[test]
fn test_dihypergraph_add_edge_one_sided_allowed() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    // all-senders edge (empty head) is allowed
    let id = dh.add_edge(vec!["3".into(), "4".into()], vec![], Some("e0".into()), v()).unwrap();
    assert_eq!(id, "e0");
    assert_eq!(dh.num_nodes(), 2);
}

#[test]
fn test_dihypergraph_add_edge_both_empty_errors() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let r = dh.add_edge(vec![], vec![], Some("e0".into()), v());
    assert!(matches!(r, Err(EdgeError::EmptyMembers)));
}

#[test]
fn test_dihypergraph_add_edge_duplicate_idx_errors() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v()).unwrap();
    let r = dh.add_edge(vec!["3".into()], vec!["4".into()], Some("e0".into()), v());
    assert!(matches!(r, Err(EdgeError::AlreadyExists { .. })));
    assert_eq!(dh.num_edges(), 1);
}
