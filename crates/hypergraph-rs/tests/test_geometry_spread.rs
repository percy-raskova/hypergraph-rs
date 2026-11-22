#![cfg(feature = "geometry")]

use hypergraph_rs::geometry::{spread, GeometryError, Point, Positions, SpreadParams};

fn dist(a: Point, b: Point) -> f64 {
    ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt()
}

fn bits(p: Point) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

#[test]
fn a_close_pair_ends_one_distance_apart() {
    let mut p = Positions::new();
    p.insert("a", (0.0, 0.0));
    p.insert("b", (0.2, 0.0));
    let report = spread(&mut p, &[], &SpreadParams::new(1.0, 1)).unwrap();
    let (a, b) = (p.get("a").unwrap(), p.get("b").unwrap());
    assert!((dist(a, b) - 1.0).abs() < 1e-12);
    assert!(((a.0 + b.0) / 2.0 - 0.1).abs() < 1e-12);
    assert!(((a.1 + b.1) / 2.0).abs() < 1e-12);
    assert_eq!(report.overlaps_left, 0);
}

#[test]
fn points_on_one_spot_part_by_their_ids_not_their_order() {
    let run = |order: [&str; 2]| {
        let mut p = Positions::new();
        for id in order {
            p.insert(id, (0.0, 0.0));
        }
        spread(&mut p, &[], &SpreadParams::new(1.0, 4)).unwrap();
        p
    };
    let first = run(["x", "y"]);
    let second = run(["y", "x"]);
    for id in ["x", "y"] {
        assert_eq!(bits(first.get(id).unwrap()), bits(second.get(id).unwrap()));
    }
    assert!(dist(first.get("x").unwrap(), first.get("y").unwrap()) >= 1.0 - 1e-9);
}

#[test]
fn a_pile_of_400_on_one_spot_spreads_without_comparing_every_pair() {
    let mut p = Positions::new();
    for k in 0..400 {
        p.insert(format!("n{k:03}"), (5.0, 5.0));
    }
    let report = spread(&mut p, &[], &SpreadParams::new(0.1, 60)).unwrap();
    assert!(report.comparisons <= 400 * 32 * 60);
    let mut all: Vec<(u64, u64)> = p.iter().map(|(_, at)| bits(at)).collect();
    all.sort_unstable();
    for pair in all.windows(2) {
        assert_ne!(pair[0], pair[1], "two points share a position");
    }
    for (_, at) in p.iter() {
        assert!(dist(at, (5.0, 5.0)) <= 60.0 * 0.05 + 1e-9);
    }
}

#[test]
fn a_far_node_changes_no_other_nodes_bits() {
    let ids: Vec<String> = (0..50).map(|k| format!("p{k:02}")).collect();
    let mut plain = Positions::new();
    for (k, id) in ids.iter().enumerate() {
        plain.insert(id.clone(), (k as f64 * 0.3, (k % 3) as f64 * 0.1));
    }
    let mut with_far = Positions::new();
    with_far.insert("far", (1000.0, 1000.0));
    for (k, id) in ids.iter().enumerate() {
        with_far.insert(id.clone(), (k as f64 * 0.3, (k % 3) as f64 * 0.1));
    }
    spread(&mut plain, &[], &SpreadParams::new(1.0, 10)).unwrap();
    spread(&mut with_far, &[], &SpreadParams::new(1.0, 10)).unwrap();
    for id in &ids {
        assert_eq!(
            bits(plain.get(id).unwrap()),
            bits(with_far.get(id).unwrap()),
            "{id}"
        );
    }
}

#[test]
fn obstacles_push_points_off_and_never_move() {
    let mut p = Positions::new();
    p.insert("a", (2.0, 2.0));
    spread(&mut p, &[(2.0, 2.0)], &SpreadParams::new(1.0, 1)).unwrap();
    assert!((dist(p.get("a").unwrap(), (2.0, 2.0)) - 1.0).abs() < 1e-12);
}

#[test]
fn bounds_too_narrow_are_reported_not_looped() {
    let mut p = Positions::new();
    for k in 0..10 {
        p.insert(format!("n{k}"), (0.0, 0.0));
    }
    let mut params = SpreadParams::new(1.0, 20);
    params.bounds = Some((0.0, 0.0, 0.5, 0.5));
    let report = spread(&mut p, &[], &params).unwrap();
    for (_, at) in p.iter() {
        assert!((0.0..=0.5).contains(&at.0) && (0.0..=0.5).contains(&at.1));
    }
    assert!(report.overlaps_left > 0);
}

#[test]
fn zero_steps_and_empty_input_change_nothing() {
    let mut p = Positions::new();
    p.insert("a", (0.0, 0.0));
    p.insert("b", (0.1, 0.0));
    let before = p.clone();
    spread(&mut p, &[], &SpreadParams::new(1.0, 0)).unwrap();
    assert_eq!(p, before);
    let mut empty = Positions::new();
    spread(&mut empty, &[], &SpreadParams::new(1.0, 5)).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn bad_parameters_are_refused() {
    let mut p = Positions::new();
    p.insert("a", (0.0, 0.0));
    for bad in [0.0, f64::NAN] {
        assert!(matches!(
            spread(&mut p, &[], &SpreadParams::new(bad, 1)),
            Err(GeometryError::BadParameter { .. })
        ));
    }
    let mut params = SpreadParams::new(1.0, 1);
    params.max_neighbours = 0;
    assert!(matches!(
        spread(&mut p, &[], &params),
        Err(GeometryError::BadParameter { .. })
    ));
    let mut params = SpreadParams::new(1.0, 1);
    params.bounds = Some((1.0, 0.0, 0.0, 1.0));
    assert!(matches!(
        spread(&mut p, &[], &params),
        Err(GeometryError::BadParameter { name: "bounds", .. })
    ));
    let mut q = Positions::new();
    q.insert("nan", (f64::NAN, 0.0));
    assert!(matches!(
        spread(&mut q, &[], &SpreadParams::new(1.0, 1)),
        Err(GeometryError::NonFinitePosition { item }) if item == "nan"
    ));
    assert!(matches!(
        spread(&mut p, &[(f64::NAN, 0.0)], &SpreadParams::new(1.0, 1)),
        Err(GeometryError::NonFinitePosition { item }) if item == "#0"
    ));
}

#[test]
fn twenty_five_thousand_points_stay_within_the_cost_bound() {
    let mut p = Positions::new();
    for k in 0..25_000usize {
        p.insert(
            format!("n{k:05}"),
            ((k % 158) as f64 * 0.05, (k / 158) as f64 * 0.05),
        );
    }
    let report = spread(&mut p, &[], &SpreadParams::new(0.1, 10)).unwrap();
    assert!(report.comparisons <= 25_000 * 32 * 10);
}

#[test]
fn the_same_input_gives_the_same_bits_twice() {
    let make = || {
        let mut state: u64 = 12345;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut p = Positions::new();
        for k in 0..500 {
            let x = next() * 10.0;
            let y = next() * 10.0;
            p.insert(format!("n{k}"), (x, y));
        }
        p
    };
    let mut first = make();
    let mut second = make();
    let r1 = spread(&mut first, &[], &SpreadParams::new(1.0, 12)).unwrap();
    let r2 = spread(&mut second, &[], &SpreadParams::new(1.0, 12)).unwrap();
    assert_eq!(r1, r2);
    let a: Vec<_> = first
        .iter()
        .map(|(i, at)| (i.to_owned(), bits(at)))
        .collect();
    let b: Vec<_> = second
        .iter()
        .map(|(i, at)| (i.to_owned(), bits(at)))
        .collect();
    assert_eq!(a, b);
}
