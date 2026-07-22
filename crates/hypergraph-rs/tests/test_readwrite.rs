use hypergraph_rs::readwrite::bipartite::{
    parse_bipartite_edgelist, read_bipartite_edgelist, write_bipartite_edgelist,
};
use hypergraph_rs::readwrite::edgelist::{parse_edgelist, read_edgelist, write_edgelist};
use hypergraph_rs::readwrite::incidence::{read_incidence_matrix, write_incidence_matrix};
use hypergraph_rs::readwrite::json::{from_hypergraph_dict, read_json, to_hypergraph_dict, write_json};
use hypergraph_rs::Hypergraph;

fn sample() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        serde_json::json!({"weight": 2}),
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into()],
        Some("e1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.set_graph_attr("name", serde_json::json!("sample"));
    h.set_graph_attr("note", serde_json::json!("second graph attr"));
    h
}

#[test]
fn test_to_hypergraph_dict_schema() {
    let h = sample();
    let d = to_hypergraph_dict(&h);
    assert_eq!(d["type"], serde_json::json!("hypergraph"));
    assert_eq!(d["hypergraph-data"]["name"], serde_json::json!("sample"));
    // every graph attr is serialized, not just "name" (XGI updates the whole
    // hypergraph-data dict from H._net_attr).
    assert_eq!(
        d["hypergraph-data"]["note"],
        serde_json::json!("second graph attr")
    );
    assert!(d["node-data"].get("1").is_some());
    assert!(d["edge-data"].get("e0").is_some());
    // edge-dict members are sorted strings
    assert_eq!(
        d["edge-dict"]["e0"],
        serde_json::json!(["1", "2", "3"])
    );
}

#[test]
fn test_json_round_trip_preserves_structure() {
    let h = sample();
    let d = to_hypergraph_dict(&h);
    let h2 = from_hypergraph_dict(&d).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    // members as sets
    let mut m = h2.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    assert_eq!(h2.edge_attrs("e0").unwrap()["weight"], 2);
    assert_eq!(h2.graph_attr("name"), Some(&serde_json::json!("sample")));
    assert_eq!(
        h2.graph_attr("note"),
        Some(&serde_json::json!("second graph attr"))
    );
}

#[test]
fn test_write_read_json_file_round_trip() {
    let h = sample();
    let dir = std::env::temp_dir();
    let path = dir.join("hypergraph_rs_test_json.json");
    write_json(&h, &path).unwrap();
    let h2 = read_json(&path).unwrap();
    assert_eq!(h2.num_nodes(), h.num_nodes());
    assert_eq!(h2.num_edges(), h.num_edges());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_edgelist_whitespace() {
    let lines = vec!["1 2 3".to_string(), "2 3 4 5".to_string(), "# a comment".to_string()];
    let h = parse_edgelist(lines, None, "#").unwrap();
    assert_eq!(h.num_edges(), 2);
    assert_eq!(h.num_nodes(), 5);
}

#[test]
fn test_parse_edgelist_custom_delimiter() {
    let lines = vec!["a,b,c".to_string(), "b,c".to_string()];
    let h = parse_edgelist(lines, Some(','), "#").unwrap();
    assert_eq!(h.num_edges(), 2);
    let mut m = h.members("0").unwrap();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_write_read_edgelist_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into(), "3".into()], None, serde_json::Value::Null).unwrap();
    h.add_edge(vec!["3".into(), "4".into()], None, serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_edgelist.csv");
    write_edgelist(&h, &path, ',').unwrap();
    let h2 = read_edgelist(&path, Some(','), "#").unwrap();
    assert_eq!(h2.num_edges(), 2);
    assert_eq!(h2.num_nodes(), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_parse_bipartite_basic() {
    // node edge pairs: 1->e0, 2->e0, 2->e1
    let lines = vec!["1 e0".to_string(), "2 e0".to_string(), "2 e1".to_string()];
    let h = parse_bipartite_edgelist(lines, None, "#", false).unwrap();
    assert_eq!(h.num_nodes(), 2);
    assert_eq!(h.num_edges(), 2);
    let mut m = h.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn test_parse_bipartite_dual_swaps_columns() {
    // dual: first column is edge, second is node
    let lines = vec!["e0 1".to_string(), "e0 2".to_string()];
    let h = parse_bipartite_edgelist(lines, None, "#", true).unwrap();
    assert_eq!(h.num_edges(), 1);
    let mut m = h.members("e0").unwrap();
    m.sort();
    assert_eq!(m, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn test_parse_bipartite_rejects_short_line() {
    let lines = vec!["only-one-token".to_string()];
    assert!(parse_bipartite_edgelist(lines, None, "#", false).is_err());
}

#[test]
fn test_write_read_bipartite_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["1".into(), "2".into()], Some("e0".into()), serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_bipartite.csv");
    write_bipartite_edgelist(&h, &path, ' ').unwrap();
    let h2 = read_bipartite_edgelist(&path, None, "#", false).unwrap();
    assert_eq!(h2.num_edges(), 1);
    assert_eq!(h2.num_nodes(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_read_incidence_round_trip() {
    // 3 nodes, 2 edges: edge0={0,1}, edge1={1,2}
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["0".into(), "1".into()], Some("0".into()), serde_json::Value::Null).unwrap();
    h.add_edge(vec!["1".into(), "2".into()], Some("1".into()), serde_json::Value::Null).unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_incidence.csv");
    write_incidence_matrix(&h, &path, ' ').unwrap();
    let h2 = read_incidence_matrix(&path, None).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    // positional edge "0" has members {0,1}
    let mut m = h2.members("0").unwrap();
    m.sort();
    assert_eq!(m, vec!["0".to_string(), "1".to_string()]);
    let _ = std::fs::remove_file(&path);
}
