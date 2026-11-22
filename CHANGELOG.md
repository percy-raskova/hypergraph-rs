# Changelog

All notable changes to this project are written here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - unreleased

### Added

- A value on every membership. `Hypergraph` has `member_data`, `set_member_data`,
  `members_with_data` (in the same order as `members()`) and `add_node_to_edge_with_data`.
  `DiHypergraph` has `member_data`, `set_member_data` and `add_node_to_edge_with_data`, which
  take the role (`DiRole::Tail` or `DiRole::Head`), and `dimembers_with_data`, which returns the
  tail and the head together. A new membership carries `M::default()`;
  a member that leaves and comes back starts from it again. XGI has no value on a membership,
  so the XGI-compatible formats, the Python package and the WASM bindings do not carry it.
- Runnable examples: `quickstart` and `membership_data`.
- A `geometry` feature, on by default, with the module `geometry`: the outline of a weighted set of
  points or of a hyperedge (`weighted_outline`, `edge_outline`, shaped as `Outline::Ring`,
  `Outline::Capsule` or `Outline::Membrane`), iso-lines of a scalar grid (`ScalarGrid`, `iso_lines`,
  `signed_area`, `contains`), placement at a weighted mix of fixed points (`barycentre`,
  `corner_mix`, `anchored_placement`, with `Positions` and `Placement`), and spreading of
  overlapping points in fixed steps (`spread`, `SpreadParams`, `SpreadReport`). Errors are
  `GeometryError`. Every result is the same bits on every machine.
- A crate-level guide in the API docs, a README, a contributing guide, and this changelog.
- Package metadata (keywords, categories, readme, documentation) and the license text in the
  packed crate, so the crate can be packed with `cargo package`.
- cargo-deny, CodeQL, Dependabot and a security policy (`SECURITY.md`).
- Checks: `mise run rust:check` now also lints, tests and documents the core crate with every
  feature on, and runs `mise run rust:features`, which builds every feature alone without the
  defaults. `mise run rust:msrv` builds the workspace, and the core crate with every feature
  on, at the minimum Rust version.

### Changed

- `members()` and `memberships()` take constant time for each neighbour. They found each id by
  scanning every id, so an edge of 100,000 members took billions of comparisons to read. A
  reverse map, kept in step with the forward one, replaces the scan. The order of the ids is
  the same as before.
- Everything is raised to its newest version: every crate (among them PyO3 and rust-numpy
  0.30, h3o 0.11 and sprs 0.11.6), the pinned toolchain (Rust 1.100.0), and the Python that
  the conformance gate runs on in CI (3.14). The code is changed where the newer lints asked.
- The stub CLI and WASM crates no longer declare dependencies they do not use.
- The yanked `chacha20` is replaced.
- The package address is now https://github.com/percy-raskova/hypergraph-rs, and the
  `homepage` entry is gone.
- The Python and WASM binding crates are marked `publish = false`. Nothing is published.

### Fixed

- The documentation of `EdgeError::EmptyMembers` no longer claims the opposite of what the
  code does.
- The conformance jobs in CI work, and every action is pinned to a full commit.

### Rust version

- The minimum Rust version is now 1.90, up from 1.85. It is the lowest version the newest
  dependencies allow, and it holds for the workspace and for every feature.
- In 0.1.0 the floor of 1.85 did not hold for `cells3d` and `raster-png`: `h3o` 0.10 needed
  Rust 1.87, and nothing checked it. `mise run rust:msrv` now builds every feature at the
  floor.

## [0.1.0]

The port of XGI to Rust, on top of rustworkx-core.

- `Hypergraph`, `DiHypergraph` and `SimplicialComplex`, with insertion-ordered listings.
- Algorithms, statistics, generators, readers and writers, layouts, a scene graph with SVG
  output, conversions, Kuramoto dynamics and spectral clustering, each behind a feature flag.
- A terminal-text rasterizer, with hexagonal-cell instruments and PNG output behind the
  `cells3d` and `raster-png` features.
- A Python package built with maturin, and the XGI conformance gate that replays XGI's test
  suite against it.
- Stub crates for a command line tool and WASM bindings.
