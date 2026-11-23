//! Marks: one entity per mark, moving along its flow's path during its
//! stream's beat; or, with reduced motion, one still line of paths per stream.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::{phase_now, FieldAssets, MarkOf, PathLines};
use crate::config::{FieldConfig, Motion};
use crate::flows::{DrawnFlow, Flow};
use crate::motion::{mark_position, mark_start, BeatClock, Phase};
use crate::style::{FieldStyles, Glyph};

/// How many pieces a path line is cut into, so `u = 0, 1/8, ..., 1`.
const PATH_STEPS: u32 = 8;
/// A path line's width as a share of its stream's mark size.
const PATH_WIDTH: f32 = 0.25;
/// The side of its straight line every mark and path bows to.
const SIDE: f32 = 1.0;

/// What a mark needs to move, fixed when it is spawned.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct MarkRun {
    from: Vec3,
    to: Vec3,
    /// The beat in which it moves.
    beat: usize,
    /// The length of that beat, in seconds.
    beat_secs: f32,
    /// When it starts inside the beat, in seconds.
    start: f32,
    /// How long it travels, in seconds.
    tau: f32,
    bow: f32,
    /// Whether it is turned along its motion.
    streak: bool,
}

/// One flat quad per pair of samples, offset across the segment in `xz`, each
/// corner at its own sample's height. Zero-length segments are skipped.
fn path_ribbon(paths: &[Vec<Vec3>], width: f32) -> Mesh {
    let w = width / 2.0;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for path in paths {
        for pair in path.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            let (dx, dz) = (q.x - p.x, q.z - p.z);
            let len = dx.hypot(dz);
            if !(len.is_finite() && len >= 1e-6) {
                continue;
            }
            let (mx, mz) = (dz / len * w, -dx / len * w);
            let base = positions.len() as u32;
            positions.extend_from_slice(&[
                [p.x + mx, p.y, p.z + mz],
                [p.x - mx, p.y, p.z - mz],
                [q.x + mx, q.y, q.z + mz],
                [q.x - mx, q.y, q.z - mz],
            ]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
    }
    let normals = vec![[0.0_f32, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Spawns the period's marks (full motion) or path lines (reduced), all
/// hidden. The caller has already despawned the last period's.
pub(super) fn spawn_marks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    assets: &FieldAssets,
    styles: &FieldStyles,
    config: &FieldConfig,
    flows: &[Flow],
    drawn: &[DrawnFlow],
) {
    match config.motion {
        Motion::Full => {
            for d in drawn {
                let style = &styles.streams[usize::from(d.stream)];
                let glyph = if d.hollow { style.hollow } else { style.full };
                let (Some(mesh), Some(material)) = (
                    assets.glyphs.get(&glyph),
                    assets.swatches.get(usize::from(style.swatch)),
                ) else {
                    continue;
                };
                let flow = &flows[d.flow];
                let beat = config.beats[style.beat];
                let tau = config.mark_travel * beat.as_secs_f32();
                for k in 0..d.marks {
                    let start =
                        mark_start(d.flow, k, d.marks, beat, config.mark_travel).as_secs_f32();
                    commands.spawn((
                        MarkOf {
                            stream: d.stream,
                            flow: d.flow,
                            k,
                        },
                        MarkRun {
                            from: flow.from,
                            to: flow.to,
                            beat: style.beat,
                            beat_secs: beat.as_secs_f32(),
                            start,
                            tau,
                            bow: config.bow,
                            streak: glyph == Glyph::Streak,
                        },
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(flow.from).with_scale(Vec3::splat(style.size)),
                        Visibility::Hidden,
                    ));
                }
            }
        }
        Motion::Reduced => {
            for (s, style) in styles.streams.iter().enumerate() {
                let stream = s as u16;
                let Some(material) = assets.swatches.get(usize::from(style.swatch)) else {
                    continue;
                };
                let paths: Vec<Vec<Vec3>> = drawn
                    .iter()
                    .filter(|d| d.stream == stream)
                    .map(|d| {
                        let flow = &flows[d.flow];
                        (0..=PATH_STEPS)
                            .map(|i| {
                                let u = i as f32 / PATH_STEPS as f32;
                                mark_position(flow.from, flow.to, u, config.bow, SIDE)
                            })
                            .collect()
                    })
                    .collect();
                commands.spawn((
                    PathLines { stream },
                    Mesh3d(meshes.add(path_ribbon(&paths, style.size * PATH_WIDTH))),
                    MeshMaterial3d(material.clone()),
                    Visibility::Hidden,
                ));
            }
        }
    }
}

fn set_visibility(visibility: &mut Visibility, to: Visibility) {
    if *visibility != to {
        *visibility = to;
    }
}

/// Moves marks through their stream's beat, shows path lines in theirs, and
/// despawns both when the clock comes to rest. Nothing while resting.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn animate_marks(
    mut commands: Commands,
    config: Res<FieldConfig>,
    styles: Res<FieldStyles>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    mut marks: Query<(Entity, &MarkRun, &mut Transform, &mut Visibility), Without<PathLines>>,
    mut lines: Query<(Entity, &PathLines, &mut Visibility), Without<MarkRun>>,
    mut rested: Local<bool>,
) {
    let phase = phase_now(&clock, &time, &config);
    let resting = phase == Phase::Resting;
    if resting && *rested {
        return;
    }
    *rested = resting;
    let Phase::Beat { index, fraction } = phase else {
        for (entity, ..) in &marks {
            commands.entity(entity).despawn();
        }
        for (entity, ..) in &lines {
            commands.entity(entity).despawn();
        }
        return;
    };
    for (_, run, mut transform, mut visibility) in &mut marks {
        if run.beat != index {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        }
        let s = fraction * run.beat_secs - run.start;
        if !(s >= 0.0 && s < run.tau) {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        }
        let at = |u: f32| mark_position(run.from, run.to, u, run.bow, SIDE);
        let position = at(s / run.tau);
        if run.streak {
            let behind = at(((s - run.tau / 50.0) / run.tau).max(0.0));
            let (dx, dz) = (position.x - behind.x, position.z - behind.z);
            if dx != 0.0 || dz != 0.0 {
                transform.rotation = Quat::from_rotation_y((-dz).atan2(dx));
            }
        }
        if transform.translation != position {
            transform.translation = position;
        }
        set_visibility(&mut visibility, Visibility::Visible);
    }
    for (_, line, mut visibility) in &mut lines {
        let own = styles
            .streams
            .get(usize::from(line.stream))
            .is_some_and(|s| s.beat == index);
        set_visibility(
            &mut visibility,
            if own {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        );
    }
}
