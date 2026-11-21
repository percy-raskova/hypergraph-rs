//! # hypergraph-rs
//!
//! A hypergraph library for Rust. A hypergraph is a graph whose edges can join any number
//! of nodes, not just two. This crate is a port of [XGI](https://github.com/xgi-org/xgi),
//! the Python hypergraph library, and it builds on
//! [rustworkx-core](https://crates.io/crates/rustworkx-core). Node and edge ids are strings,
//! and every listing keeps the order in which things were added.
//!
//! It is not on crates.io yet. Depend on it from git, as the README shows.
//!
//! ## Quick start
//!
//! ```
//! use hypergraph_rs::algorithms::connected::number_connected_components;
//! use hypergraph_rs::Hypergraph;
//!
//! // Nodes, edges and memberships carry no value here, so all three types are `()`.
//! let mut h: Hypergraph<(), (), ()> = Hypergraph::new();
//!
//! // An edge joins any number of nodes. Nodes that do not exist yet are created.
//! h.add_edge(
//!     vec!["ana".into(), "bo".into(), "cy".into()],
//!     Some("club".into()),
//!     (),
//! )
//! .unwrap();
//! h.add_edge(
//!     vec!["bo".into(), "cy".into(), "di".into(), "ed".into()],
//!     Some("choir".into()),
//!     (),
//! )
//! .unwrap();
//! h.add_edge(vec!["fay".into(), "gus".into()], Some("pair".into()), ())
//!     .unwrap();
//!
//! // Listings keep the order in which things were added.
//! assert_eq!(h.node_ids(), ["ana", "bo", "cy", "di", "ed", "fay", "gus"]);
//! assert_eq!(h.members("choir").unwrap(), ["bo", "cy", "di", "ed"]);
//! assert_eq!(h.memberships("bo").unwrap(), ["club", "choir"]);
//!
//! // One statistic: "pair" shares no member with the other two edges, so there are
//! // two connected components.
//! assert_eq!(number_connected_components(&h), 2);
//! ```
//!
//! ## What is in it
//!
//! - [`Hypergraph`]: undirected edges. [`DiHypergraph`]: each edge has a tail and a head.
//!   [`SimplicialComplex`]: a hypergraph that keeps every subface of each simplex.
//! - A hypergraph has three type parameters: the value on a node, on an edge and on a
//!   membership. Each defaults to a JSON value. The statistics, readers, writers and
//!   conversions work on that default type.
//! - Modules, switched by the feature flags below: [`algorithms`], `generators`, `stats`,
//!   `readwrite`, `layout`, `viz`, `raster`, `convert`, `dynamics`, `communities`. The
//!   modules [`core`], [`linalg`] and [`utils`] are always there.
//!
//! ## A value on a membership
//!
//! A membership is the fact that one node belongs to one edge. XGI has no value on a
//! membership. The membership value is this crate's own addition: the third type parameter,
//! read and written with `member_data`, `set_member_data`, `members_with_data` and
//! `add_node_to_edge_with_data` (on [`DiHypergraph`], `member_data`, `set_member_data` and
//! `add_node_to_edge_with_data` also take the role, and `dimembers_with_data` lists the tail
//! and the head together). A new membership carries `M::default()`. The XGI-compatible
//! formats, the Python package and the WASM bindings do not carry the value.
//!
//! ```
//! use hypergraph_rs::Hypergraph;
//!
//! let mut h: Hypergraph<(), (), u64> = Hypergraph::new();
//! h.add_node_to_edge_with_data("council", "teachers", 3).unwrap();
//! h.add_node_to_edge_with_data("union", "teachers", 40).unwrap();
//! assert_eq!(h.member_data("council", "teachers"), Some(&3));
//! assert_eq!(h.member_data("union", "teachers"), Some(&40));
//! ```
//!
//! ## Feature flags
//!
//! The default features are `algorithms`, `generators`, `stats`, `readwrite`, `layout`,
//! `viz` and `raster`.
//!
//! | Feature | Turns on |
//! | --- | --- |
//! | `algorithms` | connected components, shortest paths, clustering, centrality, assortativity, simpliciality |
//! | `generators` | classic, random, lattice, uniform and simplicial generators (seeded) |
//! | `stats` | node and edge statistics, for the default type of hypergraph |
//! | `readwrite` | edge list, bipartite edge list, JSON, HIF and incidence matrix readers and writers, and a BiGG dictionary reader |
//! | `layout` | deterministic layouts: node positions and the shape drawn for each edge |
//! | `viz` | a scene graph built from a layout, with JSON and SVG output (needs `layout`) |
//! | `raster` | the base of the terminal-text rasterizer (needs `viz` and `algorithms`) |
//! | `dynamics` | Kuramoto synchronization |
//! | `communities` | spectral clustering |
//! | `convert` | conversions between hypergraphs and edge lists, dictionaries and graphs |
//! | `network` | declares `readwrite::bigg::load_bigg_data`, which is not implemented and always returns an error |
//! | `cells3d` | hexagonal-cell scenes and instruments for the rasterizer (needs `raster`, `communities` and the `h3o` crate) |
//! | `raster-png` | PNG output for the rasterizer (needs `cells3d` and the `png` crate) |
//! | `all` | every feature above |
//!
//! Every feature builds on its own, without the defaults.
//!
//! ## Minimum Rust version
//!
//! Rust 1.85 for the workspace and for every feature of this crate except `cells3d` and
//! `raster-png`. Those two, and so `all`, need Rust 1.87, because they use `h3o`, and `h3o`
//! 0.10 needs it.

#[cfg(feature = "algorithms")]
pub mod algorithms;
pub mod core;
#[cfg(feature = "generators")]
pub mod generators;
#[cfg(feature = "layout")]
pub mod layout;
pub mod linalg;
#[cfg(feature = "raster")]
pub mod raster;
#[cfg(feature = "readwrite")]
pub mod readwrite;
#[cfg(feature = "stats")]
pub mod stats;
pub mod utils;
#[cfg(feature = "viz")]
pub mod viz;

#[cfg(feature = "convert")]
pub mod convert;

#[cfg(feature = "dynamics")]
pub mod dynamics;

#[cfg(feature = "communities")]
pub mod communities;

pub use core::dihypergraph::DiHypergraph;
pub use core::error::{EdgeError, MembershipError, NodeError, SimplexError, SwapError};
pub use core::globalviews::subhypergraph;
pub use core::hypergraph::{Hypergraph, MergeRename, MergeRule};
pub use core::kinds::{DiMembershipEdge, DiRole, MembershipEdge, NodeKind};
pub use core::simplicialcomplex::{FaceLattice, SimplicialComplex};
pub use core::views::{EdgeView, FilterMode, NodeView};
