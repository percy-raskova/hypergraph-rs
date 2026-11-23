use std::num::NonZeroU64;

use bevy::prelude::*;
use hypergraph_rs_bevy::{select_flows, FieldError, Flow, Glyph, StreamStyle};

fn stream(unit: u64, share: f64, max_marks: u32) -> StreamStyle {
    StreamStyle {
        beat: 0,
        unit: NonZeroU64::new(unit).unwrap(),
        full: Glyph::Disc,
        hollow: Glyph::Ring,
        size: 1.0,
        swatch: 0,
        share_drawn: share,
        max_marks,
    }
}

fn flow(stream: u16, amount: u64, hollow: bool) -> Flow {
    Flow {
        stream,
        from: Vec3::ZERO,
        to: Vec3::X,
        amount,
        hollow,
    }
}

fn plain(amounts: &[u64]) -> Vec<Flow> {
    amounts.iter().map(|&a| flow(0, a, false)).collect()
}

#[test]
fn paths_are_drawn_largest_first_until_the_share_is_shown() {
    let (drawn, report) =
        select_flows(&plain(&[10, 500, 40, 300, 150]), &[stream(10, 0.95, 1000)]).unwrap();
    let got: Vec<(usize, u32)> = drawn.iter().map(|d| (d.flow, d.marks)).collect();
    assert_eq!(got, vec![(1, 50), (3, 30), (4, 15)]);
    let s = report.streams[0];
    assert_eq!(
        (
            s.total,
            s.drawn,
            s.not_drawn,
            s.marks,
            s.paths,
            s.paths_drawn
        ),
        (1000, 950, 50, 95, 5, 3)
    );
}

#[test]
fn less_than_one_mark_is_counted_not_drawn() {
    let (drawn, report) = select_flows(&plain(&[25, 7]), &[stream(10, 1.0, 1000)]).unwrap();
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].marks, 2);
    let s = report.streams[0];
    assert_eq!((s.total, s.drawn, s.not_drawn), (32, 20, 12));
}

#[test]
fn the_mark_cap_stops_drawing_and_counts_the_rest() {
    let (drawn, report) = select_flows(&plain(&[10, 10]), &[stream(1, 1.0, 5)]).unwrap();
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].marks, 5);
    let s = report.streams[0];
    assert_eq!((s.drawn, s.not_drawn), (5, 15));
}

#[test]
fn streams_are_reported_apart() {
    let flows = vec![flow(0, 7, false), flow(1, 11_000, false)];
    let (_, report) =
        select_flows(&flows, &[stream(1, 1.0, 1000), stream(1000, 1.0, 1000)]).unwrap();
    assert_eq!(report.streams.len(), 4);
    let totals: Vec<u128> = report.streams.iter().map(|s| s.total).collect();
    assert_eq!(totals, vec![7, 0, 11_000, 0]);
    let keys: Vec<(u16, bool)> = report
        .streams
        .iter()
        .map(|s| (s.stream, s.hollow))
        .collect();
    assert_eq!(keys, vec![(0, false), (0, true), (1, false), (1, true)]);
}

#[test]
fn hollow_flows_are_their_own_count() {
    let flows = vec![flow(0, 30, false), flow(0, 20, true)];
    let (drawn, report) = select_flows(&flows, &[stream(10, 1.0, 1000)]).unwrap();
    assert_eq!((report.streams[0].total, report.streams[0].marks), (30, 3));
    assert_eq!((report.streams[1].total, report.streams[1].marks), (20, 2));
    assert!(report.streams[1].hollow);
    let hollow: Vec<&_> = drawn.iter().filter(|d| d.hollow).collect();
    assert_eq!(hollow.len(), 1);
    assert_eq!(hollow[0].flow, 1);
    assert_eq!(hollow[0].marks, 2);
}

#[test]
fn huge_amounts_do_not_overflow() {
    let (_, report) =
        select_flows(&plain(&[u64::MAX, u64::MAX]), &[stream(1 << 40, 1.0, 1000)]).unwrap();
    let s = report.streams[0];
    assert_eq!(s.total, 2 * (u64::MAX as u128));
    assert_eq!(s.drawn + s.not_drawn, s.total);
}

#[test]
fn a_flow_into_its_own_place_is_counted_not_drawn() {
    let at = Vec3::new(3.0, 1.0, 2.0);
    let flows = vec![Flow {
        stream: 0,
        from: at,
        to: at,
        amount: 100,
        hollow: false,
    }];
    let (drawn, report) = select_flows(&flows, &[stream(10, 1.0, 1000)]).unwrap();
    assert!(drawn.is_empty());
    let s = report.streams[0];
    assert_eq!((s.total, s.drawn, s.marks, s.paths_drawn), (100, 0, 0, 0));
}

#[test]
fn drawn_plus_not_drawn_is_the_total_for_any_input() {
    let mut state: u64 = 20261121;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state >> 33
    };
    let flows: Vec<Flow> = (0..2000)
        .map(|_| {
            let amount = next() % 5000;
            let s = (next() % 2) as u16;
            let hollow = next() % 2 == 0;
            Flow {
                stream: s,
                from: Vec3::new((next() % 5) as f32, 0.0, 0.0),
                to: Vec3::new((next() % 5) as f32, 0.0, 1.0),
                amount,
                hollow,
            }
        })
        .collect();
    let streams = [stream(7, 0.8, 300), stream(13, 1.0, 100_000)];
    let (_, report) = select_flows(&flows, &streams).unwrap();
    assert_eq!(report.streams.len(), 4);
    for r in &report.streams {
        let unit = u128::from(streams[r.stream as usize].unit.get());
        assert_eq!(r.drawn + r.not_drawn, r.total);
        assert_eq!(r.drawn, u128::from(r.marks) * unit);
    }
}

#[test]
fn bad_flows_are_refused() {
    assert_eq!(
        select_flows(&[flow(3, 1, false)], &[stream(1, 1.0, 10)]).unwrap_err(),
        FieldError::UnknownStyle {
            what: "stream",
            index: 3,
            len: 1
        }
    );
    let mut bad = flow(0, 1, false);
    bad.from = Vec3::new(f32::NAN, 0.0, 0.0);
    assert_eq!(
        select_flows(&[bad], &[stream(1, 1.0, 10)]).unwrap_err(),
        FieldError::NonFinite {
            item: "flow #0".to_string()
        }
    );
}
