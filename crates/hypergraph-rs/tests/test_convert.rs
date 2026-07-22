#![cfg(feature = "convert")]

use hypergraph_rs::convert::{
    from_hyperedge_dict, from_hyperedge_list, to_hyperedge_dict, to_hyperedge_list,
};
use hypergraph_rs::Hypergraph;

fn rows(r: &[&[&str]]) -> Vec<Vec<String>> {
    r.iter().map(|e| e.iter().map(|s| s.to_string()).collect()).collect()
}

#[test]
fn test_from_hyperedge_list_auto_ids() {
    let h = from_hyperedge_list(rows(&[&["a", "b"], &["b", "c", "d"]]));
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 4);
    assert_eq!(h.edge_ids(), vec!["0", "1"]);
}

#[test]
fn test_to_hyperedge_list() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let list = to_hyperedge_list(&h);
    assert_eq!(list.len(), 1);
    let mut m = list[0].clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_from_hyperedge_dict_keeps_ids() {
    let d = vec![
        ("first".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("second".to_string(), vec!["c".to_string()]),
    ];
    let h = from_hyperedge_dict(d);
    assert!(h.has_edge("first"));
    assert!(h.has_edge("second"));
    assert_eq!(h.num_nodes(), 3);
}

#[test]
fn test_to_hyperedge_dict() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let d = to_hyperedge_dict(&h);
    assert_eq!(d.keys().collect::<Vec<_>>(), vec!["e1", "e2"]);
    assert_eq!(d.get("e2").unwrap(), &vec!["c".to_string()]);
}

#[test]
fn test_hyperedge_dict_roundtrip_members_as_sets() {
    let d = vec![
        ("e1".to_string(), vec!["a".to_string(), "b".to_string()]),
        ("e2".to_string(), vec!["b".to_string(), "c".to_string()]),
    ];
    let h = from_hyperedge_dict(d);
    let back = to_hyperedge_dict(&h);
    let mut m = back.get("e1").unwrap().clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}
