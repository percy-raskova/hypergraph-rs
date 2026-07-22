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
        .add_edge(
            vec!["1".into(), "2".into()],
            vec!["3".into()],
            Some("e0".into()),
            v(),
        )
        .unwrap();
    assert_eq!(id, "e0");
    assert_eq!(dh.num_edges(), 1);
    assert_eq!(dh.num_nodes(), 3);
    assert!(dh.has_edge("e0"));
}

#[test]
fn test_dihypergraph_add_edge_auto_id() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    let a = dh
        .add_edge(vec!["1".into()], vec!["2".into()], None, v())
        .unwrap();
    let b = dh
        .add_edge(vec!["3".into()], vec!["4".into()], None, v())
        .unwrap();
    assert_eq!(a, "0");
    assert_eq!(b, "1");
}

#[test]
fn test_dihypergraph_add_edge_one_sided_allowed() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    // all-senders edge (empty head) is allowed
    let id = dh
        .add_edge(vec!["3".into(), "4".into()], vec![], Some("e0".into()), v())
        .unwrap();
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
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    let r = dh.add_edge(vec!["3".into()], vec!["4".into()], Some("e0".into()), v());
    assert!(matches!(r, Err(EdgeError::AlreadyExists { .. })));
    assert_eq!(dh.num_edges(), 1);
}

use hypergraph_rs::DiRole;

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

#[test]
fn test_dihypergraph_dimembers_head_tail() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(
        vec!["1".into(), "2".into()],
        vec!["3".into(), "4".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    let (tail, head) = dh.dimembers("e0").unwrap();
    assert_eq!(sorted(tail), vec!["1".to_string(), "2".to_string()]);
    assert_eq!(sorted(head), vec!["3".to_string(), "4".to_string()]);
    assert_eq!(
        sorted(dh.tail("e0").unwrap()),
        vec!["1".to_string(), "2".to_string()]
    );
    assert_eq!(
        sorted(dh.head("e0").unwrap()),
        vec!["3".to_string(), "4".to_string()]
    );
    assert_eq!(
        sorted(dh.members("e0").unwrap()),
        vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string()
        ]
    );
}

#[test]
fn test_dihypergraph_dimemberships() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    // node "2" is a receiver (head) of e0 and a sender (tail) of e1
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.add_edge(vec!["2".into()], vec!["3".into()], Some("e1".into()), v())
        .unwrap();
    let (in_edges, out_edges) = dh.dimemberships("2").unwrap();
    assert_eq!(in_edges, vec!["e0".to_string()]); // receiver in e0
    assert_eq!(out_edges, vec!["e1".to_string()]); // sender in e1
    assert_eq!(
        sorted(dh.memberships("2").unwrap()),
        vec!["e0".to_string(), "e1".to_string()]
    );
}

#[test]
fn test_dihypergraph_remove_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.remove_edge("e0").unwrap();
    assert!(!dh.has_edge("e0"));
    assert!(dh.has_node("1"));
    assert!(dh.memberships("1").unwrap().is_empty());
}

#[test]
fn test_dihypergraph_remove_node_weak() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(
        vec!["1".into(), "2".into()],
        vec!["3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    dh.remove_node("1", false, true).unwrap();
    assert!(!dh.has_node("1"));
    assert!(dh.has_edge("e0"));
    assert_eq!(sorted(dh.tail("e0").unwrap()), vec!["2".to_string()]);
}

#[test]
fn test_dihypergraph_remove_node_weak_removes_empty_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec![], Some("e0".into()), v())
        .unwrap();
    dh.remove_node("1", false, true).unwrap();
    assert!(!dh.has_edge("e0")); // edge became empty -> removed
}

#[test]
fn test_dihypergraph_remove_node_strong() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.add_edge(vec!["1".into()], vec!["3".into()], Some("e1".into()), v())
        .unwrap();
    dh.remove_node("1", true, true).unwrap();
    assert!(!dh.has_edge("e0"));
    assert!(!dh.has_edge("e1"));
    assert!(dh.has_node("2"));
}

#[test]
fn test_dihypergraph_add_node_to_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.add_node_to_edge("e0", "9", DiRole::Head).unwrap();
    assert_eq!(
        sorted(dh.head("e0").unwrap()),
        vec!["2".to_string(), "9".to_string()]
    );
}

#[test]
fn test_dihypergraph_add_node_to_edge_numeric_id_bumps_counter() {
    // D11-extension: XGI's DiHypergraph.add_node_to_edge does NOT bump
    // the uid counter (probed: next auto id 0 after auto-creating edge
    // 5); the Rust core bumps iff the id parses as u64 — the D3/D11 rule
    // extended to DiHypergraph.
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_node_to_edge("5", "x", DiRole::Tail).unwrap();
    let auto = dh.add_edge(vec!["y".into()], vec![], None, v()).unwrap();
    assert_eq!(auto, "6");
    // Non-numeric ids don't bump.
    let mut dh2: DiHypergraph = DiHypergraph::new();
    dh2.add_node_to_edge("e", "x", DiRole::Tail).unwrap();
    let auto2 = dh2.add_edge(vec!["y".into()], vec![], None, v()).unwrap();
    assert_eq!(auto2, "0");
}

#[test]
fn test_dihypergraph_remove_node_from_edge() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(
        vec!["1".into(), "2".into()],
        vec!["3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    dh.remove_node_from_edge("e0", "1", DiRole::Tail, true)
        .unwrap();
    assert_eq!(sorted(dh.tail("e0").unwrap()), vec!["2".to_string()]);
    // removing a node not in the named role errors
    assert!(dh
        .remove_node_from_edge("e0", "1", DiRole::Tail, true)
        .is_err());
}

#[test]
fn test_dihypergraph_node_stats() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.add_edge(vec!["2".into()], vec!["3".into()], Some("e1".into()), v())
        .unwrap();
    assert_eq!(dh.degree("2"), 2);
    assert_eq!(dh.in_degree("2"), 1); // receiver in e0
    assert_eq!(dh.out_degree("2"), 1); // sender in e1
}

#[test]
fn test_dihypergraph_edge_stats() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(
        vec!["1".into(), "2".into()],
        vec!["3".into()],
        Some("e0".into()),
        v(),
    )
    .unwrap();
    assert_eq!(dh.size("e0"), 3);
    assert_eq!(dh.order("e0"), 2);
    assert_eq!(dh.tail_size("e0"), 2);
    assert_eq!(dh.head_size("e0"), 1);
    assert_eq!(dh.tail_order("e0"), 1);
    assert_eq!(dh.head_order("e0"), 0);
}

#[test]
fn test_dihypergraph_isolates() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.add_node("lonely", v());
    assert_eq!(dh.isolate_node_ids(), vec!["lonely".to_string()]);
}

#[test]
fn test_dihypergraph_copy_is_independent() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    let mut c = dh.copy();
    c.add_node("x", v());
    assert!(!dh.has_node("x"));
    assert_eq!(c.num_nodes(), 3);
}

#[test]
fn test_dihypergraph_clear() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    dh.clear();
    assert_eq!(dh.num_nodes(), 0);
    assert_eq!(dh.num_edges(), 0);
}

#[test]
fn test_dihypergraph_freeze() {
    let mut dh: DiHypergraph = DiHypergraph::new();
    dh.add_node("a", v());
    dh.freeze();
    assert!(dh.is_frozen());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut dh = dh;
        dh.add_node("b", v());
    }));
    assert!(r.is_err());
}

#[test]
fn test_dihypergraph_eq_and_debug() {
    let mut a: DiHypergraph = DiHypergraph::new();
    a.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    let mut b: DiHypergraph = DiHypergraph::new();
    b.add_edge(vec!["1".into()], vec!["2".into()], Some("e0".into()), v())
        .unwrap();
    assert!(a == b);
    b.add_node("x", v());
    assert!(a != b);
    let repr = format!("{a:?}");
    assert!(repr.contains("DiHypergraph"));
}
