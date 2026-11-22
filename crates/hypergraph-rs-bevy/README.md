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

The runnable example and the measured frame times are added with the plugin.
