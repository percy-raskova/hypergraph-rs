use hypergraph_rs::utils::{dual_dict, powerset};
use indexmap::IndexMap;

fn edge_dict(pairs: &[(&str, &[&str])]) -> IndexMap<String, Vec<String>> {
    let mut m = IndexMap::new();
    for (k, vs) in pairs {
        m.insert(k.to_string(), vs.iter().map(|s| s.to_string()).collect());
    }
    m
}

#[test]
fn test_dual_dict_reverses() {
    // XGI: dual_dict({0:[1,2,3], 1:[0,2]}) == {1:{0}, 2:{0,1}, 3:{0}, 0:{1}}
    let d = edge_dict(&[("0", &["1", "2", "3"]), ("1", &["0", "2"])]);
    let dual = dual_dict(&d);
    assert_eq!(dual.get("1").unwrap(), &vec!["0".to_string()]);
    assert_eq!(dual.get("2").unwrap(), &vec!["0".to_string(), "1".to_string()]);
    assert_eq!(dual.get("3").unwrap(), &vec!["0".to_string()]);
    assert_eq!(dual.get("0").unwrap(), &vec!["1".to_string()]);
    // First-seen node order: 1, 2, 3, 0
    let keys: Vec<&String> = dual.keys().collect();
    assert_eq!(keys, vec!["1", "2", "3", "0"]);
}

#[test]
fn test_dual_dict_empty() {
    let dual = dual_dict(&IndexMap::new());
    assert!(dual.is_empty());
}

#[test]
fn test_powerset_default_len14() {
    // XGI doctest: powerset([1,2,3,4]) -> singletons..triples (no full set) = 14 subsets.
    let items: Vec<String> = ["1", "2", "3", "4"].iter().map(|s| s.to_string()).collect();
    let ps = powerset(&items, false, true, false, None);
    assert_eq!(ps.len(), 14);
    assert_eq!(ps[0], vec!["1".to_string()]);
    assert_eq!(ps.last().unwrap(), &vec!["2".to_string(), "3".to_string(), "4".to_string()]);
}

#[test]
fn test_powerset_include_empty_and_full() {
    let items: Vec<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
    // include_empty + include_full: {} , {a}, {b}, {a,b} = 4
    let ps = powerset(&items, true, true, true, None);
    assert_eq!(ps.len(), 4);
    assert!(ps[0].is_empty());
    assert_eq!(ps.last().unwrap(), &vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_powerset_max_size() {
    let items: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    // singletons + pairs only (max_size 2): 3 + 3 = 6
    let ps = powerset(&items, false, true, false, Some(2));
    assert_eq!(ps.len(), 6);
    assert!(ps.iter().all(|s| s.len() <= 2 && !s.is_empty()));
}

use hypergraph_rs::utils::{get_network_type, subfaces};
use hypergraph_rs::Hypergraph;

fn edges(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect()
}

#[test]
fn test_subfaces_none_all_down_to_nodes() {
    // {1,2,3} -> singletons + pairs (no full triple): C(3,1)+C(3,2) = 3+3 = 6
    let es = edges(&[&["1", "2", "3"]]);
    let faces = subfaces(&es, None).unwrap();
    assert_eq!(faces.len(), 6);
    assert!(faces.contains(&vec!["1".to_string()]));
    assert!(faces.contains(&vec!["1".to_string(), "2".to_string()]));
}

#[test]
fn test_subfaces_order_minus1() {
    // {1,2,3} one order below (size 2): the 3 pairs.
    let es = edges(&[&["1", "2", "3"]]);
    let faces = subfaces(&es, Some(-1)).unwrap();
    assert_eq!(faces.len(), 3);
    assert!(faces.iter().all(|f| f.len() == 2));
}

#[test]
fn test_subfaces_order_d() {
    // order=2 -> subfaces of size 3. {1,2,3,4} has C(4,3)=4 triples.
    let es = edges(&[&["1", "2", "3", "4"]]);
    let faces = subfaces(&es, Some(2)).unwrap();
    assert_eq!(faces.len(), 4);
    assert!(faces.iter().all(|f| f.len() == 3));
}

#[test]
fn test_subfaces_order_too_large_errors() {
    let es = edges(&[&["1", "2"]]); // max order = 1
    let result = subfaces(&es, Some(5));
    assert!(result.is_err());
}

#[test]
fn test_get_network_type() {
    let h: Hypergraph = Hypergraph::new();
    assert_eq!(get_network_type(&h), "hypergraph");
}

use hypergraph_rs::utils::convert_labels_to_integers;

#[test]
fn test_convert_labels_basic() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into()], Some("first".into()), serde_json::Value::Null)
        .unwrap();
    h.add_edge(vec!["y".into(), "z".into()], Some("second".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    // Nodes x,y,z -> "0","1","2" ; edges first,second -> "0","1"
    assert_eq!(relabeled.node_ids(), vec!["0", "1", "2"]);
    assert_eq!(relabeled.edge_ids(), vec!["0", "1"]);
    assert_eq!(relabeled.num_nodes(), 3);
    assert_eq!(relabeled.num_edges(), 2);
}

#[test]
fn test_convert_labels_stores_old_label() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into()], Some("first".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    assert_eq!(relabeled.node_attrs("0").unwrap()["label"], "x");
    assert_eq!(relabeled.edge_attrs("0").unwrap()["label"], "first");
}

#[test]
fn test_convert_labels_preserves_members_as_sets() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["x".into(), "y".into(), "z".into()], Some("e".into()), serde_json::Value::Null)
        .unwrap();
    let relabeled = convert_labels_to_integers(&h, "label");
    let mut members = relabeled.members("0").unwrap();
    members.sort();
    assert_eq!(members, vec!["0".to_string(), "1".to_string(), "2".to_string()]);
}
