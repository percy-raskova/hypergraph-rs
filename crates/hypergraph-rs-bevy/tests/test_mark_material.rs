use std::num::NonZeroU64;
use std::time::Duration;

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use hypergraph_rs_bevy::{
    glyph_mesh, mark_batch_mesh, mark_start, select_flows, FieldConfig, Flow, Glyph, StreamStyle,
};

fn stream(beat: usize, unit: u64, full: Glyph, hollow: Glyph, swatch: u16) -> StreamStyle {
    StreamStyle {
        beat,
        unit: NonZeroU64::new(unit).unwrap(),
        full,
        hollow,
        size: 0.2,
        swatch,
        share_drawn: 1.0,
        max_marks: 1000,
    }
}

fn streams() -> Vec<StreamStyle> {
    vec![
        stream(0, 1, Glyph::Disc, Glyph::Ring, 0),
        stream(1, 1, Glyph::Square, Glyph::SquareFrame, 1),
        stream(2, 100, Glyph::Streak, Glyph::Streak, 2),
    ]
}

fn config() -> FieldConfig {
    FieldConfig::three_beats(Duration::from_secs(1))
}

fn flow(stream: u16, amount: u64, from: Vec3, to: Vec3) -> Flow {
    Flow {
        stream,
        from,
        to,
        amount,
        hollow: false,
    }
}

fn vec3s(mesh: &Mesh, name: &str) -> Vec<[f32; 3]> {
    match mesh.attributes().find(|(a, _)| a.name == name) {
        Some((_, VertexAttributeValues::Float32x3(v))) => v.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

fn vec4s(mesh: &Mesh, name: &str) -> Vec<[f32; 4]> {
    match mesh.attributes().find(|(a, _)| a.name == name) {
        Some((_, VertexAttributeValues::Float32x4(v))) => v.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn the_batch_holds_every_mark_once() {
    let flows = vec![
        flow(0, 6, Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)),
        flow(1, 4, Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)),
        flow(2, 300, Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)),
    ];
    let styles = streams();
    let (drawn, _) = select_flows(&flows, &styles).unwrap();
    let disc = glyph_mesh(Glyph::Disc, 16).count_vertices();
    let batch = mark_batch_mesh(&flows, &drawn, &styles, &config(), 0);
    assert_eq!(batch.count_vertices(), 6 * disc);
    let square = glyph_mesh(Glyph::Square, 16).count_vertices();
    let batch = mark_batch_mesh(&flows, &drawn, &styles, &config(), 1);
    assert_eq!(batch.count_vertices(), 4 * square);
    let streak = glyph_mesh(Glyph::Streak, 16).count_vertices();
    let batch = mark_batch_mesh(&flows, &drawn, &styles, &config(), 2);
    assert_eq!(batch.count_vertices(), 3 * streak);
}

#[test]
fn each_vertex_carries_its_marks_path_and_start() {
    let (a, b) = (Vec3::new(1.0, 0.5, -2.0), Vec3::new(4.0, 0.25, -6.0));
    let (c, d) = (Vec3::new(-3.0, 0.0, 1.0), Vec3::new(2.0, 1.0, 7.0));
    let flows = vec![
        flow(1, 9, Vec3::ZERO, Vec3::X),
        flow(0, 5, a, b),
        flow(0, 3, c, d),
    ];
    let styles = streams();
    let cfg = config();
    let (drawn, _) = select_flows(&flows, &styles).unwrap();
    let batch = mark_batch_mesh(&flows, &drawn, &styles, &cfg, 0);
    let disc = glyph_mesh(Glyph::Disc, 16).count_vertices();
    let from = vec3s(&batch, "Mark_From");
    let to = vec3s(&batch, "Mark_To");
    let timing = vec4s(&batch, "Mark_Timing");
    assert_eq!(from.len(), 8 * disc);
    let mut at = 0;
    for (f, n, ends) in [(1_usize, 5_u32, (a, b)), (2, 3, (c, d))] {
        for k in 0..n {
            let start = mark_start(f, k, n, cfg.beats[0], cfg.mark_travel).as_secs_f32();
            let side = if k % 2 == 0 { 1.0_f32 } else { -1.0 };
            for _ in 0..disc {
                assert_eq!(from[at], ends.0.to_array());
                assert_eq!(to[at], ends.1.to_array());
                assert_eq!(
                    timing[at][0].to_bits(),
                    start.to_bits(),
                    "flow {f} mark {k}"
                );
                assert_eq!(timing[at][1], side, "flow {f} mark {k}");
                assert_eq!(timing[at][2], 0.2);
                assert_eq!(timing[at][3], 0.0, "a disc is not turned along its motion");
                at += 1;
            }
        }
    }
    assert_eq!(at, from.len());
}

#[test]
fn a_stream_without_marks_gives_an_empty_batch() {
    let flows = vec![flow(0, 6, Vec3::ZERO, Vec3::X)];
    let styles = streams();
    let (drawn, _) = select_flows(&flows, &styles).unwrap();
    assert_eq!(
        mark_batch_mesh(&flows, &drawn, &styles, &config(), 1).count_vertices(),
        0
    );
    assert_eq!(
        mark_batch_mesh(&flows, &drawn, &styles, &config(), 9).count_vertices(),
        0
    );
}

#[test]
fn a_streak_is_told_to_turn_along_its_motion() {
    let flows = vec![flow(2, 100, Vec3::ZERO, Vec3::X)];
    let styles = streams();
    let (drawn, _) = select_flows(&flows, &styles).unwrap();
    let batch = mark_batch_mesh(&flows, &drawn, &styles, &config(), 2);
    let timing = vec4s(&batch, "Mark_Timing");
    assert_eq!(timing.len(), glyph_mesh(Glyph::Streak, 16).count_vertices());
    assert!(timing.iter().all(|t| t[3] == 1.0));
}
