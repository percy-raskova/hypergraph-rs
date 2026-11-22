# The Bevy crate: ground, sets, particles and marks: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> A fresh implementer builds one task, a fresh reviewer checks it, then the next begins.

**Goal:** Add to the hypergraph-rs repository a crate `hypergraph-rs-bevy` that draws any hypergraph in Bevy 0.20: a
ground of regions that bends or rises and sinks in blocks, hatched outlines of hyperedges, particles, and marks that
move along paths in beats, behind one plugin, with one example that runs without any game.

**Architecture:** Two layers, as in the geometry plan. The lower layer is pure functions that return `Mesh` values or
plain numbers (`lines`, `glyph`, `ground`, `flows`, `motion`, `frame_windows`), tested without a window. The upper
layer is `FieldPlugin`: the caller sends one `FieldFrame` per period as a message; the plugin checks it, keeps the
previous frame, and plays the change in a fixed number of beats by a clock read from `Time`. Nothing moves while the
clock rests. Ordinary meshes and `StandardMaterial` only; one optional shader for marks (Task 12) is built only if the
measured trial of Task 11 says plain marks miss the frame budget.

**Tech Stack:** Rust (pinned toolchain 1.100.0; this crate's floor `rust-version = "1.97.1"`, Bevy 0.20.1's; the rest
of the workspace keeps 1.90). `bevy = "0.20.1"` with `default-features = false, features = ["std", "bevy_pbr"]`.
`hypergraph-rs` by path with `default-features = false, features = ["geometry"]`. `thiserror` 2 (workspace). No other
dependency.

**Spec:** the approved design `2026-11-21-field-and-particles-design.md`, sections 6.1, 6.5 to 6.8, 8, 10 (step 3),
11 and 12, with the rulings of section 2. The design speaks of a game; this plan and everything it builds speak only
of hypergraphs, regions, particles, sets, streams and amounts. The sketch `field-and-particles.html` (functions
`contour`, `drawMembranes`, `tiesStep`, `wellsStep`, `relax`, `wellTargets`, `mpos`, `drawMarks`, `hatch`)
illustrates one working form; where it differs from this plan (it uses `Math.sin` for its bow and its hash,
caps marks per flow at 90, draws on a 2D canvas), this plan binds. The public API consumed from the core is the
geometry plan's,
`plans/2026-11-21-field-geometry.md`, section "The public API this plan
produces".

**Sources (read-only):** the design, in the first user's repository; the sketch; and the geometry plan:

```text
https://github.com/percy-raskova/babylon/blob/a2b9a482b/docs/superpowers/specs/2026-11-21-field-and-particles-design.md
The sketch is kept outside the repositories. The lead hands its path to builders and reviewers.
plans/2026-11-21-field-geometry.md
```

## Global Constraints

Carried by every plan for this library:

- The library is public and must stay usable by itself: nothing of any game in its code, tests, docs or examples.
  Names, ids and doc examples use neutral words (`a`, `b`, `club`, `team`, `r0`, `p17`, stream `0`).
- Everything at its newest version: crates, the pinned toolchain, CI's Python, workflow actions. The minimum Rust
  version rises when a dependency needs it; nothing is held back.
- The gates are the repository's own: `mise run rust:check` (fmt, clippy with warnings denied, tests, rustdoc with
  warnings denied, all features, each feature alone), `mise run rust:msrv`, cargo-deny, CodeQL, and for the Python
  bindings `mise run py:gate`. A task ends with the narrowest of these that covers what it touched; the whole set runs
  once at the end of the plan (Task 13). This plan adds one gate of the same kind, `mise run rust:bevy` (Task 1).
- One cargo or pytest process at a time on the machine. Never pass `-j` or `--jobs`. Running the example binary
  counts as one such process.
- Commits: the repository has no git identity of its own. The lead supplies the author and committer through the
  environment (`GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`) on every commit. A
  builder never runs `git config` and never commits without that environment. Messages follow the history's style
  (`feat(bevy): ...`), with any trailer the lead names.
- No release to crates.io or PyPI. That is the owner's to order.
- The one known user (a game) takes the library by git revision with default features off and uses only `Hypergraph`
  and `subhypergraph`. No task may break a build with default features off.

This plan's own:

- **Starts after the geometry plan is merged.** Run `git fetch origin`, then
  `git show origin/main:crates/hypergraph-rs/Cargo.toml | grep -q '^geometry = '`. If that fails, stop and tell the
  lead. Otherwise work on the branch `feat/bevy-crate` from the fetched `origin/main`, in a git worktree of
  its own (the lead names the directory). Never push. "The base" means `$(git merge-base HEAD origin/main)`.
- **No file of the core crate changes:** nothing under `crates/hypergraph-rs/`, `crates/hypergraph-rs-python/`,
  `crates/hypergraph-rs-wasm/`, `crates/hypergraph-rs-cli/`, `python/`, `tests/` (the Python suite) or `tools/`.
- **Bevy is one version, the game's minor at its newest patch: 0.20.1** (released 2026-10-28, `rust_version` 1.97.1
  on crates.io). The game is on 0.20.0; it moves to 0.20.1 (a patch) when it takes this crate.
- **The crate's own Bevy features are `std` and `bevy_pbr`**, nothing else. `bevy_pbr` brings, through Bevy's own
  feature list, `bevy_light`, `bevy_material`, `bevy_core_pipeline`, `bevy_render`, `bevy_camera`, `bevy_mesh`,
  `bevy_image`, `bevy_asset`, `bevy_color` and `bevy_window`. Every one is in the game's set already (the game names
  `bevy_pbr` and gets `std` through `2d`), so adding this crate to the game turns on no new Bevy feature. The example
  alone adds, as a dev-dependency, `bevy_winit`, `x11` and `multi_threaded`. Not `wayland`: winit's Wayland
  decorations pull `ttf-parser`, which has an open "unmaintained" advisory (RUSTSEC-2026-0192). On a Wayland desktop
  the example runs under XWayland. If the example needs one more Bevy feature to compile or open a window, add the
  smallest one, to the dev-dependency only, and name it in the commit message.
- **Positions are measurements.** Nothing is random, nothing reads the wall clock except through Bevy's `Time`, and no
  `HashMap` or `HashSet` iteration decides any output (lookups only; `BTreeMap` or sorted `Vec` where order matters).
  The same frames and the same `Time` steps give the same entities, transforms and mesh bytes on two runs.
- **Nothing moves between beats** except what the caller changes. While the clock rests no system writes a
  `Transform`, a `Visibility` or a mesh.
- **Two streams never meet.** Each stream has its own unit per mark. No function, field or report combines the amounts
  of two streams. `FlowReport` lists streams apart and has no grand total.
- **Hatching marks sets, never the ground.** The ground carries vertex colours and nothing else.
- **The palette is the caller's.** No colour constant in `src/` except `Color::WHITE` as the ground material's base,
  which multiplies the vertex colours. `Palette` has no `Default`.
- **Coordinates.** A plan point `(x, y)` from the core's `geometry` lands in the scene at `Vec3::new(x as f32, h,
  -(y as f32))`, as the known user's client already places its map. `plan_to_scene` (Task 2) is the one place that
  does it. Every material this crate makes has `cull_mode: None` and `double_sided: true`, and every mesh carries
  explicit normals, so winding never decides what is seen.
- No `unsafe` (`#![forbid(unsafe_code)]`). `#![deny(missing_docs)]`. Clippy's `too_many_arguments` and
  `type_complexity` may be allowed on a system function only, each with `reason = "..."`.
- Tests are files directly under `crates/hypergraph-rs-bevy/tests/`, sharing `tests/common/mod.rs`, which starts
  `#![allow(dead_code, reason = "each test file uses only some of the helpers")]` (without it, clippy `-D warnings`
  fails on every test file that leaves a helper unused). Tests never open a window and never need a graphics card:
  the headless app is `MinimalPlugins`, `AssetPlugin::default()`, `init_asset::<Mesh>()`, `init_asset::<Image>()`,
  `init_asset::<StandardMaterial>()`, and `TimeUpdateStrategy::ManualDuration(Duration::from_millis(10))`, as the
  known user's client tests do, but with a 10 ms step: every time a test names (`t0 + 2.5 s`, `t0 + 1.5 s`) is then a
  whole number of steps, so `advance_to` lands on it exactly and an eased value is exact. (With 16 ms, 2.5 s is not
  reached exactly and a "within 1e-5" check fails.) Bevy's first update has a zero delta; every later one adds
  exactly 10 ms. In the tests, `t0` is `Time::elapsed()` read just after the `app.update()` that follows writing the
  frame's message.
- Size: the scene holds 25,000 particles. No cost may grow with the square of the number of particles, marks, regions
  or set members.
- An expected compile failure names the missing item; whether rustc says "unresolved import" or "cannot find" depends
  on the import. Either wording is the expected RED.

## Review Focus

1. **A frame arrives while the beats play.** The picture must restart from the last settled frame, not from whatever
   the ease had reached (Task 7, `a_frame_during_the_beats_restarts_from_the_last_settled_picture`).
2. **A frame the plugin cannot draw** (an unknown style, a duplicate id, a region in no ground, a `NaN`). It is
   refused whole, the refusal names the item, and the picture stays as it was (Task 7,
   `a_bad_frame_is_refused_and_the_picture_stays`; Task 5, `bad_flows_are_refused`).
3. **Amounts at the edge.** Two flows of `u64::MAX`, a flow below one mark, a flow from a place into itself. Totals
   must not overflow, and drawn plus not drawn must equal the total exactly (Task 5).
4. **A set that empties or vanishes.** Every member at zero gives no outline; a set missing from the next frame is
   removed at the end of the ease beat; neither leaves a stale mesh (Task 8,
   `a_set_of_zeros_draws_nothing_and_says_so`).
5. **A vertex exactly on a hatch line**, and members on one spot. A crossing is counted once; zero-length hatch strokes
   and zero-length ribbons are dropped, never drawn as degenerate triangles (Task 2).

## The public API this plan produces

All of it is in `hypergraph_rs_bevy`, re-exported from the crate root. `Point`, `ClosedPath`, `Outline`, `Positions`,
`OutlineParams` and `GeometryError` are the core's (`hypergraph_rs::geometry`), used as the geometry plan defines them.

```rust
// error.rs
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FieldError {
    #[error("{what} {index} is not defined (there are {len})")]
    UnknownStyle { what: &'static str, index: u16, len: usize },   // what: "swatch" | "particle style" | "set style" | "stream" | "beat"
    #[error("{0} appears twice")]
    DuplicateId(String),
    #[error("region {0} is in no ground")]
    UnknownRegion(String),
    #[error("{item} is not finite")]
    NonFinite { item: String },
    #[error("tone {value} of {region} is outside -1..=1")]
    ToneOutOfRange { region: String, value: f32 },
    #[error("parameter {name} = {value} is out of range")]
    BadParameter { name: &'static str, value: f64 },
    #[error("region {region}: {problem}")]
    BadShape { region: String, problem: &'static str },
    #[error("an outline of a kind this crate does not draw")]
    UnsupportedOutline,
    #[error(transparent)]
    Geometry(#[from] GeometryError),
}

// palette.rs
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Palette { pub ramp: [Color; 3], pub no_reading: Color, pub swatches: Vec<Color> }
impl Palette {
    pub fn tone(&self, tone: Option<f32>) -> LinearRgba;    // None -> no_reading; -1 ramp[0], 0 ramp[1], 1 ramp[2]
    pub fn swatch(&self, index: u16) -> Option<Color>;
}

// style.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Glyph { Disc, Ring, Square, SquareFrame, Streak }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchDirection { Deg0, Deg45, Deg90, Deg135 }
#[derive(Clone, Debug, PartialEq)] pub struct LineStyle { pub width: f32, pub dash: Option<(f32, f32)> }
#[derive(Clone, Debug, PartialEq)] pub struct Hatch { pub direction: HatchDirection, pub spacing: f32, pub width: f32 }
#[derive(Clone, Debug, PartialEq)] pub struct ParticleStyle { pub glyph: Glyph, pub size: f32, pub swatch: u16 }
#[derive(Clone, Debug, PartialEq)] pub struct SetStyle { pub swatch: u16, pub line: LineStyle, pub hatch: Option<Hatch> }
#[derive(Clone, Debug, PartialEq)]
pub struct StreamStyle {
    pub beat: usize, pub unit: NonZeroU64, pub full: Glyph, pub hollow: Glyph,
    pub size: f32, pub swatch: u16, pub share_drawn: f64, pub max_marks: u32,
}
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct FieldStyles {
    pub particles: Vec<ParticleStyle>, pub sets: Vec<SetStyle>, pub streams: Vec<StreamStyle>,
    pub region_outline: Option<u16>,                       // swatch of region outlines; None draws none
}
impl FieldStyles { pub fn validate(&self, palette: &Palette, config: &FieldConfig) -> Result<(), FieldError>; }

// config.rs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum Motion { #[default] Full, Reduced }
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct FieldConfig {
    pub beats: Vec<Duration>, pub ease_beat: usize, pub mark_travel: f32, pub bow: f32,
    pub motion: Motion, pub curve_segments: u32, pub outline_lift: f32,
}
impl FieldConfig {
    pub fn three_beats(beat: Duration) -> Self;            // [beat; 3], ease_beat 2, travel 0.4, bow 0.08, Full, 48, 0.004
    pub fn validate(&self) -> Result<(), FieldError>;
}

// lines.rs
pub fn plan_to_scene(p: Point, height: f32) -> Vec3;
pub fn outline_paths(outline: &Outline, segments: u32) -> Result<Vec<ClosedPath>, FieldError>;
pub fn hatch_segments(paths: &[ClosedPath], direction: HatchDirection, spacing: f64) -> Vec<(Point, Point)>;
pub fn dashed(path: &[Point], dash: Option<(f64, f64)>) -> Vec<(Point, Point)>;
pub fn ribbon_mesh(segments: &[(Point, Point)], width: f32, height: f32) -> Mesh;

// glyph.rs
pub fn glyph_mesh(glyph: Glyph, segments: u32) -> Mesh;

// ground.rs
#[derive(Clone, Debug, PartialEq)]
pub struct RegionShape { pub id: String, pub triangles: Vec<[Point; 3]>, pub rings: Vec<ClosedPath> }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum GroundForm { #[default] Surface, Blocks }
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct Ground { pub shapes: Arc<[RegionShape]>, pub form: GroundForm }
#[derive(Clone, Debug)]
pub struct GroundTopology { /* private */ }
impl GroundTopology {
    pub fn new(shapes: &[RegionShape], form: GroundForm) -> Result<Self, FieldError>;
    pub fn form(&self) -> GroundForm;
    pub fn region_count(&self) -> usize;
    pub fn region_index(&self, id: &str) -> Option<usize>;
    pub fn body_mesh(&self, heights: &[f32], colours: &[LinearRgba]) -> Mesh;
    pub fn write_body(&self, mesh: &mut Mesh, heights: &[f32], colours: &[LinearRgba]);
    pub fn outline_mesh(&self, heights: &[f32], lift: f32) -> Mesh;
    pub fn write_outline(&self, mesh: &mut Mesh, heights: &[f32], lift: f32);
}

// flows.rs
#[derive(Clone, Debug, PartialEq)]
pub struct Flow { pub stream: u16, pub from: Vec3, pub to: Vec3, pub amount: u64, pub hollow: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnFlow { pub flow: usize, pub stream: u16, pub hollow: bool, pub marks: u32 }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamReport { pub stream: u16, pub hollow: bool, pub total: u128, pub drawn: u128, pub not_drawn: u128,
                          pub marks: u32, pub paths: u32, pub paths_drawn: u32 }
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct FlowReport { pub period: Option<u64>, pub streams: Vec<StreamReport> }
pub fn select_flows(flows: &[Flow], streams: &[StreamStyle]) -> Result<(Vec<DrawnFlow>, FlowReport), FieldError>;

// motion.rs
pub fn smoothstep(x: f32) -> f32;
pub fn mark_start(flow: usize, k: u32, n: u32, beat: Duration, travel: f32) -> Duration;
pub fn mark_position(from: Vec3, to: Vec3, u: f32, bow: f32, side: f32) -> Vec3;
#[derive(Clone, Copy, Debug, PartialEq)] pub enum Phase { Resting, Beat { index: usize, fraction: f32 } }
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct BeatClock { /* private: started_at: Option<Duration>, period: Option<u64> */ }
impl BeatClock {
    pub fn start(&mut self, now: Duration, period: u64);
    pub fn phase(&self, now: Duration, beats: &[Duration]) -> Phase;
    pub fn period(&self) -> Option<u64>;
}
pub fn eased(from: f32, to: f32, phase: Phase, ease_beat: usize, motion: Motion) -> f32;

// frame.rs
#[derive(Clone, Debug, PartialEq)] pub struct RegionReading { pub region: String, pub height: Option<f32>, pub tone: Option<f32> }
#[derive(Clone, Debug, PartialEq)] pub struct ParticleFrame { pub id: String, pub at: Vec3, pub style: u16 }
#[derive(Clone, Debug, PartialEq)]
pub struct SetFrame { pub id: String, pub style: u16, pub height: f32, pub members: Vec<(String, f64)>,
                      pub outline: Option<Outline>, pub held_share: Option<f64> }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldFrame { pub period: u64, pub regions: Vec<RegionReading>, pub particles: Vec<ParticleFrame>,
                        pub sets: Vec<SetFrame>, pub flows: Vec<Flow> }
impl FieldFrame { pub fn validate(&self, styles: &FieldStyles) -> Result<(), FieldError>; }
pub fn set_frames<N, E, M>(h: &Hypergraph<N, E, M>, sets: &[(&str, u16)], positions: &Positions,
    weight: impl Fn(&M) -> f64, params: &OutlineParams, height: f32) -> Result<Vec<SetFrame>, FieldError>;
#[derive(Clone, Debug, PartialEq)]
pub enum SetChange {
    Appeared { set: String },
    Vanished { set: String },
    Changed { set: String, joined: Vec<String>, left: Vec<String>, renumbered: Vec<String> },
}
pub fn diff_sets(previous: &[SetFrame], next: &[SetFrame]) -> Vec<SetChange>;

// plugin/mod.rs
pub struct FieldPlugin { /* config, styles, palette */ }
impl FieldPlugin { pub fn new(config: FieldConfig, styles: FieldStyles, palette: Palette) -> Result<Self, FieldError>; }
impl Plugin for FieldPlugin { fn build(&self, app: &mut App); }
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)] pub enum FieldSystems { Install, Animate }
#[derive(Message, Clone, Debug)] pub struct ShowPeriod(pub Box<FieldFrame>);
#[derive(Message, Clone, Copy, Debug, Default)] pub struct ReplayPeriod;
#[derive(Message, Clone, Debug, PartialEq)] pub struct FrameRefused { pub period: u64, pub error: FieldError }
#[derive(Message, Clone, Debug, PartialEq)] pub struct SetChanged(pub SetChange);
#[derive(Component, Clone, Debug, PartialEq, Eq)] pub struct ParticleId(pub String);
#[derive(Component, Clone, Debug, PartialEq, Eq)] pub struct SetId(pub String);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)] pub struct MarkOf { pub stream: u16, pub flow: usize, pub k: u32 }
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)] pub struct PathLines { pub stream: u16 }
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)] pub enum GroundPart { Body, Outline }

// frame_windows.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct WindowStats { pub index: usize, pub median: Duration, pub p95: Duration }
#[derive(Clone, Debug)] pub struct FrameWindows { /* private */ }
impl FrameWindows {
    pub fn new(window: usize, settle: Duration) -> Self;
    pub fn push(&mut self, interval: Duration) -> Option<WindowStats>;
    pub fn slowest_p95(&self) -> Option<Duration>;
}
```

**How a caller hands it a period.** Once: spawn one or more `Ground` entities with the region shapes (triangulated by
the caller; the crate has no triangulator). Each period: build a `FieldFrame` (regions' heights and tones in scene
units the caller fixed for the session, particles' positions in scene coordinates, sets from `set_frames`, flows with
integer amounts in each stream's smallest unit) and write `ShowPeriod(Box::new(frame))`. The plugin plays it in the
beats of `FieldConfig`, then rests. `ReplayPeriod` plays the same change again. The caller's own systems that write
these messages run `.before(FieldSystems::Install)`; anything that reads the picture runs
`.after(FieldSystems::Animate)`.

**Systems and their order.** Both sets are in `Update`, configured `(FieldSystems::Install, FieldSystems::Animate)
.chain()`. `Install` runs, chained: `refresh_materials` (when `Palette` changed), `build_grounds` (`Added<Ground>` or
`Changed<Ground>`), `install_frame` (reads `ShowPeriod` and `ReplayPeriod`, refuses or accepts, spawns and despawns,
starts the clock, writes `FlowReport`, sends `SetChanged`). `Animate` runs, chained: `animate_ground`,
`animate_particles`, `animate_sets`, `animate_marks`. Each reads the phase once from `BeatClock` and `Time`, and writes
nothing when the phase is `Resting` and was `Resting` on the frame before.

**What the beats do.** The period's clock starts on the update that accepts a frame. During beats before `ease_beat`
the previous frame's heights, tones and positions show. During `ease_beat` they ease, by `smoothstep`, to the new
frame's (with `Motion::Reduced`, they jump at the end of `ease_beat`). New particles and sets appear at the start of
`ease_beat`; particles and sets that left go at its end; set meshes are swapped at its end. A stream's marks move only
during its own `beat`. After the last beat the clock rests, mark entities are despawned, and the frame is still.

## File Structure

| File | Responsibility | Task |
| --- | --- | --- |
| `Cargo.toml` (root) | the member `crates/hypergraph-rs-bevy` | 1 (shared) |
| `Cargo.lock` | Bevy's tree | 1 (shared) |
| `.mise.toml` | `rust:check` and `rust:msrv` exclude the crate; new task `rust:bevy` | 1 (shared) |
| `.github/workflows/ci.yml` | new job `bevy-gate` | 1 (shared) |
| `deny.toml` | `MIT-0` and `ISC` on the allow list, with the crates that bring them | 1 (shared) |
| `crates/hypergraph-rs-bevy/Cargo.toml` (new) | the crate, its floor, its Bevy features, the example's | 1 |
| `crates/hypergraph-rs-bevy/README.md` (new) | what the crate is, how to run the example, the measured frame times | 1, 10, 11, 13 |
| `crates/hypergraph-rs-bevy/src/lib.rs` (new) | crate docs, lints, modules, re-exports | 1 to 7, 10, 13 (12 if built) |
| `src/error.rs`, `src/palette.rs`, `src/style.rs`, `src/config.rs` (new) | `FieldError`, `Palette`, styles, `FieldConfig` | 1 |
| `src/lines.rs` (new) | `plan_to_scene`, `outline_paths`, `hatch_segments`, `dashed`, `ribbon_mesh` | 2 |
| `src/glyph.rs` (new) | `glyph_mesh` | 3 |
| `src/ground.rs` (new) | `RegionShape`, `GroundForm`, `Ground`, `GroundTopology` | 4 |
| `src/flows.rs` (new) | `Flow`, `DrawnFlow`, `StreamReport`, `FlowReport`, `select_flows` | 5 |
| `src/motion.rs` (new) | `smoothstep`, `mark_start`, `mark_position`, `Phase`, `BeatClock`, `eased` | 5 |
| `src/frame.rs` (new) | `FieldFrame` and its parts, `validate`, `set_frames`, `SetChange`, `diff_sets` | 6 |
| `src/plugin/mod.rs` (new) | `FieldPlugin`, `FieldSystems`, messages, components, materials, `install_frame` | 7, 8, 9 |
| `src/plugin/ground.rs`, `particles.rs` (new) | `build_grounds`, `animate_ground`, `animate_particles` | 7 |
| `src/plugin/sets.rs` (new) | `animate_sets`, set meshes | 8 |
| `src/plugin/marks.rs` (new) | mark entities, `animate_marks`, reduced-motion path lines | 9 |
| `src/frame_windows.rs` (new) | `FrameWindows`, `WindowStats` | 10 |
| `examples/field/main.rs`, `examples/field/world.rs` (new) | the example and its invented world | 10 |
| `tests/common/mod.rs` (new) | `field_app`, `advance_to`, `snapshot` | 7 |
| `tests/test_*.rs` (new) | one file per task, named in each task | 1 to 10 (12 if built) |
| `README.md` (root), `CHANGELOG.md`, `CONTRIBUTING.md` | one section and one line in "Checks"; one "Added" entry and one "Rust version" line; the floor paragraph and one row of the checks table | 13 (shared) |

**Files the XGI 1.1 plan also touches.** The root `Cargo.toml` (that plan raises the workspace version to 0.3.0; this
one adds a member, which inherits that version), `Cargo.lock` (that plan also runs `cargo update` at its close),
`.mise.toml` (its XGI pin in the descriptions of `rust:fixtures` and `py:*`, and one new task of its own; this one
edits `rust:check` and `rust:msrv` and adds `rust:bevy`), `.github/workflows/ci.yml` (its XGI pin and Python version
in `xgi-conformance`; this one adds a job), `deny.toml` (only if it brings a crate with a new licence), `README.md`,
`CHANGELOG.md` (that plan writes a `## [0.3.0] - unreleased` section: this plan's entries go under the topmost
unreleased section, whichever it is when Task 13 runs) and `CONTRIBUTING.md`.
This plan only adds lines and one job there and edits the two task bodies named. Whichever branch lands second rebases
and keeps both sides' lines. On a conflict in `Cargo.lock`, take `origin/main`'s file, then run
`cargo tree -p hypergraph-rs-bevy -e normal > /dev/null` once without `--locked`, which adds this crate's entries and
nothing else, and commit the result. No file under `crates/hypergraph-rs/` is touched by this plan, so the geometry
plan's files and the XGI plan's core files are never shared with it.

---

### Task 1: The crate, its floor, its gates, the palette and the styles

**Files:**
- Modify: root `Cargo.toml` (`members`), `Cargo.lock`, `.mise.toml`, `.github/workflows/ci.yml`, `deny.toml`
- Create: `crates/hypergraph-rs-bevy/Cargo.toml`, `README.md`, `src/lib.rs`, `src/error.rs`, `src/palette.rs`,
  `src/style.rs`, `src/config.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_palette.rs` (Create)

**Interfaces:**
- Consumes: `hypergraph_rs::geometry::GeometryError` (geometry plan).
- Produces: `FieldError`, `Palette`, `Glyph`, `HatchDirection`, `LineStyle`, `Hatch`, `ParticleStyle`, `SetStyle`,
  `StreamStyle`, `FieldStyles` (with `validate`), `Motion`, `FieldConfig` (with `three_beats`, `validate`), exactly as
  in "The public API". The gate `mise run rust:bevy`.

**Decisions:**
- *Manifest:* `name = "hypergraph-rs-bevy"`, workspace-inherited `version`, `edition`, `license`, `authors`,
  `repository`; `rust-version = "1.97.1"` written out (not inherited); `publish = false`; `description = "Bevy
  integration for hypergraph-rs: a ground of regions, hatched hyperedge outlines, particles and marks moving in
  beats"`. `[dependencies]`: `bevy = { version = "0.20.1", default-features = false, features = ["std", "bevy_pbr"] }`,
  `hypergraph-rs = { path = "../hypergraph-rs", default-features = false, features = ["geometry"] }`,
  `thiserror.workspace = true`. `[dev-dependencies]`: `bevy = { version = "0.20.1", default-features = false, features
  = ["std", "bevy_pbr", "bevy_winit", "x11", "multi_threaded"] }`. `[[example]] name = "field", path =
  "examples/field/main.rs"` (Task 10 writes it; until then the `[[example]]` table is left out).
- *`rust:check` and `rust:msrv`:* each `--workspace` leg gains `--exclude hypergraph-rs-bevy` (clippy, test, doc in
  `rust:check`; the workspace `check` in `rust:msrv`). `cargo fmt --all` stays as it is (it covers the new crate).
- *`rust:bevy`*, a new task, run by CI's new job and at the end of the plan:

  ```toml
  [tasks."rust:bevy"]
  description = "The Bevy crate, in its own job: its lib with only its own Bevy features, clippy -D warnings, tests, docs, then its own floor (rust-version in its Cargo.toml). Sequential."
  run = """
  set -e
  cargo check -p hypergraph-rs-bevy --lib --locked
  cargo clippy -p hypergraph-rs-bevy --all-targets --locked -- -D warnings
  cargo test -p hypergraph-rs-bevy --locked
  RUSTDOCFLAGS='-D warnings' cargo doc -p hypergraph-rs-bevy --no-deps --locked
  FLOOR=$(grep '^rust-version' crates/hypergraph-rs-bevy/Cargo.toml | cut -d'"' -f2)
  echo "Bevy crate floor: $FLOOR"
  rustup toolchain install "$FLOOR" --profile minimal
  cargo +"$FLOOR" check -p hypergraph-rs-bevy --all-targets --locked
  """
  ```

  The first line proves the crate builds with its own two Bevy features: under resolver 2 a `--lib` check does not
  turn on the dev-dependency's features.
- *CI job* `bevy-gate` in `ci.yml`, after `rust-msrv`, in the file's style: `name: Bevy crate (clippy -D warnings,
  test, doc, its own floor)`, `runs-on: ubuntu-latest`, `timeout-minutes: 45`; steps: the same pinned
  `actions/checkout`, `jdx/mise-action` and `Swatinem/rust-cache` (with `key: bevy`) as `rust-gate`; then
  `rustup show active-toolchain`; then `mise run rust:bevy`. No `continue-on-error`. No system package is installed:
  with these features every system library is opened at run time (`x11-dl`, whose build script only asks
  `pkg-config` and accepts no answer, `x11rb` with `dl-libxcb`, `xkbcommon-dl`), so the build needs none. If the
  build on CI proves otherwise, add the one package it names and say so in the commit message.
- *Licences:* the game's lockfile shows every licence in a Bevy 0.20 tree that is a superset of this one. Two are not
  on the list and are not offered with an alternative that is: `MIT-0` (`encase`, `encase_derive`,
  `encase_derive_impl`, Bevy's GPU buffer layout) and `ISC` (`libloading`, which loads the Vulkan driver). Read each
  crate's licence file, then add both names to `allow`, each with a comment naming the crates and saying it is
  permissive and compatible with shipping under BSD-3-Clause. If `cargo deny check` names any other licence, any
  source that is not crates.io, or any advisory, stop and report it to the lead: no `ignore` entry and no further
  licence is added in this plan without the lead.
- *`Palette::tone`:* `None` gives `no_reading` in linear RGBA. `t` is clamped to `[-1, 1]`; for `t < 0` the result is
  `ramp[0] * (-t) + ramp[1] * (1 + t)`, for `t >= 0` `ramp[1] * (1 - t) + ramp[2] * t`, component by component on
  `LinearRgba` (alpha included). This weighted form gives each ramp colour exactly at `-1`, `0` and `1` (the form
  `a + (b - a) * t` can miss `b` by one unit in the last place, and the first test compares with `assert_eq!`). The
  arithmetic is written out, not delegated to Bevy's `Mix`, so it does not change with Bevy.
- *`FieldConfig::validate`:* `beats` not empty, each above zero; `ease_beat < beats.len()`; `mark_travel` in
  `(0, 1]`; `bow` finite and `>= 0`; `curve_segments >= 8`; `outline_lift` finite. Each failure is `BadParameter`
  naming the field (`"beats"` for an empty list or a zero beat).
- *`FieldStyles::validate`:* every swatch index below `palette.swatches.len()` (`UnknownStyle { what: "swatch" }`);
  every stream's `beat < config.beats.len()` (`what: "beat"`); sizes, widths and spacings finite and above zero;
  `share_drawn` in `(0, 1]`; `max_marks >= 1`; a dash's two lengths finite and above zero. Parameter names are the
  field names (`"size"`, `"width"`, `"spacing"`, `"share_drawn"`, `"max_marks"`, `"dash"`).

- [ ] **Step 1: Write the failing tests** in `tests/test_palette.rs`:

  ```rust
  fn palette() -> Palette {
      Palette { ramp: [Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.5, 0.5, 0.5), Color::srgb(0.0, 0.0, 1.0)],
                no_reading: Color::srgb(0.2, 0.2, 0.2), swatches: vec![Color::WHITE, Color::BLACK] }
  }
  #[test] fn the_ramp_runs_through_its_three_colours() {
      let p = palette();
      assert_eq!(p.tone(Some(-1.0)), p.ramp[0].to_linear());
      assert_eq!(p.tone(Some(0.0)), p.ramp[1].to_linear());
      assert_eq!(p.tone(Some(1.0)), p.ramp[2].to_linear());
      // Some(0.5): each component == (ramp[1] + ramp[2]) / 2 in linear, within 1e-6
      // Some(7.0) == Some(1.0); Some(-7.0) == Some(-1.0)
  }
  #[test] fn no_reading_is_its_own_colour() { assert_eq!(palette().tone(None), palette().no_reading.to_linear()); }
  #[test] fn swatches_are_looked_up_by_index() { /* swatch(1) == Some(BLACK); swatch(2) == None */ }
  #[test] fn three_beats_is_a_valid_config() {
      let c = FieldConfig::three_beats(Duration::from_millis(1200));
      assert_eq!(c.beats, vec![Duration::from_millis(1200); 3]);
      assert_eq!((c.ease_beat, c.mark_travel, c.bow, c.motion, c.curve_segments, c.outline_lift),
                 (2, 0.4, 0.08, Motion::Full, 48, 0.004));
      assert_eq!(c.validate(), Ok(()));
  }
  #[test] fn a_bad_config_names_its_field() {
      // beats empty -> BadParameter { name: "beats", .. }; ease_beat 3 of 3 -> "ease_beat";
      // mark_travel 0.0 -> "mark_travel"; curve_segments 4 -> "curve_segments"
  }
  #[test] fn styles_refuse_an_unknown_swatch_and_a_stream_without_its_beat() {
      // a ParticleStyle with swatch 5 over 2 swatches -> UnknownStyle { what: "swatch", index: 5, len: 2 };
      // a StreamStyle with beat 3 over three_beats -> UnknownStyle { what: "beat", index: 3, len: 3 };
      // share_drawn 0.0 -> BadParameter { name: "share_drawn", .. }
  }
  ```

- [ ] **Step 2: Add the crate with an empty `lib.rs`, then run and see them fail.** Write the manifest, the member
  line in the root `Cargo.toml`, and `src/lib.rs` with only the crate attributes.
  Run: `cargo test -p hypergraph-rs-bevy --test test_palette` (this first run, without `--locked`, writes Bevy's tree
  into `Cargo.lock`).
  Expected: FAIL to compile, "cannot find type `Palette`".

- [ ] **Step 3: Implement** `error.rs`, `palette.rs`, `style.rs`, `config.rs` and their re-exports in `lib.rs`, with
  a short crate doc (what it draws, that it reads the core's `geometry`, that nothing of any game is in it).

- [ ] **Step 4: Run and see them pass.** `cargo test -p hypergraph-rs-bevy --locked --test test_palette`.
  Expected: PASS.

- [ ] **Step 5: The gates and the supply chain.** Edit `.mise.toml`, `ci.yml` and `deny.toml` as decided. Then, one
  at a time: `mise run rust:bevy` (expected: PASS, the floor leg on 1.97.1; if a dependency in the tree needs a newer
  Rust, raise this crate's `rust-version` to the lowest version that passes, say why in the commit message, and run
  again); `cargo deny check` (expected: PASS); `mise run rust:msrv` (expected: PASS on 1.90; if cargo 1.90 cannot read
  a manifest in Bevy's tree, stop and report to the lead, who decides between raising the floor and moving this crate
  into a workspace of its own).

- [ ] **Step 6: Prove the core is untouched.** At the base and at `HEAD`, run
  `cargo tree -p hypergraph-rs --no-default-features -e normal,build --locked` and
  `cargo tree -p hypergraph-rs -e normal,build --locked`, saving each output to a file under the worktree's
  `target/` directory, and diff base against `HEAD`. For the base, check it out detached in a second worktree,
  `git worktree add --detach <the lead's worktree directory>/bevy-crate-base "$(git merge-base HEAD origin/main)"`, run
  the two commands there (one at a time; `cargo tree` builds nothing), then `git worktree remove` exactly that path.
  Expected: identical. If a crate shared with Bevy moved to a newer
  version, the diff shows only that version change: list it in the commit message (newest versions are the rule),
  and nothing else may differ. Then `cargo check -p hypergraph-rs --no-default-features --locked`. Expected: PASS.
  And `git diff "$(git merge-base HEAD origin/main)" --stat -- crates/hypergraph-rs/` prints nothing (the trailing
  slash keeps the pathspec to the core crate's directory).

- [ ] **Step 7: Commit** `feat(bevy): a Bevy crate with its own floor, gate and CI job; the palette and the styles`.

---

### Task 2: Paths, hatching and ribbons

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/lines.rs`; Modify: `src/lib.rs` (module, re-exports)
- Test: `crates/hypergraph-rs-bevy/tests/test_lines.rs` (Create)

**Interfaces:**
- Consumes: `Point`, `ClosedPath`, `Outline` (geometry plan); `FieldError`, `HatchDirection` (Task 1).
- Produces: `plan_to_scene`, `outline_paths`, `hatch_segments`, `dashed`, `ribbon_mesh`.

**Decisions:**
- *`outline_paths`:* `Ring { center, radius }` gives one path of `segments` vertices, counter-clockwise, vertex `k` at
  angle `2πk / segments` from `+x`. `Capsule { a, radius_a, b, radius_b }` gives one counter-clockwise path: with
  `d = |b − a|`, if `d <= |radius_a − radius_b|` (one circle holds the other, `d == 0` included) it is the ring of the
  larger radius round its own centre; otherwise, with `u = (b − a) / d`, `v = (−u.y, u.x)`,
  `s = (radius_a − radius_b) / d`, `c = sqrt(1 − s²)`, the two tangent normals are `n₊ = s·u + c·v` and
  `n₋ = s·u − c·v`. The path is, in order: the arc round `b` from `b + radius_b·n₋` to `b + radius_b·n₊` through
  `b + radius_b·u`; the side to `a + radius_a·n₊`; the arc round `a` to `a + radius_a·n₋` through `a − radius_a·u`;
  the side back to the start.
  Each arc has `segments / 2` steps, made by rotating the start normal by a fixed angle step (one `cos` and one `sin`
  of the step, computed once). `Membrane { paths }` gives the paths unchanged. Any other variant (the enum is
  `#[non_exhaustive]`) is `UnsupportedOutline`. `segments < 8` is `BadParameter { name: "segments" }`.
- *`hatch_segments` (no triangulation):* direction vectors are the literals `Deg0 (1, 0)`,
  `Deg45 (0.7071067811865476, 0.7071067811865476)`, `Deg90 (0, 1)`, `Deg135 (−0.7071067811865476,
  0.7071067811865476)`; the normal is `n = (−d.y, d.x)`. Hatch lines sit at `n·x = k·spacing` for whole `k`, so the
  hatching is fixed to the origin and does not shimmer as a shape changes. For every edge `(p, q)` of every path, with
  `sp = n·p`, `sq = n·q`, `sp != sq`: the edge crosses line `k` when `min(sp, sq) <= k·spacing < max(sp, sq)`
  (half-open, so a vertex on a line is counted once); the pair `(k, t)`, with `t` the crossing's coordinate along
  `d`, goes into one `Vec`. Edge by edge, only the lines an edge crosses are visited, so the cost is the number of
  edges plus the number of crossings (sorted once), never edges × lines, and no array is sized by the number of
  lines (a tiny `spacing` over a large shape must not allocate one slot per line). The pairs are sorted by `k`, then
  by `t` with `total_cmp`; within each `k` they are paired `(0,1), (2,3), …` (even-odd, so holes stay empty and two
  sets' hatchings never cancel). A stroke's end is `t·d + (k·spacing)·n`, so its offset across the lines is the same
  bits for every shape. A stroke shorter than `1e-9 × spacing` is
  dropped. Output order: by `k`, then along `d`. `spacing` not finite or not above zero gives an empty list.
- *`dashed`:* `None` gives every edge of the closed path in order. `Some((on, off))` walks the closed path from its
  first vertex and keeps the stretches `[j·(on+off), j·(on+off) + on)`; a dash crossing a vertex is split there.
- *`ribbon_mesh`:* `TriangleList`, indexed `u32`. Each segment `(p, q)` becomes one flat quad in the plane at
  `height`: four vertices `plan_to_scene(p ± w·m)`, `plan_to_scene(q ± w·m)` with `m` the unit normal of `q − p` in
  the plan and `w = width / 2`, then six indices. Normals all `[0, 1, 0]`. Segments shorter than `1e-9` are skipped.
  `RenderAssetUsages::default()`.

- [ ] **Step 1: Write the failing tests** in `tests/test_lines.rs`:

  ```rust
  #[test] fn plan_points_land_with_y_reversed() { assert_eq!(plan_to_scene((2.0, 3.0), 1.5), Vec3::new(2.0, 1.5, -3.0)); }
  #[test] fn a_ring_is_a_counter_clockwise_polygon() {
      // Ring { center: (0,0), radius: 1 }, segments 8: one path of 8 vertices; vertex 0 == (1, 0) and vertex 2 ==
      // (0, 1) within 1e-12; signed_area > 0
  }
  #[test] fn a_capsule_is_two_ends_and_two_sides() {
      // Capsule { a: (0,0), radius_a: 1, b: (4,0), radius_b: 1 }, segments 48: one path, signed_area within 1% of
      // 8.0 + PI; every vertex within 1e-9 of distance 1 from the segment ab
  }
  #[test] fn a_capsule_inside_one_end_is_that_end() {
      // a (0,0) r 2, b (0.5,0) r 1 -> equal (vertex by vertex) to outline_paths(Ring { center: (0,0), radius: 2 }, 48)
      // a == b, r 1 and 1 -> one ring of radius 1 round (0,0)
  }
  #[test] fn hatching_fills_a_square_and_leaves_its_hole() {
      // square (0,0),(2,0),(2,2),(0,2), Deg0, spacing 0.5 -> 4 strokes, each from x 0 to x 2, at y 0, 0.5, 1, 1.5
      // same square plus hole (0.5,0.5),(0.5,1.5),(1.5,1.5),(1.5,0.5): the stroke at y 0.5 is two strokes,
      // [(0,0.5),(0.5,0.5)] and [(1.5,0.5),(2,0.5)]
  }
  #[test] fn a_vertex_on_a_hatch_line_is_counted_once() {
      // diamond (1,0),(2,1),(1,2),(0,1), Deg0, spacing 1.0 -> exactly one stroke, ((0,1),(2,1)) within 1e-12
  }
  #[test] fn two_directions_cross() {
      // square above, Deg45 and Deg135 -> both non-empty; every stroke of each lies inside the square (endpoints
      // within 1e-9 of [0,2]x[0,2])
  }
  #[test] fn hatching_stays_on_its_lattice_when_the_shape_grows() {
      // square side 2 and side 3 (same corner at 0,0), Deg0 spacing 0.5: the strokes of the first are at y values
      // that are all among the second's, compared by to_bits()
  }
  #[test] fn dashes_split_a_path_by_length() {
      // square side 2 (perimeter 8), dash (1, 1) -> 4 segments, each of length 1 within 1e-12; None -> 4 edges
  }
  #[test] fn a_ribbon_is_one_flat_quad_per_segment() {
      // [((0,0),(2,0)), ((1,1),(1,1))], width 0.5, height 3.0 -> 4 positions, 6 indices; positions ==
      // {(0,3,±0.25), (2,3,±0.25)} as a set; every normal == [0,1,0]
  }
  #[test] fn an_unknown_outline_or_few_segments_is_refused() { /* segments 4 -> BadParameter { name: "segments" } */ }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_lines`.
  Expected: FAIL to compile, "cannot find function `outline_paths`".

- [ ] **Step 3: Implement** the five functions in `lines.rs` by the decisions above and re-export them.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate, then commit.** `cargo fmt --all -- --check`;
  `cargo clippy -p hypergraph-rs-bevy --all-targets --locked -- -D warnings`;
  `RUSTDOCFLAGS='-D warnings' cargo doc -p hypergraph-rs-bevy --no-deps --locked`. Commit
  `feat(bevy): outline paths, hatching fixed to its lattice, dashes and ribbons`.

---

### Task 3: Glyph meshes

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/glyph.rs`; Modify: `src/lib.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_glyph.rs` (Create)

**Interfaces:**
- Consumes: `Glyph` (Task 1).
- Produces: `glyph_mesh(glyph: Glyph, segments: u32) -> Mesh`.

**Decisions:**
- Every glyph is flat in the plane `y = 0`, centred on the origin, of unit size, normals `[0, 1, 0]`, indexed
  `TriangleList`, `RenderAssetUsages::default()`. The caller's `size` is the entity's uniform scale.
- `Disc`: centre plus `segments` rim vertices at radius 1. `Ring`: rims at radius 0.6 and 1. `Square`: corners at
  `(±1, 0, ±1)`. `SquareFrame`: outer corners `±1`, inner `±0.6`. `Streak`: the rectangle `x ∈ [−1, 0]`,
  `z ∈ [−0.125, 0.125]`, so its head is at the origin and it trails behind; the plugin turns `+x` to the direction of
  motion. Rim angles as in `outline_paths` (Task 2). `segments < 8` is taken as 8.
- A full glyph and its hollow pair (`Disc`/`Ring`, `Square`/`SquareFrame`) have the same outer size, so a hollow mark
  reads as "the same thing, not counted".

- [ ] **Step 1: Write the failing tests** in `tests/test_glyph.rs`:

  ```rust
  // helper: positions(mesh) -> Vec<[f32; 3]>, normals(mesh) -> Vec<[f32; 3]>
  #[test] fn every_glyph_is_flat_and_faces_up() {
      // for each of the five glyphs at segments 16: every position.y == 0.0; every normal == [0,1,0];
      // index count is a positive multiple of 3
  }
  #[test] fn a_disc_and_its_ring_have_the_same_outer_size() {
      // max distance from the origin in xz == 1.0 within 1e-6 for Disc and Ring; min for Ring == 0.6 within 1e-6
  }
  #[test] fn a_square_and_its_frame_have_the_same_outer_size() {
      // Square and SquareFrame: max |x| == 1 and max |z| == 1; SquareFrame has a vertex at (0.6, 0, 0.6)
  }
  #[test] fn a_streak_trails_behind_its_head() { /* Streak: max x == 0.0, min x == -1.0, max |z| == 0.125 */ }
  #[test] fn the_same_glyph_gives_the_same_bytes_twice() { /* Ring at 32: positions equal by to_bits() */ }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_glyph`.
  Expected: FAIL to compile, "cannot find function `glyph_mesh`".

- [ ] **Step 3: Implement `glyph_mesh`** in `glyph.rs` and re-export it.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): flat glyph meshes for particles and
  marks`.

---

### Task 4: The ground in two forms

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/ground.rs`; Modify: `src/lib.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_ground.rs` (Create)

**Interfaces:**
- Consumes: `Point`, `ClosedPath` (geometry plan); `FieldError` (Task 1); `plan_to_scene` (Task 2).
- Produces: `RegionShape`, `GroundForm`, `Ground`, `GroundTopology` and its methods, as in "The public API".

**Decisions (design 6.3 and 12: outlines never move in plan, both forms share one set of shapes):**
- *Validation in `new`:* duplicate ids (`DuplicateId`), a coordinate not finite (`NonFinite { item: region id }`), a
  region with no triangles (`BadShape { problem: "no triangles" }`).
- *Mesh layout, non-indexed `TriangleList` with flat normals and `ATTRIBUTE_COLOR`, regions in input order:*
  - `Blocks`: for each region, its triangles at `y = h` (3 vertices each, in input order), then for each edge of each
    ring a wall from `y = 0` to `y = h` (two triangles, 6 vertices). `h` may be negative: the block sinks and its
    walls go down. Vertex count `Σ (3 × triangles + 6 × ring edges)`.
  - `Surface`: each triangle `(v0, v1, v2)` is split at its centroid `c` into `(v0, v1, c)`, `(v1, v2, c)`,
    `(v2, v0, c)`: 9 vertices per triangle. A corner's height is the mean of the heights of the regions that have a
    triangle vertex at exactly that plan position (key: the two `f64::to_bits`, each region counted once); the
    centroid's height is its own region's. So shared borders meet without a step, each region rises toward its own
    height, and the outline in plan is the same as in `Blocks`. Positions that are not bit-equal are not joined.
  - Colour: every vertex of a region carries that region's colour (no blending of colour across borders).
  - Normals are recomputed on every write, per triangle; a degenerate triangle gets `[0, 1, 0]`.
- *Cost:* `new` builds the corner-to-regions table once (sorted keys, then a compact list per corner); every write
  is linear in the number of vertices.
- *Outline mesh:* `LineList`, two vertices per ring edge, at the corner height (the `Surface` rule, or the block's
  `h` in `Blocks`) plus `lift`; a ring vertex that is no triangle vertex of its region takes its region's height.
- `write_body` and `write_outline` overwrite positions, normals and colours of a mesh made by `body_mesh` /
  `outline_mesh` of the same topology, in place. They panic, and say so under `# Panics`, if `heights` or `colours`
  is not `region_count()` long.

- [ ] **Step 1: Write the failing tests** in `tests/test_ground.rs`, on two unit squares: `a` with triangles
  `[(0,0),(1,0),(1,1)]`, `[(0,0),(1,1),(0,1)]` and ring `[(0,0),(1,0),(1,1),(0,1)]`; `b` the same shifted by `+1` in x.

  ```rust
  #[test] fn blocks_rise_and_sink_with_walls() {
      // Blocks, heights [2.0, -1.0]: 60 vertices; a's first 6 vertices all y == 2.0; b's first 6 all y == -1.0;
      // every wall vertex of b has y in {0.0, -1.0}
  }
  #[test] fn the_surface_meets_at_shared_corners() {
      // Surface, heights [2.0, -1.0]: 36 vertices; every vertex at plan (1,0) or (1,1) has y == 0.5;
      // every vertex at plan (0,0) has y == 2.0; every centroid vertex of a has y == 2.0
  }
  #[test] fn both_forms_have_the_same_outline_in_plan() {
      // outline_mesh of each form, same heights: the (x, z) pairs of every vertex are equal, in order, by to_bits()
  }
  #[test] fn each_region_has_its_own_colour() {
      // colours [red, blue]: the first 30 Blocks vertices carry red, the rest blue (as [f32; 4] linear)
  }
  #[test] fn writing_in_place_equals_building_anew() {
      // body_mesh(h1, c1), then write_body(&mut m, h2, c2): positions, normals and colours equal body_mesh(h2, c2)'s
      // by to_bits(), for both forms
  }
  #[test] fn region_lookup_is_by_id() { /* region_index("b") == Some(1); region_index("z") == None */ }
  #[test] fn bad_shapes_are_refused() {
      // two regions "a" -> DuplicateId("a"); a NaN coordinate in "a" -> NonFinite { item: "a" };
      // a region with no triangles -> BadShape { region: "a", problem: "no triangles" }
  }
  #[test] fn a_grid_of_3168_regions_builds_in_linear_time() {
      // 72 x 44 unit squares, two triangles each, Surface: new() and body_mesh() succeed;
      // vertex count == 3168 * 18
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_ground`.
  Expected: FAIL to compile, "cannot find type `GroundTopology`".

- [ ] **Step 3: Implement** `ground.rs` by the decisions above and re-export its items.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): a ground of regions, as one bending
  surface or as blocks that rise and sink`.

---

### Task 5: Which flows are drawn, and how marks move in a beat

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/flows.rs`, `src/motion.rs`; Modify: `src/lib.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_flows.rs`, `tests/test_motion.rs` (Create)

**Interfaces:**
- Consumes: `StreamStyle`, `Motion`, `FieldError` (Task 1).
- Produces: `Flow`, `DrawnFlow`, `StreamReport`, `FlowReport`, `select_flows`; `smoothstep`, `mark_start`,
  `mark_position`, `Phase`, `BeatClock`, `eased`.

**Decisions (design 6.2, 6.4, 11 "nothing dropped in silence"):**
- *Amounts are integers* in each stream's own smallest unit (`u64`), summed in `u128`. A mark stands for `unit` of its
  stream. No code relates two streams.
- *`select_flows`*, for each stream index in order and, within it, `hollow == false` then `true` (one `StreamReport`
  each, always, even with no flows):
  - `total` = the sum of every flow's amount of that stream and variant; `paths` = their number.
  - A flow whose `from` and `to` are equal in `x` and `z` is never drawn (it counts in `total`, not in `drawn`).
  - The rest are sorted by amount, largest first, ties by input index. `target = ceil(share_drawn × total)` (in
    `f64`, then capped at `total`). Walking that order: stop when `drawn >= target`; a flow gives
    `n = min(amount / unit, max_marks − marks)` marks; skip it if `n == 0` and `marks < max_marks`; stop if
    `marks == max_marks`; else `drawn += n × unit`, `marks += n`, `paths_drawn += 1`, and record a `DrawnFlow`.
  - `not_drawn = total − drawn`, exactly.
  - `DrawnFlow`s are listed in the order they were chosen, stream by stream.
  - Errors: a stream index not in `streams` is `UnknownStyle { what: "stream" }`; an endpoint not finite is
    `NonFinite { item: "flow #<index>" }`.
- *`mark_start(flow, k, n, beat, travel)`:* with `B = beat`, `τ = travel × B`, and a phase
  `φ = (fnv1a(&(flow as u64).to_le_bytes()) % 1024) as f32 / 1024.0` (64-bit FNV-1a, offset 0xcbf29ce484222325,
  prime 0x100000001b3), mark `k` of `n` starts at `((k as f32 + φ) / n as f32) × (B − τ)`. So marks of one flow are
  evenly spaced through the beat, flows do not start in step, and every mark arrives before the beat ends.
- *`mark_position(from, to, u, bow, side)`:* `s = smoothstep(u)`; with `δ = to − from`, `L` the length of `δ` in the
  `xz` plane, and `p = (−δ.z, 0, δ.x) / L` (zero when `L == 0`), the position is
  `from + δ × s + p × (bow × L × side × 4u(1 − u))`. The bow is a parabola in `u`, not a sine. `side` is `+1` for even
  `k`, `−1` for odd, so two marks of one path do not ride the same line.
- *`smoothstep(x)`* = `x²(3 − 2x)` with `x` clamped to `[0, 1]`.
- *`BeatClock`:* `start(now, period)` records the start and the period. `phase(now, beats)` is `Resting` when never
  started or when `now − start >= Σ beats`; otherwise the beat holding `now − start` and the fraction into it.
- *`eased(from, to, phase, ease_beat, motion)`:* `Resting` gives `to`; a beat before `ease_beat` gives `from`; a beat
  after it gives `to`; during it, `Full` gives `from + (to − from) × smoothstep(fraction)` and `Reduced` gives `from`.

- [ ] **Step 1: Write the failing tests.** In `tests/test_flows.rs`, with one stream of `unit` 10,
  `share_drawn` 0.95 and `max_marks` 1000 unless stated, flows from `(0,0,0)` to `(1,0,0)`:

  ```rust
  #[test] fn paths_are_drawn_largest_first_until_the_share_is_shown() {
      // amounts [10, 500, 40, 300, 150] -> drawn flows in order: indices 1, 3, 4 with marks 50, 30, 15;
      // report: total 1000, drawn 950, not_drawn 50, marks 95, paths 5, paths_drawn 3
  }
  #[test] fn less_than_one_mark_is_counted_not_drawn() {
      // share 1.0, amounts [25, 7] -> marks 2 then nothing; total 32, drawn 20, not_drawn 12
  }
  #[test] fn the_mark_cap_stops_drawing_and_counts_the_rest() {
      // unit 1, max_marks 5, amounts [10, 10] -> one DrawnFlow with 5 marks; drawn 5, not_drawn 15
  }
  #[test] fn streams_are_reported_apart() {
      // stream 0 unit 1, stream 1 unit 1000; amounts 7 (stream 0) and 11_000 (stream 1), share 1.0:
      // report.streams has 4 entries (two streams x full, hollow); totals [7, 0, 11_000, 0] in that order
  }
  #[test] fn hollow_flows_are_their_own_count() {
      // amounts 30 full, 20 hollow, share 1.0 -> full: total 30, marks 3; hollow: total 20, marks 2;
      // the hollow DrawnFlow has hollow == true
  }
  #[test] fn huge_amounts_do_not_overflow() {
      // unit 1 << 40, two flows of u64::MAX -> total == 2 * (u64::MAX as u128); drawn + not_drawn == total
  }
  #[test] fn a_flow_into_its_own_place_is_counted_not_drawn() {
      // from == to == (3, 1, 2), amount 100, share 1.0 -> total 100, drawn 0, marks 0, paths_drawn 0
  }
  #[test] fn drawn_plus_not_drawn_is_the_total_for_any_input() {
      // 2_000 flows from a fixed LCG (seed 20261121), amounts 0..5_000, two streams, both variants:
      // for every report, drawn + not_drawn == total and drawn == marks as u128 * unit
  }
  #[test] fn bad_flows_are_refused() {
      // stream 3 of 1 -> UnknownStyle { what: "stream", index: 3, len: 1 };
      // from (NaN, 0, 0) at index 0 -> NonFinite { item: "flow #0" }
  }
  ```

  In `tests/test_motion.rs`:

  ```rust
  #[test] fn smoothstep_holds_its_ends() { /* (−1)=0, 0=0, 0.5=0.5, 1=1, 2=1 */ }
  #[test] fn a_mark_starts_at_from_and_ends_at_to() {
      // from (0,0,0), to (10,0,0), bow 0.08, side 1: u 0 -> from exactly; u 1 -> to exactly;
      // u 0.5 -> (5, 0, 0.8) within 1e-5; side -1 -> (5, 0, -0.8)
  }
  #[test] fn marks_of_one_flow_spread_through_their_beat() {
      // n 4, beat 1.2 s, travel 0.4: starts for k 0..4 strictly increase; all in [0, 0.72 s]
      // flows 0 and 1 with n 1 start at different times
  }
  #[test] fn the_clock_names_the_beat_and_rests_after() {
      // beats [1 s; 3], start at 10 s: phase(10.5 s) == Beat { index: 0, fraction: 0.5 };
      // phase(12.25 s) == Beat { index: 2, fraction: 0.25 }; phase(13 s) == Resting; never started == Resting
  }
  #[test] fn values_ease_only_in_their_beat() {
      // eased(2, 4, Beat{0, 0.9}, 2, Full) == 2; eased(2, 4, Beat{2, 0.5}, 2, Full) == 3;
      // eased(2, 4, Beat{2, 0.5}, 2, Reduced) == 2; eased(2, 4, Resting, 2, Reduced) == 4
  }
  ```

- [ ] **Step 2: Run and see them fail.**
  `cargo test -p hypergraph-rs-bevy --locked --test test_flows --test test_motion`.
  Expected: FAIL to compile, "cannot find function `select_flows`".

- [ ] **Step 3: Implement** `flows.rs` and `motion.rs` by the decisions above and re-export their items.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): choose the flows to draw, count the
  rest, and move marks through their beat`.

---

### Task 6: The frame, and sets read from a hypergraph

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/frame.rs`; Modify: `src/lib.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_frame.rs` (Create)

**Interfaces:**
- Consumes: `Hypergraph::members_with_data`, `Positions`, `OutlineParams`, `Outline`, `edge_outline`,
  `GeometryError` (core and geometry plan); `FieldStyles`, `FieldError` (Task 1); `Flow` (Task 5).
- Produces: `RegionReading`, `ParticleFrame`, `SetFrame`, `FieldFrame` (with `validate`), `set_frames`, `SetChange`,
  `diff_sets`.

**Decisions (design 6.7, and design 8 "tells a change of membership from a change of number"):**
- *`FieldFrame::validate`* checks, in this order, and returns the first error: region ids unique
  (`DuplicateId`), heights finite (`NonFinite { item: region id }`), tones finite (`NonFinite { item: region id }`)
  and in `[-1, 1]` (`ToneOutOfRange`); particle ids unique, positions finite (`NonFinite { item: particle id }`),
  `style` known (`UnknownStyle { what: "particle style" }`); set ids unique, `height`, every member's number, every
  outline coordinate and radius, and `held_share` finite (`NonFinite { item: set id }`: a caller may build a
  `SetFrame` by hand, not through `set_frames`), `style` known (`what: "set style"`); flows as `select_flows` checks
  them. Regions are checked against the grounds by the plugin (Task 7), not here.
- *`set_frames`*: for each `(edge id, style)` in order: `members` is every bare member of the edge, in
  `members_with_data` order, with `weight(&value)`, zeros included; `outline` and `held_share` come from
  `edge_outline(h, id, positions, &weight, params)`: `None` and `None` when it returns `Ok(None)`, else
  `Some(outline.outline.outline)` and `Some(held_weight / total_weight)`. An unknown edge is
  `FieldError::Geometry(GeometryError::UnknownEdge(id))`.
- *`diff_sets`*: first, in `next`'s order, `Appeared` for a set not in `previous` and `Changed` for one in both whose
  `joined`, `left` or `renumbered` is not empty; then, in `previous`'s order, `Vanished` for a set not in `next`.
  Membership is the bare membership: `joined` lists ids in `next` and not in `previous` (in `next`'s order), `left`
  the reverse (in `previous`'s order), `renumbered` the ids in both whose number differs by `to_bits` (in `next`'s
  order). A member whose number went from 1 to 0 is `renumbered`, not `left`: the bare membership stands. Lookups use
  a `BTreeMap<&str, usize>` built once per set, so the cost is linear in the members.

- [ ] **Step 1: Write the failing tests** in `tests/test_frame.rs`, on `Hypergraph<(), (), f64>` and
  `OutlineParams::new(1.0, 0.125, 0.9)`:

  ```rust
  #[test] fn set_frames_read_every_membership_and_the_outline() {
      // edge "club" {a: 2.0, b: 1.0, z: 0.0}; positions a (0,0), b (5,0); set_frames(&h, &[("club", 1)], ..., 0.5):
      // one SetFrame { id: "club", style: 1, height: 0.5, members: [("a",2.0),("b",1.0),("z",0.0)] };
      // outline == Some(Capsule { a: (0,0), radius_a: 1.0, b: (5,0), radius_b: 0.75 }); held_share == Some(1.0)
  }
  #[test] fn a_set_of_zeros_has_no_outline() { /* {a: 0, b: 0} -> outline None, held_share None, members kept */ }
  #[test] fn an_unknown_edge_is_refused() { /* "nope" -> Err(Geometry(UnknownEdge("nope"))) */ }
  #[test] fn a_change_of_number_is_not_a_change_of_membership() {
      // previous club {a:1, b:1}; next club {a:1, b:0}:
      // [Changed { set: "club", joined: [], left: [], renumbered: ["b"] }]
  }
  #[test] fn joins_leaves_appearances_and_vanishings_are_told_apart() {
      // previous [club {a,b}, team {c}]; next [club {b,d}, crew {e}]:
      // [Changed { set: "club", joined: ["d"], left: ["a"], renumbered: [] }, Appeared { set: "crew" },
      //  Vanished { set: "team" }] — in exactly this order
  }
  #[test] fn an_unchanged_set_reports_nothing() { /* same frames -> [] */ }
  #[test] fn a_frame_names_what_is_wrong_with_it() {
      // two particles "p1" -> DuplicateId("p1"); tone 1.5 on "r0" -> ToneOutOfRange { region: "r0", value: 1.5 };
      // particle style 9 of 2 -> UnknownStyle { what: "particle style", index: 9, len: 2 };
      // a set height NaN on "club" -> NonFinite { item: "club" };
      // a hand-built SetFrame "club" with member ("a", f64::INFINITY) -> NonFinite { item: "club" }
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_frame`.
  Expected: FAIL to compile, "cannot find type `FieldFrame`".

- [ ] **Step 3: Implement** `frame.rs` by the decisions above and re-export its items.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): the period's frame, sets read from a
  hypergraph, and changes of membership told from changes of number`.

---

### Task 7: The plugin, the ground and the particles

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/plugin/mod.rs`, `src/plugin/ground.rs`, `src/plugin/particles.rs`;
  Modify: `src/lib.rs`
- Test: `crates/hypergraph-rs-bevy/tests/common/mod.rs`, `tests/test_plugin_ground.rs` (Create)

**Interfaces:**
- Consumes: everything of Tasks 1 to 6.
- Produces: `FieldPlugin` (with `new`), `FieldSystems`, `ShowPeriod`, `ReplayPeriod`, `FrameRefused`, `SetChanged`,
  `ParticleId`, `SetId`, `MarkOf`, `PathLines`, `GroundPart`; the resources `FieldConfig`, `FieldStyles`, `Palette`,
  `BeatClock`, `FlowReport` inserted by the plugin. Test helpers in `tests/common/mod.rs`:
  `pub fn field_app(config: FieldConfig, styles: FieldStyles, palette: Palette) -> App`,
  `pub fn advance_to(app: &mut App, at: Duration)` (updates until `Time::elapsed() >= at`),
  `pub fn snapshot(app: &mut App) -> Vec<u8>` (a byte record of every `ParticleId`, `SetId`, `MarkOf`, `PathLines`
  and `GroundPart` entity, sorted by its key, with its `Transform` and `Visibility`, and the positions and colours of
  every mesh those entities and their children hold (a set's line and hatch are children of its `SetId` entity), all
  floats as `to_bits`; `Entity` ids are never written, so two apps compare).

**Decisions:**
- *`FieldPlugin::build`* inserts the three resources it was given, `BeatClock::default()` and `FlowReport::default()`;
  adds the four messages; configures `(FieldSystems::Install, FieldSystems::Animate).chain()` in `Update`; adds the
  systems as listed in "Systems and their order". It does not add or init any Bevy plugin or asset type: a full app
  has them from Bevy's own plugins, and the test app inits them itself.
- *Materials:* a private resource holds one unlit `StandardMaterial` per swatch (`unlit: true`, `cull_mode: None`,
  `double_sided: true`, `alpha_mode: AlphaMode::Opaque`) and one lit ground material (`base_color: Color::WHITE`,
  `perceptual_roughness: 0.78`, `metallic: 0.08`, `cull_mode: None`, `double_sided: true`). `build` cannot make
  assets (the asset types are not yet registered then), so the first run of `refresh_materials` creates this
  resource, and every later run changes the colours in place when `Palette` changed; handles never change. One mesh
  handle per `Glyph`, made once in the same place as `glyph_mesh(glyph, 16)` (16 rim steps: a disc of 17 vertices,
  so 25,000 particles stay under half a million vertices). So every particle and mark of one style shares one mesh
  and one material, which lets Bevy batch them.
- *`build_grounds`:* for a new or changed `Ground`, build its `GroundTopology` and two children, `GroundPart::Body`
  (`Mesh3d` of `body_mesh`, the ground material) and, if `styles.region_outline` is `Some`, `GroundPart::Outline`.
  A changed `form` replaces the meshes, keeping the current heights and colours. A shape error is logged with
  `error!` naming the ground's entity, and that ground draws nothing.
- *`install_frame`:* takes the last `ShowPeriod` of the update (earlier ones that update are dropped, and that is
  stated in the rustdoc). It checks `frame.validate(&styles)`, then that every region id is in some ground
  (`UnknownRegion`); on error it sends `FrameRefused { period, error }` and changes nothing. On success: the frame
  that was current becomes the previous one, taken at its settled values (so a frame that arrives during the beats
  restarts from the last settled picture); the new one becomes current; the clock starts at `Time::elapsed()`.
  `ReplayPeriod` restarts the clock with the same previous and current frames; with no current frame it does nothing.
  A `ShowPeriod` and a `ReplayPeriod` in the same update: the frame is installed and the replay is dropped. Regions
  not in a frame have no reading: height 0, `palette.tone(None)`; a reading whose `height` is `None` stands at 0, and
  one whose `tone` is `None` takes `palette.tone(None)`. The first accepted frame has no previous one: its previous
  picture is every region at height 0 with `palette.tone(None)`, and no particles or sets.
- *Motion is fixed per period:* `config.motion` is read when a frame or a replay is accepted and kept with the clock
  for that period's beats; every animate system uses the kept value, so switching it mid-beats changes nothing until
  the next frame or replay (Task 9 does the same for marks).
- *`animate_ground`:* each region's height and colour are `eased` between previous and current (colours component by
  component in linear RGBA); the meshes are written with `write_body` / `write_outline` only on updates where the
  phase is not `Resting`, on the first `Resting` update after beats, or when `Palette` changed (so a new palette
  recolours a resting ground on the next update; that is the caller's change, not motion).
- *Particles:* one entity per id, `(ParticleId, Mesh3d, MeshMaterial3d<StandardMaterial>, Transform)`, scale = the
  style's `size`. A new id is spawned at the start of `ease_beat`, already at its new position. An id that left is
  despawned at the end of `ease_beat`. A kept id eases from its previous to its current position (each coordinate by
  `eased`); a changed style swaps its mesh and material at the start of `ease_beat`.
- *Nothing while resting:* every animate system returns at once when the phase is `Resting` and was `Resting` on its
  previous run (a `Local<bool>`), except for the `Palette` case above.

- [ ] **Step 1: Write the failing tests** in `tests/test_plugin_ground.rs`, with the two squares of Task 4 as one
  `Ground`, `FieldConfig::three_beats(1 s)`, two particle styles (`Disc` and `Ring`) and the palette of Task 1.
  "a's top vertex y" is the largest `y` among the first 18 positions of the `Body` mesh (region `a`'s, in `Surface`):
  `a`'s corners `(0,0)` and `(0,1)` and its centroids carry its own height; its corners shared with `b` carry the mean.

  ```rust
  #[test] fn the_plugin_runs_without_a_window() { /* field_app(..); three updates; no panic */ }
  #[test] fn a_ground_is_built_once_and_its_form_switches() {
      // after one update: one GroundPart::Body child whose mesh has 36 vertices (Surface);
      // set form = Blocks: after one update, 60 vertices; GroundPart::Outline (x, z) pairs unchanged by to_bits()
  }
  #[test] fn heights_ease_in_the_ease_beat_and_rest_after() {
      // ShowPeriod a: 2.0; advance_to(4 s): a's top vertex y == 2.0
      // ShowPeriod a: 4.0 at t0: at t0 + 1.5 s (beat 1) a's top y == 2.0; at t0 + 2.5 s (ease beat, half) == 3.0
      // within 1e-5; at t0 + 3.5 s == 4.0; then 100 more updates: snapshot unchanged
  }
  #[test] fn no_reading_is_gray_and_flat() {
      // GroundForm::Blocks (in Surface, b's corners shared with a stand at the mean, 1.0, by design);
      // a frame with only "a" (height 2, tone 1): at rest, b's 30 vertices have y == 0 and colour == palette.tone(None)
  }
  #[test] fn the_last_frame_of_an_update_wins() {
      // two ShowPeriod in one update, periods 1 (a = 2) and 2 (a = 4), and a ReplayPeriod: no FrameRefused;
      // BeatClock::period() == Some(2); at rest a's top y == 4.0
  }
  #[test] fn motion_is_fixed_when_a_frame_is_accepted() {
      // Full motion; frame a = 4 at t0 (settled a = 2); at t0 + 2.2 s set FieldConfig.motion = Reduced:
      // at t0 + 2.5 s a == 3.0 within 1e-5 (still eased)
  }
  #[test] fn particles_arrive_move_once_and_leave() {
      // frame 1: p1 (0,0,0), p2 (1,0,0); settle. frame 2: p1 (2,0,0), p3 (5,0,0) at t0:
      // at t0 + 1.5 s: p1.x == 0, p3 absent, p2 present; at t0 + 2.5 s: p1.x == 1.0 within 1e-5, p2 and p3 present;
      // at rest: ParticleId set == {p1, p3}, p1.x == 2.0
  }
  #[test] fn particles_share_meshes_and_materials() {
      // 25_000 particles alternating styles 0 and 1: 25_000 ParticleId entities; distinct Mesh3d handles == 2;
      // distinct material handles == number of distinct swatches used
  }
  #[test] fn a_bad_frame_is_refused_and_the_picture_stays() {
      // settled frame 1; then a frame with "p1" twice: one FrameRefused { period, error: DuplicateId("p1") };
      // snapshot after 50 updates == snapshot before; a frame naming region "zz" -> UnknownRegion("zz")
  }
  #[test] fn a_frame_during_the_beats_restarts_from_the_last_settled_picture() {
      // settled a = 2; frame a = 4 at t0; at t0 + 2.5 s a == 3.0; then frame a = 10 at t1:
      // on the next update (beat 0) a == 4.0; at t1 + 2.5 s a == 7.0 within 1e-5
  }
  #[test] fn replay_plays_the_last_move_again() {
      // settled after frame a = 4 (previous a = 2); ReplayPeriod at t2: at t2 + 2.5 s a == 3.0; at rest 4.0
  }
  #[test] fn the_palette_is_the_callers() {
      // settled with a tone 1.0; replace Palette's ramp[2] with green: after one update a's vertex colour == green
      // in linear, and the swatch materials' base_color follow their swatches
  }
  #[test] fn reduced_motion_jumps_at_the_end_of_the_ease_beat() {
      // Motion::Reduced: at t0 + 2.5 s a == 2.0; at t0 + 3.0 s and after, a == 4.0
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_plugin_ground`.
  Expected: FAIL to compile, "cannot find type `FieldPlugin`".

- [ ] **Step 3: Implement** the plugin, `install_frame`, `refresh_materials`, `build_grounds`, `animate_ground` and
  `animate_particles` by the decisions above; `animate_sets` and `animate_marks` are registered in Tasks 8 and 9.
  Write `tests/common/mod.rs`.

- [ ] **Step 4: Run and see them pass.** The same command. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): the field plugin, its clock, the
  ground and the particles`.

---

### Task 8: Sets in the plugin

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/plugin/sets.rs`; Modify: `src/plugin/mod.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_plugin_sets.rs` (Create)

**Interfaces:**
- Consumes: `outline_paths`, `hatch_segments`, `dashed`, `ribbon_mesh` (Task 2); `SetFrame`, `diff_sets` (Task 6);
  the plugin and `tests/common` (Task 7).
- Produces: set entities `(SetId, Transform, Visibility)` with children: one line ribbon and, when the style has a
  hatch, one hatch ribbon; `SetChanged` messages.

**Decisions:**
- On accepting a frame, `install_frame` sends one `SetChanged` per item of `diff_sets(previous.sets, current.sets)`,
  in that order.
- A set's meshes are built from `outline_paths(outline, config.curve_segments)` at the set's `height`: the line is
  `ribbon_mesh` of the segments of `dashed(path, dash)` of every path, joined in path order, with width
  `style.line.width` at `height`; the hatch is `ribbon_mesh(hatch_segments(paths, hatch.direction, spacing),
  hatch.width, height + 0.001)`. The styles hold `f32` and the line functions take `f64`: `dash` is
  `style.line.dash.map(|(on, off)| (f64::from(on), f64::from(off)))` and `spacing` is `f64::from(hatch.spacing)`.
  Both meshes use the style's swatch material. Hatching is only ever made here: the ground has none.
- Timing: a new set appears at the start of `ease_beat`; a changed set's meshes are swapped at the end of
  `ease_beat`; a vanished set is despawned at the end of `ease_beat`. A set whose `outline` is `None` has no children
  (its `SetId` entity stays, so the caller can say on screen that it holds nothing). Meshes are rebuilt only for sets
  whose `SetFrame` changed (`!=`), so an unchanged set costs nothing.

- [ ] **Step 1: Write the failing tests** in `tests/test_plugin_sets.rs`, with set style 0 hatched `Deg45` and set
  style 1 plain with a dash:

  ```rust
  #[test] fn a_set_gets_a_line_and_a_hatch() {
      // one SetFrame "club", style 0, Ring { center: (0,0), radius: 1 }: at rest, SetId("club") has two children;
      // every child mesh vertex has y == height or height + 0.001
  }
  #[test] fn a_plain_grouping_gets_only_a_dashed_line() { /* style 1 -> one child */ }
  #[test] fn two_overlapping_sets_both_show_their_hatching() {
      // "club" Deg45 and "team" Deg135, overlapping rings: both have a hatch child with vertices
  }
  #[test] fn set_changes_are_sent_in_order() {
      // frame 1 [club {a,b}, team {c}], frame 2 [club {b,d}, crew {e}]: SetChanged messages after frame 2 ==
      // diff_sets(frame1.sets, frame2.sets), in order
  }
  #[test] fn a_changed_set_swaps_its_meshes_at_the_end_of_the_ease_beat() {
      // line width 0.05; club ring radius 1 then radius 2 at t0: at t0 + 2.5 s the line mesh's max |x| is in
      // [0.99, 1.03]; at t0 + 3.0 s and after, in [1.99, 2.03] (a ribbon's offset follows each segment's normal,
      // so the bound is not exactly radius + width / 2)
  }
  #[test] fn a_set_of_zeros_draws_nothing_and_says_so() {
      // SetFrame "club" with outline None: SetId("club") exists with no children; held_share stays None in the frame
      // then a frame without "club": after the ease beat, no SetId("club")
  }
  #[test] fn an_unchanged_set_is_not_rebuilt() {
      // same SetFrame in frames 1 and 2: the child Mesh3d handles are the same before and after frame 2
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_plugin_sets`.
  Expected: FAIL (assertions: no `SetId` entities yet).

- [ ] **Step 3: Implement** `animate_sets` and the set part of `install_frame` by the decisions above.

- [ ] **Step 4: Run and see them pass.** The same command, then `--test test_plugin_ground`. Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): hatched outlines of sets, swapped once
  a period`.

---

### Task 9: Marks in their beats, reduced motion, and the same picture twice

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/plugin/marks.rs`; Modify: `src/plugin/mod.rs`
- Test: `crates/hypergraph-rs-bevy/tests/test_plugin_marks.rs` (Create)

**Interfaces:**
- Consumes: `select_flows`, `FlowReport`, `mark_start`, `mark_position`, `smoothstep` (Task 5); `glyph_mesh`
  (Task 3); the plugin, its glyph meshes and materials, and `tests/common` (Task 7).
- Produces: mark entities `(MarkOf, Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, Visibility)`;
  path-line entities `(PathLines, Mesh3d, ..., Visibility)`; `FlowReport` written once per accepted frame.

**Decisions:**
- On accepting a frame (or a replay), `install_frame` runs `select_flows`, writes the `FlowReport` (with
  `period: Some(frame.period)`), despawns any marks or path lines left over, and:
  - with `Motion::Full`, spawns one entity per mark of every `DrawnFlow`, hidden, with the stream's `full` or `hollow`
    glyph, scale `size`;
  - with `Motion::Reduced`, spawns per stream one `PathLines` entity, hidden: a ribbon of width `size × 0.25` along
    each drawn flow's bowed path, sampled with `mark_position` at `u = 0, 1/8, …, 1` with `side = +1`. `ribbon_mesh`
    cannot draw it (it lies at one height, and a flow's ends may differ in `y`), so a private
    `fn path_ribbon(paths: &[Vec<Vec3>], width: f32) -> Mesh` in `marks.rs` makes it: one flat quad per pair of
    samples, offset across the segment in `xz` as `ribbon_mesh` does, each corner at its own sample's `y`, normals
    `[0, 1, 0]`, zero-length segments skipped.
- `animate_marks`, during the beat of a mark's stream, at `t = fraction × beat`: with `s = t − mark_start(..)` and
  `τ = mark_travel × beat`, the mark is visible while `0 <= s < τ`, at `mark_position(from, to, s / τ, bow, side)`;
  a `Streak` is turned so its `+x` lies along the motion, from the position at `s/τ` minus the position at
  `(s − τ/50)/τ` (clamped at 0), and keeps its last turn when that difference is zero. Outside its window a mark is
  hidden. A `PathLines` entity is visible only during its stream's beat. When the clock comes to rest, marks and path
  lines are despawned.
- `config.motion` is read when a frame is accepted; a change takes effect with the next frame or replay.

- [ ] **Step 1: Write the failing tests** in `tests/test_plugin_marks.rs`, with three streams on beats 0, 1 and 2
  (units 1, 1 and 100; glyphs `Disc/Ring`, `Square/SquareFrame`, `Streak/Streak`; size 0.2, `share_drawn` 1.0 and
  `max_marks` 1000 each), `three_beats(1 s)`:

  ```rust
  #[test] fn marks_move_only_in_their_own_beat() {
      // flows: stream 0 amount 6, stream 1 amount 4, stream 2 amount 300, each (0,0,0) -> (10,0,0):
      // MarkOf entity count == 6 + 4 + 3; during beat 0 no stream-1 or stream-2 mark is visible and at least one
      // stream-0 mark is; during beat 1 only stream-1 marks are visible; at rest there are no MarkOf entities
  }
  #[test] fn the_report_matches_the_marks() {
      // FlowReport.period == Some(period); for each stream, marks == number of MarkOf entities of that stream
  }
  #[test] fn hollow_flows_get_the_hollow_glyph() {
      // one hollow flow on stream 1 and one full flow on stream 1: every hollow mark's mesh (read from Assets<Mesh>)
      // has the positions of glyph_mesh(Glyph::SquareFrame, 16), every full mark's those of Glyph::Square, and all
      // hollow marks share one Mesh3d handle
  }
  #[test] fn a_visible_mark_is_on_its_path() {
      // every visible stream-0 mark at any update lies within 0.8 + 1e-4 of the segment (0,0,0)-(10,0,0) in xz,
      // with x in [0, 10]
  }
  #[test] fn reduced_motion_draws_still_paths_and_no_marks() {
      // Motion::Reduced: no MarkOf entities; three PathLines entities, each visible only in its stream's beat
  }
  #[test] fn reduced_motion_changes_the_picture_only_at_beats() {
      // Motion::Reduced, two frames: record (phase, snapshot) every update through both periods; every pair of
      // consecutive snapshots that differ has a different phase index (or Resting) on the two sides
  }
  #[test] fn nothing_moves_while_resting() {
      // Full motion, settled: 200 updates, snapshot equal on every one
  }
  #[test] fn the_same_picture_twice() {
      // two apps, the same ground and the same three frames of 500 particles, 3 sets and 300 flows from a fixed LCG,
      // shown at the same updates: snapshots equal on every one of 400 updates
  }
  #[test] fn huge_flows_are_capped_and_counted() {
      // stream 0 max_marks 1000, one flow of u64::MAX: 1000 MarkOf entities; report not_drawn == u64::MAX - 1000
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_plugin_marks`.
  Expected: FAIL (assertions: no `MarkOf` entities yet).

- [ ] **Step 3: Implement** `animate_marks` and the mark part of `install_frame` by the decisions above.

- [ ] **Step 4: Run and see them pass.** The same command, then `--test test_plugin_ground --test test_plugin_sets`.
  Expected: PASS.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): marks that move in their stream's
  beat, still paths for reduced motion`.

---

### Task 10: The frame sampler and the example

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/frame_windows.rs`, `examples/field/main.rs`, `examples/field/world.rs`;
  Modify: `src/lib.rs`, `crates/hypergraph-rs-bevy/Cargo.toml` (the `[[example]]` table), `README.md` of the crate
- Test: `crates/hypergraph-rs-bevy/tests/test_frame_windows.rs`, `tests/test_example_world.rs` (Create)

**Interfaces:**
- Consumes: the whole crate; `Hypergraph`, `Positions`, `OutlineParams` (core and geometry plan).
- Produces: `FrameWindows`, `WindowStats`; in `examples/field/world.rs`:
  `pub fn styles() -> (FieldConfig, FieldStyles, Palette)`, `pub fn regions() -> Vec<RegionShape>`,
  `pub fn frame(period: u64) -> FieldFrame`.

**Decisions:**
- *`FrameWindows`* follows the method of the known user's own sampler, so the two figures compare: an interval is
  ignored while the sum of the intervals ignored before it is below `settle` (so the interval that completes the
  settle time is ignored too); then every `window` intervals make one `WindowStats`, with the intervals sorted
  (`Duration`'s own order), `median` the mean of `sorted[n/2 − 1]` and `sorted[n/2]` for even `n` and `sorted[n/2]`
  for odd `n`, and `p95 = sorted[(95 n).div_ceil(100) − 1]` (nearest rank, in integers, never through a float). The
  sampler of the known user does exactly this for its fixed `n` of 300. `slowest_p95` is the largest `p95` of the
  windows so far.
- *The invented world* (`world.rs`, no randomness but a fixed LCG, neutral names): 3,168 square regions
  `r0000..r3167` on a 72 × 44 grid of side 1, two triangles each; 25,000 particles `p00000..p24999`, eight per region
  at fixed offsets round its centre (the eight literals `(±0.3, 0)`, `(0, ±0.3)`, `(±0.2121, ±0.2121)`), in region
  order, stopping at 25,000; heights in `[−3, 6]` and tones in `[−1, 1]` from the LCG seeded by the period, every 50th
  region with no reading; six sets of about 2,000 particles each, built as a `Hypergraph<(), (), f64>` in which the
  `j`-th member of a set has the number `(j % 5 + 1) as f64 / 5.0`, placed for `set_frames` by a `Positions` of each
  particle's plan point (the same point its `ParticleFrame.at` is `plan_to_scene` of), outlined by `set_frames`
  (`OutlineParams::new(1.5, 0.25, 0.9)`), styles alternating three hatch directions;
  three streams on beats 0, 1 and 2 (`Disc/Ring`, `Square/SquareFrame`, `Streak/Streak`), each with 3,000 flows
  between region centres, unit 1, amounts 1 to 20, `share_drawn` 0.95, `max_marks` 30,000, every tenth flow hollow.
  `FieldConfig::three_beats(Duration::from_millis(1200))`. The palette is the example's own: neutral values, chosen
  in the example and nowhere else.
- *`main.rs`:* arguments by hand (`std::env::args`, no new dependency): `--size WxH` (default `1366x768`),
  `--measure-seconds N` (optional). `DefaultPlugins` with one window of that size, title `hypergraph-rs-bevy field`,
  `present_mode: PresentMode::AutoNoVsync` (so a frame's cost is measured, not the display's rate); one `Camera3d`
  with `Tonemapping::AcesFitted` (this build has no tonemapping lookup tables) and `Msaa::Sample4`, looking at the
  grid's centre from above at an angle; one directional light. Keys: `Space` replays, `N` shows the next period, `F`
  switches the ground's form, `R` switches reduced motion. When measuring, a new period is shown 0.5 s after each
  rest, so the beats always play; `FrameWindows::new(300, 2 s)` is fed `Time<Real>::delta()` in `Last`; each window
  prints `frame_window index=<i> median_ms=<..> p95_ms=<..>`; after N seconds it prints
  `field size=<physical WxH> regions=3168 particles=25000 marks_per_beat=<a>,<b>,<c> slowest_p95_ms=<..>` and exits
  with `AppExit::Success`.
- *Test without a window:* `tests/test_example_world.rs` includes the world with
  `#[path = "../examples/field/world.rs"] mod world;`. So `world.rs` makes public only the three functions above,
  which that test uses; anything else `main.rs` needs (the grid's centre for the camera, the key handling) lives in
  `main.rs`. An unused public item of `world.rs` is dead code in the test crate and fails clippy `-D warnings`.

- [ ] **Step 1: Write the failing tests.** In `tests/test_frame_windows.rs`:

  ```rust
  #[test] fn a_window_reports_its_median_and_its_95th_percentile() {
      // FrameWindows::new(300, 0 s); push 1..=300 ms: the 300th push returns Some(WindowStats { index: 0,
      // median: 150.5 ms (Duration::from_micros(150_500)), p95: 285 ms }); the first 299 return None;
      // FrameWindows::new(5, 0 s), push 5, 1, 4, 2, 3 ms -> median 3 ms, p95 5 ms
  }
  #[test] fn the_settle_time_is_skipped() { /* settle 2 s: 200 pushes of 10 ms give nothing; window starts after */ }
  #[test] fn the_slowest_window_is_kept() { /* windows with p95 20 ms then 12 ms -> slowest_p95 == Some(20 ms) */ }
  ```

  In `tests/test_example_world.rs`:

  ```rust
  #[test] fn the_example_world_is_the_size_the_design_holds() {
      // regions().len() == 3168; frame(1).particles.len() == 25_000; frame(1).sets.len() == 6;
      // frame(1).validate(&styles().1) == Ok(())
  }
  #[test] fn the_example_plays_three_periods_without_a_window() {
      // let (c, s, p) = styles(); field_app(c, s, p) with one Ground of regions(); ShowPeriod frame(1), (2), (3),
      // each played to rest:
      // no FrameRefused; for every StreamReport, drawn + not_drawn == total and marks <= 30_000
  }
  #[test] fn the_example_world_is_the_same_twice() { /* frame(2) == frame(2) */ }
  ```

- [ ] **Step 2: Run and see them fail.**
  `cargo test -p hypergraph-rs-bevy --locked --test test_frame_windows --test test_example_world`.
  Expected: FAIL to compile, "cannot find type `FrameWindows`".

- [ ] **Step 3: Implement** `frame_windows.rs`, `world.rs` and `main.rs` by the decisions above; add the
  `[[example]]` table; write the crate README's "Run the example" section (`cargo run -p hypergraph-rs-bevy
  --release --example field -- --size 1366x768`).

- [ ] **Step 4: Run and see them pass.** The same command. Then `cargo build -p hypergraph-rs-bevy --release
  --example field --locked`. Expected: PASS, and the example builds.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): a frame-time sampler and an example
  that runs without any game`.

---

### Task 11: The measured trial: plain marks against the frame budget

**Files:**
- Modify: `crates/hypergraph-rs-bevy/README.md` (section "Frame time")

**Interfaces:**
- Consumes: the example (Task 10). Produces: two measured figures and the decision on Task 12.

**Decisions (design 11, "Frame time"):**
- *Where:* the reference machine, NVIDIA GeForce RTX 3060 on the Vulkan backend (the machine of the known user's
  figures of 2026-11-14), with a display. If the builder has no display session (`DISPLAY` unset), it stops here and
  hands the two commands below to the lead, who runs them.
- *What:* the release example, beats playing, 60 seconds at each size, one run at a time:
  `cargo run -p hypergraph-rs-bevy --release --locked --example field -- --size 1366x768 --measure-seconds 60`, then
  the same with `--size 1920x1080`.
- *The budget:* the slowest window's 95th percentile must be no worse than the design's figures for today's money map:
  **15.4 ms at 1366x768** and **18.4 ms at 1920x1080**. The printed physical size must equal the asked size (a scale
  factor other than 1 makes the run void; rerun with the window at scale 1).
- *The decision:* if both figures are within budget, plain marks stand: Task 12 is not built and the README says so
  with the figures. If either is over, Task 12 is built, and this trial is run again after it.

- [ ] **Step 1: Run the two measurements** above. Expected: each ends with a `field ... slowest_p95_ms=<x>` line.

- [ ] **Step 2: Write the "Frame time" section** of the crate README: the machine (GPU, driver version from
  `nvidia-smi`, backend), the date, the two commands, the two `field ...` lines as printed, the budget, and the
  decision in one sentence.

- [ ] **Step 3: Commit** `docs(bevy): frame time of the example with plain marks` (with the figures in the message).

---

### Task 12 (only if Task 11 is over budget): one shader that moves marks by a clock

**Files:**
- Create: `crates/hypergraph-rs-bevy/src/plugin/mark_material.rs`, `src/plugin/marks.wgsl`;
  Modify: `src/plugin/marks.rs`, `src/plugin/mod.rs`, the crate README
- Test: `crates/hypergraph-rs-bevy/tests/test_mark_material.rs` (Create)

**Interfaces:**
- Consumes: `mark_start`, `mark_position`, `smoothstep` (Task 5); `glyph_mesh` (Task 3).
- Produces: `pub fn mark_batch_mesh(flows: &[Flow], drawn: &[DrawnFlow], styles: &[StreamStyle], config:
  &FieldConfig, stream: u16) -> Mesh`; a private `MarkMaterial` (`Material`, `AsBindGroup`, one uniform `clock: f32`
  in seconds since the beat began, `bow: f32`, `travel: f32` in seconds, `colour: LinearRgba`).

**Decisions:**
- With full motion, marks of one stream are one mesh and one entity, not one entity per mark: every mark's glyph
  vertices are copied into the batch, each carrying three custom vertex attributes (`from: Float32x3`,
  `to: Float32x3`, `timing: Float32x4` = start, side, size, unused). The vertex shader computes the same
  `mark_position` and visibility window as Task 9 (the same formula, written in WGSL) and moves a vertex out of view
  when its mark is outside its window. The CPU sets `clock` once a frame. The shader is embedded with
  `embedded_asset!`; it needs no Bevy feature beyond `bevy_pbr`. Reduced motion is unchanged (path lines).
- `MarkOf` entities are no longer spawned when the shader is in use; Task 9's tests that count `MarkOf` are changed
  to count marks in the batch (vertex count / vertices per glyph), with the same expected numbers.

- [ ] **Step 1: Write the failing tests** in `tests/test_mark_material.rs`:

  ```rust
  #[test] fn the_batch_holds_every_mark_once() {
      // the flows of Task 9's first test: mark_batch_mesh(stream 0) vertex count == 6 * vertices of the Disc glyph
  }
  #[test] fn each_vertex_carries_its_marks_path_and_start() {
      // for mark k of flow f: from, to equal the flow's endpoints; timing.x == mark_start(f, k, n, beat, travel)
      // in seconds by to_bits(); timing.y == +1 for even k, -1 for odd
  }
  ```

- [ ] **Step 2: Run and see them fail.** `cargo test -p hypergraph-rs-bevy --locked --test test_mark_material`.
  Expected: FAIL to compile, "cannot find function `mark_batch_mesh`".

- [ ] **Step 3: Implement** the batch mesh, the material and the shader; switch full-motion marks to the batch.

- [ ] **Step 4: Run and see them pass**, then the whole crate: `cargo test -p hypergraph-rs-bevy --locked`. Expected:
  PASS. Then rerun Task 11. Expected: both figures within budget. If not, stop and report both runs to the lead.

- [ ] **Step 5: Scoped gate (as in Task 2, Step 5), then commit** `feat(bevy): move marks by a clock in one shader`.

---

### Task 13: Documents, and every gate

**Files:**
- Modify: `crates/hypergraph-rs-bevy/src/lib.rs` (crate guide), `crates/hypergraph-rs-bevy/README.md`, root
  `README.md` (one section), `CHANGELOG.md` (one entry), `CONTRIBUTING.md` (the floor paragraph)

**Interfaces:**
- Consumes: everything above. Produces: no code.

- [ ] **Step 1: The crate guide.** `lib.rs` gets: what the crate draws; how a caller hands it a period (the paragraph
  of "The public API"); the beats; the rules it keeps (positions are measurements, nothing moves while resting, two
  streams never meet, hatching marks sets and never the ground, the palette is the caller's); the systems' order; and
  one doctest that builds a headless app as `tests/common` does, adds `FieldPlugin`, spawns a two-region `Ground`,
  sends one `ShowPeriod` with two particles and one flow, plays it to rest, and asserts the `FlowReport` has
  `drawn + not_drawn == total`. Neutral words only.

- [ ] **Step 2: The documents.** Root `README.md`: a section "Bevy" saying the crate exists, its Bevy version and
  features, its own floor, and pointing to `crates/hypergraph-rs-bevy/README.md`. `CHANGELOG.md`: one "Added" line
  under the topmost unreleased section naming the crate and its plugin, and one line under its "Rust version" heading:
  the Bevy crate's floor is 1.97.1 (Bevy's); the rest of the workspace keeps its own. Root `README.md` "Checks" list
  gains `mise run rust:bevy`, and its "Rust version" section says the Bevy crate has its own floor. `CONTRIBUTING.md`:
  the floor paragraph says the same; the checks table gains a row for `mise run rust:bevy` (the Bevy crate: its lib
  with its own features, clippy, tests, docs, then its own floor), and the rows of `rust:check` and `rust:msrv` say
  "the workspace except the Bevy crate".

- [ ] **Step 3: Every gate, one at a time.** `mise run rust:check`; `mise run rust:msrv`; `mise run rust:bevy`;
  `cargo deny check`; if `.venv` exists, `.venv/bin/maturin develop` then `mise run py:gate` (the Python crate is
  untouched; this shows it). Expected: each passes. Then repeat Task 1, Step 6 against the base. Expected: unchanged.

- [ ] **Step 4: Commit** `docs(bevy): crate guide, README sections, changelog and the floor`.

## Self-review notes

- Design 6.1: particles, bodies (a particle style; a ring for a body of participants), sets, marks (Tasks 3, 7, 8,
  9); 25,000 particles sharing meshes and materials (Task 7), no square cost (Tasks 2, 4, 5, 6). Design 6.2: two
  scales, printed remainder, hollow marks for idle hours and unknown goods (Task 5, `FlowReport`). Design 6.3: a
  ground that bends and blocks that sink, outlines fixed in plan, gray for no reading (Tasks 4, 7). Design 6.4: three
  beats, still after, replay, reduced motion as still frames with paths (Tasks 5, 7, 9). Design 6.7: hatched
  membranes, ring/capsule/membrane, overlaps both shown, plain outlines for groupings, measured placement eased once a
  period (Tasks 2, 6, 8, 7). Design 8: general parts only, the game's Bevy version, smallest features, own CI job,
  licences by name, open set of outline shapes handled (`UnsupportedOutline`), nothing published (Task 1). Design 11:
  same picture twice, nothing dropped in silence, reduced motion, frame time (Tasks 9, 5, 10, 11).
- The design's wells figures, the table, labels, the inspector, the rail and the land's draping of sets are the
  client's (design 8, step 5 onward), drawn with these parts.
- The geometry plan left "the fixed spot of each kind of node around a place's centre" to this step. It is not a
  part of this crate's API: a particle's position is the caller's (`ParticleFrame.at`), and design 8 gives particles
  to the client. The example shows one way (eight fixed offsets round each region's centre, Task 10).
- Task numbering: the conditional shader is Task 12 and the final gate is Task 13; every cross-reference above uses
  those numbers.
