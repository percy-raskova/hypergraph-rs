use hypergraph_rs::layout::{
    BipartiteAlign, HyperedgeGeometry, LayoutResult, LayoutStrategy, DEFAULT_LAYOUT_SEED,
};

#[test]
fn test_layout_strategy_variants_construct() {
    let _a = LayoutStrategy::Spring { iterations: 50, seed: Some(7) };
    let _b = LayoutStrategy::Bipartite { align: BipartiteAlign::Vertical };
    let _c = LayoutStrategy::HyperedgeAware { iterations: 100, halo_padding: 0.5 };
    let _d = LayoutStrategy::Radial { rings: 3 };
    assert_eq!(DEFAULT_LAYOUT_SEED, 0);
}

#[test]
fn test_layout_strategy_serde_round_trip() {
    let s = LayoutStrategy::HyperedgeAware { iterations: 100, halo_padding: 0.25 };
    let json = serde_json::to_string(&s).unwrap();
    let back: LayoutStrategy = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}

#[test]
fn test_hyperedge_geometry_variants_serde() {
    let variants = vec![
        HyperedgeGeometry::ConvexHull { points: vec![(0.0, 0.0), (1.0, 1.0)] },
        HyperedgeGeometry::Spline { control_points: vec![(0.0, 0.0)] },
        HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 2.0 },
        HyperedgeGeometry::Arc { center: (0.0, 0.0), radius: 1.0, angle_range: (0.0, 3.14) },
    ];
    for g in variants {
        let json = serde_json::to_string(&g).unwrap();
        let back: HyperedgeGeometry = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}

#[test]
fn test_layout_result_serde_round_trip() {
    let r = LayoutResult {
        node_positions: vec![("a".to_string(), (0.1, 0.2))],
        edge_geometries: vec![(
            "e1".to_string(),
            HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 1.0 },
        )],
        bounding_box: (0.0, 0.0, 1.0, 1.0),
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: LayoutResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
}

use hypergraph_rs::viz::{HyperedgeGlyph, HyperedgeStyle, NodeGlyph, NodeStyle, SceneGraph};

#[test]
fn test_scene_graph_serde_round_trip() {
    let scene = SceneGraph {
        nodes: vec![NodeGlyph {
            id: "a".to_string(),
            x: 0.0,
            y: 0.0,
            radius: 0.02,
            attrs: serde_json::Value::Null,
            style: NodeStyle::default(),
        }],
        hyperedges: vec![HyperedgeGlyph {
            id: "e1".to_string(),
            member_ids: vec!["a".to_string()],
            geometry: HyperedgeGeometry::Stripe { x: 1.0, y_min: 0.0, y_max: 1.0 },
            attrs: serde_json::Value::Null,
            style: HyperedgeStyle::default(),
        }],
        bounding_box: (0.0, 0.0, 1.0, 1.0),
        metadata: serde_json::Map::new(),
    };
    let json = serde_json::to_string(&scene).unwrap();
    let back: SceneGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}

use hypergraph_rs::layout::convex_hull;

#[test]
fn test_convex_hull_square_excludes_interior() {
    let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)];
    let hull = convex_hull(&pts);
    assert_eq!(hull.len(), 4);
    assert!(!hull.contains(&(0.5, 0.5)));
    for corner in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
        assert!(hull.contains(&corner), "hull must contain corner {:?}", corner);
    }
}

#[test]
fn test_convex_hull_triangle() {
    let pts = vec![(0.0, 0.0), (2.0, 0.0), (0.0, 2.0)];
    let hull = convex_hull(&pts);
    assert_eq!(hull.len(), 3);
}

#[test]
fn test_convex_hull_two_points() {
    let hull = convex_hull(&[(0.0, 0.0), (1.0, 1.0)]);
    assert_eq!(hull.len(), 2);
}

#[test]
fn test_convex_hull_single_point() {
    let hull = convex_hull(&[(0.5, 0.5)]);
    assert_eq!(hull, vec![(0.5, 0.5)]);
}

#[test]
fn test_convex_hull_empty() {
    let hull = convex_hull(&[]);
    assert!(hull.is_empty());
}

#[test]
fn test_convex_hull_deterministic() {
    let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.3, 0.7)];
    assert_eq!(convex_hull(&pts), convex_hull(&pts));
}
