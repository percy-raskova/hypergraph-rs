use hypergraph_rs::Hypergraph;

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_remove_edges_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e3".into()), v()).unwrap();
    h.remove_edges_from(&["e1".to_string(), "e3".to_string()]);
    assert_eq!(h.num_edges(), 1);
    assert!(h.has_edge("e2"));
    assert!(!h.has_edge("e1"));
}

#[test]
fn test_remove_edges_from_ignores_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.remove_edges_from(&["e1".to_string(), "nope".to_string()]);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_remove_nodes_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    h.remove_nodes_from(&["a".to_string(), "c".to_string()]);
    assert!(!h.has_node("a"));
    assert!(!h.has_node("c"));
    assert!(h.has_node("b"));
    assert_eq!(h.members("e1").unwrap(), vec!["b".to_string()]);
}

#[test]
fn test_update_adds_nodes_and_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.update(
        vec![("solo".to_string(), v())],
        vec![(vec!["a".into(), "b".into()], Some("e1".into()), v())],
    );
    assert!(h.has_node("solo"));
    assert!(h.has_edge("e1"));
    assert_eq!(h.num_nodes(), 3);
}

#[test]
fn test_isolate_node_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    assert_eq!(h.isolate_node_ids(), vec!["lonely".to_string()]);
}

#[test]
fn test_singleton_edge_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), v()).unwrap();
    assert_eq!(h.singleton_edge_ids(), vec!["e2".to_string()]);
}

#[test]
fn test_graph_attrs_accessor() {
    let mut h: Hypergraph = Hypergraph::new();
    h.set_graph_attr("name", serde_json::json!("wayne"));
    assert_eq!(h.graph_attrs().get("name"), Some(&serde_json::json!("wayne")));
    assert_eq!(h.graph_attrs().len(), 1);
}

#[test]
fn test_dual_swaps_nodes_and_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::json!({"w": 1})).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::json!({"w": 2})).unwrap();

    let d = h.dual();
    // original edges e1, e2 become dual nodes carrying the edge attrs
    assert!(d.has_node("e1"));
    assert!(d.has_node("e2"));
    assert_eq!(d.node_attrs("e1").unwrap()["w"], 1);
    // original nodes become dual edges whose members are the memberships
    assert!(d.has_edge("a"));
    assert!(d.has_edge("b"));
    assert_eq!(d.members("a").unwrap(), vec!["e1".to_string()]);
    let mut b_members = d.members("b").unwrap();
    b_members.sort();
    assert_eq!(b_members, vec!["e1".to_string(), "e2".to_string()]);
}

#[test]
fn test_dual_drops_isolate_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    let d = h.dual();
    // "lonely" has no memberships -> would be an empty dual-edge -> dropped
    assert!(!d.has_edge("lonely"));
    assert!(d.has_edge("a"));
}
