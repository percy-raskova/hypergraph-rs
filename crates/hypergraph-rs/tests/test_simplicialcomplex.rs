use hypergraph_rs::{SimplexError, SimplicialComplex};

fn v() -> serde_json::Value {
    serde_json::Value::Null
}

#[test]
fn test_add_simplex_closes_under_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v()).unwrap();
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
    s.add_simplex(vec!["4".into()], Some("myedge".into()), v()).unwrap();
    assert_eq!(s.num_edges(), 1);
    assert!(s.has_simplex(&["4".to_string()]));
}

#[test]
fn test_add_simplex_duplicate_members_is_noop() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into()], None, v()).unwrap();
    let before = s.num_edges();
    s.add_simplex(vec!["2".into(), "1".into()], None, v()).unwrap(); // same set
    assert_eq!(s.num_edges(), before);
}

#[test]
fn test_add_simplex_shares_faces() {
    let mut s: SimplicialComplex = SimplicialComplex::new();
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], None, v()).unwrap();
    let after_first = s.num_edges();
    // {2,3,4}: adds {2,3,4},{2,4},{3,4} but NOT {2,3} (already present) = +3
    s.add_simplex(vec!["2".into(), "3".into(), "4".into()], None, v()).unwrap();
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
    s.add_simplex(vec!["1".into(), "2".into(), "3".into()], Some("top".into()), v()).unwrap();
    let lattice = s.face_lattice();
    // {1,2,3} has three immediate (codim-1) faces present in the complex
    assert_eq!(lattice.immediate_faces.get("top").map(|f| f.len()), Some(3));
}
