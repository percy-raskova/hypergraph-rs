# hypergraph-rs

A hypergraph library for Rust. A hypergraph is a graph whose edges can join any number of
nodes, not just two.

It is a port of [XGI](https://github.com/xgi-org/xgi), the Python hypergraph library, and it
builds on [rustworkx-core](https://crates.io/crates/rustworkx-core). A hypergraph is stored as
a bipartite graph, so the algorithms of rustworkx-core work on it. Node and edge ids are
strings. Every listing keeps the order in which things were added, so the same steps always
give the same output.

## What it is, and what it is not

It has:

- an undirected `Hypergraph`, a directed `DiHypergraph` (each edge has a tail and a head) and
  a `SimplicialComplex`;
- a value on every membership, which XGI does not have (see below);
- algorithms, statistics, generators, readers and writers, layouts, conversions, Kuramoto
  dynamics, spectral clustering, and a text and PNG rasterizer, each behind a feature flag.

It is not all of XGI. The symbols of XGI that are not ported are listed in
`tests/DEFERRED_SYMBOLS.md`. It does not draw with matplotlib, and it has no pandas
conversions. The `hypergraph-rs-cli` and `hypergraph-rs-wasm` crates in this repository are
stubs: they do nothing yet.

## Use it

It is not on crates.io yet. Depend on it from git:

```toml
[dependencies]
hypergraph-rs = { git = "https://github.com/percy-raskova/hypergraph-rs" }
```

Pin a commit with `rev = "..."` if you want a build that does not move.

## Quick start

```rust
use hypergraph_rs::algorithms::connected::number_connected_components;
use hypergraph_rs::Hypergraph;

// Nodes, edges and memberships carry no value here, so all three types are `()`.
let mut h: Hypergraph<(), (), ()> = Hypergraph::new();

// An edge joins any number of nodes. Nodes that do not exist yet are created.
h.add_edge(
    vec!["ana".into(), "bo".into(), "cy".into()],
    Some("club".into()),
    (),
)
.unwrap();
h.add_edge(
    vec!["bo".into(), "cy".into(), "di".into(), "ed".into()],
    Some("choir".into()),
    (),
)
.unwrap();
h.add_edge(vec!["fay".into(), "gus".into()], Some("pair".into()), ())
    .unwrap();

// Listings keep the order in which things were added.
assert_eq!(h.node_ids(), ["ana", "bo", "cy", "di", "ed", "fay", "gus"]);
assert_eq!(h.members("choir").unwrap(), ["bo", "cy", "di", "ed"]);
assert_eq!(h.memberships("bo").unwrap(), ["club", "choir"]);

// One statistic: "pair" shares no member with the other two edges, so there are
// two connected components.
assert_eq!(number_connected_components(&h), 2);
```

The same code runs as `cargo run -p hypergraph-rs --example quickstart`.

The statistics in `hypergraph_rs::stats`, the readers and writers, and the conversions work
on the default type, `Hypergraph`, which carries a JSON value on every node, edge and
membership. The algorithms work on any type.

## Feature flags

The default features are `algorithms`, `generators`, `stats`, `readwrite`, `layout`, `viz` and
`raster`. Every feature builds on its own, without the defaults.

| Feature | Turns on |
| --- | --- |
| `algorithms` | connected components, shortest paths, clustering, centrality, assortativity, simpliciality |
| `generators` | classic, random, lattice, uniform and simplicial generators, all seeded |
| `stats` | node and edge statistics, for the default type of hypergraph |
| `readwrite` | edge list, bipartite edge list, JSON, HIF and incidence matrix readers and writers, and a BiGG dictionary reader |
| `layout` | deterministic layouts: node positions and the shape drawn for each edge |
| `viz` | a scene graph built from a layout, with JSON and SVG output (needs `layout`) |
| `raster` | the base of the terminal-text rasterizer (needs `viz` and `algorithms`) |
| `dynamics` | Kuramoto synchronization |
| `communities` | spectral clustering |
| `convert` | conversions between hypergraphs and edge lists, dictionaries and graphs |
| `network` | declares `readwrite::bigg::load_bigg_data`, which is not implemented and always returns an error |
| `cells3d` | hexagonal-cell scenes and instruments for the rasterizer (needs `raster`, `communities` and the `h3o` crate) |
| `raster-png` | PNG output for the rasterizer (needs `cells3d` and the `png` crate) |
| `all` | every feature above |

## A value on a membership

A membership is the fact that one node belongs to one edge. XGI has no value on a membership.
The membership value is this crate's own addition: it is the third type parameter, and a new
membership carries `M::default()`.

```rust
use hypergraph_rs::Hypergraph;

let mut h: Hypergraph<(), (), u64> = Hypergraph::new();
h.add_node_to_edge_with_data("council", "teachers", 3).unwrap();
h.add_node_to_edge_with_data("union", "teachers", 40).unwrap();
h.set_member_data("council", "teachers", 4).unwrap();
assert_eq!(h.member_data("council", "teachers"), Some(&4));
assert_eq!(h.member_data("union", "teachers"), Some(&40));
assert_eq!(h.members_with_data("union").unwrap(), [("teachers".to_string(), &40)]);
```

On `DiHypergraph`, `member_data`, `set_member_data` and `add_node_to_edge_with_data` also take
the role (`DiRole::Tail` or `DiRole::Head`), and `dimembers_with_data(edge_id)` returns the tail
and the head lists together. The XGI-compatible formats, the Python package and the WASM bindings
do not carry the value. See `cargo run -p hypergraph-rs --example membership_data`.

## Python

A Python package follows XGI's names. It is built from `crates/hypergraph-rs-python` with
[maturin](https://www.maturin.rs). A conformance gate replays XGI's own test suite against it
and checks it against a pinned XGI 0.10.2: run it with `mise run py:gate` (it needs a `.venv`
with XGI and the test tools, as `.github/workflows/ci.yml` sets one up). Nothing is published
to PyPI.

## Rust version

The minimum Rust version is 1.90, for the workspace and for every feature of the core crate.
It is the lowest version the newest dependencies allow, and it rises when they need it.
`rust-toolchain.toml` pins the toolchain used for development, 1.100.0. `mise run rust:msrv`
checks the floor.

## Checks

Run these with [mise](https://mise.jdx.dev):

- `mise run rust:check`: format, lint, tests and docs, then the same with every feature on,
  then every feature alone. CI runs the same task.
- `mise run rust:msrv`: the build at the minimum Rust version.
- `mise run py:gate`: the XGI conformance gate.
- `cargo deny check`: advisories, licenses and sources.

`CONTRIBUTING.md` has more.

## Security

To report a vulnerability, see `SECURITY.md`.

## License

BSD-3-Clause, the same license as XGI. See `LICENSE`.
