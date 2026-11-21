# Changelog

All notable changes to this project are written here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - unreleased

### Added

- A value on every membership. `Hypergraph` has `member_data`, `set_member_data`,
  `members_with_data` and `add_node_to_edge_with_data`. `DiHypergraph` has the same four, each
  taking the role (`DiRole::Tail` or `DiRole::Head`). A new membership carries `M::default()`;
  a member that leaves and comes back starts from it again. XGI has no value on a membership,
  so the XGI-compatible formats, the Python package and the WASM bindings do not carry it.
- Runnable examples: `quickstart` and `membership_data`.
- A crate-level guide in the API docs, a README, a contributing guide, and this changelog.
- Package metadata (keywords, categories, readme, documentation) and the license text in the
  packed crate, so the crate can be packed with `cargo package`.
- cargo-deny, CodeQL, Dependabot and a security policy (`SECURITY.md`).
- Checks: `mise run rust:check` now also lints, tests and documents the core crate with every
  feature on, and runs `mise run rust:features`, which builds every feature alone without the
  defaults. `mise run rust:msrv` checks two floors.

### Changed

- `members()` and `memberships()` take constant time for each neighbour. They found each id by
  scanning every id, so an edge of 100,000 members took billions of comparisons to read. A
  reverse map, kept in step with the forward one, replaces the scan. The order of the ids is
  the same as before.
- The dependencies are updated to the latest versions the minimum Rust version allows. The
  workspace uses resolver 3, so `cargo update` keeps the lockfile under that floor. `sprs` is
  held at 0.11.4 (0.11.5 and later need Rust 1.88) and `ordered-float` at 5.4.0 (5.5.1 needs
  Rust 1.90).
- The yanked `chacha20` is replaced.
- The package address is now https://github.com/percy-raskova/hypergraph-rs, and the
  `homepage` entry is gone.
- The Python and WASM binding crates are marked `publish = false`. Nothing is published.

### Fixed

- `members_with_data` lists members in the same order as `members()`.
- The documentation of `EdgeError::EmptyMembers` no longer claims the opposite of what the
  code does.
- The conformance jobs in CI work, and every action is pinned to a full commit.

### Rust version

- The minimum Rust version stays 1.85 for the workspace and for every feature of the core
  crate except `cells3d` and `raster-png`. Those two need Rust 1.87, because they use `h3o`,
  and `h3o` 0.10 needs it. That was already so in 0.1.0; it is now checked and written down.

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
