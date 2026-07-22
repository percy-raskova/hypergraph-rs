use hypergraph_rs::layout::{
    BipartiteAlign, HyperedgeGeometry, LayoutResult, LayoutStrategy, DEFAULT_LAYOUT_SEED,
};

#[test]
fn test_layout_strategy_variants_construct() {
    let _a = LayoutStrategy::Spring {
        iterations: 50,
        seed: Some(7),
    };
    let _b = LayoutStrategy::Bipartite {
        align: BipartiteAlign::Vertical,
    };
    let _c = LayoutStrategy::HyperedgeAware {
        iterations: 100,
        halo_padding: 0.5,
    };
    let _d = LayoutStrategy::Radial { rings: 3 };
    assert_eq!(DEFAULT_LAYOUT_SEED, 0);
}

#[test]
fn test_layout_strategy_serde_round_trip() {
    let s = LayoutStrategy::HyperedgeAware {
        iterations: 100,
        halo_padding: 0.25,
    };
    let json = serde_json::to_string(&s).unwrap();
    let back: LayoutStrategy = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}

#[test]
fn test_hyperedge_geometry_variants_serde() {
    let variants = vec![
        HyperedgeGeometry::ConvexHull {
            points: vec![(0.0, 0.0), (1.0, 1.0)],
        },
        HyperedgeGeometry::Spline {
            control_points: vec![(0.0, 0.0)],
        },
        HyperedgeGeometry::Stripe {
            x: 1.0,
            y_min: 0.0,
            y_max: 2.0,
        },
        HyperedgeGeometry::Arc {
            center: (0.0, 0.0),
            radius: 1.0,
            angle_range: (0.0, 3.0),
        },
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
            HyperedgeGeometry::Stripe {
                x: 1.0,
                y_min: 0.0,
                y_max: 1.0,
            },
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
            geometry: HyperedgeGeometry::Stripe {
                x: 1.0,
                y_min: 0.0,
                y_max: 1.0,
            },
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
        assert!(
            hull.contains(&corner),
            "hull must contain corner {:?}",
            corner
        );
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

use hypergraph_rs::layout::bipartite_layout;
use hypergraph_rs::Hypergraph;

fn sample_h() -> Hypergraph {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h.add_edge(
        vec!["b".to_string(), "c".to_string(), "d".to_string()],
        Some("e2".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    h
}

#[test]
fn test_bipartite_positions_every_node() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_bipartite_vertical_agents_in_left_column() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    // Vertical align: all agents share x = 0.0.
    for (_, (x, _)) in &r.node_positions {
        assert_eq!(*x, 0.0);
    }
}

#[test]
fn test_bipartite_insertion_order_preserved() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    let ids: Vec<&str> = r.node_positions.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_bipartite_emits_stripe_geometry() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Stripe { .. }));
    }
}

#[test]
fn test_bipartite_deterministic() {
    let h = sample_h();
    assert_eq!(
        bipartite_layout(&h, BipartiteAlign::Vertical),
        bipartite_layout(&h, BipartiteAlign::Vertical)
    );
}

#[test]
fn test_bipartite_bounding_box_well_formed() {
    let h = sample_h();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    let (min_x, min_y, max_x, max_y) = r.bounding_box;
    assert!(max_x >= min_x);
    assert!(max_y >= min_y);
}

#[test]
fn test_bipartite_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = bipartite_layout(&h, BipartiteAlign::Vertical);
    assert!(r.node_positions.is_empty());
    assert!(r.edge_geometries.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}

use hypergraph_rs::layout::radial_layout;

#[test]
fn test_radial_positions_every_node() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_radial_emits_arc_geometry() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Arc { .. }));
    }
}

#[test]
fn test_radial_deterministic() {
    let h = sample_h();
    assert_eq!(radial_layout(&h, 3), radial_layout(&h, 3));
}

#[test]
fn test_radial_zero_rings_does_not_panic() {
    let h = sample_h();
    // rings is clamped to at least 1 internally.
    let r = radial_layout(&h, 0);
    assert_eq!(r.node_positions.len(), h.num_nodes());
}

#[test]
fn test_radial_positions_within_unit_disk() {
    let h = sample_h();
    let r = radial_layout(&h, 3);
    for (_, (x, y)) in &r.node_positions {
        assert!((x * x + y * y).sqrt() <= 1.0 + 1e-9);
    }
}

#[test]
fn test_radial_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = radial_layout(&h, 3);
    assert!(r.node_positions.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}

use hypergraph_rs::layout::spring_layout;

#[test]
fn test_spring_positions_every_node() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_spring_same_seed_is_bit_identical() {
    let h = sample_h();
    let a = spring_layout(&h, 50, Some(42));
    let b = spring_layout(&h, 50, Some(42));
    assert_eq!(a, b);
}

#[test]
fn test_spring_different_seed_differs() {
    let h = sample_h();
    let a = spring_layout(&h, 50, Some(1));
    let b = spring_layout(&h, 50, Some(2));
    assert!(a.node_positions != b.node_positions);
}

#[test]
fn test_spring_none_seed_is_deterministic() {
    let h = sample_h();
    // seed=None falls back to DEFAULT_LAYOUT_SEED — still deterministic.
    let a = spring_layout(&h, 50, None);
    let b = spring_layout(&h, 50, None);
    assert_eq!(a, b);
}

#[test]
fn test_spring_emits_spline_geometry() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::Spline { .. }));
    }
}

#[test]
fn test_spring_positions_finite() {
    let h = sample_h();
    let r = spring_layout(&h, 50, Some(42));
    for (_, (x, y)) in &r.node_positions {
        assert!(x.is_finite() && y.is_finite());
    }
}

#[test]
fn test_spring_single_node() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_node("solo", serde_json::Value::Null);
    let r = spring_layout(&h, 50, Some(42));
    assert_eq!(r.node_positions.len(), 1);
}

#[test]
fn test_spring_empty_hypergraph() {
    let h: Hypergraph = Hypergraph::new();
    let r = spring_layout(&h, 50, Some(42));
    assert!(r.node_positions.is_empty());
    assert_eq!(r.bounding_box, (0.0, 0.0, 0.0, 0.0));
}

use hypergraph_rs::layout::hyperedge_aware_layout;

#[test]
fn test_hyperedge_aware_positions_every_node() {
    let h = sample_h();
    let r = hyperedge_aware_layout(&h, 50, 0.05);
    assert_eq!(r.node_positions.len(), h.num_nodes());
    assert_eq!(r.edge_geometries.len(), h.num_edges());
}

#[test]
fn test_hyperedge_aware_emits_convex_hull_geometry() {
    let h = sample_h();
    let r = hyperedge_aware_layout(&h, 50, 0.05);
    for (_, g) in &r.edge_geometries {
        assert!(matches!(g, HyperedgeGeometry::ConvexHull { .. }));
    }
}

#[test]
fn test_hyperedge_aware_deterministic() {
    let h = sample_h();
    assert_eq!(
        hyperedge_aware_layout(&h, 50, 0.05),
        hyperedge_aware_layout(&h, 50, 0.05)
    );
}

#[test]
fn test_hyperedge_aware_padding_grows_hull() {
    let h = sample_h();
    let no_pad = hyperedge_aware_layout(&h, 50, 0.0);
    let padded = hyperedge_aware_layout(&h, 50, 0.3);
    // The bounding box must grow (or stay equal) when the halo padding grows.
    let (nx0, ny0, nx1, ny1) = no_pad.bounding_box;
    let (px0, py0, px1, py1) = padded.bounding_box;
    let no_area = (nx1 - nx0) * (ny1 - ny0);
    let pad_area = (px1 - px0) * (py1 - py0);
    assert!(pad_area >= no_area - 1e-9);
}

#[test]
fn test_hyperedge_aware_hull_has_at_least_two_points_for_triangle_edge() {
    let mut h: Hypergraph = Hypergraph::new();
    h.add_edge(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        Some("e1".to_string()),
        serde_json::Value::Null,
    )
    .unwrap();
    let r = hyperedge_aware_layout(&h, 80, 0.05);
    let (_, geom) = &r.edge_geometries[0];
    if let HyperedgeGeometry::ConvexHull { points } = geom {
        // >= 2, not == 3: FR-placed members could be (near-)collinear, which
        // collapses the hull to a 2-point segment. >= 2 is the safe bound.
        assert!(
            points.len() >= 2,
            "a 3-member edge should yield at least a degenerate (segment) hull"
        );
    } else {
        panic!("expected ConvexHull geometry");
    }
}

use hypergraph_rs::layout::compute_layout;

#[test]
fn test_compute_layout_spring_arm() {
    let h = sample_h();
    let r = compute_layout(
        &h,
        LayoutStrategy::Spring {
            iterations: 50,
            seed: Some(42),
        },
    );
    assert_eq!(r, spring_layout(&h, 50, Some(42)));
}

#[test]
fn test_compute_layout_bipartite_arm() {
    let h = sample_h();
    let r = compute_layout(
        &h,
        LayoutStrategy::Bipartite {
            align: BipartiteAlign::Vertical,
        },
    );
    assert_eq!(r, bipartite_layout(&h, BipartiteAlign::Vertical));
}

#[test]
fn test_compute_layout_hyperedge_aware_arm() {
    let h = sample_h();
    let r = compute_layout(
        &h,
        LayoutStrategy::HyperedgeAware {
            iterations: 50,
            halo_padding: 0.05,
        },
    );
    assert_eq!(r, hyperedge_aware_layout(&h, 50, 0.05));
}

#[test]
fn test_compute_layout_radial_arm() {
    let h = sample_h();
    let r = compute_layout(&h, LayoutStrategy::Radial { rings: 3 });
    assert_eq!(r, radial_layout(&h, 3));
}
