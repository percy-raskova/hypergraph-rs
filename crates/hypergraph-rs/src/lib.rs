//! # hypergraph-rs
//!
//! A Rust port of XGI (the Python hypergraph library), built as a genuine
//! rustworkx-core plugin. The hypergraph IS a
//! [`rustworkx_core::petgraph::stable_graph::StableDiGraph`] with two node
//! kinds (`Agent` and `Hyperedge`) connected by `MembershipEdge` edges.

#[cfg(feature = "algorithms")]
pub mod algorithms;
pub mod core;
#[cfg(feature = "layout")]
pub mod layout;
#[cfg(feature = "viz")]
pub mod viz;
#[cfg(feature = "generators")]
pub mod generators;
pub mod linalg;
#[cfg(feature = "readwrite")]
pub mod readwrite;
#[cfg(feature = "stats")]
pub mod stats;
pub mod utils;

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
