//! Draws an invented world behind `FieldPlugin`: regions under soft hills,
//! particles seated on them, sets as hatched outlines and three streams of
//! marks, one a beat. The world is small enough to read: 1,728 regions, 240
//! particles (nine of them posts, one a place) and five sets. With `--large`
//! it is the measured world of 3,168 regions, 25,000 particles and six sets.
//!
//! The view is from one side and above, with no perspective, so sizes compare
//! across the whole field.
//!
//! Keys: `Space` replays the period, `N` shows the next one, `F` switches the
//! ground's form, `R` switches reduced motion. With `--measure-seconds N` a
//! new period is shown 0.5 s after each rest, frame times are printed in
//! windows, and the program exits after N seconds. With `--fullscreen` the
//! window is borderless and covers its whole monitor (`--size` is still read;
//! the summary line prints the window's size as it is).

mod options;
mod world;

use std::sync::Arc;
use std::time::Duration;

use bevy::camera::ScalingMode;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use hypergraph_rs_bevy::{
    BeatClock, FieldConfig, FieldPlugin, FieldSystems, FlowReport, FrameWindows, Ground,
    GroundForm, Motion, Phase, ReplayPeriod, ShowPeriod,
};
use options::Options;
use world::Size;

/// The colour behind everything.
const BACKGROUND: Color = Color::srgb(0.071, 0.016, 0.024);
/// The share of the view's width that the grid takes.
const GRID_SHARE: f32 = 0.9;

fn options() -> Options {
    options::parse(std::env::args().skip(1)).unwrap_or_else(|message| panic!("{message}"))
}

/// The state of the example's own systems.
#[derive(Resource)]
struct Run {
    size: Size,
    period: u64,
    measure_until: Option<Duration>,
    rested_since: Option<Duration>,
    windows: FrameWindows,
    last_report: FlowReport,
    reported: bool,
}

fn main() {
    let options = options();
    let size = if options.large {
        Size::Large
    } else {
        Size::Small
    };
    let (config, styles, palette) = world::styles(size);
    let plugin = FieldPlugin::new(config, styles, palette).expect("the example's styles are valid");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "hypergraph-rs-bevy field".into(),
                resolution: (options.width, options.height).into(),
                present_mode: PresentMode::AutoNoVsync,
                mode: if options.fullscreen {
                    WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                } else {
                    WindowMode::Windowed
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(plugin)
        .insert_resource(ClearColor(BACKGROUND))
        // Enough light from everywhere that no slope of the ground goes black.
        .insert_resource(GlobalAmbientLight {
            brightness: 450.0,
            ..default()
        })
        .insert_resource(Run {
            size,
            period: 0,
            measure_until: options.measure_seconds.map(Duration::from_secs_f64),
            rested_since: None,
            windows: FrameWindows::new(300, Duration::from_secs(2)),
            last_report: FlowReport::default(),
            reported: false,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (keys, next_when_measuring)
                .chain()
                .before(FieldSystems::Install),
        )
        .add_systems(Last, sample)
        .run();
}

fn setup(mut commands: Commands, run: Res<Run>) {
    let shape = run.size.shape();
    let (wide, deep) = (shape.columns as f32, shape.rows as f32);
    let middle = Vec3::new(wide / 2.0, 0.0, -deep / 2.0);
    commands.spawn(Ground {
        shapes: Arc::from(world::regions(run.size)),
        form: GroundForm::Surface,
    });
    // How far above the ground the view stands, in radians: 38 degrees over
    // the small world, 52 over the large one, which is wider than it is deep.
    let angle: f32 = match run.size {
        Size::Small => 0.663,
        Size::Large => 0.91,
    };
    let back = Vec3::new(0.0, angle.sin(), angle.cos());
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedHorizontal {
                viewport_width: wide / GRID_SHARE,
            },
            ..OrthographicProjection::default_3d()
        }),
        // The palette's colours reach the screen as they are.
        Tonemapping::None,
        Msaa::Sample4,
        Transform::from_translation(middle + back * 2.0 * wide).looking_at(middle, Vec3::Y),
    ));
    // One light from the near left, so a slope shows which way it faces.
    commands.spawn((
        DirectionalLight {
            illuminance: 2_300.0,
            ..default()
        },
        Transform::from_xyz(-0.5, 1.0, 0.6).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn show(run: &mut Run, shows: &mut MessageWriter<ShowPeriod>) {
    run.period += 1;
    shows.write(ShowPeriod(Box::new(world::frame(run.size, run.period))));
}

fn keys(
    input: Res<ButtonInput<KeyCode>>,
    mut run: ResMut<Run>,
    mut shows: MessageWriter<ShowPeriod>,
    mut replays: MessageWriter<ReplayPeriod>,
    mut config: ResMut<FieldConfig>,
    mut grounds: Query<&mut Ground>,
) {
    if run.period == 0 {
        show(&mut run, &mut shows);
    }
    if input.just_pressed(KeyCode::Space) {
        replays.write(ReplayPeriod);
    }
    if input.just_pressed(KeyCode::KeyN) {
        show(&mut run, &mut shows);
    }
    if input.just_pressed(KeyCode::KeyF) {
        for mut ground in &mut grounds {
            ground.form = match ground.form {
                GroundForm::Surface => GroundForm::Blocks,
                GroundForm::Blocks => GroundForm::Surface,
            };
        }
    }
    if input.just_pressed(KeyCode::KeyR) {
        config.motion = match config.motion {
            Motion::Full => Motion::Reduced,
            Motion::Reduced => Motion::Full,
        };
    }
}

/// When measuring, shows a new period 0.5 s after each rest, so the beats
/// always play.
fn next_when_measuring(
    time: Res<Time>,
    clock: Res<BeatClock>,
    config: Res<FieldConfig>,
    mut run: ResMut<Run>,
    mut shows: MessageWriter<ShowPeriod>,
) {
    if run.measure_until.is_none() || clock.period().is_none() {
        return;
    }
    let now = time.elapsed();
    if clock.phase(now, &config.beats) != Phase::Resting {
        run.rested_since = None;
        return;
    }
    let since = *run.rested_since.get_or_insert(now);
    if now.saturating_sub(since) >= Duration::from_millis(500) {
        run.rested_since = None;
        show(&mut run, &mut shows);
    }
}

fn sample(
    real: Res<Time<Real>>,
    report: Res<FlowReport>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut run: ResMut<Run>,
    mut exit: MessageWriter<AppExit>,
) {
    if report.period.is_some() {
        run.last_report = report.clone();
    }
    let Some(until) = run.measure_until else {
        return;
    };
    if let Some(stats) = run.windows.push(real.delta()) {
        println!(
            "frame_window index={} median_ms={:.3} p95_ms={:.3}",
            stats.index,
            stats.median.as_secs_f64() * 1000.0,
            stats.p95.as_secs_f64() * 1000.0
        );
    }
    if run.reported || real.elapsed() < until {
        return;
    }
    run.reported = true;
    let (w, h) = window
        .single()
        .map_or((0, 0), |w| (w.physical_width(), w.physical_height()));
    let marks = |stream: u16| -> u32 {
        run.last_report
            .streams
            .iter()
            .filter(|s| s.stream == stream)
            .map(|s| s.marks)
            .sum()
    };
    let shape = run.size.shape();
    println!(
        "field size={w}x{h} regions={} particles={} marks_per_beat={},{},{} slowest_p95_ms={:.3}",
        shape.columns * shape.rows,
        shape.particles,
        marks(0),
        marks(1),
        marks(2),
        run.windows
            .slowest_p95()
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    );
    exit.write(AppExit::Success);
}
