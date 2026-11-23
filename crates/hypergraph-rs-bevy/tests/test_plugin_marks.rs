mod common;

use std::sync::Arc;
use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use common::{advance_to, field_app, snapshot};
use hypergraph_rs::geometry::{Outline, Point};
use hypergraph_rs_bevy::{
    glyph_mesh, BeatClock, FieldConfig, FieldFrame, FieldStyles, Flow, FlowReport, Glyph, Ground,
    GroundForm, LineStyle, MarkOf, Motion, Palette, ParticleFrame, ParticleStyle, PathLines, Phase,
    RegionReading, RegionShape, ReplayPeriod, SetFrame, SetStyle, ShowPeriod, StreamStyle,
};
use std::num::NonZeroU64;

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

fn stream(beat: usize, unit: u64, full: Glyph, hollow: Glyph, swatch: u16) -> StreamStyle {
    StreamStyle {
        beat,
        unit: NonZeroU64::new(unit).unwrap(),
        full,
        hollow,
        size: 0.2,
        swatch,
        share_drawn: 1.0,
        max_marks: 1000,
    }
}

fn styles() -> FieldStyles {
    FieldStyles {
        particles: vec![ParticleStyle {
            glyph: Glyph::Disc,
            size: 0.1,
            swatch: 0,
        }],
        sets: vec![SetStyle {
            swatch: 1,
            line: LineStyle {
                width: 0.05,
                dash: None,
            },
            hatch: None,
        }],
        streams: vec![
            stream(0, 1, Glyph::Disc, Glyph::Ring, 0),
            stream(1, 1, Glyph::Square, Glyph::SquareFrame, 1),
            stream(2, 100, Glyph::Streak, Glyph::Streak, 2),
        ],
        region_outline: None,
    }
}

fn config(motion: Motion) -> FieldConfig {
    FieldConfig {
        motion,
        ..FieldConfig::three_beats(Duration::from_secs(1))
    }
}

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

fn app_with(motion: Motion, styles: FieldStyles) -> App {
    let mut app = field_app(config(motion), styles, palette());
    app.world_mut().spawn(Ground {
        shapes: Arc::from(vec![square("a", 0.0), square("b", 1.0)]),
        form: GroundForm::Surface,
    });
    app.update();
    app
}

fn flow(stream: u16, amount: u64, hollow: bool) -> Flow {
    Flow {
        stream,
        from: Vec3::ZERO,
        to: Vec3::new(10.0, 0.0, 0.0),
        amount,
        hollow,
    }
}

fn frame(period: u64, flows: Vec<Flow>) -> FieldFrame {
    FieldFrame {
        period,
        flows,
        ..FieldFrame::default()
    }
}

fn send(app: &mut App, frame: FieldFrame) -> Duration {
    app.world_mut().write_message(ShowPeriod(Box::new(frame)));
    app.update();
    app.world().resource::<Time>().elapsed()
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

/// `(stream, visible)` of every mark.
fn marks(app: &mut App) -> Vec<(u16, bool, Vec3)> {
    let world = app.world_mut();
    let mut q = world.query::<(&MarkOf, &Visibility, &Transform)>();
    q.iter(world)
        .map(|(m, v, t)| (m.stream, *v == Visibility::Visible, t.translation))
        .collect()
}

fn lines(app: &mut App) -> Vec<(u16, bool)> {
    let world = app.world_mut();
    let mut q = world.query::<(&PathLines, &Visibility)>();
    let mut out: Vec<_> = q
        .iter(world)
        .map(|(l, v)| (l.stream, *v == Visibility::Visible))
        .collect();
    out.sort();
    out
}

fn mesh_positions(mesh: &Mesh) -> Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
        other => panic!("positions: {other:?}"),
    }
}

#[test]
fn marks_move_only_in_their_own_beat() {
    let mut app = app_with(Motion::Full, styles());
    let t0 = send(
        &mut app,
        frame(
            1,
            vec![flow(0, 6, false), flow(1, 4, false), flow(2, 300, false)],
        ),
    );
    assert_eq!(marks(&mut app).len(), 6 + 4 + 3);
    let mut seen = [[false; 3]; 3];
    let mut at = t0;
    while at < t0 + secs(3.0) {
        at += Duration::from_millis(10);
        advance_to(&mut app, at);
        let phase = {
            let w = app.world();
            let c = w.resource::<FieldConfig>();
            w.resource::<BeatClock>()
                .phase(w.resource::<Time>().elapsed(), &c.beats)
        };
        if let Phase::Beat { index, .. } = phase {
            for (s, visible, _) in marks(&mut app) {
                if visible {
                    seen[index][usize::from(s)] = true;
                    assert_eq!(usize::from(s), index, "a mark moved outside its beat");
                }
            }
        }
    }
    assert!(seen[0][0] && seen[1][1] && seen[2][2], "{seen:?}");
    advance_to(&mut app, t0 + secs(4.0));
    assert!(marks(&mut app).is_empty());
}

#[test]
fn the_report_matches_the_marks() {
    let mut app = app_with(Motion::Full, styles());
    send(
        &mut app,
        frame(
            7,
            vec![flow(0, 6, false), flow(1, 4, false), flow(2, 300, false)],
        ),
    );
    let report = app.world().resource::<FlowReport>().clone();
    assert_eq!(report.period, Some(7));
    let all = marks(&mut app);
    let streams: std::collections::BTreeSet<u16> =
        report.streams.iter().map(|r| r.stream).collect();
    for stream in streams {
        let total: u32 = report
            .streams
            .iter()
            .filter(|r| r.stream == stream)
            .map(|r| r.marks)
            .sum();
        let n = all.iter().filter(|(s, _, _)| *s == stream).count();
        assert_eq!(n as u32, total, "stream {stream}");
    }
}

#[test]
fn a_replay_rewrites_the_report_for_the_changed_styles() {
    let mut app = app_with(Motion::Full, styles());
    send(&mut app, frame(7, vec![flow(0, 6, false)]));
    assert_eq!(app.world().resource::<FlowReport>().streams[0].marks, 6);
    app.world_mut().resource_mut::<FieldStyles>().streams[0].max_marks = 2;
    app.world_mut().write_message(ReplayPeriod);
    app.update();
    let report = app.world().resource::<FlowReport>().clone();
    assert_eq!(report.period, Some(7));
    assert_eq!(report.streams[0].marks, 2);
    let n = marks(&mut app).iter().filter(|(s, _, _)| *s == 0).count();
    assert_eq!(n, 2);
}

#[test]
fn hollow_flows_get_the_hollow_glyph() {
    let mut app = app_with(Motion::Full, styles());
    send(
        &mut app,
        frame(1, vec![flow(1, 3, true), flow(1, 5, false)]),
    );
    let world = app.world_mut();
    let mut q = world.query::<(&MarkOf, &Mesh3d)>();
    let found: Vec<_> = q.iter(world).map(|(m, h)| (m.flow, h.0.clone())).collect();
    let meshes = world.resource::<Assets<Mesh>>();
    let frame_pos = mesh_positions(&glyph_mesh(Glyph::SquareFrame, 16));
    let square_pos = mesh_positions(&glyph_mesh(Glyph::Square, 16));
    let mut hollow_handles = Vec::new();
    for (flow, handle) in found {
        let got = mesh_positions(meshes.get(&handle).unwrap());
        if flow == 0 {
            assert_eq!(got, frame_pos);
            hollow_handles.push(handle.id());
        } else {
            assert_eq!(got, square_pos);
        }
    }
    assert_eq!(hollow_handles.len(), 3);
    hollow_handles.dedup();
    assert_eq!(hollow_handles.len(), 1, "hollow marks share one mesh");
}

#[test]
fn a_visible_mark_is_on_its_path() {
    let mut app = app_with(Motion::Full, styles());
    let t0 = send(&mut app, frame(1, vec![flow(0, 6, false)]));
    let mut visible = 0;
    let (mut above, mut below) = (false, false);
    let mut at = t0;
    while at < t0 + secs(1.2) {
        at += Duration::from_millis(10);
        advance_to(&mut app, at);
        for (s, shown, p) in marks(&mut app) {
            if s == 0 && shown {
                visible += 1;
                assert!(p.z.abs() <= 0.8 + 1e-4, "{p:?}");
                above |= p.z > 1e-4;
                below |= p.z < -1e-4;
                assert!((0.0..=10.0).contains(&p.x), "{p:?}");
            }
        }
    }
    assert!(visible > 0);
    assert!(above && below, "marks must alternate sides");
}

#[test]
fn reduced_motion_draws_still_paths_and_no_marks() {
    let mut app = app_with(Motion::Reduced, styles());
    let t0 = send(
        &mut app,
        frame(
            1,
            vec![flow(0, 6, false), flow(1, 4, false), flow(2, 300, false)],
        ),
    );
    assert!(marks(&mut app).is_empty());
    assert_eq!(lines(&mut app), vec![(0, true), (1, false), (2, false)]);
    advance_to(&mut app, t0 + secs(1.5));
    assert_eq!(lines(&mut app), vec![(0, false), (1, true), (2, false)]);
    advance_to(&mut app, t0 + secs(2.5));
    assert_eq!(lines(&mut app), vec![(0, false), (1, false), (2, true)]);
    advance_to(&mut app, t0 + secs(3.5));
    assert!(lines(&mut app).is_empty());
}

fn phase_index(app: &App) -> usize {
    let w = app.world();
    let c = w.resource::<FieldConfig>();
    match w
        .resource::<BeatClock>()
        .phase(w.resource::<Time>().elapsed(), &c.beats)
    {
        Phase::Resting => usize::MAX,
        Phase::Beat { index, .. } => index,
    }
}

#[test]
fn reduced_motion_changes_the_picture_only_at_beats() {
    let mut app = app_with(Motion::Reduced, styles());
    let mut record: Vec<(usize, Vec<u8>)> = Vec::new();
    for n in 0..700 {
        if n == 10 || n == 400 {
            let p = if n == 10 { 1 } else { 2 };
            let mut f = frame(p, vec![flow(0, 6, false), flow(1, 4, false)]);
            f.regions = vec![RegionReading {
                region: "a".into(),
                height: Some(p as f32),
                tone: Some(0.5),
            }];
            f.particles = vec![ParticleFrame {
                id: "p1".into(),
                at: Vec3::new(p as f32, 0.0, 0.0),
                style: 0,
            }];
            app.world_mut().write_message(ShowPeriod(Box::new(f)));
        }
        app.update();
        record.push((phase_index(&app), snapshot(&mut app)));
    }
    let mut changes = 0;
    for pair in record.windows(2) {
        if pair[0].1 != pair[1].1 {
            changes += 1;
            assert_ne!(pair[0].0, pair[1].0, "the picture changed inside a beat");
        }
    }
    assert!(changes > 4, "{changes}");
}

#[test]
fn nothing_moves_while_resting() {
    let mut app = app_with(Motion::Full, styles());
    let t0 = send(&mut app, frame(1, vec![flow(0, 6, false)]));
    advance_to(&mut app, t0 + secs(3.5));
    let settled = snapshot(&mut app);
    for _ in 0..200 {
        app.update();
        assert_eq!(snapshot(&mut app), settled);
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 1000.0
    }
}

fn random_frame(seed: u64, period: u64) -> FieldFrame {
    let mut r = Lcg(seed);
    FieldFrame {
        period,
        regions: vec![RegionReading {
            region: "a".into(),
            height: Some(r.unit()),
            tone: Some(r.unit() / 5.0 - 1.0),
        }],
        particles: (0..500)
            .map(|i| ParticleFrame {
                id: format!("p{i}"),
                at: Vec3::new(r.unit(), 0.0, -r.unit()),
                style: 0,
            })
            .collect(),
        sets: (0..3)
            .map(|i| SetFrame {
                id: format!("s{i}"),
                style: 0,
                height: 0.5,
                members: vec![("a".into(), 1.0), ("b".into(), 2.0)],
                outline: Some(Outline::Ring {
                    center: (f64::from(r.unit()), f64::from(r.unit())),
                    radius: 1.0 + f64::from(r.unit()) / 4.0,
                }),
                held_share: Some(1.0),
            })
            .collect(),
        flows: (0..300)
            .map(|_| Flow {
                stream: (r.next() % 3) as u16,
                from: Vec3::new(r.unit(), 0.0, -r.unit()),
                to: Vec3::new(r.unit(), 0.0, -r.unit()),
                amount: r.next() % 400,
                hollow: r.next().is_multiple_of(5),
            })
            .collect(),
    }
}

#[test]
fn the_same_picture_twice() {
    let mut a = app_with(Motion::Full, styles());
    let mut b = app_with(Motion::Full, styles());
    for n in 0..400 {
        for (at, seed) in [(5, 11), (150, 12), (290, 13)] {
            if n == at {
                for app in [&mut a, &mut b] {
                    app.world_mut()
                        .write_message(ShowPeriod(Box::new(random_frame(seed, seed))));
                }
            }
        }
        a.update();
        b.update();
        assert_eq!(snapshot(&mut a), snapshot(&mut b), "update {n}");
    }
}

#[test]
fn huge_flows_are_capped_and_counted() {
    let mut app = app_with(Motion::Full, styles());
    send(&mut app, frame(1, vec![flow(0, u64::MAX, false)]));
    assert_eq!(marks(&mut app).len(), 1000);
    let report = app.world().resource::<FlowReport>();
    let r = report
        .streams
        .iter()
        .find(|r| r.stream == 0 && !r.hollow)
        .unwrap();
    assert_eq!(r.not_drawn, u128::from(u64::MAX) - 1000);
}
