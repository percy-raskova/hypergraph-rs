use std::time::Duration;

use bevy::prelude::*;
use hypergraph_rs_bevy::{eased, mark_position, mark_start, smoothstep, BeatClock, Motion, Phase};

#[test]
fn smoothstep_holds_its_ends() {
    assert_eq!(smoothstep(-1.0), 0.0);
    assert_eq!(smoothstep(0.0), 0.0);
    assert_eq!(smoothstep(0.5), 0.5);
    assert_eq!(smoothstep(1.0), 1.0);
    assert_eq!(smoothstep(2.0), 1.0);
}

#[test]
fn a_mark_starts_at_from_and_ends_at_to() {
    let (from, to) = (Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
    assert_eq!(mark_position(from, to, 0.0, 0.08, 1.0), from);
    assert_eq!(mark_position(from, to, 1.0, 0.08, 1.0), to);
    let mid = mark_position(from, to, 0.5, 0.08, 1.0);
    assert!((mid - Vec3::new(5.0, 0.0, 0.8)).abs().max_element() < 1e-5);
    let other = mark_position(from, to, 0.5, 0.08, -1.0);
    assert!((other - Vec3::new(5.0, 0.0, -0.8)).abs().max_element() < 1e-5);
}

#[test]
fn marks_of_one_flow_spread_through_their_beat() {
    let beat = Duration::from_millis(1200);
    let starts: Vec<Duration> = (0..4).map(|k| mark_start(7, k, 4, beat, 0.4)).collect();
    assert!(starts.windows(2).all(|w| w[0] < w[1]));
    assert!(starts.iter().all(|s| *s <= Duration::from_millis(720)));
    assert_ne!(
        mark_start(0, 0, 1, beat, 0.4),
        mark_start(1, 0, 1, beat, 0.4)
    );
}

#[test]
fn the_clock_names_the_beat_and_rests_after() {
    let beats = [Duration::from_secs(1); 3];
    let mut clock = BeatClock::default();
    assert_eq!(clock.phase(Duration::from_secs(10), &beats), Phase::Resting);
    assert_eq!(clock.period(), None);
    clock.start(Duration::from_secs(10), 4);
    assert_eq!(clock.period(), Some(4));
    assert_eq!(
        clock.phase(Duration::from_millis(10_500), &beats),
        Phase::Beat {
            index: 0,
            fraction: 0.5
        }
    );
    assert_eq!(
        clock.phase(Duration::from_millis(12_250), &beats),
        Phase::Beat {
            index: 2,
            fraction: 0.25
        }
    );
    assert_eq!(clock.phase(Duration::from_secs(13), &beats), Phase::Resting);
}

#[test]
fn values_ease_only_in_their_beat() {
    let b = |index, fraction| Phase::Beat { index, fraction };
    assert_eq!(eased(2.0, 4.0, b(0, 0.9), 2, Motion::Full), 2.0);
    assert_eq!(eased(2.0, 4.0, b(2, 0.5), 2, Motion::Full), 3.0);
    assert_eq!(eased(2.0, 4.0, b(2, 0.5), 2, Motion::Reduced), 2.0);
    assert_eq!(eased(2.0, 4.0, b(3, 0.0), 2, Motion::Reduced), 4.0);
    assert_eq!(eased(2.0, 4.0, Phase::Resting, 2, Motion::Reduced), 4.0);
}
