//! An invented world for the example and its test: a grid of square regions,
//! particles round their centres, sets of particles read from a hypergraph,
//! and three streams of flows. Nothing is random: one fixed generator.

use std::num::NonZeroU64;
use std::sync::OnceLock;
use std::time::Duration;

use bevy::prelude::Color;
use hypergraph_rs::geometry::{OutlineParams, Point, Positions};
use hypergraph_rs::Hypergraph;
use hypergraph_rs_bevy::{
    plan_to_scene, set_frames, FieldConfig, FieldFrame, FieldStyles, Flow, Glyph, Hatch,
    HatchDirection, LineStyle, Palette, ParticleFrame, ParticleStyle, RegionReading, RegionShape,
    SetFrame, SetStyle, StreamStyle,
};

const COLUMNS: usize = 72;
const ROWS: usize = 44;
const REGIONS: usize = COLUMNS * ROWS;
const PARTICLES: usize = 25_000;
const PER_REGION: usize = 8;
const SETS: usize = 6;
const SET_SIZE: usize = 2_000;
const FLOWS_PER_STREAM: usize = 3_000;
const OFFSETS: [(f64, f64); PER_REGION] = [
    (0.3, 0.0),
    (-0.3, 0.0),
    (0.0, 0.3),
    (0.0, -0.3),
    (0.2121, 0.2121),
    (0.2121, -0.2121),
    (-0.2121, 0.2121),
    (-0.2121, -0.2121),
];

/// A fixed linear congruential generator.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        let mut g = Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
        g.unit();
        g
    }

    /// The next number in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }

    /// The next whole number below `n`.
    fn below(&mut self, n: usize) -> usize {
        ((self.unit() * n as f64) as usize).min(n - 1)
    }
}

fn centre(region: usize) -> Point {
    (
        (region % COLUMNS) as f64 + 0.5,
        (region / COLUMNS) as f64 + 0.5,
    )
}

/// The beats, the styles and the palette of the example.
pub fn styles() -> (FieldConfig, FieldStyles, Palette) {
    let config = FieldConfig::three_beats(Duration::from_millis(1200));
    let palette = Palette {
        ramp: [
            Color::srgb(0.18, 0.28, 0.45),
            Color::srgb(0.62, 0.60, 0.55),
            Color::srgb(0.70, 0.22, 0.20),
        ],
        no_reading: Color::srgb(0.25, 0.25, 0.25),
        swatches: vec![
            Color::srgb(0.95, 0.93, 0.85),
            Color::srgb(0.95, 0.80, 0.25),
            Color::srgb(0.35, 0.80, 0.75),
            Color::srgb(0.85, 0.45, 0.80),
            Color::srgb(0.95, 0.60, 0.20),
            Color::srgb(0.40, 0.85, 0.40),
            Color::srgb(0.90, 0.90, 0.30),
            Color::srgb(0.10, 0.10, 0.10),
        ],
    };
    let direction = [
        HatchDirection::Deg0,
        HatchDirection::Deg45,
        HatchDirection::Deg90,
    ];
    let sets = (0..3)
        .map(|i| SetStyle {
            swatch: 1 + i as u16,
            line: LineStyle {
                width: 0.06,
                dash: (i == 1).then_some((0.3, 0.15)),
            },
            hatch: Some(Hatch {
                direction: direction[i],
                spacing: 0.25,
                width: 0.03,
            }),
        })
        .collect();
    let stream = |beat: usize, full: Glyph, hollow: Glyph, swatch: u16| StreamStyle {
        beat,
        unit: NonZeroU64::MIN,
        full,
        hollow,
        size: 0.18,
        swatch,
        share_drawn: 0.95,
        max_marks: 30_000,
    };
    let styles = FieldStyles {
        particles: vec![ParticleStyle {
            glyph: Glyph::Disc,
            size: 0.1,
            swatch: 0,
        }],
        sets,
        streams: vec![
            stream(0, Glyph::Disc, Glyph::Ring, 4),
            stream(1, Glyph::Square, Glyph::SquareFrame, 5),
            stream(2, Glyph::Streak, Glyph::Streak, 6),
        ],
        region_outline: Some(7),
    };
    (config, styles, palette)
}

/// The 3,168 square regions, two triangles each.
pub fn regions() -> Vec<RegionShape> {
    (0..REGIONS)
        .map(|i| {
            let (x, y) = ((i % COLUMNS) as f64, (i / COLUMNS) as f64);
            let (a, b, c, d) = ((x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0));
            RegionShape {
                id: format!("r{i:04}"),
                triangles: vec![[a, b, c], [a, c, d]],
                rings: vec![vec![a, b, c, d]],
            }
        })
        .collect()
}

/// What never changes between periods: the particles and the sets.
fn still() -> &'static (Vec<ParticleFrame>, Vec<SetFrame>) {
    static STILL: OnceLock<(Vec<ParticleFrame>, Vec<SetFrame>)> = OnceLock::new();
    STILL.get_or_init(|| {
        let mut plan: Vec<(String, Point)> = Vec::with_capacity(PARTICLES);
        for i in 0..PARTICLES {
            let (cx, cy) = centre(i / PER_REGION);
            let (dx, dy) = OFFSETS[i % PER_REGION];
            plan.push((format!("p{i:05}"), (cx + dx, cy + dy)));
        }
        let particles = plan
            .iter()
            .map(|(id, p)| ParticleFrame {
                id: id.clone(),
                at: plan_to_scene(*p, 0.1),
                style: 0,
            })
            .collect();
        let mut graph: Hypergraph<(), (), f64> = Hypergraph::new();
        let mut names = Vec::with_capacity(SETS);
        for s in 0..SETS {
            let edge = format!("s{s}");
            let first = s * (PARTICLES / SETS);
            for j in 0..SET_SIZE {
                let weight = (j % 5 + 1) as f64 / 5.0;
                graph
                    .add_node_to_edge_with_data(&edge, &plan[first + j].0, weight)
                    .expect("a fresh membership");
            }
            names.push((edge, (s % 3) as u16));
        }
        let listed: Vec<(&str, u16)> = names.iter().map(|(e, k)| (e.as_str(), *k)).collect();
        let positions: Positions = plan.iter().cloned().collect();
        let sets = set_frames(
            &graph,
            &listed,
            &positions,
            |w| *w,
            &OutlineParams::new(1.5, 0.25, 0.9),
            0.2,
        )
        .expect("the invented sets have outlines");
        (particles, sets)
    })
}

/// The frame of a period: the same particles and sets, readings and flows
/// from a generator seeded by the period.
pub fn frame(period: u64) -> FieldFrame {
    let (particles, sets) = still();
    let mut readings = Lcg::new(period);
    let regions = (0..REGIONS)
        .map(|i| {
            let height = -3.0 + 9.0 * readings.unit() as f32;
            let tone = -1.0 + 2.0 * readings.unit() as f32;
            let read = i % 50 != 0;
            RegionReading {
                region: format!("r{i:04}"),
                height: read.then_some(height),
                tone: read.then_some(tone),
            }
        })
        .collect();
    let mut flows = Vec::with_capacity(3 * FLOWS_PER_STREAM);
    for stream in 0..3_u16 {
        let mut g = Lcg::new(period * 16 + u64::from(stream) + 1);
        for j in 0..FLOWS_PER_STREAM {
            let a = g.below(REGIONS);
            let b = (a + 1 + g.below(REGIONS - 1)) % REGIONS;
            flows.push(Flow {
                stream,
                from: plan_to_scene(centre(a), 0.3),
                to: plan_to_scene(centre(b), 0.3),
                amount: 1 + g.below(20) as u64,
                hollow: j % 10 == 9,
            });
        }
    }
    FieldFrame {
        period,
        regions,
        particles: particles.clone(),
        sets: sets.clone(),
        flows,
    }
}
