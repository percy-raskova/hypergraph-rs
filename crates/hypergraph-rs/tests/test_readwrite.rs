use hypergraph_rs::readwrite::bipartite::{
    parse_bipartite_edgelist, read_bipartite_edgelist, write_bipartite_edgelist,
};
use hypergraph_rs::readwrite::edgelist::{parse_edgelist, read_edgelist, write_edgelist};
use hypergraph_rs::readwrite::hif::{from_hif_dict, read_hif, to_hif_dict, write_hif};
use hypergraph_rs::readwrite::incidence::{read_incidence_matrix, write_incidence_matrix};
use hypergraph_rs::Hypergraph;

#[test]
fn test_parse_edgelist_whitespace() {
    let lines = vec![
        "1 2 3".to_string(),
        "2 3 4 5".to_string(),
        "# a comment".to_string(),
    ];
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
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        None,
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(vec!["3".into(), "4".into()], None, serde_json::Value::Null)
        .unwrap();
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
    h.add_edge(
        vec!["1".into(), "2".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
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
    h.add_edge(
        vec!["0".into(), "1".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["1".into(), "2".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
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

#[test]
fn test_to_hif_dict_schema() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into()],
        Some("e0".into()),
        serde_json::json!({"w": 3}),
    )
    .unwrap();
    h.add_node("iso", serde_json::json!({"color": "red"}));
    h.set_graph_attr("name", serde_json::json!("hif-sample"));
    h.set_graph_attr("note", serde_json::json!("second graph attr"));
    let d = to_hif_dict(&h);
    assert_eq!(d["network-type"], serde_json::json!("undirected"));
    // every graph attr is serialized into metadata, not just "name"
    assert_eq!(d["metadata"]["name"], serde_json::json!("hif-sample"));
    assert_eq!(
        d["metadata"]["note"],
        serde_json::json!("second graph attr")
    );
    // two incidences for e0
    let inc = d["incidences"].as_array().unwrap();
    assert_eq!(inc.len(), 2);
    // isolate node with attrs appears in nodes
    let nodes = d["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["node"] == serde_json::json!("iso")));
    // edge with attrs appears in edges
    let edges = d["edges"].as_array().unwrap();
    assert!(edges.iter().any(|e| e["edge"] == serde_json::json!("e0")));
}

#[test]
fn test_hif_round_trip() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("e0".into()),
        serde_json::json!({"w": 3}),
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into()],
        Some("e1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let d = to_hif_dict(&h);
    let h2 = from_hif_dict(&d).unwrap();
    assert_eq!(h2.num_nodes(), 3);
    assert_eq!(h2.num_edges(), 2);
    assert_eq!(h2.edge_attrs("e0").unwrap()["w"], 3);
}

#[test]
fn test_write_read_hif_file() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".into(), "b".into()],
        Some("e0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    let path = std::env::temp_dir().join("hypergraph_rs_test_hif.json");
    write_hif(&h, &path).unwrap();
    let h2 = read_hif(&path).unwrap();
    assert_eq!(h2.num_nodes(), 2);
    assert_eq!(h2.num_edges(), 1);
    let _ = std::fs::remove_file(&path);
}

use hypergraph_rs::readwrite::bigg::bigg_dict_to_dihypergraph;

#[test]
fn test_bigg_forward_reaction() {
    // minimal BIGG model dict: metabolites A,B; one forward reaction A -> B
    let model = serde_json::json!({
        "id": "toy",
        "metabolites": [{"id": "A", "name": "alpha"}, {"id": "B", "name": "beta"}],
        "reactions": [{
            "id": "r1", "name": "reaction one",
            "lower_bound": 0.0, "upper_bound": 1000.0,
            "metabolites": {"A": -1.0, "B": 1.0}
        }]
    });
    let dh = bigg_dict_to_dihypergraph(&model, None).unwrap();
    assert_eq!(dh.num_nodes(), 2);
    assert_eq!(dh.num_edges(), 1);
    // A is reactant (tail), B is product (head)
    assert_eq!(dh.tail("r1").unwrap(), vec!["A".to_string()]);
    assert_eq!(dh.head("r1").unwrap(), vec!["B".to_string()]);
    // FINDING 3: XGI's `_bigg_to_dihypergraph` sets each reaction edge's
    // `name` attr from `r["name"]` (`DH.add_edge(..., idx=r["id"], name=r["name"])`,
    // bigg_data.py:129). We must carry that through, not `Value::Null`.
    assert_eq!(
        dh.edge_attrs("r1").unwrap()["name"],
        serde_json::json!("reaction one")
    );
}

#[test]
fn test_bigg_reversible_reaction_emits_two_edges() {
    let model = serde_json::json!({
        "id": "toy",
        "metabolites": [{"id": "A", "name": "alpha"}, {"id": "B", "name": "beta"}],
        "reactions": [{
            "id": "r1", "name": "rev",
            "lower_bound": -1000.0, "upper_bound": 1000.0,
            "metabolites": {"A": -1.0, "B": 1.0}
        }]
    });
    let dh = bigg_dict_to_dihypergraph(&model, None).unwrap();
    assert_eq!(dh.num_edges(), 2);
    assert!(dh.has_edge("r1"));
    assert!(dh.has_edge("r1_reverse"));
    // FINDING 3: both the forward and `_reverse` edge share `r["name"]`
    // (bigg_data.py:156-160 passes the same `name=r["name"]` to both add_edge calls).
    assert_eq!(
        dh.edge_attrs("r1").unwrap()["name"],
        serde_json::json!("rev")
    );
    assert_eq!(
        dh.edge_attrs("r1_reverse").unwrap()["name"],
        serde_json::json!("rev")
    );
}
