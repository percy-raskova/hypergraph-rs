#![cfg(feature = "geometry")]

use hypergraph_rs::geometry::{barycentre, corner_mix, GeometryError, Positions};

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
