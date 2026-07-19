//! Hypergraph algorithms. All functions take `&Hypergraph<N,E,M>` first.
//!
//! Gated behind the `algorithms` feature (in the default set). Component and
//! distance orderings follow node insertion order (III.7 determinism), a
//! deliberate improvement over XGI's hash-set iteration.

pub mod assortativity;
pub mod centrality;
pub mod clustering;
pub mod connected;
pub mod properties;
pub mod shortest_path;
pub mod simpliciality;
