use hypergraph_rs::{FilterMode, Hypergraph};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_nodeview_len_contains_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.contains("a"));
    assert!(!nodes.contains("z"));
    assert_eq!(nodes.ids(), &["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_nodeview_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e2".into()), v()).unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.degree("a"), 2);
    assert_eq!(nodes.degree("b"), 1);
}

#[test]
fn test_nodeview_isolates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v()).unwrap();
    h.add_node("lonely", v());
    let iso = h.nodes().isolates();
    assert_eq!(iso.ids(), &["lonely".to_string()]);
}

#[test]
fn test_nodeview_isolates_ignoring_singletons() {
    let mut h: Hypergraph = Hypergraph::new();
    // "a" is only in a singleton edge -> counts as isolated when ignoring singletons
    h.add_edge(vec!["a".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    let iso = h.nodes().isolates_ignoring_singletons();
    assert!(iso.contains("a"));
    assert!(!iso.contains("b"));
}

#[test]
fn test_nodeview_filterby_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into(), "c".into()], Some("e1".into()), v()).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v()).unwrap();
    // degrees: a=1, b=2, c=2
    let deg2 = h.nodes().filterby_degree(2, FilterMode::Eq);
    let mut ids = deg2.ids().to_vec();
    ids.sort();
    assert_eq!(ids, vec!["b".to_string(), "c".to_string()]);
    let lt2 = h.nodes().filterby_degree(2, FilterMode::Lt);
    assert_eq!(lt2.ids(), &["a".to_string()]);
}

#[test]
fn test_nodeview_attrs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"color": "red"}));
    assert_eq!(h.nodes().attrs("a").unwrap()["color"], "red");
    assert!(h.nodes().attrs("missing").is_none());
}
