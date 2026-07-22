use hypergraph_rs::Hypergraph;

#[test]
fn test_add_node_creates_new_node() {
    let mut h: Hypergraph = Hypergraph::new();
    let created = h.add_node("a", serde_json::Value::Null);
    assert!(created);
    assert_eq!(h.num_nodes(), 1);
    assert!(h.has_node("a"));
}

#[test]
fn test_add_node_returns_false_for_existing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    let created = h.add_node("a", serde_json::Value::Null);
    assert!(!created);
    assert_eq!(h.num_nodes(), 1);
}

#[test]
fn test_has_node_returns_false_for_missing() {
    let h: Hypergraph = Hypergraph::new();
    assert!(!h.has_node("nonexistent"));
}

#[test]
fn test_num_nodes_starts_at_zero() {
    let h: Hypergraph = Hypergraph::new();
    assert_eq!(h.num_nodes(), 0);
}

use hypergraph_rs::EdgeError;

#[test]
fn test_add_edge_with_explicit_idx() {
    let mut h: Hypergraph = Hypergraph::new();
    let edge_id = h
        .add_edge(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            Some("myedge".to_string()),
            serde_json::Value::Null,
        )
        .unwrap();
    assert_eq!(edge_id, "myedge");
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.num_nodes(), 3);
}

#[test]
fn test_add_edge_auto_generates_id() {
    let mut h: Hypergraph = Hypergraph::new();
    let id1 = h
        .add_edge(
            vec!["a".to_string(), "b".to_string()],
            None,
            serde_json::Value::Null,
        )
        .unwrap();
    assert_eq!(id1, "0");
    let id2 = h
        .add_edge(
            vec!["c".to_string(), "d".to_string()],
            None,
            serde_json::Value::Null,
        )
        .unwrap();
    assert_eq!(id2, "1");
}

#[test]
fn test_add_edge_duplicate_idx_returns_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let result = h.add_edge(
        vec!["b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    );
    assert!(matches!(result, Err(EdgeError::AlreadyExists { .. })));
    assert_eq!(h.num_edges(), 1);
}

#[test]
fn test_add_edge_deduplicates_members() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(h.num_nodes(), 2);
}

#[test]
fn test_add_edge_empty_members_creates_edge() {
    // D1: XGI runtime CREATES empty edges (the docstring claim of an
    // error is inaccurate — see the divergence register).
    let mut h: Hypergraph = Hypergraph::new();
    let result = h.add_edge(vec![], None, serde_json::Value::Null);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "0");
    assert_eq!(h.num_edges(), 1);
    assert_eq!(h.num_nodes(), 0);
}

#[test]
fn test_add_edge_auto_id_after_explicit_idx() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("5".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let next = h
        .add_edge(vec!["b".to_string()], None, serde_json::Value::Null)
        .unwrap();
    assert_eq!(next, "6");
}

#[test]
fn test_has_edge_returns_true_for_existing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert!(h.has_edge("e1"));
}

#[test]
fn test_has_edge_returns_false_for_missing() {
    let h: Hypergraph = Hypergraph::new();
    assert!(!h.has_edge("nonexistent"));
}

#[test]
fn test_num_edges_counts_correctly() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["c".to_string()],
        Some("e3".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(h.num_edges(), 3);
}

#[test]
fn test_memberships_returns_edge_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let mships = h.memberships("a").unwrap();
    assert_eq!(mships.len(), 2);
    assert!(mships.contains(&"e1".to_string()));
    assert!(mships.contains(&"e2".to_string()));
}

#[test]
fn test_memberships_returns_empty_for_isolate() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("lonely", serde_json::Value::Null);
    assert!(h.memberships("lonely").unwrap().is_empty());
}

#[test]
fn test_memberships_returns_none_for_missing() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.memberships("nonexistent").is_none());
}

#[test]
fn test_members_returns_node_ids() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let members = h.members("e1").unwrap();
    assert_eq!(members.len(), 3);
    assert!(members.contains(&"a".to_string()));
}

#[test]
fn test_members_returns_none_for_missing() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.members("nonexistent").is_none());
}

#[test]
fn test_node_ids_insertion_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("c", serde_json::Value::Null);
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);
    assert_eq!(h.node_ids(), vec!["c", "a", "b"]);
}

#[test]
fn test_edge_ids_insertion_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["x".to_string()],
        Some("e3".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["x".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["x".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(h.edge_ids(), vec!["e3", "e1", "e2"]);
}

#[test]
fn test_node_ids_empty() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.node_ids().is_empty());
}

use hypergraph_rs::NodeError;

#[test]
fn test_remove_node_weak_removes_from_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node("b", false, true).unwrap();
    assert!(!h.has_node("b"));
    assert!(h.has_edge("e1"));
    let members = h.members("e1").unwrap();
    assert!(!members.contains(&"b".to_string()));
    assert!(members.contains(&"a".to_string()));
}

#[test]
fn test_remove_node_weak_removes_singleton_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node("a", false, true).unwrap();
    assert!(!h.has_node("a"));
    assert!(!h.has_edge("e1"));
}

#[test]
fn test_remove_node_strong_removes_all_containing_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "c".to_string(), "d".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node("a", true, true).unwrap();
    assert!(!h.has_node("a"));
    assert!(!h.has_edge("e1"));
    assert!(!h.has_edge("e2"));
    assert!(h.has_node("b"));
    assert!(h.has_node("c"));
}

#[test]
fn test_remove_node_missing_returns_error() {
    let mut h: Hypergraph = Hypergraph::new();
    let result = h.remove_node("nonexistent", false, true);
    assert!(matches!(result, Err(NodeError::NotFound { .. })));
}

#[test]
fn test_remove_node_preserves_insertion_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);
    h.add_node("c", serde_json::Value::Null);
    h.remove_node("b", false, true).unwrap();
    assert_eq!(h.node_ids(), vec!["a", "c"]);
}

#[test]
fn test_remove_node_weak_remove_empty_false_keeps_emptied_edge() {
    // D9 (Phase 2 Task 2, probed): weak removal with remove_empty=False
    // leaves an emptied edge IN PLACE — still listed, queryable as
    // members(e) == Some(vec![]), num_edges unchanged.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node("b", false, false).unwrap();
    assert!(!h.has_node("b"));
    assert_eq!(h.num_edges(), 2);
    assert!(h.has_edge("e2"));
    assert_eq!(h.members("e2"), Some(vec![])); // emptied, alive
    assert_eq!(h.members("e1").unwrap(), vec!["a"]);
    assert_eq!(h.memberships("a").unwrap(), vec!["e1"]);
}

#[test]
fn test_remove_node_strong_ignores_remove_empty_false() {
    // Probed: strong mode removes every incident edge REGARDLESS of
    // remove_empty — the flag is irrelevant in strong mode.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "c".to_string(), "d".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["c".to_string(), "d".to_string()],
        Some("e4".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node("a", true, false).unwrap();
    assert!(!h.has_node("a"));
    assert!(!h.has_edge("e1"));
    assert!(!h.has_edge("e2"));
    assert!(h.has_edge("e4"));
    assert_eq!(h.members("e4").unwrap(), vec!["c", "d"]);
}

#[test]
fn test_remove_edge_basic() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_edge("e1").unwrap();
    assert!(!h.has_edge("e1"));
    assert_eq!(h.num_edges(), 0);
    assert!(h.has_node("a"));
    assert!(h.has_node("b"));
    assert!(h.memberships("a").unwrap().is_empty());
}

#[test]
fn test_remove_edge_missing_returns_error() {
    let mut h: Hypergraph = Hypergraph::new();
    let result = h.remove_edge("nonexistent");
    assert!(matches!(result, Err(EdgeError::NotFound { .. })));
}

#[test]
fn test_remove_edge_preserves_other_edges() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_edge("e1").unwrap();
    assert!(!h.has_edge("e1"));
    assert!(h.has_edge("e2"));
    assert_eq!(h.memberships("a").unwrap(), vec!["e2"]);
}

#[test]
fn test_remove_edge_preserves_insertion_order() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".to_string()],
        Some("e3".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_edge("e2").unwrap();
    assert_eq!(h.edge_ids(), vec!["e1", "e3"]);
}

#[test]
fn test_node_attrs_read() {
    let mut h: Hypergraph = Hypergraph::new();
    let attrs = serde_json::json!({"color": "red", "weight": 42});
    h.add_node("a", attrs.clone());
    let read = h.node_attrs("a").unwrap();
    assert_eq!(read, &attrs);
}

#[test]
fn test_node_attrs_mut() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"count": 0}));
    {
        let attrs = h.node_attrs_mut("a").unwrap();
        attrs["count"] = serde_json::json!(5);
    }
    assert_eq!(h.node_attrs("a").unwrap()["count"], 5);
}

#[test]
fn test_node_attrs_missing_returns_none() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.node_attrs("nonexistent").is_none());
}

#[test]
fn test_edge_attrs_read() {
    let mut h: Hypergraph = Hypergraph::new();
    let attrs = serde_json::json!({"heat": 0.5, "cohesion": 0.8});
    h.add_edge(vec!["a".to_string()], Some("e1".to_string()), attrs.clone())
        .unwrap();
    assert_eq!(h.edge_attrs("e1").unwrap(), &attrs);
}

#[test]
fn test_edge_attrs_mut() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"heat": 0.3}),
    )
    .unwrap();
    {
        let attrs = h.edge_attrs_mut("e1").unwrap();
        attrs["heat"] = serde_json::json!(0.9);
    }
    assert_eq!(h.edge_attrs("e1").unwrap()["heat"], 0.9);
}

#[test]
fn test_edge_attrs_missing_returns_none() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.edge_attrs("nonexistent").is_none());
}

#[test]
fn test_graph_attr_set_and_get() {
    let mut h: Hypergraph = Hypergraph::new();
    h.set_graph_attr("name", serde_json::json!("wayne_county"));
    h.set_graph_attr("tick", serde_json::json!(42));
    assert_eq!(
        h.graph_attr("name"),
        Some(&serde_json::json!("wayne_county"))
    );
    assert_eq!(h.graph_attr("tick"), Some(&serde_json::json!(42)));
}

#[test]
fn test_graph_attr_missing_returns_none() {
    let h: Hypergraph = Hypergraph::new();
    assert!(h.graph_attr("nonexistent").is_none());
}

#[test]
fn test_graph_attr_overwrite() {
    let mut h: Hypergraph = Hypergraph::new();
    h.set_graph_attr("tick", serde_json::json!(1));
    h.set_graph_attr("tick", serde_json::json!(99));
    assert_eq!(h.graph_attr("tick"), Some(&serde_json::json!(99)));
}

#[test]
fn test_clear_removes_everything() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"heat": 0.5}),
    )
    .unwrap();
    h.add_node("lonely", serde_json::json!({"x": 1}));
    h.set_graph_attr("name", serde_json::json!("test"));
    h.clear();
    assert_eq!(h.num_nodes(), 0);
    assert_eq!(h.num_edges(), 0);
    assert!(h.node_ids().is_empty());
    assert!(h.edge_ids().is_empty());
    assert!(h.graph_attr("name").is_none());
}

#[test]
fn test_copy_produces_independent_clone() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"heat": 0.5}),
    )
    .unwrap();
    h.set_graph_attr("name", serde_json::json!("test"));

    let mut h2 = h.copy();
    assert_eq!(h2.num_nodes(), 2);
    assert_eq!(h2.num_edges(), 1);
    assert_eq!(h2.edge_attrs("e1").unwrap()["heat"], 0.5);

    h2.add_node("c", serde_json::Value::Null);
    h2.set_graph_attr("name", serde_json::json!("modified"));
    assert_eq!(h.num_nodes(), 2);
    assert!(!h.has_node("c"));
    assert_eq!(h.graph_attr("name"), Some(&serde_json::json!("test")));
}

#[test]
fn test_copy_of_empty() {
    let h: Hypergraph = Hypergraph::new();
    let h2 = h.copy();
    assert_eq!(h2.num_nodes(), 0);
    assert_eq!(h2.num_edges(), 0);
}

#[test]
fn test_add_nodes_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_nodes_from(vec![
        ("a".to_string(), serde_json::json!({"x": 1})),
        ("b".to_string(), serde_json::json!({"x": 2})),
    ]);
    assert_eq!(h.num_nodes(), 2);
    assert_eq!(h.node_attrs("a").unwrap()["x"], 1);
}

#[test]
fn test_add_edges_from_bulk() {
    let mut h: Hypergraph = Hypergraph::new();
    let results = h.add_edges_from(vec![
        (
            vec!["a".to_string(), "b".to_string()],
            Some("e1".to_string()),
            serde_json::json!({"w": 1}),
        ),
        (
            vec!["b".to_string(), "c".to_string()],
            None,
            serde_json::Value::Null,
        ),
    ]);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 3);
    assert_eq!(h.edge_ids(), vec!["e1", "0"]);
}

#[test]
fn test_add_edges_from_with_duplicate_idx() {
    let mut h: Hypergraph = Hypergraph::new();
    let results = h.add_edges_from(vec![
        (
            vec!["a".to_string()],
            Some("e1".to_string()),
            serde_json::Value::Null,
        ),
        (
            vec!["b".to_string()],
            Some("e1".to_string()),
            serde_json::Value::Null,
        ),
    ]);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert_eq!(h.num_edges(), 1);
}

#[test]
fn test_eq_same_structure() {
    let mut h1: Hypergraph = Hypergraph::new();
    h1.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"w": 1}),
    )
    .unwrap();
    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"w": 1}),
    )
    .unwrap();
    assert!(h1 == h2);
}

#[test]
fn test_eq_different_edge_attrs() {
    let mut h1: Hypergraph = Hypergraph::new();
    h1.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"w": 1}),
    )
    .unwrap();
    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"w": 2}),
    )
    .unwrap();
    assert!(h1 != h2);
}

#[test]
fn test_eq_different_members() {
    let mut h1: Hypergraph = Hypergraph::new();
    h1.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let mut h2: Hypergraph = Hypergraph::new();
    h2.add_edge(
        vec!["a".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert!(h1 != h2);
}

#[test]
fn test_eq_both_empty() {
    let h1: Hypergraph = Hypergraph::new();
    let h2: Hypergraph = Hypergraph::new();
    assert!(h1 == h2);
}

#[test]
fn test_add_node_to_edge_existing_edge() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_node_to_edge("e1", "c").unwrap();
    let members = h.members("e1").unwrap();
    assert!(members.contains(&"c".to_string()));
    assert_eq!(members.len(), 3);
}

#[test]
fn test_add_node_to_edge_auto_creates_edge_and_node() {
    let mut h: Hypergraph = Hypergraph::new();
    // Neither edge "new_edge" nor node "new_node" exist
    h.add_node_to_edge("new_edge", "new_node").unwrap();
    assert!(h.has_edge("new_edge"));
    assert!(h.has_node("new_node"));
    let members = h.members("new_edge").unwrap();
    assert_eq!(members, vec!["new_node"]);
}

#[test]
fn test_remove_node_from_edge_basic() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node_from_edge("e1", "b", true).unwrap();
    let members = h.members("e1").unwrap();
    assert!(!members.contains(&"b".to_string()));
    assert_eq!(members.len(), 2);
    // Node b still exists (just not in e1 anymore)
    assert!(h.has_node("b"));
}

#[test]
fn test_remove_node_from_edge_removes_empty_edge() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node_from_edge("e1", "a", true).unwrap();
    // e1 was a singleton, now empty — should be removed
    assert!(!h.has_edge("e1"));
    assert!(h.has_node("a"));
}

#[test]
fn test_remove_node_from_edge_keep_empty() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.remove_node_from_edge("e1", "a", false).unwrap();
    // remove_empty=false: edge stays (now empty)
    assert!(h.has_edge("e1"));
    assert!(h.members("e1").unwrap().is_empty());
}

#[test]
fn test_remove_node_from_edge_missing_edge_returns_error() {
    let mut h: Hypergraph = Hypergraph::new();
    let result = h.remove_node_from_edge("nonexistent", "a", true);
    assert!(result.is_err());
}

#[test]
fn test_remove_node_from_edge_node_not_in_edge_returns_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_node("b", serde_json::Value::Null);
    let result = h.remove_node_from_edge("e1", "b", true);
    assert!(result.is_err());
}

// D2: MembershipError family for remove_node_from_edge (Task 6).
use hypergraph_rs::MembershipError;

#[test]
fn test_remove_node_from_edge_missing_edge_returns_membership_error() {
    let mut h: Hypergraph = Hypergraph::new();
    let err = h.remove_node_from_edge("e1", "a", true).unwrap_err();
    assert_eq!(
        err,
        MembershipError::EdgeNotFound {
            edge_id: "e1".to_string()
        }
    );
    assert_eq!(format!("{err}"), "edge e1 does not exist");
}

#[test]
fn test_remove_node_from_edge_missing_node_returns_membership_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let err = h.remove_node_from_edge("e1", "ghost", true).unwrap_err();
    assert_eq!(
        err,
        MembershipError::NodeNotFound {
            node_id: "ghost".to_string()
        }
    );
    assert_eq!(format!("{err}"), "node ghost does not exist");
}

#[test]
fn test_remove_node_from_edge_node_not_a_member_returns_membership_error() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_node("b", serde_json::Value::Null);
    let err = h.remove_node_from_edge("e1", "b", true).unwrap_err();
    assert_eq!(
        err,
        MembershipError::NotAMember {
            node_id: "b".to_string(),
            edge_id: "e1".to_string()
        }
    );
    assert_eq!(format!("{err}"), "node b is not a member of edge e1");
}

#[test]
fn test_set_node_attributes_from_pairs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.add_node("b", serde_json::Value::Null);

    let mut attrs_a = serde_json::Map::new();
    attrs_a.insert("color".to_string(), serde_json::json!("red"));
    let mut attrs_b = serde_json::Map::new();
    attrs_b.insert("color".to_string(), serde_json::json!("blue"));

    h.set_node_attributes(vec![("a".to_string(), attrs_a), ("b".to_string(), attrs_b)]);

    assert_eq!(h.node_attrs("a").unwrap()["color"], "red");
    assert_eq!(h.node_attrs("b").unwrap()["color"], "blue");
}

#[test]
fn test_set_node_attributes_skips_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);

    let mut attrs = serde_json::Map::new();
    attrs.insert("x".to_string(), serde_json::json!(1));

    h.set_node_attributes(vec![
        ("a".to_string(), attrs.clone()),
        ("nonexistent".to_string(), attrs),
    ]);

    // "a" got the attribute, "nonexistent" was silently skipped
    assert_eq!(h.node_attrs("a").unwrap()["x"], 1);
}

#[test]
fn test_set_edge_attributes_from_pairs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();

    let mut attrs_e1 = serde_json::Map::new();
    attrs_e1.insert("weight".to_string(), serde_json::json!(5));
    let mut attrs_e2 = serde_json::Map::new();
    attrs_e2.insert("weight".to_string(), serde_json::json!(10));

    h.set_edge_attributes(vec![
        ("e1".to_string(), attrs_e1),
        ("e2".to_string(), attrs_e2),
    ]);

    assert_eq!(h.edge_attrs("e1").unwrap()["weight"], 5);
    assert_eq!(h.edge_attrs("e2").unwrap()["weight"], 10);
}

#[test]
fn test_clear_edges_keeps_nodes() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"w": 1}),
    )
    .unwrap();
    h.add_node("lonely", serde_json::json!({"x": 1}));
    h.clear_edges();
    assert_eq!(h.num_nodes(), 3); // a, b, lonely all kept
    assert_eq!(h.num_edges(), 0);
    // Node attrs preserved
    assert_eq!(h.node_attrs("lonely").unwrap()["x"], 1);
    // No memberships
    assert!(h.memberships("a").unwrap().is_empty());
}

#[test]
fn test_freeze_prevents_mutation() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::Value::Null);
    h.freeze();
    assert!(h.is_frozen());
    // Mutation should panic
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut h = h;
        h.add_node("b", serde_json::Value::Null);
    }));
    assert!(result.is_err(), "add_node should panic when frozen");
}

#[test]
fn test_is_frozen_false_by_default() {
    let h: Hypergraph = Hypergraph::new();
    assert!(!h.is_frozen());
}

#[test]
fn test_debug_format_matches_xgi_repr() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        None,
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        None,
        serde_json::Value::Null,
    )
    .unwrap();
    let repr = format!("{:?}", h);
    // XGI __repr__ returns: Hypergraph([{a, b, c}, {b, c}])
    // Our Debug should include the class name and edge members
    assert!(repr.contains("Hypergraph"));
    assert!(repr.contains("a"));
    assert!(repr.contains("b"));
    assert!(repr.contains("c"));
}

#[test]
fn test_debug_format_empty() {
    let h: Hypergraph = Hypergraph::new();
    let repr = format!("{:?}", h);
    assert!(repr.contains("Hypergraph"));
}

#[test]
fn test_skeleton_projects_pairwise() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let g = h.skeleton();
    // triangle a-b, a-c, b-c => 3 nodes, 3 undirected edges
    assert_eq!(g.node_count(), 3);
    assert_eq!(g.edge_count(), 3);
}

#[test]
fn test_skeleton_node_order_positional() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("x", serde_json::Value::Null);
    h.add_node("y", serde_json::Value::Null);
    let g = h.skeleton();
    use rustworkx_core::petgraph::graph::NodeIndex;
    assert_eq!(g.node_weight(NodeIndex::new(0)).unwrap(), "x");
    assert_eq!(g.node_weight(NodeIndex::new(1)).unwrap(), "y");
}

#[test]
fn test_node_neighbors() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["c".into(), "d".into()],
        Some("e1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let mut nb = h.node_neighbors("c").unwrap();
    nb.sort();
    assert_eq!(nb, vec!["a", "b", "d"]);
    assert!(!nb.contains(&"c".to_string()));
}

#[test]
fn test_node_degree_and_edge_size() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["a".into(), "b".into(), "c".into()],
        Some("e1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    assert_eq!(h.node_degree("a", None), Some(2));
    assert_eq!(h.node_degree("a", Some(1)), Some(1)); // only e0 (size 2 => order 1)
    assert_eq!(h.edge_size("e1"), Some(3));
    assert_eq!(h.edge_size("nope"), None);
}

#[test]
fn test_remove_nodes_from_missing_per_item_err_continues() {
    // XGI warns ("Node ghost not in hypergraph") + skips + CONTINUES on a
    // missing id; the core records a per-item Err and continues (D2-class
    // channel translation — the binding warns per Err item).
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let results = h.remove_nodes_from(
        &["b".to_string(), "ghost".to_string(), "c".to_string()],
        false,
        true,
    );
    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert_eq!(
        results[1],
        Err(NodeError::NotFound {
            node_id: "ghost".to_string()
        })
    );
    assert!(results[2].is_ok());
    // b and c both removed; e2 emptied (c gone) -> dropped; e1 = [a].
    assert_eq!(h.node_ids(), vec!["a"]);
    assert_eq!(h.edge_ids(), vec!["e1"]);
    assert_eq!(h.members("e1").unwrap(), vec!["a"]);
}

#[test]
fn test_remove_edges_from_missing_stops_after_first_err() {
    // XGI RAISES IDNotFound mid-iteration on the first missing id: ids
    // before it are already removed (partial effects), ids after it are
    // never attempted. The core records per-item results and STOPS after
    // the first Err (Vec length = attempted count <= input length), so
    // the binding can reproduce the raise exactly.
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["c".to_string(), "d".to_string()],
        Some("e3".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let results = h.remove_edges_from(&["e1".to_string(), "ghost".to_string(), "e3".to_string()]);
    assert_eq!(results.len(), 2, "e3 was never attempted");
    assert!(results[0].is_ok());
    assert_eq!(
        results[1],
        Err(EdgeError::NotFound {
            edge_id: "ghost".to_string()
        })
    );
    assert!(!h.has_edge("e1")); // removed before the error
    assert!(h.has_edge("e2"));
    assert!(h.has_edge("e3")); // never attempted
}

#[test]
fn test_add_node_existing_replaces_attrs() {
    // XGI: "If node is already in the hypergraph, its attributes are still
    // updated." Core semantics replace (a generic N cannot merge; the PyO3
    // layer merges dicts before calling). Divergence D6.
    let mut h: Hypergraph = Hypergraph::new();
    assert!(h.add_node("a", serde_json::json!({"x": 1})));
    assert!(!h.add_node("a", serde_json::json!({"y": 2})));
    assert_eq!(h.node_attrs("a").unwrap(), &serde_json::json!({"y": 2}));
    assert_eq!(h.num_nodes(), 1);
}
