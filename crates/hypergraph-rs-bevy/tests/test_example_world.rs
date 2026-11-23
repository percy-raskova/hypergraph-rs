mod common;

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use common::{advance_to, field_app};
use hypergraph_rs_bevy::{FlowReport, Ground, GroundForm, ShowPeriod};

#[path = "../examples/field/world.rs"]
mod world;

#[test]
fn the_example_world_is_the_size_the_design_holds() {
    assert_eq!(world::regions().len(), 3168);
    let f = world::frame(1);
    assert_eq!(f.particles.len(), 25_000);
    assert_eq!(f.sets.len(), 6);
    assert_eq!(f.validate(&world::styles().1), Ok(()));
}

#[test]
fn the_example_plays_three_periods_without_a_window() {
    let (c, s, p) = world::styles();
    let total_beats: Duration = c.beats.iter().sum();
    let mut app = field_app(c, s, p);
    app.world_mut().spawn(Ground {
        shapes: Arc::from(world::regions()),
        form: GroundForm::Surface,
    });
    app.update();
    for period in 1..=3 {
        app.world_mut()
            .write_message(ShowPeriod(Box::new(world::frame(period))));
        app.update();
        let t0 = app.world().resource::<Time>().elapsed();
        let report = app.world().resource::<FlowReport>().clone();
        assert_eq!(report.period, Some(period));
        assert_eq!(report.streams.len(), 6, "full and hollow of three streams");
        for s in &report.streams {
            assert_eq!(s.drawn + s.not_drawn, s.total, "{s:?}");
            assert!(s.marks <= 30_000, "{s:?}");
        }
        advance_to(&mut app, t0 + total_beats + Duration::from_millis(100));
        let refused = &app.world().resource::<common::Collected>().refused;
        assert!(refused.is_empty(), "{refused:?}");
    }
}

#[test]
fn the_example_world_is_the_same_twice() {
    assert_eq!(world::frame(2), world::frame(2));
}
