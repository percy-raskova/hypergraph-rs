use std::time::Duration;

use hypergraph_rs_bevy::{FrameWindows, WindowStats};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn a_window_reports_its_median_and_its_95th_percentile() {
    let mut w = FrameWindows::new(300, Duration::ZERO);
    for n in 1..=299 {
        assert_eq!(w.push(ms(n)), None, "push {n}");
    }
    assert_eq!(
        w.push(ms(300)),
        Some(WindowStats {
            index: 0,
            median: Duration::from_micros(150_500),
            p95: ms(285),
        })
    );

    let mut small = FrameWindows::new(5, Duration::ZERO);
    let out: Vec<_> = [5, 1, 4, 2, 3]
        .into_iter()
        .filter_map(|n| small.push(ms(n)))
        .collect();
    assert_eq!(
        out,
        vec![WindowStats {
            index: 0,
            median: ms(3),
            p95: ms(5),
        }]
    );
}

#[test]
fn the_settle_time_is_skipped() {
    let mut w = FrameWindows::new(4, Duration::from_secs(2));
    for n in 0..200 {
        assert_eq!(w.push(ms(10)), None, "settle push {n}");
    }
    let got: Vec<_> = (0..4).filter_map(|_| w.push(ms(10))).collect();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].index, 0);
    assert_eq!(got[0].median, ms(10));
}

#[test]
fn the_slowest_window_is_kept() {
    let mut w = FrameWindows::new(2, Duration::ZERO);
    assert_eq!(w.slowest_p95(), None);
    w.push(ms(10));
    w.push(ms(20));
    w.push(ms(12));
    w.push(ms(5));
    assert_eq!(w.slowest_p95(), Some(ms(20)));
}
