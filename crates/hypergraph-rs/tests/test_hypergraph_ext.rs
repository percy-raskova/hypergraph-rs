use hypergraph_rs::Hypergraph;

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn add_empty_edge_creates_memberless_hyperedge_and_returns_id() {
    use hypergraph_rs::Hypergraph;
    use serde_json::Value;
    let mut h: Hypergraph<Value, Value, Value> = Hypergraph::new();
    // Auto-id path: returns the assigned id ("0"), edge exists, has zero members.
    let id = h.add_empty_edge(None, Value::Null);
    assert_eq!(id, "0");
    assert!(h.has_edge(&id));
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.num_nodes(), 0); // no members => no agent nodes
    assert_eq!(h.members(&id).map(|m| m.len()), Some(0));
    // Explicit-id path: returned id echoes the request; counter still advances past numeric ids.
    let id2 = h.add_empty_edge(Some("e7".to_string()), Value::Null);
    assert_eq!(id2, "e7");
    assert!(h.has_edge("e7"));
    // Auto-id after an explicit non-numeric id continues the uid counter (parity with add_edge).
    let id3 = h.add_empty_edge(None, Value::Null);
    assert_eq!(id3, "1");
}

#[test]
fn test_remove_edges_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), v())
        .unwrap();
    h.add_edge(vec!["c".into(), "d".into()], Some("e3".into()), v())
        .unwrap();
    h.remove_edges_from(&["e1".to_string(), "e3".to_string()]);
    assert_eq!(h.num_edges(), 1);
    assert!(h.has_edge("e2"));
    assert!(!h.has_edge("e1"));
}

#[test]
fn test_remove_edges_from_ignores_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("e1".into()), v())
        .unwrap();
    h.remove_edges_from(&["e1".to_string(), "nope".to_string()]);
    assert_eq!(h.num_edges(), 0);
}

#[test]
fn test_remove_nodes_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    h.remove_nodes_from(&["a".to_string(), "c".to_string()], false, true);
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
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_node("lonely", v());
    assert_eq!(h.isolate_node_ids(), vec!["lonely".to_string()]);
}

#[test]
fn test_singleton_edge_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), v())
        .unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), v())
        .unwrap();
    assert_eq!(h.singleton_edge_ids(), vec!["e2".to_string()]);
}

#[test]
fn test_graph_attrs_accessor() {
    let mut h: Hypergraph = Hypergraph::new();
    h.set_graph_attr("name", serde_json::json!("wayne"));
    assert_eq!(
        h.graph_attrs().get("name"),
        Some(&serde_json::json!("wayne"))
    );
    assert_eq!(h.graph_attrs().len(), 1);
}

#[test]
fn test_dual_swaps_nodes_and_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into()],
        Some("e1".into()),
        serde_json::json!({"w": 1}),
    )
    .unwrap();
    h.add_edge(
        vec!["b".into(), "c".into()],
        Some("e2".into()),
        serde_json::json!({"w": 2}),
    )
    .unwrap();

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
    h.add_edge(vec!["a".into()], Some("e1".into()), v())
        .unwrap();
    h.add_node("lonely", v());
    let d = h.dual();
    // "lonely" has no memberships -> would be an empty dual-edge -> dropped
    assert!(!d.has_edge("lonely"));
    assert!(d.has_edge("a"));
}

use hypergraph_rs::SwapError;

#[test]
fn test_double_edge_swap_basic() {
    // XGI docstring example: H([[1,2,3],[3,4]]); swap(1,4,e0,e1) -> [{2,3,4},{1,3}]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    h.double_edge_swap("1", "4", "e0", "e1").unwrap();
    let mut m0 = h.members("e0").unwrap();
    m0.sort();
    assert_eq!(m0, vec!["2".to_string(), "3".to_string(), "4".to_string()]);
    let mut m1 = h.members("e1").unwrap();
    m1.sort();
    assert_eq!(m1, vec!["1".to_string(), "3".to_string()]);
}

#[test]
fn test_double_edge_swap_node_not_in_edge_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v())
        .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    // node "9" is not a member of e0
    let r = h.double_edge_swap("9", "3", "e0", "e1");
    assert!(matches!(r, Err(SwapError::IdNotFound { .. })));
}

#[test]
fn test_double_edge_swap_size_not_preserved_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    // 2 is in BOTH edges; moving 1 into e1 collides with nothing, but moving 2's
    // counterpart would shrink. Construct a genuine collision: swap 1 and 2 where
    // 2 is already in e0.
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), v())
        .unwrap();
    h.add_edge(vec!["2".into(), "3".into()], Some("e1".into()), v())
        .unwrap();
    // swap n1=1 (in e0) and n2=2 (in e1): e0 would become {2,2}->{2} (shrink)
    let r = h.double_edge_swap("1", "2", "e0", "e1");
    assert!(matches!(r, Err(SwapError::SizeNotPreserved)));
    // graph unchanged
    assert_eq!(h.members("e0").unwrap().len(), 2);
}

#[test]
fn test_double_edge_swap_missing_edge_errors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into()], Some("e0".into()), v())
        .unwrap();
    let r = h.double_edge_swap("1", "1", "e0", "nope");
    assert!(matches!(r, Err(SwapError::IdNotFound { .. })));
}

#[test]
fn test_random_edge_shuffle_preserves_sizes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
        .unwrap();
    let s0 = h.members("e0").unwrap().len();
    let s1 = h.members("e1").unwrap().len();
    h.random_edge_shuffle(Some("e0"), Some("e1"), 42).unwrap();
    assert_eq!(h.members("e0").unwrap().len(), s0);
    assert_eq!(h.members("e1").unwrap().len(), s1);
}

#[test]
fn test_random_edge_shuffle_preserves_combined_node_multiset() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "5".into()],
        Some("e1".into()),
        v(),
    )
    .unwrap();
    let before: std::collections::BTreeSet<String> = h
        .members("e0")
        .unwrap()
        .into_iter()
        .chain(h.members("e1").unwrap())
        .collect();
    h.random_edge_shuffle(Some("e0"), Some("e1"), 7).unwrap();
    let after: std::collections::BTreeSet<String> = h
        .members("e0")
        .unwrap()
        .into_iter()
        .chain(h.members("e1").unwrap())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn test_random_edge_shuffle_is_reproducible() {
    let build = || {
        let mut h: Hypergraph = Hypergraph::new();
        h.add_edge(
            vec!["1".into(), "2".into(), "3".into()],
            Some("e0".into()),
            v(),
        )
        .unwrap();
        h.add_edge(vec!["3".into(), "4".into()], Some("e1".into()), v())
            .unwrap();
        h
    };
    let mut h1 = build();
    let mut h2 = build();
    h1.random_edge_shuffle(Some("e0"), Some("e1"), 99).unwrap();
    h2.random_edge_shuffle(Some("e0"), Some("e1"), 99).unwrap();
    let mut a = h1.members("e0").unwrap();
    a.sort();
    let mut b = h2.members("e0").unwrap();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn test_random_edge_shuffle_needs_two_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into()], Some("e0".into()), v())
        .unwrap();
    assert!(h.random_edge_shuffle(None, None, 1).is_err());
}

use hypergraph_rs::{MergeRename, MergeRule};

#[test]
fn test_merge_duplicate_edges_rename_first() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into()],
        Some("0".into()),
        serde_json::json!({"color": "blue"}),
    )
    .unwrap();
    h.add_edge(
        vec!["1".into(), "2".into()],
        Some("1".into()),
        serde_json::json!({"color": "red"}),
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "5".into()],
        Some("3".into()),
        v(),
    )
    .unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::First, None);
    // {1,2} duplicates (ids 0,1) merge into id "0" with first's attrs
    assert!(h.has_edge("0"));
    assert!(!h.has_edge("1"));
    assert!(h.has_edge("3"));
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.edge_attrs("0").unwrap()["color"], "blue");
}

#[test]
fn test_merge_duplicate_edges_multiplicity() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v())
        .unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v())
        .unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("2".into()), v())
        .unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::First, Some("mult"));
    assert_eq!(h.edge_attrs("0").unwrap()["mult"], 3);
    assert_eq!(h.num_edges(), 1);
}

#[test]
fn test_merge_duplicate_edges_intersection() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["3".into(), "4".into()],
        Some("0".into()),
        serde_json::json!({"color": "purple"}),
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into()],
        Some("1".into()),
        serde_json::json!({"color": "purple", "name": "test"}),
    )
    .unwrap();
    h.merge_duplicate_edges(MergeRename::First, MergeRule::Intersection, None);
    // color agrees -> kept; name differs (missing in one) -> null
    assert_eq!(h.edge_attrs("0").unwrap()["color"], "purple");
    assert_eq!(h.edge_attrs("0").unwrap()["name"], serde_json::Value::Null);
}

#[test]
fn test_cleanup_removes_isolates_singletons_and_multiedges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v())
        .unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), v())
        .unwrap(); // duplicate
    h.add_edge(vec!["3".into()], Some("2".into()), v()).unwrap(); // singleton
    h.add_node("lonely", v()); // isolate
    h.cleanup(hypergraph_rs::CleanupOptions::default());
    assert!(!h.has_node("lonely")); // isolate removed
    assert!(!h.has_edge("2")); // singleton removed
    assert_eq!(h.num_edges(), 1); // duplicate merged
}

#[test]
fn test_cleaned_leaves_original_untouched() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("0".into()), v())
        .unwrap();
    h.add_node("lonely", v());
    let c = h.cleaned(hypergraph_rs::CleanupOptions {
        singletons: true,
        multiedges: true,
        empties: true,
        ..Default::default()
    }); // remove isolates only
    assert!(!c.has_node("lonely"));
    assert!(h.has_node("lonely")); // original unchanged
}
