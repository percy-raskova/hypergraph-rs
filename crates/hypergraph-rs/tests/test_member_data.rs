//! The value a membership carries: defaults, writes, refusals, copy, frozen, roles.

use hypergraph_rs::{DiHypergraph, DiRole, Hypergraph, MembershipError};

type H = Hypergraph<(), (), u64>;
type D = DiHypergraph<(), (), u64>;

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

/// Edge `e` of `a, b`, plus a node `z` that belongs to no edge.
fn graph() -> H {
    let mut h = H::new();
    h.add_edge(s(&["a", "b"]), Some("e".into()), ()).unwrap();
    h.add_node("z", ());
    h
}

// ---------------------------------------------------------------- Hypergraph

#[test]
fn a_new_membership_carries_the_default() {
    let h = graph();
    assert_eq!(h.member_data("e", "a"), Some(&0));
    assert_eq!(h.member_data("e", "b"), Some(&0));
}

#[test]
fn a_value_is_written_and_read_back() {
    let mut h = graph();
    assert_eq!(h.set_member_data("e", "a", 7), Ok(()));
    assert_eq!(h.member_data("e", "a"), Some(&7));
    assert_eq!(h.member_data("e", "b"), Some(&0));
    assert_eq!(
        h.members_with_data("e"),
        Some(vec![("a".to_string(), &7), ("b".to_string(), &0)])
    );
}

#[test]
fn the_read_is_none_for_what_is_absent() {
    let h = graph();
    assert_eq!(h.member_data("nope", "a"), None);
    assert_eq!(h.member_data("e", "nope"), None);
    assert_eq!(h.member_data("e", "z"), None);
    assert_eq!(h.members_with_data("nope"), None);
}

#[test]
fn the_write_refuses_by_name() {
    let mut h = graph();
    assert_eq!(
        h.set_member_data("nope", "a", 1),
        Err(MembershipError::EdgeNotFound {
            edge_id: "nope".into()
        })
    );
    assert_eq!(
        h.set_member_data("e", "nope", 1),
        Err(MembershipError::NodeNotFound {
            node_id: "nope".into()
        })
    );
    assert_eq!(
        h.set_member_data("e", "z", 1),
        Err(MembershipError::NotAMember {
            node_id: "z".into(),
            edge_id: "e".into()
        })
    );
}

#[test]
fn adding_with_data_creates_and_replaces() {
    let mut h = H::new();
    h.add_node_to_edge_with_data("e", "a", 5).unwrap();
    assert!(h.has_edge("e") && h.has_node("a"));
    assert_eq!(h.member_data("e", "a"), Some(&5));
    h.add_node_to_edge_with_data("e", "a", 9).unwrap();
    assert_eq!(h.member_data("e", "a"), Some(&9));
    assert_eq!(h.members("e"), Some(s(&["a"])));
}

#[test]
fn a_member_that_leaves_and_returns_starts_again() {
    let mut h = graph();
    h.set_member_data("e", "a", 7).unwrap();
    h.remove_node_from_edge("e", "a", false).unwrap();
    h.add_node_to_edge("e", "a").unwrap();
    assert_eq!(h.member_data("e", "a"), Some(&0));
}

#[test]
fn a_copy_carries_the_values() {
    let mut h = graph();
    h.set_member_data("e", "a", 7).unwrap();
    let mut c = h.copy();
    assert_eq!(c.member_data("e", "a"), Some(&7));
    c.set_member_data("e", "a", 8).unwrap();
    assert_eq!(c.member_data("e", "a"), Some(&8));
    assert_eq!(h.member_data("e", "a"), Some(&7));
}

#[test]
#[should_panic(expected = "Frozen hypergraph can't be modified")]
fn a_frozen_graph_refuses_the_write() {
    let mut h = graph();
    h.freeze();
    let _ = h.set_member_data("e", "a", 7);
}

#[test]
#[should_panic(expected = "Frozen hypergraph can't be modified")]
fn a_frozen_graph_refuses_adding_with_data() {
    let mut h = graph();
    h.freeze();
    let _ = h.add_node_to_edge_with_data("e", "c", 7);
}

// ------------------------------------------------------------- DiHypergraph

fn digraph() -> D {
    let mut d = D::new();
    d.add_edge(s(&["x"]), s(&["y"]), Some("e".into()), ())
        .unwrap();
    d.add_node("z", ());
    d
}

#[test]
fn tail_and_head_keep_separate_values() {
    let mut d = D::new();
    d.add_node_to_edge("e", "x", DiRole::Tail).unwrap();
    d.add_node_to_edge("e", "x", DiRole::Head).unwrap();
    d.set_member_data("e", "x", DiRole::Tail, 3).unwrap();
    d.set_member_data("e", "x", DiRole::Head, 4).unwrap();
    assert_eq!(d.member_data("e", "x", DiRole::Tail), Some(&3));
    assert_eq!(d.member_data("e", "x", DiRole::Head), Some(&4));
    assert_eq!(
        d.dimembers_with_data("e"),
        Some((vec![("x".to_string(), &3)], vec![("x".to_string(), &4)]))
    );
}

#[test]
fn a_new_directed_membership_carries_the_default() {
    let d = digraph();
    assert_eq!(d.member_data("e", "x", DiRole::Tail), Some(&0));
    assert_eq!(d.member_data("e", "y", DiRole::Head), Some(&0));
}

#[test]
fn the_directed_read_is_none_for_what_is_absent() {
    let d = digraph();
    assert_eq!(d.member_data("nope", "x", DiRole::Tail), None);
    assert_eq!(d.member_data("e", "nope", DiRole::Tail), None);
    assert_eq!(d.member_data("e", "z", DiRole::Tail), None);
    // `x` is in the tail only, so the head role is absent.
    assert_eq!(d.member_data("e", "x", DiRole::Head), None);
    assert_eq!(d.dimembers_with_data("nope"), None);
}

#[test]
fn the_directed_write_refuses_by_name() {
    let mut d = digraph();
    assert_eq!(
        d.set_member_data("nope", "x", DiRole::Tail, 1),
        Err(MembershipError::EdgeNotFound {
            edge_id: "nope".into()
        })
    );
    assert_eq!(
        d.set_member_data("e", "nope", DiRole::Tail, 1),
        Err(MembershipError::NodeNotFound {
            node_id: "nope".into()
        })
    );
    assert_eq!(
        d.set_member_data("e", "z", DiRole::Tail, 1),
        Err(MembershipError::NotAMember {
            node_id: "z".into(),
            edge_id: "e".into()
        })
    );
    assert_eq!(
        d.set_member_data("e", "x", DiRole::Head, 1),
        Err(MembershipError::NotAMember {
            node_id: "x".into(),
            edge_id: "e".into()
        })
    );
}

#[test]
fn directed_adding_with_data_creates_and_replaces() {
    let mut d = D::new();
    d.add_node_to_edge_with_data("e", "a", DiRole::Head, 5)
        .unwrap();
    assert_eq!(d.member_data("e", "a", DiRole::Head), Some(&5));
    d.add_node_to_edge_with_data("e", "a", DiRole::Head, 9)
        .unwrap();
    assert_eq!(d.member_data("e", "a", DiRole::Head), Some(&9));
    assert_eq!(d.head("e"), Some(s(&["a"])));
    assert_eq!(d.tail("e"), Some(vec![]));
}

#[test]
fn a_directed_copy_carries_the_values() {
    let mut d = digraph();
    d.set_member_data("e", "x", DiRole::Tail, 7).unwrap();
    let mut c = d.copy();
    assert_eq!(c.member_data("e", "x", DiRole::Tail), Some(&7));
    c.set_member_data("e", "x", DiRole::Tail, 8).unwrap();
    assert_eq!(d.member_data("e", "x", DiRole::Tail), Some(&7));
}

#[test]
fn a_directed_member_that_leaves_and_returns_starts_again() {
    let mut d = digraph();
    d.add_node_to_edge("e", "w", DiRole::Tail).unwrap();
    d.set_member_data("e", "w", DiRole::Tail, 7).unwrap();
    d.remove_node_from_edge("e", "w", DiRole::Tail, false)
        .unwrap();
    d.add_node_to_edge("e", "w", DiRole::Tail).unwrap();
    assert_eq!(d.member_data("e", "w", DiRole::Tail), Some(&0));
}

#[test]
#[should_panic(expected = "Frozen higher-order network can't be modified")]
fn a_frozen_digraph_refuses_the_write() {
    let mut d = digraph();
    d.freeze();
    let _ = d.set_member_data("e", "x", DiRole::Tail, 7);
}

#[test]
#[should_panic(expected = "Frozen higher-order network can't be modified")]
fn a_frozen_digraph_refuses_adding_with_data() {
    let mut d = digraph();
    d.freeze();
    let _ = d.add_node_to_edge_with_data("e", "c", DiRole::Tail, 7);
}
