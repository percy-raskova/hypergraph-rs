//! # hypergraph-rs
//!
//! A Rust port of XGI (the Python hypergraph library), built as a genuine
//! rustworkx-core plugin. The hypergraph IS a
//! [`rustworkx_core::petgraph::stable_graph::StableDiGraph`] with two node
//! kinds (`Agent` and `Hyperedge`) connected by `MembershipEdge` edges.

#[cfg(feature = "algorithms")]
pub mod algorithms;
pub mod core;
pub mod linalg;
#[cfg(feature = "generators")]
pub mod generators;
#[cfg(feature = "stats")]
pub mod stats;
#[cfg(feature = "readwrite")]
pub mod readwrite;

pub use core::dihypergraph::DiHypergraph;
pub use core::error::{EdgeError, NodeError, SimplexError, SwapError};
pub use core::globalviews::subhypergraph;
pub use core::hypergraph::{Hypergraph, MergeRename, MergeRule};
pub use core::kinds::{DiMembershipEdge, DiRole, MembershipEdge, NodeKind};
pub use core::simplicialcomplex::{FaceLattice, SimplicialComplex};
pub use core::views::{EdgeView, FilterMode, NodeView};
