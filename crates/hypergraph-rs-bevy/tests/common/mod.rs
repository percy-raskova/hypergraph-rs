#![allow(dead_code, reason = "each test file uses only some of the helpers")]

use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use hypergraph_rs_bevy::{
    FieldConfig, FieldPlugin, FieldStyles, FieldSystems, FrameRefused, GroundPart, MarkBatch,
    MarkOf, Palette, ParticleId, PathLines, SetChanged, SetId,
};

/// What the plugin said, collected after it ran.
#[derive(Resource, Default)]
pub struct Collected {
    pub refused: Vec<FrameRefused>,
    pub changes: Vec<SetChanged>,
}

fn collect(
    mut refused: MessageReader<FrameRefused>,
    mut changes: MessageReader<SetChanged>,
    mut out: ResMut<Collected>,
) {
    out.refused.extend(refused.read().cloned());
    out.changes.extend(changes.read().cloned());
}

/// A headless app with the plugin: no window, no graphics card.
pub fn field_app(config: FieldConfig, styles: FieldStyles, palette: Palette) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            10,
        )))
        .init_resource::<Collected>()
        .add_plugins(FieldPlugin::new(config, styles, palette).expect("a valid plugin"))
        .add_systems(Update, collect.after(FieldSystems::Animate));
    app.finish();
    app.cleanup();
    app
}

/// Updates until `Time::elapsed()` reaches `at`.
pub fn advance_to(app: &mut App, at: Duration) {
    while app.world().resource::<Time>().elapsed() < at {
        app.update();
    }
}

fn push_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_bits().to_le_bytes());
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u64).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn push_mesh(out: &mut Vec<u8>, mesh: &Mesh) {
    if let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        out.extend_from_slice(&(p.len() as u64).to_le_bytes());
        for v in p {
            v.iter().for_each(|c| push_f32(out, *c));
        }
    }
    if let Some(VertexAttributeValues::Float32x4(c)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        out.extend_from_slice(&(c.len() as u64).to_le_bytes());
        for v in c {
            v.iter().for_each(|x| push_f32(out, *x));
        }
    }
    for (attribute, values) in mesh.attributes() {
        if !attribute.name.starts_with("Mark_") {
            continue;
        }
        match values {
            VertexAttributeValues::Float32x3(v) => {
                v.iter()
                    .for_each(|c| c.iter().for_each(|x| push_f32(out, *x)));
            }
            VertexAttributeValues::Float32x4(v) => {
                v.iter()
                    .for_each(|c| c.iter().for_each(|x| push_f32(out, *x)));
            }
            _ => {}
        }
    }
}

/// A byte record of every entity the plugin makes, sorted by its key, with
/// transforms, visibility, and the positions and colours of its meshes
/// (children's included). Entity ids are never written, so two apps compare.
pub fn snapshot(app: &mut App) -> Vec<u8> {
    let world = app.world_mut();
    let mut records: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut state = world.query::<(
        Entity,
        Option<&ParticleId>,
        Option<&SetId>,
        Option<&MarkOf>,
        Option<&PathLines>,
        Option<&GroundPart>,
        Option<&MarkBatch>,
    )>();
    let found: Vec<_> = state
        .iter(world)
        .filter(|(_, p, s, m, l, g, b)| {
            p.is_some() || s.is_some() || m.is_some() || l.is_some() || g.is_some() || b.is_some()
        })
        .map(|(e, p, s, m, l, g, b)| {
            (
                e,
                p.cloned(),
                s.cloned(),
                m.copied(),
                l.copied(),
                g.copied(),
                b.copied(),
            )
        })
        .collect();
    for (entity, p, s, m, l, g, b) in found {
        let mut key = Vec::new();
        if let Some(p) = p {
            key.push(0);
            push_str(&mut key, &p.0);
        }
        if let Some(s) = s {
            key.push(1);
            push_str(&mut key, &s.0);
        }
        if let Some(m) = m {
            key.push(2);
            key.extend_from_slice(&m.stream.to_le_bytes());
            key.extend_from_slice(&(m.flow as u64).to_be_bytes());
            key.extend_from_slice(&m.k.to_be_bytes());
        }
        if let Some(l) = l {
            key.push(3);
            key.extend_from_slice(&l.stream.to_le_bytes());
        }
        if let Some(g) = g {
            key.push(4);
            key.push(match g {
                GroundPart::Body => 0,
                GroundPart::Outline => 1,
            });
        }
        if let Some(b) = b {
            key.push(5);
            key.extend_from_slice(&b.stream.to_le_bytes());
        }
        let mut body = Vec::new();
        let mut family = vec![entity];
        if let Some(children) = world.get::<Children>(entity) {
            family.extend(children.iter());
        }
        for e in family {
            if let Some(t) = world.get::<Transform>(e) {
                t.translation
                    .to_array()
                    .iter()
                    .for_each(|c| push_f32(&mut body, *c));
                t.rotation
                    .to_array()
                    .iter()
                    .for_each(|c| push_f32(&mut body, *c));
                t.scale
                    .to_array()
                    .iter()
                    .for_each(|c| push_f32(&mut body, *c));
            }
            if let Some(v) = world.get::<Visibility>(e) {
                body.push(match v {
                    Visibility::Inherited => 0,
                    Visibility::Hidden => 1,
                    Visibility::Visible => 2,
                });
            }
            if let Some(handle) = world.get::<Mesh3d>(e) {
                if let Some(mesh) = world.resource::<Assets<Mesh>>().get(&handle.0) {
                    push_mesh(&mut body, mesh);
                }
            }
        }
        records.push((key, body));
    }
    records.sort();
    let mut out = Vec::new();
    for (k, b) in records {
        out.extend_from_slice(&(k.len() as u64).to_le_bytes());
        out.extend_from_slice(&k);
        out.extend_from_slice(&(b.len() as u64).to_le_bytes());
        out.extend_from_slice(&b);
    }
    out
}
