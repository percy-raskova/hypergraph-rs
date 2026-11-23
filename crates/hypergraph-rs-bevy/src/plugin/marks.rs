//! Marks: one batch entity per stream, its marks moved by a clock in a
//! shader during the stream's beat; or, with reduced motion, one still line of
//! paths per stream.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::mark_material::{mark_batch_mesh, MarkMaterial, MarkParams};
use super::{phase_now, FieldAssets, MarkBatch, PathLines};
use crate::config::{FieldConfig, Motion};
use crate::flows::{DrawnFlow, Flow};
use crate::motion::{mark_position, BeatClock, Phase};
use crate::palette::Palette;
use crate::style::FieldStyles;

/// How many pieces a path line is cut into, so `u = 0, 1/8, ..., 1`.
const PATH_STEPS: u32 = 8;
/// A path line's width as a share of its stream's mark size.
const PATH_WIDTH: f32 = 0.25;
/// The side a reduced-motion path line bows to (marks alternate by `k`).
const PATH_SIDE: f32 = 1.0;

/// What a stream's batch needs to be shown in its beat, fixed when spawned.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct BatchRun {
    /// The beat in which its marks move.
    beat: usize,
    /// The length of that beat, in seconds.
    beat_secs: f32,
    /// The stream's swatch.
    swatch: u16,
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

/// The asset stores a frame's marks are added to.
pub(super) struct MarkStores<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<MarkMaterial>,
}

/// Spawns the period's mark batches (full motion) or path lines (reduced), all
/// hidden. The caller has already despawned the last period's.
pub(super) fn spawn_marks(
    commands: &mut Commands,
    stores: &mut MarkStores<'_>,
    assets: &FieldAssets,
    styles: &FieldStyles,
    config: &FieldConfig,
    flows: &[Flow],
    drawn: &[DrawnFlow],
) {
    match config.motion {
        Motion::Full => {
            for (s, style) in styles.streams.iter().enumerate() {
                let stream = s as u16;
                if !drawn.iter().any(|d| d.stream == stream && d.marks > 0) {
                    continue;
                }
                let beat = config.beats[style.beat];
                let beat_secs = beat.as_secs_f32();
                let params = MarkParams {
                    clock: 0.0,
                    bow: config.bow,
                    travel: config.mark_travel * beat_secs,
                    unused: 0.0,
                    colour: Vec4::ZERO,
                };
                commands.spawn((
                    MarkBatch { stream },
                    BatchRun {
                        beat: style.beat,
                        beat_secs,
                        swatch: style.swatch,
                    },
                    Mesh3d(stores.meshes.add(mark_batch_mesh(
                        flows,
                        drawn,
                        &styles.streams,
                        config,
                        stream,
                    ))),
                    MeshMaterial3d(stores.materials.add(MarkMaterial { params })),
                    Transform::IDENTITY,
                    NoFrustumCulling,
                    Visibility::Hidden,
                ));
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
                                mark_position(flow.from, flow.to, u, config.bow, PATH_SIDE)
                            })
                            .collect()
                    })
                    .collect();
                commands.spawn((
                    PathLines { stream },
                    Mesh3d(
                        stores
                            .meshes
                            .add(path_ribbon(&paths, style.size * PATH_WIDTH)),
                    ),
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

/// Shows each batch in its stream's beat and writes its clock, shows path
/// lines in theirs, and despawns both when the clock comes to rest. Nothing
/// while resting.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn animate_marks(
    mut commands: Commands,
    config: Res<FieldConfig>,
    styles: Res<FieldStyles>,
    palette: Res<Palette>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    mut materials: ResMut<Assets<MarkMaterial>>,
    mut batches: Query<
        (
            Entity,
            &BatchRun,
            &MeshMaterial3d<MarkMaterial>,
            &mut Visibility,
        ),
        Without<PathLines>,
    >,
    mut lines: Query<(Entity, &PathLines, &mut Visibility), Without<BatchRun>>,
    mut rested: Local<bool>,
) {
    let phase = phase_now(&clock, &time, &config);
    let resting = phase == Phase::Resting;
    if resting && *rested {
        return;
    }
    *rested = resting;
    let Phase::Beat { index, fraction } = phase else {
        for (entity, ..) in &batches {
            commands.entity(entity).despawn();
        }
        for (entity, ..) in &lines {
            commands.entity(entity).despawn();
        }
        return;
    };
    for (_, run, material, mut visibility) in &mut batches {
        if run.beat != index {
            set_visibility(&mut visibility, Visibility::Hidden);
            continue;
        }
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.params.clock = fraction * run.beat_secs;
            if let Some(colour) = palette.swatch(run.swatch) {
                m.params.colour = LinearRgba::from(colour).to_vec4();
            }
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
