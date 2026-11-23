//! Particles: one entity per id, arriving, easing and leaving with the beats.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use super::{phase_now, FieldAssets, ParticleId, Picture};
use crate::config::{FieldConfig, Motion};
use crate::motion::{eased, BeatClock, Phase};
use crate::style::FieldStyles;

/// The entity of each particle on screen and the style it was drawn with.
#[derive(Resource, Default)]
pub(crate) struct ParticleEntities {
    by_id: BTreeMap<String, (Entity, u16)>,
}

fn ease_point(from: Vec3, to: Vec3, phase: Phase, ease: usize, motion: Motion) -> Vec3 {
    Vec3::new(
        eased(from.x, to.x, phase, ease, motion),
        eased(from.y, to.y, phase, ease, motion),
        eased(from.z, to.z, phase, ease, motion),
    )
}

/// The particles that should be on screen at `phase`, with position and style.
///
/// Before the ease beat the previous frame shows; during it, the previous
/// frame's particles ease to the new ones (those that left stay where they
/// were, new ones stand at their place); after it, the new frame.
fn wanted<'a>(picture: &'a Picture, phase: Phase, ease: usize) -> Vec<(&'a str, Vec3, u16)> {
    let (previous, current) = (&picture.previous, &picture.current);
    let from_frame = |frame: &'a crate::frame::FieldFrame| {
        frame
            .particles
            .iter()
            .map(|p| (p.id.as_str(), p.at, p.style))
            .collect::<Vec<_>>()
    };
    match phase {
        Phase::Beat { index, .. } if index < ease => from_frame(&previous.frame),
        Phase::Beat { index, .. } if index == ease => {
            let mut out = Vec::new();
            for p in &previous.frame.particles {
                match current.particle(&p.id) {
                    Some(c) => out.push((
                        p.id.as_str(),
                        ease_point(p.at, c.at, phase, ease, picture.motion),
                        c.style,
                    )),
                    None => out.push((p.id.as_str(), p.at, p.style)),
                }
            }
            for c in &current.frame.particles {
                if previous.particle(&c.id).is_none() {
                    out.push((c.id.as_str(), c.at, c.style));
                }
            }
            out
        }
        _ => from_frame(&current.frame),
    }
}

/// Spawns, moves and despawns particle entities. Nothing while resting.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn animate_particles(
    mut commands: Commands,
    picture: Res<Picture>,
    config: Res<FieldConfig>,
    styles: Res<FieldStyles>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    assets: Res<FieldAssets>,
    mut entities: ResMut<ParticleEntities>,
    mut query: Query<(
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut rested: Local<bool>,
) {
    let phase = phase_now(&clock, &time, &config);
    let resting = phase == Phase::Resting;
    if resting && *rested {
        return;
    }
    *rested = resting;
    let wanted = wanted(&picture, phase, config.ease_beat);
    let mut keep: BTreeSet<&str> = BTreeSet::new();
    for (id, at, style) in &wanted {
        let Some(s) = styles.particles.get(usize::from(*style)) else {
            continue;
        };
        let (Some(mesh), Some(material)) = (
            assets.glyphs.get(&s.glyph),
            assets.swatches.get(usize::from(s.swatch)),
        ) else {
            continue;
        };
        keep.insert(id);
        let scale = Vec3::splat(s.size);
        match entities.by_id.get_mut(*id) {
            Some((entity, drawn_style)) => {
                if let Ok((mut transform, mut m, mut mat)) = query.get_mut(*entity) {
                    if transform.translation != *at {
                        transform.translation = *at;
                    }
                    if *drawn_style != *style {
                        transform.scale = scale;
                        m.0 = mesh.clone();
                        mat.0 = material.clone();
                        *drawn_style = *style;
                    }
                }
            }
            None => {
                let entity = commands
                    .spawn((
                        ParticleId((*id).to_owned()),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(*at).with_scale(scale),
                    ))
                    .id();
                entities.by_id.insert((*id).to_owned(), (entity, *style));
            }
        }
    }
    entities.by_id.retain(|id, (entity, _)| {
        let stays = keep.contains(id.as_str());
        if !stays {
            commands.entity(*entity).despawn();
        }
        stays
    });
}
