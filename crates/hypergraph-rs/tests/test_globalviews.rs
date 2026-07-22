use hypergraph_rs::{subhypergraph, Hypergraph};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_subhypergraph_induced_on_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    // keep only nodes {1,2,3}: e0 survives (subset), e1 dropped (4 excluded)
    let sub = subhypergraph(
        &h,
        Some(&["1".to_string(), "2".to_string(), "3".to_string()]),
        None,
        true,
    );
    assert!(sub.has_edge("e0"));
    assert!(!sub.has_edge("e1"));
    assert!(sub.has_node("1"));
    assert!(!sub.has_node("4"));
    assert!(sub.is_frozen());
}

#[test]
fn test_subhypergraph_by_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v())
        .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    let sub = subhypergraph(&h, None, Some(&["e0".to_string()]), true);
    assert!(sub.has_edge("e0"));
    assert!(!sub.has_edge("e1"));
    // all nodes kept by default (keep_isolates=true)
    assert!(sub.has_node("3"));
}

#[test]
fn test_subhypergraph_drop_isolates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v())
        .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    let sub = subhypergraph(&h, None, Some(&["e0".to_string()]), false);
    // e1's nodes 3,4 become isolates and are dropped
    assert!(!sub.has_node("3"));
    assert!(sub.has_node("1"));
}

// D1-ripple(a): a selected empty edge is a trivial subset of any node set and
// must be retained by subhypergraph, not dropped.
#[test]
fn test_subhypergraph_retains_selected_empty_edge() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(Vec::<String>::new(), Some("e0".into()), v())
        .unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("e1".into()), v())
        .unwrap();
    let sub = subhypergraph(&h, None, Some(&["e0".to_string()]), true);
    assert!(sub.has_edge("e0"));
    assert!(!sub.has_edge("e1"));
}
