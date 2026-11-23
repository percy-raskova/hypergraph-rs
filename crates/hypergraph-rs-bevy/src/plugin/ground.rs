//! Building grounds and moving their heights and colours through the beats.

use bevy::color::LinearRgba;
use bevy::prelude::*;

use super::{phase_now, FieldAssets, GroundPart, Picture};
use crate::config::FieldConfig;
use crate::ground::{Ground, GroundTopology};
use crate::motion::{eased, BeatClock, Phase};
use crate::palette::Palette;
use crate::style::FieldStyles;

/// The fixed structure of a built ground and the meshes it draws.
#[derive(Component)]
pub(crate) struct GroundBuilt {
    topology: GroundTopology,
    body: Handle<Mesh>,
    outline: Option<Handle<Mesh>>,
}

/// The heights and colours of a ground's regions at a phase.
fn values(
    ground: &Ground,
    picture: &Picture,
    palette: &Palette,
    phase: Phase,
    config: &FieldConfig,
) -> (Vec<f32>, Vec<LinearRgba>) {
    let (ease, motion) = (config.ease_beat, picture.motion);
    let mut heights = Vec::with_capacity(ground.shapes.len());
    let mut colours = Vec::with_capacity(ground.shapes.len());
    for shape in ground.shapes.iter() {
        let (hp, tp) = picture.previous.region(&shape.id);
        let (hc, tc) = picture.current.region(&shape.id);
        heights.push(eased(hp, hc, phase, ease, motion));
        let (a, b) = (palette.tone(tp), palette.tone(tc));
        colours.push(LinearRgba {
            red: eased(a.red, b.red, phase, ease, motion),
            green: eased(a.green, b.green, phase, ease, motion),
            blue: eased(a.blue, b.blue, phase, ease, motion),
            alpha: eased(a.alpha, b.alpha, phase, ease, motion),
        });
    }
    (heights, colours)
}

/// For a new or changed [`Ground`]: build its topology and two child
/// entities, keeping the heights and colours the picture shows now.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn build_grounds(
    mut commands: Commands,
    grounds: Query<(Entity, &Ground, Option<&Children>), Changed<Ground>>,
    parts: Query<(), With<GroundPart>>,
    assets: Res<FieldAssets>,
    picture: Res<Picture>,
    palette: Res<Palette>,
    config: Res<FieldConfig>,
    styles: Res<FieldStyles>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let phase = phase_now(&clock, &time, &config);
    for (entity, ground, children) in &grounds {
        for child in children.into_iter().flat_map(|c| c.iter()) {
            if parts.contains(child) {
                commands.entity(child).despawn();
            }
        }
        let topology = match GroundTopology::new(&ground.shapes, ground.form) {
            Ok(t) => t,
            Err(error) => {
                // `bevy_log` is not among this crate's Bevy features, so the
                // refusal goes to standard error.
                eprintln!("ground {entity}: {error}; it draws nothing");
                commands.entity(entity).remove::<GroundBuilt>();
                continue;
            }
        };
        let (heights, colours) = values(ground, &picture, &palette, phase, &config);
        let body = meshes.add(topology.body_mesh(&heights, &colours));
        commands.spawn((
            GroundPart::Body,
            Mesh3d(body.clone()),
            MeshMaterial3d(assets.ground.clone()),
            ChildOf(entity),
        ));
        let outline = styles
            .region_outline
            .and_then(|i| assets.swatches.get(usize::from(i)))
            .map(|material| {
                let handle = meshes.add(topology.outline_mesh(&heights, config.outline_lift));
                commands.spawn((
                    GroundPart::Outline,
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(material.clone()),
                    ChildOf(entity),
                ));
                handle
            });
        commands.entity(entity).insert(GroundBuilt {
            topology,
            body,
            outline,
        });
    }
}

/// Writes the ground's meshes while the beats play, once when they end, and
/// when the palette changed. Never while the clock rests.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn animate_ground(
    grounds: Query<(&Ground, &GroundBuilt)>,
    picture: Res<Picture>,
    palette: Res<Palette>,
    config: Res<FieldConfig>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut rested: Local<bool>,
) {
    let phase = phase_now(&clock, &time, &config);
    let resting = phase == Phase::Resting;
    if resting && *rested && !palette.is_changed() {
        return;
    }
    *rested = resting;
    for (ground, built) in &grounds {
        let (heights, colours) = values(ground, &picture, &palette, phase, &config);
        if let Some(mut mesh) = meshes.get_mut(&built.body) {
            built.topology.write_body(&mut mesh, &heights, &colours);
        }
        if let Some(handle) = &built.outline {
            if let Some(mut mesh) = meshes.get_mut(handle) {
                built
                    .topology
                    .write_outline(&mut mesh, &heights, config.outline_lift);
            }
        }
    }
}
