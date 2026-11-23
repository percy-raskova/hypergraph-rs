//! A first frame has no earlier picture to ease from, so it stands from its
//! first beat; later frames still ease in the ease beat.

mod common;

use std::sync::Arc;
use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use common::{advance_to, field_app};
use hypergraph_rs::geometry::{Outline, Point};
use hypergraph_rs_bevy::{
    FieldConfig, FieldFrame, FieldStyles, Glyph, Ground, GroundForm, GroundPart, Hatch,
    HatchDirection, LineStyle, Motion, Palette, ParticleFrame, ParticleId, ParticleStyle,
    RegionReading, RegionShape, ReplayPeriod, SetFrame, SetId, SetStyle, ShowPeriod,
};

fn square(id: &str, dx: f64) -> RegionShape {
    let p = |x: f64, y: f64| -> Point { (x + dx, y) };
    RegionShape {
        id: id.to_string(),
        triangles: vec![
            [p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
            [p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
        ],
        rings: vec![vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]],
    }
}

fn palette() -> Palette {
    Palette {
        ramp: [
            Color::srgb(0.0, 0.0, 1.0),
            Color::srgb(0.5, 0.5, 0.5),
            Color::srgb(1.0, 0.0, 0.0),
        ],
        no_reading: Color::srgb(0.3, 0.3, 0.3),
        swatches: vec![Color::srgb(1.0, 1.0, 0.0), Color::srgb(0.0, 1.0, 1.0)],
    }
}

fn styles() -> FieldStyles {
    FieldStyles {
        particles: vec![
            ParticleStyle {
                glyph: Glyph::Disc,
                size: 1.0,
                swatch: 0,
            },
            ParticleStyle {
                glyph: Glyph::Ring,
                size: 2.5,
                swatch: 1,
            },
        ],
        sets: vec![
            SetStyle {
                swatch: 0,
                line: LineStyle {
                    width: 0.05,
                    dash: None,
                },
                hatch: Some(Hatch {
                    direction: HatchDirection::Deg45,
                    spacing: 0.2,
                    width: 0.02,
                }),
            },
            SetStyle {
                swatch: 1,
                line: LineStyle {
                    width: 0.05,
                    dash: None,
                },
                hatch: None,
            },
        ],
        streams: vec![],
        region_outline: None,
    }
}

fn app(motion: Motion) -> App {
    let mut config = FieldConfig::three_beats(Duration::from_secs(1));
    config.motion = motion;
    let mut app = field_app(config, styles(), palette());
    app.world_mut().spawn(Ground {
        shapes: Arc::from(vec![square("a", 0.0)]),
        form: GroundForm::Surface,
    });
    app.update();
    app
}

fn particle(id: &str, x: f32, style: u16) -> ParticleFrame {
    ParticleFrame {
        id: id.to_string(),
        at: Vec3::new(x, 0.0, 0.0),
        style,
    }
}

fn ring(id: &str, style: u16, radius: f64) -> SetFrame {
    SetFrame {
        id: id.to_string(),
        style,
        height: 0.5,
        members: [("m".to_string(), 1.0)].into_iter().collect(),
        outline: Some(Outline::Ring {
            center: (0.0, 0.0),
            radius,
        }),
        held_share: Some(1.0),
    }
}

fn frame_one() -> FieldFrame {
    FieldFrame {
        period: 1,
        regions: vec![RegionReading {
            region: "a".to_string(),
            height: Some(2.0),
            tone: Some(1.0),
        }],
        particles: vec![particle("p", 1.0, 0), particle("q", -3.0, 1)],
        sets: vec![ring("club", 0, 1.0), ring("team", 1, 1.0)],
        ..FieldFrame::default()
    }
}

fn frame_two() -> FieldFrame {
    FieldFrame {
        period: 2,
        regions: vec![RegionReading {
            region: "a".to_string(),
            height: Some(4.0),
            tone: Some(-1.0),
        }],
        particles: vec![particle("p", 5.0, 0), particle("q", -7.0, 1)],
        sets: vec![ring("club", 0, 2.0), ring("team", 1, 1.0)],
        ..FieldFrame::default()
    }
}

fn show(app: &mut App, frame: FieldFrame) -> Duration {
    app.world_mut().write_message(ShowPeriod(Box::new(frame)));
    app.update();
    app.world().resource::<Time>().elapsed()
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

fn particles(app: &mut App) -> Vec<(String, Vec3, Vec3)> {
    let world = app.world_mut();
    let mut q = world.query::<(&ParticleId, &Transform)>();
    let mut out: Vec<_> = q
        .iter(world)
        .map(|(p, t)| (p.0.clone(), t.translation, t.scale))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// A set's meshes' positions, one list per child, in child order.
fn set_shape(app: &mut App, id: &str) -> Option<Vec<Vec<[f32; 3]>>> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &SetId)>();
    let entity = q.iter(world).find(|(_, s)| s.0 == id).map(|(e, _)| e)?;
    let kids: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    Some(
        kids.into_iter()
            .map(|k| {
                let handle = world.get::<Mesh3d>(k).expect("a mesh").0.clone();
                match world
                    .resource::<Assets<Mesh>>()
                    .get(&handle)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
                    other => panic!("positions: {other:?}"),
                }
            })
            .collect(),
    )
}

/// The body mesh's positions and colours.
fn ground_body(app: &mut App) -> (Vec<[f32; 3]>, Vec<[f32; 4]>) {
    let world = app.world_mut();
    let mut q = world.query::<(&GroundPart, &Mesh3d)>();
    let handle = q
        .iter(world)
        .find(|(p, _)| **p == GroundPart::Body)
        .map(|(_, m)| m.0.clone())
        .expect("a ground body");
    let mesh = world.resource::<Assets<Mesh>>().get(&handle).unwrap();
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("positions: {other:?}"),
    };
    let colours = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(v)) => v.clone(),
        other => panic!("colours: {other:?}"),
    };
    (positions, colours)
}

fn top_y(positions: &[[f32; 3]]) -> f32 {
    positions.iter().map(|v| v[1]).fold(f32::MIN, f32::max)
}

/// Everything the three marks show, in comparable form.
type Shown = (
    Vec<(String, Vec3, Vec3)>,
    Vec<Option<Vec<Vec<[f32; 3]>>>>,
    (Vec<[f32; 3]>, Vec<[f32; 4]>),
);

fn shown(app: &mut App) -> Shown {
    (
        particles(app),
        vec![set_shape(app, "club"), set_shape(app, "team")],
        ground_body(app),
    )
}

/// What a frame shows once the beats have rested, in an app of its own.
fn settled(frame: FieldFrame, motion: Motion) -> Shown {
    let mut app = app(motion);
    show(&mut app, frame);
    advance_to(&mut app, secs(4.0));
    shown(&mut app)
}

fn stands_from_the_first_beat(motion: Motion) {
    let mut app = app(motion);
    let t0 = show(&mut app, frame_one());
    advance_to(&mut app, t0 + Duration::from_millis(10));
    let early = shown(&mut app);
    // 10 ms into beat 0 of 1 s beats
    assert!(app.world().resource::<Time>().elapsed() - t0 < secs(0.1));
    let ps = &early.0;
    assert_eq!(ps.len(), 2);
    assert_eq!(ps[0].0, "p");
    assert_eq!(ps[0].1, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(ps[0].2, Vec3::splat(1.0));
    assert_eq!(ps[1].0, "q");
    assert_eq!(ps[1].1, Vec3::new(-3.0, 0.0, 0.0));
    assert_eq!(ps[1].2, Vec3::splat(2.5));
    assert_eq!(early.1[0].as_ref().expect("club").len(), 2);
    assert_eq!(early.1[1].as_ref().expect("team").len(), 1);
    assert_eq!(top_y(&(early.2).0), 2.0);
    assert!(
        early == settled(frame_one(), motion),
        "the first frame stands as it will rest"
    );
}

fn later_frames_still_ease(motion: Motion) {
    let mut app = app(motion);
    show(&mut app, frame_one());
    advance_to(&mut app, secs(4.0));
    let first = shown(&mut app);
    let t0 = show(&mut app, frame_two());
    advance_to(&mut app, t0 + Duration::from_millis(10));
    assert!(shown(&mut app) == first, "10 ms in: still the first frame");
    advance_to(&mut app, t0 + secs(1.5));
    assert!(shown(&mut app) == first, "beat 1: still the first frame");
    advance_to(&mut app, t0 + secs(3.5));
    assert!(
        shown(&mut app) == settled(frame_two(), motion),
        "after the ease beat: the second frame"
    );
}

#[test]
fn a_first_frame_stands_from_its_first_beat() {
    stands_from_the_first_beat(Motion::Full);
}

#[test]
fn a_first_frame_stands_from_its_first_beat_with_reduced_motion() {
    stands_from_the_first_beat(Motion::Reduced);
}

#[test]
fn a_second_frame_still_eases_in_the_ease_beat() {
    later_frames_still_ease(Motion::Full);
}

#[test]
fn a_second_frame_still_eases_in_the_ease_beat_with_reduced_motion() {
    later_frames_still_ease(Motion::Reduced);
}

#[test]
fn a_second_frame_eases_between_the_two_pictures_in_full_motion() {
    let mut app = app(Motion::Full);
    show(&mut app, frame_one());
    advance_to(&mut app, secs(4.0));
    let t0 = show(&mut app, frame_two());
    advance_to(&mut app, t0 + secs(2.5));
    let mid = particles(&mut app);
    assert!((mid[0].1.x - 3.0).abs() < 1e-4, "x = {}", mid[0].1.x);
    assert!((top_y(&ground_body(&mut app).0) - 3.0).abs() < 1e-4);
}

#[test]
fn a_replay_of_the_first_frame_stands_from_its_first_beat() {
    let mut app = app(Motion::Full);
    show(&mut app, frame_one());
    advance_to(&mut app, secs(4.0));
    app.world_mut().write_message(ReplayPeriod);
    app.update();
    let t0 = app.world().resource::<Time>().elapsed();
    advance_to(&mut app, t0 + Duration::from_millis(10));
    assert!(
        shown(&mut app) == settled(frame_one(), Motion::Full),
        "a replay of the first frame stands from its first beat"
    );
}
