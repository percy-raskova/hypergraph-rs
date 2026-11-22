use hypergraph_rs::{SimplexError, SimplicialComplex};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_add_simplex_closes_under_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v())
        .unwrap();
    // {1,2,3} + its three 2-faces {1,2},{1,3},{2,3} = 4 simplices (no singletons)
    assert_eq!(s.num_edges(), 4);
    assert!(s.has_simplex(&["1".to_string(), "2".to_string(), "3".to_string()]));
    assert!(s.has_simplex(&["1".to_string(), "2".to_string()]));
    assert!(s.has_simplex(&["2".to_string(), "3".to_string()]));
    assert!(!s.has_simplex(&["1".to_string()])); // singleton not auto-added
}

#[test]
fn test_add_simplex_singleton_is_one_simplex() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["4".into()], Some("myedge".into()), v())
        .unwrap();
    assert_eq!(s.num_edges(), 1);
    assert!(s.has_simplex(&["4".to_string()]));
}

#[test]
fn test_add_simplex_duplicate_members_is_noop() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v())
        .unwrap();
    let before = s.num_edges();
    s.add_simplex(vec!["2".into(), "1".into()], None, v())
        .unwrap(); // same set
    assert_eq!(s.num_edges(), before);
}

#[test]
fn test_add_simplex_empty_simplex_is_created() {
    // D1: XGI's docstring claims empty simplices cannot be added, but the
    // runtime CREATES one. A second empty add is a silent no-op (D8: main
    // keeps Result<(), SimplexError> — assert state, not a returned id).
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec![], None, v()).unwrap();
    assert_eq!(s.num_edges(), 1);
    assert_eq!(s.num_nodes(), 0);
    assert_eq!(s.edge_ids(), vec!["0"]);
    assert!(s.members("0").unwrap().is_empty());
    assert!(s.has_simplex(&[]));
    s.add_simplex(vec![], None, v()).unwrap();
    assert_eq!(s.num_edges(), 1);
}

#[test]
fn test_add_simplex_shares_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v())
        .unwrap();
    let after_first = s.num_edges();
    // {2,3,4}: adds {2,3,4},{2,4},{3,4} but NOT {2,3} (already present) = +3
    s.add_simplex(vec!["2".into(), "3".into(), "4".into()], None, v())
        .unwrap();
    assert_eq!(s.num_edges(), after_first + 3);
}

#[test]
fn test_add_edge_is_blocked() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    let r = s.add_edge(vec!["1".into(), "2".into()], None, v());
    assert!(matches!(r, Err(SimplexError::NotAvailable { .. })));
}

#[test]
fn test_face_lattice_immediate_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(
        vec!["1".into(), "2".into(), "3".into()],
        Some("top".into()),
        v(),
    )
    .unwrap();
    let lattice = s.face_lattice();
    // {1,2,3} has three immediate (codim-1) faces present in the complex
    assert_eq!(lattice.immediate_faces.get("top").map(|f| f.len()), Some(3));
}

#[test]
fn test_add_simplices_from() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplices_from(vec![
        (vec!["0".into(), "1".into()], None, v()),
        (vec!["1".into(), "2".into()], None, v()),
        (vec!["2".into(), "3".into(), "4".into()], None, v()),
    ]);
    // 2 edges (order-1) + {2,3,4} and its 3 faces = 2 + 1 + 3 = 6
    assert_eq!(s.num_edges(), 6);
    assert!(s.has_simplex(&["2".to_string(), "3".to_string()]));
}

#[test]
fn test_close_adds_missing_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    // seed a bare 2-simplex via add_simplex, then remove one face to force a gap
    s.add_simplex(
        vec!["1".into(), "2".into(), "3".into()],
        Some("top".into()),
        v(),
    )
    .unwrap();
    s.remove_simplex_id("top").unwrap(); // removes {1,2,3} only (no supfaces)
    let before = s.num_edges();
    s.close(); // no gaps -> no change (the 2-faces are already closed)
    assert_eq!(s.num_edges(), before);
}

#[test]
fn test_remove_simplex_id_removes_supfaces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v())
        .unwrap();
    // find the id of face {1,2}
    // removing {1,2} must also remove {1,2,3} (its supface) to preserve closure
    let face_id = s
        .edge_ids()
        .into_iter()
        .find(|e| {
            let mut m = s.members(e).unwrap();
            m.sort();
            m == vec!["1".to_string(), "2".to_string()]
        })
        .unwrap();
    s.remove_simplex_id(&face_id).unwrap();
    assert!(!s.has_simplex(&["1".to_string(), "2".to_string()]));
    assert!(!s.has_simplex(&["1".to_string(), "2".to_string(), "3".to_string()]));
    // {1,3} and {2,3} survive
    assert!(s.has_simplex(&["1".to_string(), "3".to_string()]));
}

#[test]
fn test_remove_simplex_id_missing_errors() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    assert!(matches!(
        s.remove_simplex_id("nope"),
        Err(SimplexError::NotFound { .. })
    ));
}

#[test]
fn test_simplicialcomplex_copy_independent() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v())
        .unwrap();
    let mut c = s.copy();
    c.add_simplex(vec!["3".into(), "4".into()], None, v())
        .unwrap();
    assert!(!s.has_simplex(&["3".to_string(), "4".to_string()]));
    assert_eq!(s.num_edges(), 1);
}

#[test]
fn test_uid_counter_rules_match_the_core() {
    // Auto ids sequence; explicit numeric str idx bumps (D3 rule);
    // non-numeric idx does not — faces consume the auto sequence.
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(
        vec!["1".into(), "2".into(), "3".into()],
        Some("10".into()),
        v(),
    )
    .unwrap();
    // "10" parses as u64 -> bump: faces take 11, 12, 13.
    assert_eq!(s.edge_ids(), vec!["10", "11", "12", "13"]);

    let mut s2: SimplicialComplex = SimplicialComplex::new();
    s2.add_simplex(
        vec!["1".into(), "2".into(), "3".into()],
        Some("top".into()),
        v(),
    )
    .unwrap();
    // "top" does not parse -> no bump: faces take 0, 1, 2.
    assert_eq!(s2.edge_ids(), vec!["top", "0", "1", "2"]);
}

#[test]
fn test_falsy_idx_is_explicit_in_rust() {
    // D17: XGI's add_simplex treats a FALSY idx (0, "") as auto; the
    // core's Option<String> is exact — Some("0") is explicit.
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(
        vec!["1".into(), "2".into(), "3".into()],
        Some("5".into()),
        v(),
    )
    .unwrap();
    s.add_simplex(vec!["4".into(), "5".into()], Some("0".into()), v())
        .unwrap();
    assert!(
        s.has_simplex(&["4".to_string(), "5".to_string()]),
        "Some(\"0\") is an explicit id, not auto"
    );
    let mut members = s.members("0").unwrap();
    members.sort();
    assert_eq!(members, vec!["4", "5"]);
}

#[test]
fn test_simplicialcomplex_freeze() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v())
        .unwrap();
    s.freeze();
    assert!(s.is_frozen());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut s = s;
        let _ = s.add_simplex(vec!["3".into(), "4".into()], None, v());
    }));
    assert!(r.is_err()); // add_simplex -> inner.add_edge panics when frozen
}

#[test]
fn next_edge_id_follows_auto_ids_and_explicit_numeric_ids() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    assert_eq!(s.next_edge_id(), 0);
    s.add_simplex(vec!["a".into(), "b".into()], None, v())
        .unwrap();
    s.add_simplex(vec!["c".into(), "d".into()], Some("7".into()), v())
        .unwrap();
    assert_eq!(s.next_edge_id(), 8);
    s.update_uid_counter(3);
    assert_eq!(s.next_edge_id(), 8);
    s.update_uid_counter(10);
    assert_eq!(s.next_edge_id(), 11);
}

#[test]
fn from_closed_simplices_keeps_ids_and_order() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["a".into(), "b".into(), "c".into()], None, v())
        .unwrap();
    s.add_simplex(vec!["c".into(), "d".into()], Some("9".into()), v())
        .unwrap();
    let nodes: Vec<(String, serde_json::Value)> =
        s.node_ids().into_iter().map(|n| (n, v())).collect();
    let simplices: Vec<(String, Vec<String>, serde_json::Value)> = s
        .edge_ids()
        .into_iter()
        .map(|e| {
            let m = s.members(&e).unwrap();
            (e, m, v())
        })
        .collect();
    let t: SimplicialComplex = SimplicialComplex::from_closed_simplices(nodes, simplices).unwrap();
    assert_eq!(t.edge_ids(), s.edge_ids());
    assert_eq!(t.node_ids(), s.node_ids());
    for e in s.edge_ids() {
        assert_eq!(t.members(&e), s.members(&e));
    }
}

#[test]
fn from_closed_simplices_refuses_a_missing_face() {
    let nodes = vec![
        ("a".to_string(), v()),
        ("b".to_string(), v()),
        ("c".to_string(), v()),
    ];
    let simplices = vec![(
        "t".to_string(),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        v(),
    )];
    let r: Result<SimplicialComplex, SimplexError> =
        SimplicialComplex::from_closed_simplices(nodes, simplices);
    assert_eq!(r.err(), Some(SimplexError::NotClosed { idx: "t".into() }));
}

#[test]
fn from_closed_simplices_keeps_the_first_of_a_repeated_id_or_member_set() {
    let nodes = vec![("a".to_string(), v()), ("b".to_string(), v())];
    let ab = || vec!["a".to_string(), "b".to_string()];
    let simplices = vec![
        ("x".to_string(), ab(), v()),
        // same member set, another id: skipped
        ("y".to_string(), vec!["b".to_string(), "a".to_string()], v()),
        // same id as the first, other members: skipped
        ("x".to_string(), vec!["a".to_string()], v()),
    ];
    let s: SimplicialComplex = SimplicialComplex::from_closed_simplices(nodes, simplices).unwrap();
    assert_eq!(s.edge_ids(), vec!["x".to_string()]);
    assert_eq!(s.members("x"), Some(ab()));
}
