use hypergraph_rs::{FilterMode, Hypergraph};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_nodeview_len_contains_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.contains("a"));
    assert!(!nodes.contains("z"));
    assert_eq!(
        nodes.ids(),
        &["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn test_nodeview_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_edge(vec!["a".into(), "c".into()], Some("e2".into()), v())
        .unwrap();
    let nodes = h.nodes();
    assert_eq!(nodes.degree("a"), 2);
    assert_eq!(nodes.degree("b"), 1);
}

#[test]
fn test_nodeview_isolates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_node("lonely", v());
    let iso = h.nodes().isolates();
    assert_eq!(iso.ids(), &["lonely".to_string()]);
}

#[test]
fn test_nodeview_isolates_ignoring_singletons() {
    let mut h: Hypergraph = Hypergraph::new();
    // "a" is only in a singleton edge -> counts as isolated when ignoring singletons
    h.add_edge(vec!["a".into()], Some("e1".into()), v())
        .unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v())
        .unwrap();
    let iso = h.nodes().isolates_ignoring_singletons();
    assert!(iso.contains("a"));
    assert!(!iso.contains("b"));
}

#[test]
fn test_nodeview_filterby_degree() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v())
        .unwrap();
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

#[test]
fn test_edgeview_size_and_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    let edges = h.edges();
    assert_eq!(edges.size("e1"), 3);
    assert_eq!(edges.order("e1"), 2);
}

#[test]
fn test_edgeview_singletons_and_empty() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), v())
        .unwrap();
    assert_eq!(h.edges().singletons().ids(), &["e2".to_string()]);
    assert!(h.edges().empty().is_empty());
}

#[test]
fn test_edgeview_filterby_size() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["a".into(), "b".into()], Some("e2".into()), v())
        .unwrap();
    let big = h.edges().filterby_size(3, FilterMode::Geq);
    assert_eq!(big.ids(), &["e1".to_string()]);
}

#[test]
fn test_edgeview_duplicates() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "2".into()],
        Some("1".into()),
        v(),
    )
    .unwrap();
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("2".into()),
        v(),
    )
    .unwrap();
    // ids 0 and 2 share member-set {0,1,2}; first-sorted (0) kept, 2 is a duplicate
    assert_eq!(h.edges().duplicates().ids(), &["2".to_string()]);
}

#[test]
fn test_edgeview_lookup() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["0".into(), "1".into(), "2".into()],
        Some("0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("1".into()), v())
        .unwrap();
    let found = h.edges().lookup(&["3".to_string(), "4".to_string()]);
    assert_eq!(found.ids(), &["1".to_string()]);
}

#[test]
fn test_edgeview_maximal_nonstrict() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v())
        .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("2".into()), v())
        .unwrap();
    let mut ids = h.edges().maximal(false).ids().to_vec();
    ids.sort();
    // {1,2} is a strict subset of {1,2,3} -> not maximal
    assert_eq!(ids, vec!["0".to_string(), "2".to_string()]);
}

#[test]
fn duplicate_groups_in_first_occurrence_order() {
    let mut h: Hypergraph = Hypergraph::new();
    for (id, m) in [
        ("e0", vec!["a", "b"]),
        ("e1", vec!["c", "d"]),
        ("e2", vec!["b", "a"]),
        ("e3", vec!["d", "c"]),
        ("e4", vec!["x"]),
    ] {
        h.add_edge(
            m.into_iter().map(String::from).collect(),
            Some(id.into()),
            v(),
        )
        .unwrap();
    }
    assert_eq!(
        h.edges().duplicate_groups(),
        vec![vec!["e0", "e2"], vec!["e1", "e3"]]
    );
    assert!(
        Hypergraph::<serde_json::Value, serde_json::Value, serde_json::Value>::new()
            .edges()
            .duplicate_groups()
            .is_empty()
    );
}

#[test]
fn duplicate_groups_of_nodes_use_membership_sets() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e0".into()), v())
        .unwrap();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    assert_eq!(h.nodes().duplicate_groups(), vec![vec!["a", "b"]]);
}
