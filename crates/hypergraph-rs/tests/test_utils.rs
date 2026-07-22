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
