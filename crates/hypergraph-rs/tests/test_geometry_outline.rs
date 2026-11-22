#![cfg(feature = "geometry")]

use hypergraph_rs::geometry::{
    contains, edge_outline, signed_area, weighted_outline, GeometryError, Outline, OutlineParams,
    Positions, WeightedOutline, DEFAULT_LEVEL_FRACTION, DEFAULT_MAX_GRID_NODES,
};
use hypergraph_rs::Hypergraph;

type Point = (f64, f64);

fn params(share: f64) -> OutlineParams {
    OutlineParams::new(1.0, 0.125, share)
}

fn outline_of(members: &[(Point, f64)], share: f64) -> WeightedOutline {
    weighted_outline(members, &params(share))
        .expect("valid input")
        .expect("some member above zero")
}

fn paths_of(w: &WeightedOutline) -> &Vec<Vec<Point>> {
    match &w.outline {
        Outline::Membrane { paths } => paths,
        other => panic!("expected a membrane, got {other:?}"),
    }
}

fn square3(at: Point) -> Vec<(Point, f64)> {
    let mut v = Vec::new();
    for i in -1..=1 {
        for j in -1..=1 {
            v.push((
                (at.0 + f64::from(i) * 0.25, at.1 + f64::from(j) * 0.25),
                1.0,
            ));
        }
    }
    v
}

fn u_shape() -> Vec<Point> {
    let mut v = Vec::new();
    for k in 0..=8 {
        let y = f64::from(k) * 0.5;
        v.push((0.0, y));
        v.push((4.0, y));
    }
    for k in 1..=7 {
        v.push((f64::from(k) * 0.5, 0.0));
    }
    v
}

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 40) as f64 / (1_u64 << 24) as f64
}

fn random_set() -> Vec<(Point, f64)> {
    let mut state = 99_u64;
    (0..300)
        .map(|k| {
            let p = (lcg(&mut state) * 10.0, lcg(&mut state) * 10.0);
            (p, f64::from(k % 5 + 1) / 5.0)
        })
        .collect()
}

fn bits(w: &WeightedOutline) -> Vec<(u64, u64)> {
    paths_of(w)
        .iter()
        .flatten()
        .map(|p| (p.0.to_bits(), p.1.to_bits()))
        .collect()
}

#[test]
fn defaults_are_those_of_the_plan() {
    let p = OutlineParams::new(2.0, 0.5, 0.8);
    assert_eq!(p.level_fraction, DEFAULT_LEVEL_FRACTION);
    assert_eq!(p.max_grid_nodes, DEFAULT_MAX_GRID_NODES);
    assert_eq!((p.reach, p.cell, p.share), (2.0, 0.5, 0.8));
}

#[test]
fn one_member_is_a_ring() {
    let w = outline_of(&[((3.0, 4.0), 2.0)], 0.9);
    assert_eq!(
        w.outline,
        Outline::Ring {
            center: (3.0, 4.0),
            radius: 1.0
        }
    );
    assert_eq!(w.held, vec![0]);
    assert_eq!(w.held_weight, 2.0);
    assert_eq!(w.total_weight, 2.0);
}

#[test]
fn two_members_are_a_capsule_thicker_at_the_heavier_end() {
    let w = outline_of(&[((0.0, 0.0), 2.0), ((5.0, 0.0), 1.0)], 1.0);
    assert_eq!(
        w.outline,
        Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 1.0,
            b: (5.0, 0.0),
            radius_b: 0.75
        }
    );
}

#[test]
fn two_members_on_one_spot_are_a_capsule_with_equal_ends() {
    let w = outline_of(&[((1.0, 1.0), 1.0), ((1.0, 1.0), 1.0)], 1.0);
    assert_eq!(
        w.outline,
        Outline::Capsule {
            a: (1.0, 1.0),
            radius_a: 1.0,
            b: (1.0, 1.0),
            radius_b: 1.0
        }
    );
    assert_eq!(w.held, vec![0, 1]);
}

#[test]
fn three_members_on_one_spot_make_one_round_membrane() {
    let w = outline_of(&[((1.0, 1.0), 1.0); 3], 1.0);
    let paths = paths_of(&w);
    assert_eq!(paths.len(), 1);
    let area = signed_area(&paths[0]);
    let expected = std::f64::consts::PI * (1.0 - 0.5_f64.sqrt());
    assert!(area > 0.0);
    assert!(
        (area - expected).abs() < 0.08 * expected,
        "{area} vs {expected}"
    );
    for pair in paths[0].windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
    assert_ne!(paths[0][0], *paths[0].last().unwrap());
    assert_eq!(w.held, vec![0, 1, 2]);
}

#[test]
fn three_or_more_are_a_membrane_that_need_not_be_convex() {
    let members: Vec<(Point, f64)> = u_shape().into_iter().map(|p| (p, 1.0)).collect();
    let w = outline_of(&members, 1.0);
    let paths = paths_of(&w);
    assert!(!contains(paths, (2.0, 3.0)));
    assert_eq!(w.held.len(), members.len());
}

#[test]
fn far_apart_groups_make_separate_pieces() {
    let mut members = square3((0.0, 0.0));
    members.extend(square3((20.0, 0.0)));
    let w = outline_of(&members, 1.0);
    let paths = paths_of(&w);
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|p| signed_area(p) > 0.0));
}

#[test]
fn a_ring_of_members_leaves_a_hole() {
    let members: Vec<(Point, f64)> = (0..24)
        .map(|k| {
            // 24 points on a circle of radius 3, from exact fractions of the turn.
            let (c, s) = circle24(k);
            ((3.0 * c, 3.0 * s), 1.0)
        })
        .collect();
    let w = outline_of(&members, 1.0);
    let paths = paths_of(&w);
    assert_eq!(paths.iter().filter(|p| signed_area(p) > 0.0).count(), 1);
    assert_eq!(paths.iter().filter(|p| signed_area(p) < 0.0).count(), 1);
    assert!(!contains(paths, (0.0, 0.0)));
}

/// Cosine and sine of `k` twenty-fourths of a turn, by symmetry from the 0 to 6 range (no trigonometry in tests
/// either, so the values are the same everywhere).
fn circle24(k: usize) -> (f64, f64) {
    let r3 = 3.0_f64.sqrt() / 2.0;
    let table = [
        (1.0, 0.0),
        (0.965_925_826_289_068_3, 0.258_819_045_102_520_74),
        (r3, 0.5),
        (
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ),
        (0.5, r3),
        (0.258_819_045_102_520_74, 0.965_925_826_289_068_3),
    ];
    let quarter = k / 6;
    let (c, s) = table[k % 6];
    match quarter {
        0 => (c, s),
        1 => (-s, c),
        2 => (-c, -s),
        _ => (s, -c),
    }
}

#[test]
fn the_outline_holds_the_stated_share_and_leaves_the_stray_out() {
    let mut members = square3((0.0, 0.0));
    members.push(((10.0, 0.0), 1.0));
    let w = outline_of(&members, 0.9);
    assert_eq!(w.held, (0..9).collect::<Vec<_>>());
    assert_eq!(w.held_weight, 9.0);
    assert_eq!(w.total_weight, 10.0);
    assert!(!contains(paths_of(&w), (10.0, 0.0)));
}

#[test]
fn a_membership_of_zero_is_outside() {
    let mut members = square3((0.0, 0.0));
    members.push(((0.125, 0.0), 0.0));
    members.push(((f64::NAN, f64::NAN), 0.0));
    let w = outline_of(&members, 0.9);
    assert!(!w.held.contains(&9));
    assert!(!w.held.contains(&10));
    assert_eq!(w.total_weight, 9.0);
    assert!(contains(paths_of(&w), (0.125, 0.0)));
    let none = weighted_outline(&[((0.0, 0.0), 0.0), ((f64::NAN, 1.0), 0.0)], &params(0.9));
    assert_eq!(none, Ok(None));
}

#[test]
fn the_heavier_member_has_the_wider_margin() {
    let members = [((0.0, 0.0), 1.0), ((4.0, 0.0), 1.0), ((8.0, 0.0), 0.6)];
    let w = outline_of(&members, 1.0);
    let paths = paths_of(&w);
    assert_eq!(paths.len(), 3);
    let at = |x: f64| {
        let path = paths
            .iter()
            .find(|p| {
                let mean = p.iter().map(|v| v.0).sum::<f64>() / p.len() as f64;
                (mean - x).abs() < 1.0
            })
            .expect("a piece near the member");
        signed_area(path)
    };
    let (first, last) = (at(0.0), at(8.0));
    assert!(first > 1.3 * last, "{first} vs {last}");
    for (area, weight) in [(first, 1.0_f64), (last, 0.6)] {
        let r2 = 1.0 - (0.3 / weight).sqrt();
        let expected = std::f64::consts::PI * r2;
        assert!(
            (area - expected).abs() < 0.08 * expected,
            "{area} vs {expected}"
        );
    }
}

#[test]
fn scaling_every_weight_by_a_power_of_two_changes_nothing() {
    let base: Vec<(Point, f64)> = u_shape()
        .into_iter()
        .enumerate()
        .map(|(k, p)| (p, (k % 3 + 1) as f64))
        .collect();
    let scaled: Vec<(Point, f64)> = base.iter().map(|&(p, w)| (p, w * 1024.0)).collect();
    let a = outline_of(&base, 1.0);
    let b = outline_of(&scaled, 1.0);
    assert_eq!(bits(&a), bits(&b));
    assert_eq!(a.held, b.held);
}

#[test]
fn moving_the_set_by_whole_cells_moves_the_outline_exactly() {
    let base: Vec<(Point, f64)> = u_shape().into_iter().map(|p| (p, 1.0)).collect();
    let (dx, dy) = (3.0 * 0.125, -5.0 * 0.125);
    let moved: Vec<(Point, f64)> = base
        .iter()
        .map(|&(p, w)| ((p.0 + dx, p.1 + dy), w))
        .collect();
    let a = outline_of(&base, 1.0);
    let b = outline_of(&moved, 1.0);
    let (pa, pb) = (paths_of(&a), paths_of(&b));
    assert_eq!(pa.len(), pb.len());
    for (x, y) in pa.iter().zip(pb) {
        assert_eq!(x.len(), y.len());
        for (p, q) in x.iter().zip(y) {
            assert!((p.0 + dx - q.0).abs() < 1e-9 && (p.1 + dy - q.1).abs() < 1e-9);
        }
    }
}

#[test]
fn held_agrees_with_the_even_odd_test() {
    let members = random_set();
    let w = outline_of(&members, 0.9);
    let paths = paths_of(&w);
    for (k, (p, weight)) in members.iter().enumerate() {
        assert!(*weight > 0.0);
        assert_eq!(
            w.held.contains(&k),
            contains(paths, *p),
            "member {k} at {p:?}"
        );
    }
    assert!(w.held_weight >= 0.9 * w.total_weight);
}

#[test]
fn bad_input_is_refused() {
    let ok = [((0.0, 0.0), 1.0)];
    assert_eq!(
        weighted_outline(&[((0.0, 0.0), -1.0)], &params(0.9)),
        Err(GeometryError::BadWeight {
            item: "#0".into(),
            value: -1.0
        })
    );
    assert!(matches!(
        weighted_outline(&[((0.0, 0.0), 1.0), ((1.0, 1.0), f64::NAN)], &params(0.9)),
        Err(GeometryError::BadWeight { item, .. }) if item == "#1"
    ));
    assert_eq!(
        weighted_outline(&[((f64::NAN, 0.0), 1.0)], &params(0.9)),
        Err(GeometryError::NonFinitePosition { item: "#0".into() })
    );
    for share in [0.0, 1.5] {
        assert!(matches!(
            weighted_outline(&ok, &params(share)),
            Err(GeometryError::BadParameter { name: "share", .. })
        ));
    }
    assert!(matches!(
        weighted_outline(&ok, &OutlineParams::new(1.0, 0.6, 0.9)),
        Err(GeometryError::BadParameter { name: "cell", .. })
    ));
    let mut p = params(0.9);
    p.level_fraction = 0.0;
    assert!(matches!(
        weighted_outline(&ok, &p),
        Err(GeometryError::BadParameter {
            name: "level_fraction",
            ..
        })
    ));
    let mut p = params(0.9);
    p.max_grid_nodes = 1000;
    assert!(matches!(
        weighted_outline(&[((0.0, 0.0), 1.0), ((1e6, 0.0), 1.0)], &p),
        Err(GeometryError::GridTooLarge { limit: 1000, .. })
    ));
    assert!(matches!(
        weighted_outline(&[((-1e300, 0.0), 1.0), ((1e300, 0.0), 1.0)], &params(0.9)),
        Err(GeometryError::GridTooLarge { .. })
    ));
}

#[test]
fn twenty_five_thousand_members_outline_in_one_call() {
    let members: Vec<(Point, f64)> = (0..25_000)
        .map(|k| (((k % 158) as f64 * 0.2, (k / 158) as f64 * 0.2), 1.0))
        .collect();
    let w = weighted_outline(&members, &OutlineParams::new(1.0, 0.25, 0.9))
        .unwrap()
        .unwrap();
    assert!(w.held_weight >= 0.9 * w.total_weight);
}

#[test]
fn the_same_input_gives_the_same_bits_twice() {
    let members = random_set();
    let a = outline_of(&members, 0.9);
    let b = outline_of(&members, 0.9);
    assert_eq!(bits(&a), bits(&b));
    assert_eq!(a, b);
}

type H = Hypergraph<(), (), f64>;

fn link(h: &mut H, edge: &str, node: &str, w: f64) {
    h.add_node_to_edge_with_data(edge, node, w).unwrap();
}

fn at(list: &[(&str, Point)]) -> Positions {
    list.iter().map(|(id, p)| ((*id).to_string(), *p)).collect()
}

#[test]
fn an_edge_outline_reads_the_numbers_on_its_memberships() {
    let mut h = H::new();
    link(&mut h, "club", "a", 2.0);
    link(&mut h, "club", "b", 1.0);
    link(&mut h, "club", "z", 0.0);
    let positions = at(&[("a", (0.0, 0.0)), ("b", (5.0, 0.0))]);
    let out = edge_outline(
        &h,
        "club",
        &positions,
        |w| *w,
        &OutlineParams::new(1.0, 0.125, 0.9),
    )
    .expect("valid")
    .expect("some member above zero");
    assert_eq!(out.edge, "club");
    assert_eq!(out.members, ["a", "b"]);
    assert_eq!(
        out.outline.outline,
        Outline::Capsule {
            a: (0.0, 0.0),
            radius_a: 1.0,
            b: (5.0, 0.0),
            radius_b: 0.75
        }
    );
}

#[test]
fn a_weighted_member_without_a_position_is_refused() {
    let mut h = H::new();
    link(&mut h, "club", "a", 1.0);
    link(&mut h, "club", "b", 1.0);
    let positions = at(&[("a", (0.0, 0.0))]);
    let err = edge_outline(
        &h,
        "club",
        &positions,
        |w| *w,
        &OutlineParams::new(1.0, 0.125, 0.9),
    )
    .unwrap_err();
    assert_eq!(
        err,
        GeometryError::NoPosition {
            edge: "club".into(),
            node: "b".into()
        }
    );
}

#[test]
fn an_unknown_edge_is_refused() {
    let h = H::new();
    let err = edge_outline(
        &h,
        "nope",
        &Positions::new(),
        |w| *w,
        &OutlineParams::new(1.0, 0.125, 0.9),
    )
    .unwrap_err();
    assert_eq!(err, GeometryError::UnknownEdge("nope".into()));
}

#[test]
fn an_edge_of_zeros_has_no_outline() {
    let mut h = H::new();
    link(&mut h, "club", "a", 0.0);
    link(&mut h, "club", "b", 0.0);
    let r = edge_outline(
        &h,
        "club",
        &Positions::new(),
        |w| *w,
        &OutlineParams::new(1.0, 0.125, 0.9),
    );
    assert_eq!(r, Ok(None));
}

#[test]
fn an_edge_outline_names_the_node_of_a_bad_number() {
    let mut h = H::new();
    link(&mut h, "club", "a", -1.0);
    let positions = at(&[("a", (0.0, 0.0))]);
    let p = OutlineParams::new(1.0, 0.125, 0.9);
    assert!(matches!(
        edge_outline(&h, "club", &positions, |w| *w, &p),
        Err(GeometryError::BadWeight { item, .. }) if item == "a"
    ));
    let mut h = H::new();
    link(&mut h, "club", "a", 1.0);
    let positions = at(&[("a", (f64::NAN, 0.0))]);
    assert_eq!(
        edge_outline(&h, "club", &positions, |w| *w, &p),
        Err(GeometryError::NonFinitePosition { item: "a".into() })
    );
}

#[test]
fn a_zero_member_at_nan_is_not_an_error_and_held_indexes_members() {
    let mut h = H::new();
    link(&mut h, "club", "z", 0.0);
    link(&mut h, "club", "a", 1.0);
    let positions = at(&[("z", (f64::NAN, f64::NAN)), ("a", (1.0, 1.0))]);
    let out = edge_outline(
        &h,
        "club",
        &positions,
        |w| *w,
        &OutlineParams::new(1.0, 0.125, 0.9),
    )
    .unwrap()
    .unwrap();
    assert_eq!(out.members, ["a"]);
    assert_eq!(out.outline.held, [0]);
}

#[test]
fn weights_whose_sum_overflows_are_refused_naming_the_member() {
    let members = [((0.0, 0.0), 1e308), ((1.0, 0.0), 1e308)];
    match weighted_outline(&members, &params(0.9)) {
        Err(GeometryError::BadWeight { item, value }) => {
            assert_eq!(item, "#1");
            assert!(value.is_infinite(), "{value}");
        }
        other => panic!("expected BadWeight, got {other:?}"),
    }
}

#[test]
fn memberships_whose_numbers_overflow_are_refused_naming_the_node() {
    let mut h = H::new();
    link(&mut h, "club", "a", 1e308);
    link(&mut h, "club", "b", 1e308);
    let positions = at(&[("a", (0.0, 0.0)), ("b", (1.0, 0.0))]);
    match edge_outline(&h, "club", &positions, |w| *w, &params(0.9)) {
        Err(GeometryError::BadWeight { item, value }) => {
            assert_eq!(item, "b");
            assert!(value.is_infinite(), "{value}");
        }
        other => panic!("expected BadWeight, got {other:?}"),
    }
}

#[test]
fn held_agrees_with_the_even_odd_test_over_weights_from_zero_up() {
    let mut state = 99_u64;
    let members: Vec<(Point, f64)> = (0..300)
        .map(|k| {
            let p = (lcg(&mut state) * 10.0, lcg(&mut state) * 10.0);
            (p, f64::from(k) / 5.0)
        })
        .collect();
    let w = outline_of(&members, 0.9);
    let paths = paths_of(&w);
    for (k, (p, weight)) in members.iter().enumerate() {
        if *weight > 0.0 {
            assert_eq!(
                w.held.contains(&k),
                contains(paths, *p),
                "member {k} at {p:?}"
            );
        }
    }
    assert!(!w.held.contains(&0));
    assert!(
        w.held_weight >= 0.9 * w.total_weight,
        "{} < 0.9 * {}",
        w.held_weight,
        w.total_weight
    );
}

#[test]
fn a_number_of_negative_zero_counts_as_zero() {
    let members = [((f64::NAN, f64::INFINITY), -0.0), ((1.0, 1.0), 1.0)];
    let w = outline_of(&members, 0.9);
    assert_eq!(w.held, [1]);
    assert_eq!(w.total_weight, 1.0);

    let mut h = H::new();
    link(&mut h, "club", "z", -0.0);
    link(&mut h, "club", "a", 1.0);
    let positions = at(&[("a", (1.0, 1.0))]);
    let out = edge_outline(&h, "club", &positions, |w| *w, &params(0.9))
        .unwrap()
        .unwrap();
    assert_eq!(out.members, ["a"]);
    assert_eq!(out.outline.held, [0]);
}
