// Marks of one stream, moved by a clock. Each vertex carries its mark's path
// (`from`, `to`) and timing (start, side, size, turned). The position is the
// same formula as `mark_position` in motion.rs; a vertex whose mark is outside
// its travel window is moved out of view.

import bevy_pbr::render::mesh_functions::{get_world_from_local, mesh_position_local_to_clip};
import bevy_pbr::render::mesh_view_bindings::view;
@if(TONEMAP_IN_SHADER) import bevy_core_pipeline::tonemapping::tone_mapping;

struct MarkParams {
    clock: f32,
    bow: f32,
    travel: f32,
    unused: f32,
    colour: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: MarkParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(8) p0: vec3<f32>,
    @location(9) p1: vec3<f32>,
    @location(10) timing: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

fn smoothstep_unit(x: f32) -> f32 {
    let c = clamp(x, 0.0, 1.0);
    return c * c * (3.0 - 2.0 * c);
}

fn mark_position(p0: vec3<f32>, p1: vec3<f32>, u: f32, bow: f32, side: f32) -> vec3<f32> {
    let s = smoothstep_unit(u);
    let delta = p1 - p0;
    let len = length(vec2<f32>(delta.x, delta.z));
    var perp = vec3<f32>(0.0, 0.0, 0.0);
    if len > 0.0 {
        perp = vec3<f32>(-delta.z, 0.0, delta.x) / len;
    }
    return p0 + delta * s + perp * (bow * len * side * 4.0 * u * (1.0 - u));
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Outside the clip volume: the triangle is dropped.
    out.clip_position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    let start = in.timing.x;
    let side = in.timing.y;
    let size = in.timing.z;
    let s = params.clock - start;
    if s >= 0.0 && s < params.travel {
        let u = s / params.travel;
        let at = mark_position(in.p0, in.p1, u, params.bow, side);
        var local = in.position * size;
        if in.timing.w > 0.5 {
            let behind = mark_position(
                in.p0,
                in.p1,
                max((s - params.travel / 50.0) / params.travel, 0.0),
                params.bow,
                side,
            );
            let dx = at.x - behind.x;
            let dz = at.z - behind.z;
            if dx != 0.0 || dz != 0.0 {
                let angle = atan2(-dz, dx);
                let c = cos(angle);
                let sn = sin(angle);
                local = vec3<f32>(
                    local.x * c + local.z * sn,
                    local.y,
                    -local.x * sn + local.z * c,
                );
            }
        }
        let world = at + local;
        out.clip_position = mesh_position_local_to_clip(
            get_world_from_local(in.instance_index),
            vec4<f32>(world, 1.0),
        );
    }
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var colour = vec4<f32>(params.colour.rgb, 1.0);
@if(TONEMAP_IN_SHADER)
    colour = tone_mapping(colour, view.color_grading);
    return colour;
}
