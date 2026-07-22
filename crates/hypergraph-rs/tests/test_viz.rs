use hypergraph_rs::layout::{BipartiteAlign, HyperedgeGeometry, LayoutStrategy};
use hypergraph_rs::viz::SceneGraph;
use hypergraph_rs::Hypergraph;

fn sample_h() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::json!({"heat": 0.5}),
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_render_produces_glyph_per_node_and_edge() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert_eq!(scene.nodes.len(), h.num_nodes());
    assert_eq!(scene.hyperedges.len(), h.num_edges());
}

#[test]
fn test_render_preserves_node_insertion_order() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical });
    let ids: Vec<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}

#[test]
fn test_render_carries_edge_attrs_and_members() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let e1 = scene.hyperedges.iter().find(|g| g.id == "e1").unwrap();
    assert_eq!(e1.attrs["heat"], 0.5);
    // members() is deterministic petgraph adjacency order, not insertion
    // order — compare as a sorted set rather than assert a specific order.
    let mut m = e1.member_ids.clone();
    m.sort();
    assert_eq!(m, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn test_render_hyperedge_aware_geometry_is_convex_hull() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    for g in &scene.hyperedges {
        assert!(matches!(g.geometry, HyperedgeGeometry::ConvexHull { .. }));
    }
}

#[test]
fn test_render_bounding_box_well_formed() {
    let h = sample_h();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let (min_x, min_y, max_x, max_y) = scene.bounding_box;
    assert!(max_x >= min_x && max_y >= min_y);
}

#[test]
fn test_render_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let scene = h.render(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    assert!(scene.nodes.is_empty());
    assert!(scene.hyperedges.is_empty());
}

#[test]
fn test_render_to_json_round_trips() {
    let h = sample_h();
    let json = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let scene: SceneGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(scene.nodes.len(), h.num_nodes());
    assert_eq!(scene.hyperedges.len(), h.num_edges());
}

#[test]
fn test_render_to_json_matches_render() {
    let h = sample_h();
    let strat = LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical };
    let direct = serde_json::to_string(&h.render(strat.clone())).unwrap();
    let via = h.render_to_json(strat);
    assert_eq!(direct, via);
}

#[test]
fn test_render_to_json_deterministic() {
    let h = sample_h();
    let a = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(7) });
    let b = h.render_to_json(LayoutStrategy::Spring { iterations: 50, seed: Some(7) });
    assert_eq!(a, b);
}

#[test]
fn test_render_to_svg_is_well_formed() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("viewBox="));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
}

#[test]
fn test_render_to_svg_has_a_circle_per_node() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(1) });
    let circles = svg.matches("<circle").count();
    assert_eq!(circles, h.num_nodes());
}

#[test]
fn test_render_to_svg_hyperedge_aware_has_polygon() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::HyperedgeAware { iterations: 50, halo_padding: 0.05 });
    assert!(svg.contains("<polygon"));
}

#[test]
fn test_render_to_svg_bipartite_has_line() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical });
    assert!(svg.contains("<line"));
}

#[test]
fn test_render_to_svg_radial_has_path() {
    let h = sample_h();
    let svg = h.render_to_svg(LayoutStrategy::Radial { rings: 3 });
    assert!(svg.contains("<path"));
}

#[test]
fn test_render_to_svg_deterministic() {
    let h = sample_h();
    let a = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(3) });
    let b = h.render_to_svg(LayoutStrategy::Spring { iterations: 50, seed: Some(3) });
    assert_eq!(a, b);
}

#[test]
fn test_render_to_svg_empty_hypergraph_is_valid() {
    let h: Hypergraph = Hypergraph::new();
    let svg = h.render_to_svg(LayoutStrategy::Spring { iterations: 10, seed: Some(1) });
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
}
