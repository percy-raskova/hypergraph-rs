# hypergraph-rs-bevy

Bevy integration for [hypergraph-rs](../../README.md): it draws a hypergraph as a ground of regions (bending, or
rising and sinking in blocks), hatched outlines of hyperedges, particles, and marks that move along paths in beats.

It reads the core crate's `geometry` module and has no game inside it: the caller supplies the palette, the styles,
the region shapes and one frame per period. This crate is not published; it is taken by git revision or by path.

It needs Rust 1.97.1 or newer (Bevy 0.20.1's floor); the rest of the workspace keeps 1.90, so `mise run rust:check`
and `mise run rust:msrv` leave this crate out and `mise run rust:bevy` is its own gate.

```text
mise run rust:bevy
```

## Run the example

The example draws an invented world with no other code behind it: square regions under soft hills, particles seated
on the ground, sets read from a hypergraph as hatched outlines, and three streams of marks, one a beat. It opens a
window. The world it opens is small enough to read: 1,728 regions, 240 particles (nine of them posts that mark the
nine places most of the others gather round) and five sets, two pairs of which share a place.

```text
cargo run -p hypergraph-rs-bevy --release --example field
```

With `--large` it draws the measured world instead: 3,168 regions, 25,000 particles, six sets and about 30,000 marks
a beat. That world is there to load the plugin, not to be read.

The first frame shown has no earlier picture to ease from, so it stands from its first beat; every later frame eases
in the ease beat.

The view is from one side and above, with no perspective, so sizes compare across the field. The ground's material is
lit, so the example gives it light from everywhere as well as one direct light; without the first, a slope that faces
away goes black. The hills are kept low on purpose: a set's outline and a mark's path are flat, so they sit a little
above the ground and do not follow it. The crate has no query for the ground's height at a point, so the example seats
its particles by its own reading between region centres; in the blocks form (`F`) a particle beside a step can sink
into its block.

Keys: `Space` plays the period again, `N` shows the next period, `F` switches the ground between a bending surface and
blocks, `R` switches reduced motion. `--size WxH` sets the window's size (1366x768 without it). With
`--measure-seconds N` a new period is shown half a second after each rest, the frame times are printed in windows of
300 frames after two seconds of settling (`frame_window index=.. median_ms=.. p95_ms=..`), and the program prints one
`field size=.. slowest_p95_ms=..` line and exits after N seconds. With `--fullscreen` the window is borderless and
covers its whole monitor, and `--size` is still read; the summary line prints the window's size as it is, so it shows
whether that equals what was asked. The example turns on `x11` and not `wayland`, so on a Wayland desktop it runs
under XWayland.

The measured frame times are added with the trial.

## How marks are drawn

With full motion, the marks of one stream are one mesh on one entity (`MarkBatch`), not one entity a mark. Every
vertex carries its mark's path and start time, and one small vertex shader (`src/plugin/marks.wesl`, embedded in the
crate; Bevy 0.20 reads shaders in the WESL dialect of WGSL) finds where each mark is from a clock the plugin writes
once a frame. A mark outside its travel window is moved out of view. The shader needs no Bevy feature beyond
`bevy_pbr`. `mark_batch_mesh` builds the mesh. With reduced motion nothing changes: one still line of paths per stream.
