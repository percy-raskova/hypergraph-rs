#![cfg(feature = "convert")]

use hypergraph_rs::convert::{
    from_bipartite_edgelist, from_hyperedge_dict, from_hyperedge_list, to_bipartite_edgelist,
    to_hyperedge_dict, to_hyperedge_list,
};
use hypergraph_rs::Hypergraph;
use hypergraph_rs::convert::{from_bipartite_graph, to_bipartite_graph, BipartiteNode};
use rustworkx_core::petgraph::stable_graph::StableDiGraph;

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

#[test]
fn test_from_bipartite_edgelist() {
    let el = vec![
        ("a".to_string(), "e1".to_string()),
        ("b".to_string(), "e1".to_string()),
        ("c".to_string(), "e2".to_string()),
    ];
    let h = from_bipartite_edgelist(el);
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 3);
    let mut m = h.members("e1").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_to_bipartite_edgelist() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let el = to_bipartite_edgelist(&h);
    assert_eq!(el.len(), 2);
    assert!(el.contains(&("a".to_string(), "e1".to_string())));
    assert!(el.contains(&("b".to_string(), "e1".to_string())));
}

#[test]
fn test_bipartite_edgelist_roundtrip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let back = from_bipartite_edgelist(to_bipartite_edgelist(&h));
    assert_eq!(back.num_nodes(), 3);
    assert_eq!(back.num_edges(), 2);
}

#[test]
fn test_to_bipartite_graph_markers() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    let g = to_bipartite_graph(&h);
    // 2 node-layer + 1 edge-layer = 3 vertices; each membership = 1 arc.
    assert_eq!(g.node_count(), 3);
    let nodes = g.node_weights().filter(|w| w.bipartite == 0).count();
    let edges = g.node_weights().filter(|w| w.bipartite == 1).count();
    assert_eq!(nodes, 2);
    assert_eq!(edges, 1);
}

#[test]
fn test_from_bipartite_graph_reconstructs() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into(), "b".into()], Some("e1".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["b".into(), "c".into()], Some("e2".into()), serde_json::Value::Null).unwrap();
    let g = to_bipartite_graph(&h);
    let back = from_bipartite_graph(&g).unwrap();
    assert_eq!(back.num_nodes(), 3);
    assert_eq!(back.num_edges(), 2);
    let mut m = back.members("e1").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_from_bipartite_graph_invalid_marker_errors() {
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    g.add_node(BipartiteNode { id: "bad".into(), bipartite: 2 });
    assert!(from_bipartite_graph(&g).is_err());
}

#[test]
fn test_from_bipartite_graph_non_bipartite_errors() {
    // Two node-layer vertices joined directly -> not bipartite.
    let mut g: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    let a = g.add_node(BipartiteNode { id: "a".into(), bipartite: 0 });
    let b = g.add_node(BipartiteNode { id: "b".into(), bipartite: 0 });
    g.add_edge(a, b, ());
    assert!(from_bipartite_graph(&g).is_err());
}
