//! Marks of one stream as one mesh, moved by a clock in a vertex shader.
//!
//! Every mark's glyph vertices are copied into the batch, each vertex carrying
//! its mark's path and timing. The CPU writes one number a frame, the seconds
//! since the stream's beat began; the shader finds where each mark is.

use bevy::asset::{embedded_asset, RenderAssetUsages};
use bevy::mesh::{
    Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexFormat,
};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use std::collections::BTreeMap;

use crate::config::FieldConfig;
use crate::flows::{DrawnFlow, Flow};
use crate::glyph::glyph_mesh;
use crate::motion::mark_start;
use crate::style::{Glyph, StreamStyle};

/// Rim segments of the round glyphs in a batch.
pub(crate) const GLYPH_SEGMENTS: u32 = 16;

/// Where a mark leaves.
const MARK_FROM: MeshVertexAttribute =
    MeshVertexAttribute::new("Mark_From", 0x4d41_524b_0001, VertexFormat::Float32x3);
/// Where a mark arrives.
const MARK_TO: MeshVertexAttribute =
    MeshVertexAttribute::new("Mark_To", 0x4d41_524b_0002, VertexFormat::Float32x3);
/// Start in seconds into the beat, side (`+1` or `-1`), size, and `1` when the
/// glyph is turned along its motion (else `0`).
const MARK_TIMING: MeshVertexAttribute =
    MeshVertexAttribute::new("Mark_Timing", 0x4d41_524b_0003, VertexFormat::Float32x4);

const SHADER: &str = "embedded://hypergraph_rs_bevy/plugin/marks.wesl";

/// Registers the shader inside the crate.
pub(crate) fn embed_shader(app: &mut App) {
    embedded_asset!(app, "marks.wesl");
}

/// What the shader reads once a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub(crate) struct MarkParams {
    /// Seconds since the stream's beat began.
    pub clock: f32,
    /// How far a path bows sideways, as a share of its length.
    pub bow: f32,
    /// How long one mark travels, in seconds.
    pub travel: f32,
    pub unused: f32,
    /// The stream's colour, linear.
    pub colour: Vec4,
}

/// The material of one stream's batch.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct MarkMaterial {
    #[uniform(0)]
    pub params: MarkParams,
}

impl Material for MarkMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            MARK_FROM.at_shader_location(8),
            MARK_TO.at_shader_location(9),
            MARK_TIMING.at_shader_location(10),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// One glyph's vertices and triangles, read once.
struct GlyphData {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn glyph_data(glyph: Glyph) -> GlyphData {
    let mesh = glyph_mesh(glyph, GLYPH_SEGMENTS);
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.clone(),
        other => panic!("glyph {glyph:?} has no Float32x3 positions: {other:?}"),
    };
    let indices = mesh
        .indices()
        .unwrap_or_else(|| panic!("glyph {glyph:?} has no indices"))
        .iter()
        .map(|v| v as u32)
        .collect();
    GlyphData { positions, indices }
}

/// The marks of one stream as one mesh.
///
/// Every mark of every drawn flow of `stream`, in the order of `drawn`, has its
/// stream's full or hollow glyph copied in, unscaled and centred on the origin.
/// Each vertex carries its mark's `from` and `to` and its timing: the mark's
/// start inside the beat in seconds (the same number as
/// [`mark_start`](crate::mark_start)), its side (`+1` for even `k`, `-1` for
/// odd), its size, and whether it is a streak, turned along its motion. A
/// stream that does not exist, or has no marks, gives an empty mesh.
#[must_use]
pub fn mark_batch_mesh(
    flows: &[Flow],
    drawn: &[DrawnFlow],
    styles: &[StreamStyle],
    config: &FieldConfig,
    stream: u16,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut from: Vec<[f32; 3]> = Vec::new();
    let mut to: Vec<[f32; 3]> = Vec::new();
    let mut timing: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    if let Some(style) = styles.get(usize::from(stream)) {
        let mut glyphs: BTreeMap<Glyph, GlyphData> = BTreeMap::new();
        for glyph in [style.full, style.hollow] {
            glyphs.entry(glyph).or_insert_with(|| glyph_data(glyph));
        }
        let beat = config.beats.get(style.beat).copied().unwrap_or_default();
        for d in drawn.iter().filter(|d| d.stream == stream) {
            let glyph = if d.hollow { style.hollow } else { style.full };
            let (Some(data), Some(flow)) = (glyphs.get(&glyph), flows.get(d.flow)) else {
                continue;
            };
            let turned = if glyph == Glyph::Streak { 1.0 } else { 0.0 };
            for k in 0..d.marks {
                let start = mark_start(d.flow, k, d.marks, beat, config.mark_travel).as_secs_f32();
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                let base =
                    u32::try_from(positions.len()).expect("a batch holds fewer than 2^32 vertices");
                for p in &data.positions {
                    positions.push(*p);
                    from.push(flow.from.to_array());
                    to.push(flow.to.to_array());
                    timing.push([start, side, style.size, turned]);
                }
                indices.extend(data.indices.iter().map(|i| base + i));
            }
        }
    }
    let normals = vec![[0.0_f32, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(MARK_FROM, from);
    mesh.insert_attribute(MARK_TO, to);
    mesh.insert_attribute(MARK_TIMING, timing);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
