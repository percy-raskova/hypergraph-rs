use hypergraph_rs::layout::{levi_ego_tree, paoh_ordering, LeviKind, LeviNode};
use hypergraph_rs::Hypergraph;

fn sample_h() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e0".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_paoh_ordering_default_is_insertion_order() {
    let h = sample_h();
    let (nodes, edges) = paoh_ordering(&h, None);
    assert_eq!(nodes, vec!["a", "b", "c"]);
    assert_eq!(edges, vec!["e1", "e0"]);
}

#[test]
fn test_paoh_ordering_sorts_by_numeric_attr_missing_last() {
    let mut h = sample_h();
    *h.edge_attrs_mut("e0").unwrap() = serde_json::json!({"formation_tick": 1});
    let (_, edges) = paoh_ordering(&h, Some("formation_tick"));
    assert_eq!(edges, vec!["e0", "e1"]);
}

#[test]
fn test_levi_ego_tree_depth_and_dedup() {
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

    let tree = levi_ego_tree(&h, "b", 2).unwrap();
    assert_eq!(tree.id, "b");
    assert_eq!(tree.kind, LeviKind::Agent);
    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].id, "e1");
    assert_eq!(tree.children[0].kind, LeviKind::Hyperedge);
    assert_eq!(tree.children[1].id, "e2");
    assert_eq!(tree.children[1].kind, LeviKind::Hyperedge);

    let e1 = &tree.children[0];
    assert_eq!(e1.children.len(), 1);
    assert_eq!(e1.children[0].id, "a");
    assert_eq!(e1.children[0].kind, LeviKind::Agent);
    assert!(e1.children[0].children.is_empty());

    let e2 = &tree.children[1];
    assert_eq!(e2.children.len(), 1);
    assert_eq!(e2.children[0].id, "c");
    assert_eq!(e2.children[0].kind, LeviKind::Agent);
    assert!(e2.children[0].children.is_empty());
}

#[test]
fn test_levi_ego_tree_missing_root_none() {
    let h = sample_h();
    assert!(levi_ego_tree(&h, "zzz", 2).is_none());
}

#[test]
fn test_orderings_serde_json_round_trip() {
    let tree = LeviNode {
        id: "b".to_string(),
        kind: LeviKind::Agent,
        children: vec![LeviNode {
            id: "e1".to_string(),
            kind: LeviKind::Hyperedge,
            children: vec![LeviNode {
                id: "a".to_string(),
                kind: LeviKind::Agent,
                children: vec![],
            }],
        }],
    };
    let json = serde_json::to_string(&tree).unwrap();
    let back: LeviNode = serde_json::from_str(&json).unwrap();
    assert_eq!(tree, back);
}
