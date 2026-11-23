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

The example draws an invented world of 3,168 square regions, 25,000 particles, six sets read from a hypergraph and
three streams of marks, with no other code behind it. It opens a window.

```text
cargo run -p hypergraph-rs-bevy --release --example field -- --size 1366x768
```

Keys: `Space` plays the period again, `N` shows the next period, `F` switches the ground between a bending surface and
blocks, `R` switches reduced motion. With `--measure-seconds N` a new period is shown half a second after each rest,
the frame times are printed in windows of 300 frames after two seconds of settling (`frame_window index=.. median_ms=..
p95_ms=..`), and the program prints one `field size=.. slowest_p95_ms=..` line and exits after N seconds. The example
turns on `x11` and not `wayland`, so on a Wayland desktop it runs under XWayland.

The measured frame times are added with the trial.
