use hypergraph_rs::stats::diedgestats::{di_edge_size, di_head_size, di_tail_size};
use hypergraph_rs::stats::dinodestats::{di_in_degree, di_out_degree};
use hypergraph_rs::stats::edgestats::{edge_attrs, edge_order, edge_size};
use hypergraph_rs::stats::nodestats::{node_attrs, node_degree};
use hypergraph_rs::DiHypergraph;
use hypergraph_rs::Hypergraph;

fn di_fixture() -> DiHypergraph {
    // XGI: DiHypergraph([[{1,2},{5,6}], [{4},{1,3}]])
    // edge 0: tail {1,2} head {5,6}; edge 1: tail {4} head {1,3}
    let mut d = DiHypergraph::new();
    d.add_edge(
        vec!["1".into(), "2".into()],
        vec!["5".into(), "6".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    d.add_edge(
        vec!["4".into()],
        vec!["1".into(), "3".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    d
}

fn triangle_plus() -> Hypergraph {
    // XGI docstring fixture: [[1,2,3],[2,3,4,5],[3,4,5]]
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["1".into(), "2".into(), "3".into()],
        Some("0".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["2".into(), "3".into(), "4".into(), "5".into()],
        Some("1".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["3".into(), "4".into(), "5".into()],
        Some("2".into()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_node_degree_matches_xgi() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // XGI: {1:1, 2:2, 3:3, 4:2, 5:2}
    assert_eq!(*deg.get("1").unwrap(), 1);
    assert_eq!(*deg.get("2").unwrap(), 2);
    assert_eq!(*deg.get("3").unwrap(), 3);
    assert_eq!(*deg.get("4").unwrap(), 2);
    assert_eq!(*deg.get("5").unwrap(), 2);
}

#[test]
fn test_node_degree_by_order() {
    let h = triangle_plus();
    // order=2 -> only size-3 edges (edges 0 and 2)
    let deg = node_degree(&h, Some(2));
    assert_eq!(*deg.get("3").unwrap(), 2); // 3 is in both size-3 edges
    assert_eq!(*deg.get("2").unwrap(), 1); // 2 only in edge 0 (size 3)
}

#[test]
fn test_node_degree_aslist_order() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    // node insertion order is 1,2,3,4,5
    assert_eq!(deg.aslist(), vec![1, 2, 3, 2, 2]);
}

#[test]
fn test_node_degree_filterby() {
    let h = triangle_plus();
    let deg = node_degree(&h, None);
    let mut two = deg.filterby(&2);
    two.sort();
    assert_eq!(two, vec!["2".to_string(), "4".to_string(), "5".to_string()]);
}

#[test]
fn test_node_attrs_single() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"color": "red"}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let colors = node_attrs(&h, Some("color"));
    assert_eq!(colors.get("a").unwrap(), &serde_json::json!("red"));
    assert_eq!(colors.get("b").unwrap(), &serde_json::json!("blue"));
}

#[test]
fn test_node_attrs_missing_imputes_null() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("a", serde_json::json!({"age": 20}));
    h.add_node("b", serde_json::json!({"color": "blue"}));
    let ages = node_attrs(&h, Some("age"));
    assert_eq!(ages.get("a").unwrap(), &serde_json::json!(20));
    assert_eq!(ages.get("b").unwrap(), &serde_json::Value::Null);
}

#[test]
fn test_edge_order_and_size_match_xgi() {
    let h = triangle_plus();
    let ord = edge_order(&h, None);
    // XGI: order {0:2, 1:3, 2:2}
    assert_eq!(*ord.get("0").unwrap(), 2);
    assert_eq!(*ord.get("1").unwrap(), 3);
    assert_eq!(*ord.get("2").unwrap(), 2);
    let sz = edge_size(&h, None);
    // size {0:3, 1:4, 2:3}
    assert_eq!(*sz.get("0").unwrap(), 3);
    assert_eq!(*sz.get("1").unwrap(), 4);
    assert_eq!(*sz.get("2").unwrap(), 3);
}

#[test]
fn test_edge_size_aslist_order() {
    let h = triangle_plus();
    assert_eq!(edge_size(&h, None).aslist(), vec![3, 4, 3]);
}

#[test]
fn test_edge_order_degree_filter_zero_matches_clamps_at_zero() {
    let h = triangle_plus();
    // No node in edge "0" ({1,2,3}) has degree 99. XGI's order(degree) would
    // report -1 (sum(0) - 1); our Stat<usize> cannot represent a negative
    // order, so this clamps at 0 (documented deviation #7).
    let ord = edge_order(&h, Some(99));
    assert_eq!(*ord.get("0").unwrap(), 0);
}

#[test]
fn test_edge_attrs_single_with_missing() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(vec!["a".into()], Some("one".into()), serde_json::json!({"color": "red"})).unwrap();
    h.add_edge(vec!["b".into()], Some("two".into()), serde_json::json!({"age": 30})).unwrap();
    let colors = edge_attrs(&h, Some("color"));
    assert_eq!(colors.get("one").unwrap(), &serde_json::json!("red"));
    assert_eq!(colors.get("two").unwrap(), &serde_json::Value::Null);
}

#[test]
fn test_di_edge_sizes() {
    let d = di_fixture();
    // union sizes: edge 0 = {1,2,5,6} = 4; edge 1 = {4,1,3} = 3
    let sz = di_edge_size(&d);
    assert_eq!(*sz.get("0").unwrap(), 4);
    assert_eq!(*sz.get("1").unwrap(), 3);
    let hs = di_head_size(&d);
    assert_eq!(*hs.get("0").unwrap(), 2); // head {5,6}
    let ts = di_tail_size(&d);
    assert_eq!(*ts.get("1").unwrap(), 1); // tail {4}
}

#[test]
fn test_di_node_in_out_degree() {
    let d = di_fixture();
    // XGI convention: in_degree counts HEAD-membership (receiver), out_degree
    // counts TAIL-membership (sender).
    // node 1: tail of edge 0 (a sender), head of edge 1 (a receiver)
    //   -> in_degree(head-membership)=1, out_degree(tail-membership)=1
    let ind = di_in_degree(&d);
    let outd = di_out_degree(&d);
    assert_eq!(*ind.get("1").unwrap(), 1);
    assert_eq!(*outd.get("1").unwrap(), 1);
    // node 4: ONLY in the tail of edge 1 (a sender, never a receiver) — this is
    // the asymmetric case that distinguishes the two conventions.
    assert_eq!(*ind.get("4").unwrap(), 0);
    assert_eq!(*outd.get("4").unwrap(), 1);
    // node 6: ONLY in the head of edge 0 (a receiver, never a sender).
    assert_eq!(*ind.get("6").unwrap(), 1);
    assert_eq!(*outd.get("6").unwrap(), 0);
}
