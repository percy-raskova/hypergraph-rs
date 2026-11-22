#![cfg(feature = "geometry")]

use hypergraph_rs::geometry::{
    anchored_placement, barycentre, corner_mix, GeometryError, Placement, Positions,
};
use hypergraph_rs::Hypergraph;

#[test]
fn barycentre_weighs_each_anchor() {
    assert_eq!(
        barycentre(&[(0.0, 0.0), (4.0, 0.0)], &[3.0, 1.0]).unwrap(),
        Some((1.0, 0.0))
    );
}

#[test]
fn barycentre_of_nothing_is_none() {
    assert_eq!(barycentre(&[(1.0, 1.0)], &[0.0]).unwrap(), None);
    assert_eq!(barycentre(&[], &[]).unwrap(), None);
}

#[test]
fn barycentre_refuses_bad_numbers() {
    assert!(matches!(
        barycentre(&[(0.0, 0.0)], &[-1.0]),
        Err(GeometryError::BadWeight { .. })
    ));
    assert!(matches!(
        barycentre(&[(0.0, 0.0)], &[f64::NAN]),
        Err(GeometryError::BadWeight { .. })
    ));
    assert!(matches!(
        barycentre(&[(f64::INFINITY, 0.0)], &[1.0]),
        Err(GeometryError::NonFinitePosition { .. })
    ));
    assert!(matches!(
        barycentre(&[(0.0, 0.0)], &[1.0, 1.0]),
        Err(GeometryError::CountMismatch { .. })
    ));
}

fn corners() -> [(f64, f64); 4] {
    [(0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)]
}

fn bits(p: (f64, f64)) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

#[test]
fn a_whole_share_stands_exactly_on_its_corner() {
    let shares = vec![
        ("a".to_string(), vec![1.0, 0.0, 0.0, 0.0]),
        ("b".to_string(), vec![0.0, 0.0, 0.5, 0.5]),
        ("c".to_string(), vec![2.0, 2.0, 0.0, 0.0]),
        ("d".to_string(), vec![0.0, 0.0, 0.0, 0.0]),
    ];
    let placement = corner_mix(&corners(), &shares).unwrap();
    assert_eq!(
        bits(placement.positions.get("a").unwrap()),
        bits((0.0, 1.0))
    );
    assert_eq!(
        bits(placement.positions.get("b").unwrap()),
        bits((0.5, 0.0))
    );
    assert_eq!(
        bits(placement.positions.get("c").unwrap()),
        bits((0.5, 1.0))
    );
    let order: Vec<&str> = placement.positions.iter().map(|(id, _)| id).collect();
    assert_eq!(order, ["a", "b", "c"]);
    assert_eq!(placement.unplaced, ["d"]);
}

#[test]
fn corner_mix_refuses_a_row_of_the_wrong_length() {
    let shares = vec![("x".to_string(), vec![1.0, 0.0, 0.0])];
    assert_eq!(
        corner_mix(&corners(), &shares),
        Err(GeometryError::CountMismatch {
            item: "x".to_string(),
            expected: 4,
            found: 3
        })
    );
}

#[test]
fn corner_mix_refuses_corners_that_are_not_finite() {
    let c = [(0.0, 0.0), (f64::NAN, 0.0)];
    let shares = vec![("x".to_string(), vec![1.0, 0.0])];
    assert_eq!(
        corner_mix(&c, &shares),
        Err(GeometryError::NonFinitePosition {
            item: "#1".to_string()
        })
    );
}

#[test]
fn positions_keep_insertion_order_and_first_place() {
    let mut p = Positions::new();
    assert!(p.is_empty());
    assert_eq!(p.insert("b", (1.0, 2.0)), None);
    assert_eq!(p.insert("a", (3.0, 4.0)), None);
    assert_eq!(p.insert("b", (5.0, 6.0)), Some((1.0, 2.0)));
    let listing: Vec<(&str, (f64, f64))> = p.iter().collect();
    assert_eq!(listing, [("b", (5.0, 6.0)), ("a", (3.0, 4.0))]);
    assert_eq!(p.get("b"), Some((5.0, 6.0)));
    assert_eq!(p.get("zz"), None);
    assert_eq!(p.len(), 2);
}

type H = Hypergraph<(), (), f64>;

fn link(h: &mut H, edge: &str, node: &str, w: f64) {
    h.add_node_to_edge_with_data(edge, node, w).unwrap();
}

fn anchors(list: &[(&str, (f64, f64))]) -> Positions {
    list.iter()
        .map(|(id, at)| ((*id).to_string(), *at))
        .collect()
}

#[test]
fn a_node_stands_nearer_its_heavier_anchor() {
    let mut h = H::new();
    link(&mut h, "top", "a", 1.0);
    link(&mut h, "bottom", "a", 3.0);
    link(&mut h, "bottom", "c", 0.0);
    link(&mut h, "loose", "b", 5.0);
    let anc = anchors(&[("top", (0.0, 10.0)), ("bottom", (0.0, 0.0))]);
    let p = anchored_placement(&h, &anc, |w| *w).unwrap();
    let listed: Vec<_> = p.positions.iter().collect();
    assert_eq!(listed, vec![("a", (0.0, 2.5))]);
    assert_eq!(p.unplaced, vec!["c".to_string(), "b".to_string()]);
}

#[test]
fn anchored_placement_follows_node_order() {
    let mut h = H::new();
    for n in ["z", "y", "x"] {
        link(&mut h, "top", n, 1.0);
    }
    let anc = anchors(&[("top", (1.0, 1.0))]);
    let p = anchored_placement(&h, &anc, |w| *w).unwrap();
    let ids: Vec<_> = p.positions.iter().map(|(id, _)| id).collect();
    assert_eq!(ids, vec!["z", "y", "x"]);
}

#[test]
fn an_unknown_anchor_is_refused() {
    let mut h = H::new();
    link(&mut h, "top", "a", 1.0);
    let anc = anchors(&[("nope", (0.0, 0.0))]);
    assert_eq!(
        anchored_placement(&h, &anc, |w| *w),
        Err(GeometryError::UnknownEdge("nope".to_string()))
    );
    let anc = anchors(&[("top", (f64::NAN, 0.0))]);
    assert_eq!(
        anchored_placement(&h, &anc, |w| *w),
        Err(GeometryError::NonFinitePosition {
            item: "top".to_string()
        })
    );
}

#[test]
fn a_bad_membership_number_names_the_node() {
    let mut h = H::new();
    link(&mut h, "top", "a", -2.0);
    let anc = anchors(&[("top", (0.0, 0.0))]);
    assert_eq!(
        anchored_placement(&h, &anc, |w| *w),
        Err(GeometryError::BadWeight {
            item: "a".to_string(),
            value: -2.0
        })
    );
}

#[test]
fn an_empty_hypergraph_places_nothing() {
    let h = H::new();
    let p = anchored_placement(&h, &Positions::new(), |w| *w).unwrap();
    assert_eq!(p, Placement::default());
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn build_big() -> (H, Positions) {
    let mut s = 20_261_121_u64;
    let mut h = H::new();
    let mut anc = Positions::new();
    for e in 0..12 {
        anc.insert(format!("e{e}"), (e as f64 * 1.5, (e * e) as f64 / 3.0));
    }
    for n in 0..200 {
        let k = 1 + lcg(&mut s) % 4;
        for _ in 0..k {
            let e = lcg(&mut s) % 12;
            let w = (lcg(&mut s) % 20) as f64 / 7.0;
            link(&mut h, &format!("e{e}"), &format!("n{n}"), w);
        }
    }
    (h, anc)
}

#[test]
fn the_same_input_gives_the_same_bits_twice() {
    let (h1, a1) = build_big();
    let (h2, a2) = build_big();
    let p1 = anchored_placement(&h1, &a1, |w| *w).unwrap();
    let p2 = anchored_placement(&h2, &a2, |w| *w).unwrap();
    assert!(!p1.positions.is_empty());
    assert_eq!(p1.unplaced, p2.unplaced);
    let l1: Vec<_> = p1.positions.iter().collect();
    let l2: Vec<_> = p2.positions.iter().collect();
    assert_eq!(l1.len(), l2.len());
    for ((i1, a), (i2, b)) in l1.iter().zip(&l2) {
        assert_eq!(i1, i2);
        assert_eq!(a.0.to_bits(), b.0.to_bits());
        assert_eq!(a.1.to_bits(), b.1.to_bits());
    }
}
