mod common;

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use common::{advance_to, field_app};
use hypergraph_rs_bevy::{FlowReport, Ground, GroundForm, ShowPeriod};

#[path = "../examples/field/world.rs"]
mod world;

use world::Size;

#[test]
fn the_large_world_is_the_size_the_design_holds() {
    assert_eq!(world::regions(Size::Large).len(), 3168);
    let f = world::frame(Size::Large, 1);
    assert_eq!(f.particles.len(), 25_000);
    assert_eq!(f.sets.len(), 6);
    assert!(
        f.sets.iter().all(|s| s.outline.is_some()),
        "a set draws nothing"
    );
    let styles = world::styles(Size::Large).1;
    assert!(styles.streams.iter().all(|s| s.max_marks == 30_000));
    assert_eq!(f.validate(&styles), Ok(()));
}

#[test]
fn the_small_world_is_one_a_person_can_read() {
    assert_eq!(world::regions(Size::Small).len(), 1728);
    let f = world::frame(Size::Small, 1);
    assert_eq!(f.particles.len(), 240);
    assert_eq!(f.sets.len(), 5);
    assert!(
        f.sets.iter().all(|s| s.outline.is_some()),
        "a set draws nothing"
    );
    assert_eq!(f.flows.len(), 3 * 48);
    assert_eq!(f.validate(&world::styles(Size::Small).1), Ok(()));
}

/// The ground is soft hills, never a bed of spikes: two regions that share an
/// edge stand within half a unit of each other, in every period tried. The
/// edge of the block with no reading is left out: the ground steps down to it.
#[test]
fn neighbouring_regions_stand_close_in_height() {
    for size in [Size::Small, Size::Large] {
        let columns = size.shape().columns;
        for period in 1..=12 {
            let heights: Vec<Option<f32>> = world::frame(size, period)
                .regions
                .iter()
                .map(|r| r.height)
                .collect();
            let mut steepest = 0.0_f32;
            for (i, here) in heights.iter().enumerate() {
                let beside = ((i + 1) % columns != 0)
                    .then(|| heights.get(i + 1))
                    .flatten();
                for other in [beside, heights.get(i + columns)].into_iter().flatten() {
                    if let (Some(a), Some(b)) = (here, other) {
                        steepest = steepest.max((a - b).abs());
                    }
                }
            }
            assert!(steepest < 0.5, "{size:?} period {period}: {steepest}");
            assert!(
                steepest > 0.01,
                "{size:?} period {period}: the ground is flat"
            );
        }
    }
}

/// Particles and the ends of flows sit on the ground of their period: inside
/// the grid, and between the lowest and the highest of the four regions whose
/// centres stand round them, with their own lift (0.3 at most) on top. They
/// are not all at one height, which is how the first form of this world
/// buried them.
#[test]
fn everything_sits_on_the_grid_and_on_the_ground_beside_it() {
    for size in [Size::Small, Size::Large] {
        let shape = size.shape();
        let (columns, rows) = (shape.columns, shape.rows);
        let f = world::frame(size, 3);
        let heights: Vec<f32> = f.regions.iter().map(|r| r.height.unwrap_or(0.0)).collect();
        let ends = f.flows.iter().flat_map(|flow| [flow.from, flow.to]);
        let (mut lowest, mut highest) = (f32::INFINITY, f32::NEG_INFINITY);
        for at in f.particles.iter().map(|p| p.at).chain(ends) {
            assert!((0.0..=columns as f32).contains(&at.x), "{size:?}: {at}");
            assert!((-(rows as f32)..=0.0).contains(&at.z), "{size:?}: {at}");
            // The four region centres round the point (x, -z) of the plan.
            let x0 = ((at.x - 0.5).floor().max(0.0) as usize).min(columns - 1);
            let y0 = ((-at.z - 0.5).floor().max(0.0) as usize).min(rows - 1);
            let (x1, y1) = ((x0 + 1).min(columns - 1), (y0 + 1).min(rows - 1));
            let round =
                [(x0, y0), (x1, y0), (x0, y1), (x1, y1)].map(|(x, y)| heights[y * columns + x]);
            let low = round.iter().copied().fold(f32::INFINITY, f32::min);
            let high = round.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(
                at.y >= low - 1e-4 && at.y <= high + 0.3 + 1e-4,
                "{size:?}: {at} not in {low}..{high}"
            );
            lowest = lowest.min(at.y);
            highest = highest.max(at.y);
        }
        assert!(
            highest - lowest > 0.5,
            "{size:?}: everything stands at one height"
        );
        for period in 1..=6 {
            for flow in &world::frame(size, period).flows {
                assert_ne!(
                    flow.from, flow.to,
                    "{size:?} period {period}: a flow to itself"
                );
            }
        }
    }
}

#[test]
fn the_example_plays_three_periods_without_a_window() {
    for size in [Size::Small, Size::Large] {
        let (c, s, p) = world::styles(size);
        let most = s.streams[0].max_marks;
        let total_beats: Duration = c.beats.iter().sum();
        let mut app = field_app(c, s, p);
        app.world_mut().spawn(Ground {
            shapes: Arc::from(world::regions(size)),
            form: GroundForm::Surface,
        });
        app.update();
        for period in 1..=3 {
            app.world_mut()
                .write_message(ShowPeriod(Box::new(world::frame(size, period))));
            app.update();
            let t0 = app.world().resource::<Time>().elapsed();
            let report = app.world().resource::<FlowReport>().clone();
            assert_eq!(report.period, Some(period));
            assert_eq!(report.streams.len(), 6, "full and hollow of three streams");
            for s in &report.streams {
                assert_eq!(s.drawn + s.not_drawn, s.total, "{s:?}");
                assert!(s.marks <= most, "{s:?}");
            }
            advance_to(&mut app, t0 + total_beats + Duration::from_millis(100));
            let refused = &app.world().resource::<common::Collected>().refused;
            assert!(refused.is_empty(), "{size:?}: {refused:?}");
        }
    }
}

#[test]
fn the_example_world_is_the_same_twice() {
    for size in [Size::Small, Size::Large] {
        assert_eq!(world::frame(size, 2), world::frame(size, 2));
    }
}

#[path = "../examples/field/options.rs"]
mod options;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn the_command_line_reads_fullscreen_and_still_reads_size() {
    let plain = options::parse(args(&["--size", "1920x1080"])).unwrap();
    assert!(!plain.fullscreen);
    let full = options::parse(args(&["--fullscreen", "--size", "1920x1080"])).unwrap();
    assert!(full.fullscreen);
    assert_eq!((full.width, full.height), (1920, 1080));
}

#[test]
fn the_command_line_reads_the_large_world() {
    assert!(!options::parse(args(&[])).unwrap().large);
    assert!(options::parse(args(&["--large"])).unwrap().large);
}

#[test]
fn the_command_line_refuses_an_unknown_argument() {
    let error = options::parse(args(&["--windowed"])).unwrap_err();
    assert!(error.contains("unknown argument --windowed"), "{error}");
}
