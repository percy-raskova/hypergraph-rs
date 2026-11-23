//! An invented world for the example and its test: a grid of square regions
//! under soft hills, particles seated on the ground, sets of particles read
//! from a hypergraph, and three streams of flows. Nothing is random: one
//! fixed generator. It comes in two sizes: a small world that a person can
//! read, and a large one for the measured run.
//!
//! The hills are kept low on purpose. A set's outline and a mark's path are
//! flat, so they sit a little above the ground and do not follow it. The
//! library has no query for the ground's height at a point, so the example
//! seats things by its own reading between region centres: close on a bending
//! surface, but in the blocks form a particle beside a step can sink into its
//! block.

use std::collections::BTreeMap;
use std::f64::consts::TAU;
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

/// Particles to a region in the large world.
const PER_REGION: usize = 8;
/// How far a particle sits above the ground.
const DOT_LIFT: f32 = 0.1;
/// How far the ends of a flow sit above the ground.
const MARK_LIFT: f32 = 0.3;
/// How far a set's outline sits above the highest ground under its members.
const SET_LIFT: f32 = 0.16;
/// The style of a post: after the four colours in three sizes.
const POST_STYLE: u16 = 12;

/// Which of the two invented worlds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// 1,728 regions, 240 particles (nine of them posts) and five sets: a
    /// world a person can read.
    Small,
    /// 3,168 regions, 25,000 particles (forty of them posts) and six sets:
    /// the measured world. Eight particles stand in each region but the last
    /// 48, which have none.
    Large,
}

/// The numbers that make a world of one size.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// Regions across.
    pub columns: usize,
    /// Regions deep.
    pub rows: usize,
    /// Particles, the posts among them.
    pub particles: usize,
    /// Places stand on a coarse lattice of this many columns and rows.
    lattice: (usize, usize),
    /// Flows in each stream, each period.
    flows: usize,
    /// The tallest hill, in scene units.
    relief: f32,
    /// How far a hill spreads, in regions.
    reach: f64,
    /// The size of the smallest particle.
    dot: f32,
    /// The size of a post's ring.
    post: f32,
    /// The size of a mark.
    mark: f32,
    /// The most marks one stream draws.
    marks: u32,
    /// The width of a set's line.
    line: f32,
    /// How far a member's own hill reaches in a set's outline.
    member_reach: f64,
    /// The largest amount of one flow.
    most: usize,
    /// How far inside the grid's edge every particle stays.
    margin: f64,
}

impl Size {
    /// The numbers of this size.
    pub const fn shape(self) -> Shape {
        match self {
            Self::Small => Shape {
                columns: 48,
                rows: 36,
                particles: 240,
                lattice: (3, 3),
                flows: 48,
                relief: 1.2,
                reach: 4.0,
                dot: 0.2,
                post: 0.65,
                mark: 0.12,
                marks: 2_000,
                line: 0.07,
                member_reach: 2.4,
                most: 8,
                margin: 2.6,
            },
            Self::Large => Shape {
                columns: 72,
                rows: 44,
                particles: 25_000,
                lattice: (8, 5),
                flows: 3_000,
                relief: 1.6,
                reach: 4.2,
                dot: 0.07,
                post: 0.8,
                mark: 0.08,
                marks: 30_000,
                line: 0.09,
                member_reach: 1.5,
                most: 20,
                margin: 0.3,
            },
        }
    }

    /// The sets of this size. Each gathers round some places and holds, of
    /// the particles nearest to them, every one or every second, up to a
    /// count; the last number is its style. In the small world a set holds
    /// only particles that belong to its places, so two sets can share a place.
    const fn sets(self) -> &'static [(&'static [usize], usize, usize, u16)] {
        match self {
            Self::Small => &[
                (&[0], 1, 240, 0),
                (&[4, 5], 1, 240, 1),
                (&[4], 2, 240, 2),
                (&[8], 1, 240, 0),
                (&[7, 8], 2, 240, 2),
            ],
            Self::Large => &[
                (&[3], 1, 2_000, 0),
                (&[11], 1, 2_000, 1),
                (&[17], 1, 2_000, 2),
                (&[22], 1, 2_000, 0),
                (&[30], 1, 2_000, 1),
                (&[36], 1, 2_000, 2),
            ],
        }
    }
}

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

/// Where a hill or a hollow stands, how far it spreads, how high it lifts the
/// ground and which way it tints it.
struct Place {
    at: Point,
    reach: f64,
    lift: f32,
    tone: f32,
}

/// What never changes between periods.
struct Still {
    places: Vec<Place>,
    /// Each particle: its id, its point in the plan and its style. The posts
    /// come last, one a place.
    particles: Vec<(String, Point, u16)>,
    sets: Vec<SetFrame>,
}

fn centre(shape: Shape, region: usize) -> Point {
    (
        (region % shape.columns) as f64 + 0.5,
        (region / shape.columns) as f64 + 0.5,
    )
}

fn region_id(region: usize) -> String {
    format!("r{region:04}")
}

/// A block of regions in one corner has no reading: it is flat and grey, and
/// the ground steps down to it from whatever hill stands beside it.
fn is_read(shape: Shape, region: usize) -> bool {
    let (column, row) = (region % shape.columns, region / shape.columns);
    !(column >= shape.columns - shape.columns / 8 && row < shape.rows / 7)
}

fn distance_squared(a: Point, b: Point) -> f64 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

/// How much of a place is felt at a point, from one at its middle down to
/// nothing three reaches away.
fn felt(place: &Place, p: Point, spread: f64) -> f32 {
    let reach = place.reach * spread;
    let d2 = distance_squared(place.at, p);
    if d2 > 9.0 * reach * reach {
        0.0
    } else {
        (-d2 / (2.0 * reach * reach)).exp() as f32
    }
}

/// How full a place stands in a period, between 0.7 and 1.
fn swell(place: usize, period: u64) -> f32 {
    0.85 + 0.15 * (0.9 * period as f32 + 2.4 * place as f32).sin()
}

/// The ground's height at a point in a period.
fn height_at(places: &[Place], p: Point, period: u64) -> f32 {
    places
        .iter()
        .enumerate()
        .map(|(k, place)| place.lift * swell(k, period) * felt(place, p, 1.0))
        .sum()
}

/// The highest the ground ever stands at a point.
fn ceiling_at(places: &[Place], p: Point) -> f32 {
    places
        .iter()
        .map(|place| place.lift.max(0.0) * felt(place, p, 1.0))
        .sum()
}

/// The tone at a point: in a period, or with every place at its fullest.
fn tone_at(places: &[Place], p: Point, period: Option<u64>) -> f32 {
    places
        .iter()
        .enumerate()
        .map(|(k, place)| {
            place.tone * period.map_or(1.0, |t| swell(k + 7, t)) * felt(place, p, 1.5)
        })
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

/// The ground's height at a point, between the four nearest region centres.
fn seat(shape: Shape, heights: &[f32], p: Point) -> f32 {
    let fx = (p.0 - 0.5).clamp(0.0, (shape.columns - 1) as f64);
    let fy = (p.1 - 0.5).clamp(0.0, (shape.rows - 1) as f64);
    let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
    let (x1, y1) = (
        (x0 + 1).min(shape.columns - 1),
        (y0 + 1).min(shape.rows - 1),
    );
    let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
    let h = |x: usize, y: usize| heights[y * shape.columns + x];
    let near = h(x0, y0) * (1.0 - tx) + h(x1, y0) * tx;
    let far = h(x0, y1) * (1.0 - tx) + h(x1, y1) * tx;
    near * (1.0 - ty) + far * ty
}

/// The places: one in each cell of the lattice, a little off its middle.
/// Every fourth is a hollow. Tones lean from one side of the grid to the other.
fn places(shape: Shape) -> Vec<Place> {
    let (across, deep) = shape.lattice;
    let cell = (
        shape.columns as f64 / across as f64,
        shape.rows as f64 / deep as f64,
    );
    let mut g = Lcg::new(0x51CE);
    (0..across * deep)
        .map(|k| {
            let at = (
                ((k % across) as f64 + 0.5 + 0.4 * (g.unit() - 0.5)) * cell.0,
                ((k / across) as f64 + 0.5 + 0.4 * (g.unit() - 0.5)) * cell.1,
            );
            let reach = shape.reach * (0.8 + 0.5 * g.unit());
            let tall = 0.45 + 0.55 * g.unit() as f32;
            let lift = if k % 4 == 3 {
                -0.45 * shape.relief * tall
            } else {
                shape.relief * tall
            };
            let lean = 2.0 * at.0 / shape.columns as f64 - 1.0;
            let tone = (0.9 * lean + 0.7 * (g.unit() - 0.5)).clamp(-1.0, 1.0) as f32;
            Place {
                at,
                reach,
                lift,
                tone,
            }
        })
        .collect()
}

/// The particles and the sets of a size.
fn build(size: Size) -> Still {
    let shape = size.shape();
    let places = places(shape);
    let ordinary = shape.particles - places.len();
    let (wide, deep) = (shape.columns as f64, shape.rows as f64);
    let mut g = Lcg::new(0xD075);
    let mut particles: Vec<(String, Point, u16)> = Vec::with_capacity(shape.particles);
    // The place a particle belongs to, in the small world.
    let mut homes: Vec<Option<usize>> = Vec::with_capacity(ordinary);
    for i in 0..ordinary {
        let (p, home) = match size {
            // Eight to a region, scattered inside it.
            Size::Large => {
                let (cx, cy) = centre(shape, i / PER_REGION);
                (
                    (cx + 0.84 * (g.unit() - 0.5), cy + 0.84 * (g.unit() - 0.5)),
                    None,
                )
            }
            // Gathered round the places, thicker in the middle; one in ten
            // stands anywhere and belongs to no place.
            Size::Small => {
                let k = g.below(places.len());
                let (angle, far) = (TAU * g.unit(), 0.95 * places[k].reach * g.unit());
                if i % 10 == 9 {
                    ((wide * g.unit(), deep * g.unit()), None)
                } else {
                    (
                        (
                            places[k].at.0 + far * angle.cos(),
                            places[k].at.1 + far * angle.sin(),
                        ),
                        Some(k),
                    )
                }
            }
        };
        homes.push(home);
        let p = (
            p.0.clamp(shape.margin, wide - shape.margin),
            p.1.clamp(shape.margin, deep - shape.margin),
        );
        let warm = (tone_at(&places, p, None) + 0.8 * (g.unit() as f32 - 0.5)).clamp(-1.0, 0.999);
        let colour = ((warm + 1.0) * 2.0) as u16;
        let big = match g.below(10) {
            0 => 2,
            1..=3 => 1,
            _ => 0,
        };
        particles.push((format!("p{i:05}"), p, colour * 3 + big));
    }

    let mut graph: Hypergraph<(), (), f64> = Hypergraph::new();
    let mut listed: Vec<(String, u16)> = Vec::new();
    let mut tops: BTreeMap<String, f32> = BTreeMap::new();
    for (s, (round, every, count, style)) in size.sets().iter().enumerate() {
        let mut nearest: Vec<(f64, usize)> = (0..ordinary)
            .filter(|i| size == Size::Large || homes[*i].is_some_and(|k| round.contains(&k)))
            .map(|i| {
                let d2 = round
                    .iter()
                    .map(|k| distance_squared(particles[i].1, places[*k].at))
                    .fold(f64::INFINITY, f64::min);
                (d2, i)
            })
            .collect();
        nearest.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let edge = format!("s{s}");
        let mut top = f32::MIN;
        let members = nearest.iter().step_by(*every).take(*count);
        for (rank, (_, i)) in members.enumerate() {
            let weight = 0.6 + 0.1 * (rank % 5) as f64;
            graph
                .add_node_to_edge_with_data(&edge, &particles[*i].0, weight)
                .expect("a fresh membership");
            top = top.max(ceiling_at(&places, particles[*i].1));
        }
        tops.insert(edge.clone(), top);
        listed.push((edge, *style));
    }
    let named: Vec<(&str, u16)> = listed.iter().map(|(e, k)| (e.as_str(), *k)).collect();
    let positions: Positions = particles
        .iter()
        .map(|(id, p, _)| (id.clone(), *p))
        .collect();
    let mut sets = set_frames(
        &graph,
        &named,
        &positions,
        |w| *w,
        &OutlineParams::new(shape.member_reach, 0.25, 0.9),
        0.0,
    )
    .expect("every invented set is in the graph and its members have points");
    for set in &mut sets {
        set.height = tops.get(&set.id).copied().unwrap_or(0.0) + SET_LIFT;
    }

    for (k, place) in places.iter().enumerate() {
        particles.push((format!("post{k:02}"), place.at, POST_STYLE));
    }
    Still {
        places,
        particles,
        sets,
    }
}

fn still(size: Size) -> &'static Still {
    static SMALL: OnceLock<Still> = OnceLock::new();
    static LARGE: OnceLock<Still> = OnceLock::new();
    match size {
        Size::Small => SMALL.get_or_init(|| build(size)),
        Size::Large => LARGE.get_or_init(|| build(size)),
    }
}

/// The beats, the styles and the palette of a size.
pub fn styles(size: Size) -> (FieldConfig, FieldStyles, Palette) {
    let shape = size.shape();
    let config = FieldConfig::three_beats(Duration::from_millis(1200));
    // Muted fills for the ground; the full colours are kept for what moves.
    let palette = Palette {
        ramp: [
            Color::srgb_u8(0x8a, 0x17, 0x30),
            Color::srgb_u8(0x2a, 0x15, 0x19),
            Color::srgb_u8(0x9a, 0x76, 0x18),
        ],
        no_reading: Color::srgb_u8(0x2e, 0x2a, 0x2b),
        swatches: vec![
            Color::srgb_u8(0xe8, 0xe8, 0xe8),
            Color::srgb_u8(0xdc, 0x14, 0x3c),
            Color::srgb_u8(0xec, 0x5a, 0x2b),
            Color::srgb_u8(0xf5, 0x8a, 0x1f),
            Color::srgb_u8(0xff, 0xd7, 0x00),
            Color::srgb_u8(0x6a, 0x34, 0x3b),
            Color::srgb_u8(0xa8, 0x99, 0x9a),
        ],
    };
    let mut particles = Vec::with_capacity(usize::from(POST_STYLE) + 1);
    for swatch in 1..=4 {
        for scale in [1.0, 1.4, 1.9] {
            particles.push(ParticleStyle {
                glyph: Glyph::Disc,
                size: shape.dot * scale,
                swatch,
            });
        }
    }
    particles.push(ParticleStyle {
        glyph: Glyph::Ring,
        size: shape.post,
        swatch: 4,
    });
    let set = |swatch: u16, dash: Option<(f32, f32)>, direction: HatchDirection| SetStyle {
        swatch,
        line: LineStyle {
            width: shape.line,
            dash,
        },
        hatch: Some(Hatch {
            direction,
            spacing: 5.5 * shape.line,
            width: 0.45 * shape.line,
        }),
    };
    // A streak is a thin line, so it is drawn three times the size of the rest.
    let stream = |beat: usize, full: Glyph, hollow: Glyph, swatch: u16| StreamStyle {
        beat,
        unit: NonZeroU64::MIN,
        full,
        hollow,
        size: if full == Glyph::Streak {
            3.0 * shape.mark
        } else {
            shape.mark
        },
        swatch,
        share_drawn: 0.95,
        max_marks: shape.marks,
    };
    let styles = FieldStyles {
        particles,
        sets: vec![
            set(6, None, HatchDirection::Deg45),
            set(
                4,
                Some((6.0 * shape.line, 3.0 * shape.line)),
                HatchDirection::Deg135,
            ),
            set(1, None, HatchDirection::Deg0),
        ],
        streams: vec![
            stream(0, Glyph::Disc, Glyph::Ring, 0),
            stream(1, Glyph::Square, Glyph::SquareFrame, 1),
            stream(2, Glyph::Streak, Glyph::Streak, 4),
        ],
        region_outline: Some(5),
    };
    (config, styles, palette)
}

/// The square regions of a size, two triangles each.
pub fn regions(size: Size) -> Vec<RegionShape> {
    let shape = size.shape();
    (0..shape.columns * shape.rows)
        .map(|i| {
            let (x, y) = ((i % shape.columns) as f64, (i / shape.columns) as f64);
            let (a, b, c, d) = ((x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y + 1.0));
            RegionShape {
                id: region_id(i),
                triangles: vec![[a, b, c], [a, c, d]],
                rings: vec![vec![a, b, c, d]],
            }
        })
        .collect()
}

/// The two ends of one flow, in the plan.
fn ends(size: Size, still: &Still, stream: u16, g: &mut Lcg) -> (Point, Point) {
    let shape = size.shape();
    match size {
        // Between regions: seven in eight to one nearby, one in eight to
        // anywhere.
        Size::Large => {
            let count = shape.columns * shape.rows;
            let a = g.below(count);
            let b = if g.below(8) == 0 {
                g.below(count)
            } else {
                let (x, y) = (a % shape.columns, a / shape.columns);
                let x = (x + g.below(13)).saturating_sub(6).min(shape.columns - 1);
                let y = (y + g.below(13)).saturating_sub(6).min(shape.rows - 1);
                y * shape.columns + x
            };
            let b = if b == a { (a + 1) % count } else { b };
            (centre(shape, a), centre(shape, b))
        }
        // Between particles and posts: in to the nearest post, out from any
        // post, and back from the nearest post or between two posts.
        Size::Small => {
            let posts = still.places.len();
            let dot = still.particles[g.below(still.particles.len() - posts)].1;
            let post = |k: usize| still.places[k].at;
            let mut nearest = 0;
            for k in 1..posts {
                if distance_squared(post(k), dot) < distance_squared(post(nearest), dot) {
                    nearest = k;
                }
            }
            match stream {
                0 => (dot, post(nearest)),
                1 => (post(g.below(posts)), dot),
                _ => {
                    if g.below(3) == 0 {
                        let a = g.below(posts);
                        (post(a), post((a + 1 + g.below(posts - 1)) % posts))
                    } else {
                        (post(nearest), dot)
                    }
                }
            }
        }
    }
}

/// The frame of a period: the same particles and sets, with the ground, the
/// tones and the flows of that period. Particles and the ends of flows sit on
/// the ground of the period.
pub fn frame(size: Size, period: u64) -> FieldFrame {
    let shape = size.shape();
    let still = still(size);
    let count = shape.columns * shape.rows;
    let mut heights = Vec::with_capacity(count);
    let mut regions = Vec::with_capacity(count);
    for i in 0..count {
        let middle = centre(shape, i);
        let read = is_read(shape, i);
        let height = height_at(&still.places, middle, period);
        heights.push(if read { height } else { 0.0 });
        regions.push(RegionReading {
            region: region_id(i),
            height: read.then_some(height),
            tone: read.then(|| tone_at(&still.places, middle, Some(period))),
        });
    }
    let on_ground = |p: Point, lift: f32| plan_to_scene(p, seat(shape, &heights, p) + lift);
    let particles = still
        .particles
        .iter()
        .map(|(id, p, style)| ParticleFrame {
            id: id.clone(),
            at: on_ground(*p, DOT_LIFT),
            style: *style,
        })
        .collect();
    let mut flows = Vec::with_capacity(3 * shape.flows);
    for stream in 0..3_u16 {
        let mut g = Lcg::new(period.wrapping_mul(16).wrapping_add(u64::from(stream) + 1));
        for j in 0..shape.flows {
            let (from, to) = ends(size, still, stream, &mut g);
            flows.push(Flow {
                stream,
                from: on_ground(from, MARK_LIFT),
                to: on_ground(to, MARK_LIFT),
                amount: 1 + g.below(shape.most) as u64,
                hollow: j % 10 == 9,
            });
        }
    }
    FieldFrame {
        period,
        regions,
        particles,
        sets: still.sets.clone(),
        flows,
    }
}
