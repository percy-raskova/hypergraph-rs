mod common;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use common::{advance_to, field_app, snapshot, Collected};
use hypergraph_rs::geometry::Point;
use hypergraph_rs_bevy::{
    BeatClock, FieldConfig, FieldError, FieldFrame, FieldStyles, Glyph, Ground, GroundForm,
    GroundPart, Motion, Palette, ParticleFrame, ParticleId, ParticleStyle, RegionReading,
    RegionShape, ReplayPeriod, ShowPeriod,
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
        swatches: vec![
            Color::srgb(1.0, 1.0, 0.0),
            Color::srgb(0.0, 1.0, 1.0),
            Color::srgb(1.0, 0.0, 1.0),
        ],
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
                size: 2.0,
                swatch: 1,
            },
        ],
        sets: vec![],
        streams: vec![],
        region_outline: Some(2),
    }
}

fn config() -> FieldConfig {
    FieldConfig::three_beats(Duration::from_secs(1))
}

fn app() -> App {
    let mut app = field_app(config(), styles(), palette());
    app.world_mut().spawn(Ground {
        shapes: Arc::from(vec![square("a", 0.0), square("b", 1.0)]),
        form: GroundForm::Surface,
    });
    app.update();
    app
}

fn reading(region: &str, height: f32, tone: f32) -> RegionReading {
    RegionReading {
        region: region.to_string(),
        height: Some(height),
        tone: Some(tone),
    }
}

fn particle(id: &str, x: f32, style: u16) -> ParticleFrame {
    ParticleFrame {
        id: id.to_string(),
        at: Vec3::new(x, 0.0, 0.0),
        style,
    }
}

fn frame_a(period: u64, height: f32) -> FieldFrame {
    FieldFrame {
        period,
        regions: vec![reading("a", height, 1.0)],
        ..FieldFrame::default()
    }
}

fn show(app: &mut App, frame: FieldFrame) {
    app.world_mut().write_message(ShowPeriod(Box::new(frame)));
}

fn now(app: &App) -> Duration {
    app.world().resource::<Time>().elapsed()
}

/// Sends a frame, updates once, and returns the time the clock started.
fn send(app: &mut App, frame: FieldFrame) -> Duration {
    show(app, frame);
    app.update();
    now(app)
}

fn part_mesh(app: &mut App, part: GroundPart) -> Mesh {
    let world = app.world_mut();
    let mut q = world.query::<(&GroundPart, &Mesh3d)>();
    let handle = q
        .iter(world)
        .find(|(p, _)| **p == part)
        .map(|(_, m)| m.0.clone())
        .expect("a ground part");
    world
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .clone()
}

fn positions(mesh: &Mesh) -> Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("positions: {other:?}"),
    }
}

fn colours(mesh: &Mesh) -> Vec<[f32; 4]> {
    match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(v)) => v.clone(),
        other => panic!("colours: {other:?}"),
    }
}

fn top_y(app: &mut App) -> f32 {
    let p = positions(&part_mesh(app, GroundPart::Body));
    p[..18].iter().map(|v| v[1]).fold(f32::MIN, f32::max)
}

fn particle_x(app: &mut App, id: &str) -> Option<f32> {
    let world = app.world_mut();
    let mut q = world.query::<(&ParticleId, &Transform)>();
    q.iter(world)
        .find(|(p, _)| p.0 == id)
        .map(|(_, t)| t.translation.x)
}

fn particle_ids(app: &mut App) -> BTreeSet<String> {
    let world = app.world_mut();
    let mut q = world.query::<&ParticleId>();
    q.iter(world).map(|p| p.0.clone()).collect()
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

#[test]
fn the_plugin_runs_without_a_window() {
    let mut app = app();
    app.update();
    app.update();
}

#[test]
fn a_ground_is_built_once_and_its_form_switches() {
    let mut app = app();
    let bodies = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<&GroundPart>();
        q.iter(world).filter(|p| **p == GroundPart::Body).count()
    };
    assert_eq!(bodies(&mut app), 1);
    assert_eq!(positions(&part_mesh(&mut app, GroundPart::Body)).len(), 36);
    let before: Vec<(u32, u32)> = positions(&part_mesh(&mut app, GroundPart::Outline))
        .iter()
        .map(|v| (v[0].to_bits(), v[2].to_bits()))
        .collect();
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut Ground>();
        for mut g in q.iter_mut(world) {
            g.form = GroundForm::Blocks;
        }
    }
    app.update();
    assert_eq!(bodies(&mut app), 1);
    assert_eq!(positions(&part_mesh(&mut app, GroundPart::Body)).len(), 60);
    let after: Vec<(u32, u32)> = positions(&part_mesh(&mut app, GroundPart::Outline))
        .iter()
        .map(|v| (v[0].to_bits(), v[2].to_bits()))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn heights_ease_in_the_ease_beat_and_rest_after() {
    let mut app = app();
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    assert_eq!(top_y(&mut app), 2.0);
    let t0 = send(&mut app, frame_a(2, 4.0));
    advance_to(&mut app, t0 + secs(1.5));
    assert_eq!(top_y(&mut app), 2.0);
    advance_to(&mut app, t0 + secs(2.5));
    assert!((top_y(&mut app) - 3.0).abs() < 1e-5);
    advance_to(&mut app, t0 + secs(3.5));
    assert_eq!(top_y(&mut app), 4.0);
    let rest = snapshot(&mut app);
    for _ in 0..100 {
        app.update();
    }
    assert_eq!(snapshot(&mut app), rest);
}

#[test]
fn no_reading_is_gray_and_flat() {
    let mut app = app();
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut Ground>();
        for mut g in q.iter_mut(world) {
            g.form = GroundForm::Blocks;
        }
    }
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    let mesh = part_mesh(&mut app, GroundPart::Body);
    let (p, c) = (positions(&mesh), colours(&mesh));
    let gray = palette().tone(None).to_f32_array();
    assert_eq!(p.len(), 60);
    for i in 30..60 {
        assert_eq!(p[i][1], 0.0);
        assert_eq!(c[i], gray);
    }
}

#[test]
fn the_last_frame_of_an_update_wins() {
    let mut app = app();
    show(&mut app, frame_a(1, 2.0));
    show(&mut app, frame_a(2, 4.0));
    app.world_mut().write_message(ReplayPeriod);
    app.update();
    assert!(app.world().resource::<Collected>().refused.is_empty());
    assert_eq!(app.world().resource::<BeatClock>().period(), Some(2));
    advance_to(&mut app, secs(5.0));
    assert_eq!(top_y(&mut app), 4.0);
}

#[test]
fn motion_is_fixed_when_a_frame_is_accepted() {
    let mut app = app();
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    let t0 = send(&mut app, frame_a(2, 4.0));
    advance_to(&mut app, t0 + secs(2.2));
    app.world_mut().resource_mut::<FieldConfig>().motion = Motion::Reduced;
    advance_to(&mut app, t0 + secs(2.5));
    assert!((top_y(&mut app) - 3.0).abs() < 1e-5);
}

#[test]
fn particles_arrive_move_once_and_leave() {
    let mut app = app();
    let f1 = FieldFrame {
        period: 1,
        particles: vec![particle("p1", 0.0, 0), particle("p2", 1.0, 0)],
        ..FieldFrame::default()
    };
    send(&mut app, f1);
    advance_to(&mut app, secs(4.0));
    let f2 = FieldFrame {
        period: 2,
        particles: vec![particle("p1", 2.0, 0), particle("p3", 5.0, 0)],
        ..FieldFrame::default()
    };
    let t0 = send(&mut app, f2);
    advance_to(&mut app, t0 + secs(1.5));
    assert_eq!(particle_x(&mut app, "p1"), Some(0.0));
    assert_eq!(particle_x(&mut app, "p3"), None);
    assert!(particle_x(&mut app, "p2").is_some());
    advance_to(&mut app, t0 + secs(2.5));
    assert!((particle_x(&mut app, "p1").unwrap() - 1.0).abs() < 1e-5);
    assert!(particle_x(&mut app, "p2").is_some());
    assert!(particle_x(&mut app, "p3").is_some());
    advance_to(&mut app, t0 + secs(3.5));
    let want: BTreeSet<String> = ["p1", "p3"].iter().map(|s| (*s).to_string()).collect();
    assert_eq!(particle_ids(&mut app), want);
    assert_eq!(particle_x(&mut app, "p1"), Some(2.0));
}

#[test]
fn particles_share_meshes_and_materials() {
    let mut app = app();
    let particles: Vec<ParticleFrame> = (0..25_000)
        .map(|i| particle(&format!("p{i}"), i as f32, (i % 2) as u16))
        .collect();
    send(
        &mut app,
        FieldFrame {
            period: 1,
            particles,
            ..FieldFrame::default()
        },
    );
    advance_to(&mut app, secs(4.0));
    let world = app.world_mut();
    let mut q = world.query::<(&ParticleId, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
    let mut count = 0;
    let mut meshes = BTreeSet::new();
    let mut materials = BTreeSet::new();
    for (_, m, mat) in q.iter(world) {
        count += 1;
        meshes.insert(format!("{:?}", m.0.id()));
        materials.insert(format!("{:?}", mat.0.id()));
    }
    assert_eq!(count, 25_000);
    assert_eq!(meshes.len(), 2);
    assert_eq!(materials.len(), 2);
}

#[test]
fn a_bad_frame_is_refused_and_the_picture_stays() {
    let mut app = app();
    send(
        &mut app,
        FieldFrame {
            period: 1,
            regions: vec![reading("a", 2.0, 1.0)],
            particles: vec![particle("p1", 0.0, 0)],
            ..FieldFrame::default()
        },
    );
    advance_to(&mut app, secs(4.0));
    let before = snapshot(&mut app);
    send(
        &mut app,
        FieldFrame {
            period: 2,
            particles: vec![particle("p1", 1.0, 0), particle("p1", 2.0, 0)],
            ..FieldFrame::default()
        },
    );
    for _ in 0..50 {
        app.update();
    }
    {
        let c = app.world().resource::<Collected>();
        assert_eq!(c.refused.len(), 1);
        assert_eq!(c.refused[0].period, 2);
        assert_eq!(c.refused[0].error, FieldError::DuplicateId("p1".into()));
    }
    assert_eq!(snapshot(&mut app), before);
    send(
        &mut app,
        FieldFrame {
            period: 3,
            regions: vec![reading("zz", 1.0, 0.0)],
            ..FieldFrame::default()
        },
    );
    let c = app.world().resource::<Collected>();
    assert_eq!(c.refused.len(), 2);
    assert_eq!(c.refused[1].error, FieldError::UnknownRegion("zz".into()));
    assert_eq!(snapshot(&mut app), before);
}

#[test]
fn a_frame_during_the_beats_restarts_from_the_last_settled_picture() {
    let mut app = app();
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    let t0 = send(&mut app, frame_a(2, 4.0));
    advance_to(&mut app, t0 + secs(2.5));
    assert!((top_y(&mut app) - 3.0).abs() < 1e-5);
    let t1 = send(&mut app, frame_a(3, 10.0));
    assert_eq!(top_y(&mut app), 4.0);
    advance_to(&mut app, t1 + secs(2.5));
    assert!((top_y(&mut app) - 7.0).abs() < 1e-5);
}

#[test]
fn replay_plays_the_last_move_again() {
    let mut app = app();
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    send(&mut app, frame_a(2, 4.0));
    advance_to(&mut app, secs(8.0));
    assert_eq!(top_y(&mut app), 4.0);
    app.world_mut().write_message(ReplayPeriod);
    app.update();
    let t2 = now(&app);
    advance_to(&mut app, t2 + secs(2.5));
    assert!((top_y(&mut app) - 3.0).abs() < 1e-5);
    advance_to(&mut app, t2 + secs(3.5));
    assert_eq!(top_y(&mut app), 4.0);
}

#[test]
fn the_palette_is_the_callers() {
    let mut app = app();
    send(
        &mut app,
        FieldFrame {
            period: 1,
            regions: vec![reading("a", 2.0, 1.0)],
            particles: vec![particle("p1", 0.0, 0)],
            ..FieldFrame::default()
        },
    );
    advance_to(&mut app, secs(4.0));
    let green = Color::srgb(0.0, 1.0, 0.0);
    let yellow_to_blue = Color::srgb(0.0, 0.0, 1.0);
    {
        let mut p = app.world_mut().resource_mut::<Palette>();
        p.ramp[2] = green;
        p.swatches[0] = yellow_to_blue;
    }
    app.update();
    let c = colours(&part_mesh(&mut app, GroundPart::Body));
    assert_eq!(c[0], green.to_linear().to_f32_array());
    let world = app.world_mut();
    let mut q = world.query::<(&ParticleId, &MeshMaterial3d<StandardMaterial>)>();
    let handle = q.iter(world).next().unwrap().1 .0.clone();
    let m = world
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap();
    assert_eq!(m.base_color, yellow_to_blue);
}

#[test]
fn reduced_motion_jumps_at_the_end_of_the_ease_beat() {
    let mut app = app();
    send(&mut app, frame_a(1, 2.0));
    advance_to(&mut app, secs(4.0));
    app.world_mut().resource_mut::<FieldConfig>().motion = Motion::Reduced;
    let t0 = send(&mut app, frame_a(2, 4.0));
    advance_to(&mut app, t0 + secs(2.5));
    assert_eq!(top_y(&mut app), 2.0);
    advance_to(&mut app, t0 + secs(3.0));
    assert_eq!(top_y(&mut app), 4.0);
}
