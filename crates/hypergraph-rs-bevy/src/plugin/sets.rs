//! Sets: one entity per id, with a line ribbon and, when the style hatches,
//! a hatch ribbon, swapped once a period at the end of the ease beat.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use super::{phase_now, FieldAssets, Picture, SetId};
use crate::config::FieldConfig;
use crate::frame::SetFrame;
use crate::lines::{dashed, hatch_segments, outline_paths, ribbon_mesh};
use crate::motion::{BeatClock, Phase};
use crate::style::{FieldStyles, SetStyle};

/// How far a hatch sits above its set's line, so the two never fight.
const HATCH_LIFT: f32 = 0.001;

/// The entity of each set on screen, the frame it was built from, and its
/// children.
#[derive(Resource, Default)]
pub(crate) struct SetEntities {
    by_id: BTreeMap<String, Drawn>,
}

struct Drawn {
    entity: Entity,
    built: SetFrame,
    children: Vec<Entity>,
}

/// The sets that should be on screen at `phase`.
///
/// Before the ease beat the previous frame shows; during it the previous
/// frame's sets stay as they were and new ones appear; after it, the new
/// frame. So a changed set swaps, and a vanished set goes, at the end of the
/// ease beat.
fn wanted(picture: &Picture, phase: Phase, ease: usize) -> Vec<&SetFrame> {
    let (previous, current) = (&picture.previous.frame, &picture.current.frame);
    match phase {
        Phase::Beat { index, .. } if index < ease => previous.sets.iter().collect(),
        Phase::Beat { index, .. } if index == ease => {
            let before: BTreeSet<&str> = previous.sets.iter().map(|s| s.id.as_str()).collect();
            previous
                .sets
                .iter()
                .chain(
                    current
                        .sets
                        .iter()
                        .filter(|s| !before.contains(s.id.as_str())),
                )
                .collect()
        }
        _ => current.sets.iter().collect(),
    }
}

/// The meshes of a set: its line and, when the style has a hatch, its hatch.
fn set_meshes(set: &SetFrame, style: &SetStyle, config: &FieldConfig) -> Vec<Mesh> {
    let Some(outline) = &set.outline else {
        return Vec::new();
    };
    let paths = match outline_paths(outline, config.curve_segments) {
        Ok(paths) => paths,
        Err(error) => {
            // `bevy_log` is not among this crate's Bevy features, so the
            // refusal goes to standard error.
            eprintln!("set {}: {error}; it draws nothing", set.id);
            return Vec::new();
        }
    };
    let dash = style
        .line
        .dash
        .map(|(on, off)| (f64::from(on), f64::from(off)));
    let line: Vec<_> = paths.iter().flat_map(|p| dashed(p, dash)).collect();
    let mut meshes = vec![ribbon_mesh(&line, style.line.width, set.height)];
    if let Some(hatch) = &style.hatch {
        let strokes = hatch_segments(&paths, hatch.direction, f64::from(hatch.spacing));
        meshes.push(ribbon_mesh(&strokes, hatch.width, set.height + HATCH_LIFT));
    }
    meshes
}

/// Spawns, rebuilds and despawns set entities. Nothing while resting.
#[allow(
    clippy::too_many_arguments,
    reason = "a Bevy system names every resource it reads"
)]
pub(super) fn animate_sets(
    mut commands: Commands,
    picture: Res<Picture>,
    config: Res<FieldConfig>,
    styles: Res<FieldStyles>,
    clock: Res<BeatClock>,
    time: Res<Time>,
    assets: Res<FieldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut entities: ResMut<SetEntities>,
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
    for set in wanted {
        let Some(style) = styles.sets.get(usize::from(set.style)) else {
            continue;
        };
        let Some(material) = assets.swatches.get(usize::from(style.swatch)) else {
            continue;
        };
        keep.insert(set.id.as_str());
        if entities
            .by_id
            .get(&set.id)
            .is_some_and(|drawn| drawn.built == *set)
        {
            continue;
        }
        let old = entities.by_id.remove(&set.id);
        let entity = match &old {
            Some(drawn) => {
                for child in &drawn.children {
                    commands.entity(*child).despawn();
                }
                drawn.entity
            }
            None => commands
                .spawn((
                    SetId(set.id.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id(),
        };
        let children: Vec<Entity> = set_meshes(set, style, &config)
            .into_iter()
            .map(|mesh| {
                commands
                    .spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(material.clone()),
                        ChildOf(entity),
                    ))
                    .id()
            })
            .collect();
        entities.by_id.insert(
            set.id.clone(),
            Drawn {
                entity,
                built: set.clone(),
                children,
            },
        );
    }
    entities.by_id.retain(|id, drawn| {
        let stays = keep.contains(id.as_str());
        if !stays {
            commands.entity(drawn.entity).despawn();
        }
        stays
    });
}
