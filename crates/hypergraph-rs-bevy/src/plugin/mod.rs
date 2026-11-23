//! The plugin: it takes one [`FieldFrame`] per period as a message, checks
//! it, and plays the change in the beats of the [`FieldConfig`].
//!
//! The caller spawns one or more [`Ground`] entities once, then writes a
//! [`ShowPeriod`] each period. Systems that write these messages run
//! `.before(FieldSystems::Install)`; systems that read the picture run
//! `.after(FieldSystems::Animate)`.

mod ground;
mod mark_material;
mod marks;
mod particles;
mod sets;

use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::config::{FieldConfig, Motion};
use crate::error::FieldError;
use crate::flows::{select_flows, FlowReport};
use crate::frame::{diff_sets, FieldFrame, ParticleFrame, SetChange};
use crate::glyph::glyph_mesh;
use crate::ground::Ground;
use crate::motion::{BeatClock, Phase};
use crate::palette::Palette;
use crate::style::{FieldStyles, Glyph};
use mark_material::MarkMaterial;
use marks::MarkStores;

pub use mark_material::mark_batch_mesh;

/// The two sets of systems, both in `Update`, in this order.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldSystems {
    /// Materials, grounds, and the check and installation of a frame.
    Install,
    /// The picture's motion through the beats.
    Animate,
}

/// Show a period. The plugin checks the frame, and plays its change in the
/// beats.
///
/// If several arrive in one update, only the last is taken; the earlier ones
/// are dropped.
///
/// The first frame accepted has no earlier picture to ease from, so it
/// stands from its first beat; every later frame eases in the ease beat.
#[derive(Message, Clone, Debug)]
pub struct ShowPeriod(pub Box<FieldFrame>);

/// Play the last accepted change again, from the same previous picture.
///
/// With no frame accepted yet it does nothing. A [`ShowPeriod`] in the same
/// update wins, and the replay is dropped.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct ReplayPeriod;

/// A frame the plugin could not draw. The picture stays as it was.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct FrameRefused {
    /// The refused frame's period.
    pub period: u64,
    /// Why, naming the item at fault.
    pub error: FieldError,
}

/// One set's change, told when a frame is accepted.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct SetChanged(pub SetChange);

/// The id of a particle, on its entity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ParticleId(pub String);

/// The id of a set, on its entity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SetId(pub String);

/// Which mark this is: its stream, its flow's index in the frame, and its
/// number within the flow.
///
/// Marks moved by the shader are not entities and carry no `MarkOf`; see
/// [`MarkBatch`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkOf {
    /// The stream.
    pub stream: u16,
    /// The flow's index in the frame.
    pub flow: usize,
    /// The mark's number within its flow.
    pub k: u32,
}

/// The one entity that holds all of a stream's marks, moved by a clock in a
/// shader. It lives from the frame's acceptance until the beats rest, and
/// is shown only in its stream's beat. A stream with no marks has none.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkBatch {
    /// The stream.
    pub stream: u16,
}

/// The lines drawn for a stream's paths. A stream with no drawn flows still gets
/// one, with an empty mesh.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathLines {
    /// The stream.
    pub stream: u16,
}

/// Which mesh of a ground an entity holds.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundPart {
    /// The body: the surface or the blocks.
    Body,
    /// The region outlines.
    Outline,
}

/// Draws the field. See the module's text for how to feed it.
pub struct FieldPlugin {
    config: FieldConfig,
    styles: FieldStyles,
    palette: Palette,
}

impl FieldPlugin {
    /// Checks the config, then the styles against the palette and the config.
    ///
    /// # Errors
    ///
    /// The first fault of [`FieldConfig::validate`] or
    /// [`FieldStyles::validate`].
    pub fn new(
        config: FieldConfig,
        styles: FieldStyles,
        palette: Palette,
    ) -> Result<Self, FieldError> {
        config.validate()?;
        styles.validate(&palette, &config)?;
        Ok(Self {
            config,
            styles,
            palette,
        })
    }
}

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        mark_material::embed_shader(app);
        app.add_plugins(MaterialPlugin::<MarkMaterial>::default())
            .insert_resource(self.config.clone())
            .insert_resource(self.styles.clone())
            .insert_resource(self.palette.clone())
            .insert_resource(BeatClock::default())
            .insert_resource(FlowReport::default())
            .init_resource::<Picture>()
            .init_resource::<particles::ParticleEntities>()
            .init_resource::<sets::SetEntities>()
            .add_message::<ShowPeriod>()
            .add_message::<ReplayPeriod>()
            .add_message::<FrameRefused>()
            .add_message::<SetChanged>()
            .configure_sets(
                Update,
                (FieldSystems::Install, FieldSystems::Animate).chain(),
            )
            .add_systems(
                Update,
                (refresh_materials, ground::build_grounds, install_frame)
                    .chain()
                    .in_set(FieldSystems::Install),
            )
            .add_systems(
                Update,
                (
                    ground::animate_ground,
                    particles::animate_particles,
                    sets::animate_sets,
                    marks::animate_marks,
                )
                    .chain()
                    .in_set(FieldSystems::Animate),
            );
    }
}

/// The materials and glyph meshes every particle and mark shares.
#[derive(Resource)]
pub(crate) struct FieldAssets {
    pub swatches: Vec<Handle<StandardMaterial>>,
    pub ground: Handle<StandardMaterial>,
    pub glyphs: BTreeMap<Glyph, Handle<Mesh>>,
}

const GLYPHS: [Glyph; 5] = [
    Glyph::Disc,
    Glyph::Ring,
    Glyph::Square,
    Glyph::SquareFrame,
    Glyph::Streak,
];

fn swatch_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        cull_mode: None,
        double_sided: true,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    }
}

/// Creates the shared materials and meshes on the first run; afterwards, when
/// the palette changed, recolours the materials in place. Handles never change.
fn refresh_materials(
    mut commands: Commands,
    palette: Res<Palette>,
    assets: Option<ResMut<FieldAssets>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    match assets {
        None => {
            let swatches = palette
                .swatches
                .iter()
                .map(|c| materials.add(swatch_material(*c)))
                .collect();
            let ground = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.78,
                metallic: 0.08,
                cull_mode: None,
                double_sided: true,
                ..default()
            });
            let glyphs = GLYPHS
                .iter()
                .map(|g| (*g, meshes.add(glyph_mesh(*g, 16))))
                .collect();
            commands.insert_resource(FieldAssets {
                swatches,
                ground,
                glyphs,
            });
        }
        Some(mut assets) => {
            if !palette.is_changed() {
                return;
            }
            for (i, color) in palette.swatches.iter().enumerate() {
                match assets.swatches.get(i) {
                    Some(handle) => {
                        if let Some(mut m) = materials.get_mut(handle) {
                            m.base_color = *color;
                        }
                    }
                    None => assets.swatches.push(materials.add(swatch_material(*color))),
                }
            }
        }
    }
}

/// A frame with its regions and particles found by id.
#[derive(Clone, Default)]
pub(crate) struct Indexed {
    pub frame: FieldFrame,
    regions: BTreeMap<String, usize>,
    particles: BTreeMap<String, usize>,
}

impl Indexed {
    fn new(frame: FieldFrame) -> Self {
        let regions = frame
            .regions
            .iter()
            .enumerate()
            .map(|(i, r)| (r.region.clone(), i))
            .collect();
        let particles = frame
            .particles
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        Self {
            frame,
            regions,
            particles,
        }
    }

    /// A region's height and tone; a region with no reading stands at 0 with
    /// no tone.
    pub fn region(&self, id: &str) -> (f32, Option<f32>) {
        self.regions.get(id).map_or((0.0, None), |i| {
            let r = &self.frame.regions[*i];
            (r.height.unwrap_or(0.0), r.tone)
        })
    }

    pub fn particle(&self, id: &str) -> Option<&ParticleFrame> {
        self.particles.get(id).map(|i| &self.frame.particles[*i])
    }
}

/// The picture: the last settled frame, the one being played, and the
/// motion that was set when it was accepted.
#[derive(Resource, Default)]
pub(crate) struct Picture {
    pub previous: Indexed,
    pub current: Indexed,
    pub accepted: bool,
    pub motion: Motion,
}

pub(crate) fn phase_now(clock: &BeatClock, time: &Time, config: &FieldConfig) -> Phase {
    clock.phase(time.elapsed(), &config.beats)
}

fn check_regions(frame: &FieldFrame, grounds: &Query<&Ground>) -> Result<(), FieldError> {
    if frame.regions.is_empty() {
        return Ok(());
    }
    let known: BTreeSet<&str> = grounds
        .iter()
        .flat_map(|g| g.shapes.iter().map(|s| s.id.as_str()))
        .collect();
    match frame
        .regions
        .iter()
        .find(|r| !known.contains(r.region.as_str()))
    {
        Some(r) => Err(FieldError::UnknownRegion(r.region.clone())),
        None => Ok(()),
    }
}

/// Reads [`ShowPeriod`] and [`ReplayPeriod`], refuses or accepts, starts the
/// clock, writes the [`FlowReport`] and spawns the period's marks.
#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "a Bevy system names every resource and query it reads"
)]
fn install_frame(
    mut shows: MessageReader<ShowPeriod>,
    mut replays: MessageReader<ReplayPeriod>,
    mut refused: MessageWriter<FrameRefused>,
    mut changed: MessageWriter<SetChanged>,
    styles: Res<FieldStyles>,
    config: Res<FieldConfig>,
    time: Res<Time>,
    grounds: Query<&Ground>,
    assets: Res<FieldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mark_materials: ResMut<Assets<MarkMaterial>>,
    old_marks: Query<Entity, Or<(With<MarkBatch>, With<PathLines>)>>,
    mut commands: Commands,
    mut picture: ResMut<Picture>,
    mut clock: ResMut<BeatClock>,
    mut report: ResMut<FlowReport>,
) {
    let last = shows.read().last().map(|s| (*s.0).clone());
    let replay = replays.read().count() > 0;
    if let Some(frame) = last {
        let period = frame.period;
        let checked = frame
            .validate(&styles)
            .and_then(|()| check_regions(&frame, &grounds))
            .and_then(|()| select_flows(&frame.flows, &styles.streams));
        match checked {
            Err(error) => {
                refused.write(FrameRefused { period, error });
            }
            Ok((drawn, mut flow_report)) => {
                flow_report.period = Some(period);
                *report = flow_report;
                let settled = std::mem::take(&mut picture.current);
                for change in diff_sets(&settled.frame.sets, &frame.sets) {
                    changed.write(SetChanged(change));
                }
                for entity in &old_marks {
                    commands.entity(entity).despawn();
                }
                marks::spawn_marks(
                    &mut commands,
                    &mut MarkStores {
                        meshes: &mut meshes,
                        materials: &mut mark_materials,
                    },
                    &assets,
                    &styles,
                    &config,
                    &frame.flows,
                    &drawn,
                );
                // The first frame has no earlier picture to ease from: it is
                // its own previous one, so it stands from its first beat.
                // Particles, sets and the ground all read `previous`, so the
                // rule lives only here.
                let first = !picture.accepted;
                let indexed = Indexed::new(frame);
                picture.previous = if first { indexed.clone() } else { settled };
                picture.current = indexed;
                picture.accepted = true;
                picture.motion = config.motion;
                clock.start(time.elapsed(), period);
            }
        }
    } else if replay && picture.accepted {
        picture.motion = config.motion;
        if let Ok((drawn, mut flow_report)) =
            select_flows(&picture.current.frame.flows, &styles.streams)
        {
            flow_report.period = Some(picture.current.frame.period);
            *report = flow_report;
            for entity in &old_marks {
                commands.entity(entity).despawn();
            }
            marks::spawn_marks(
                &mut commands,
                &mut MarkStores {
                    meshes: &mut meshes,
                    materials: &mut mark_materials,
                },
                &assets,
                &styles,
                &config,
                &picture.current.frame.flows,
                &drawn,
            );
        }
        clock.start(time.elapsed(), picture.current.frame.period);
    }
}
