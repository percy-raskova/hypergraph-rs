//! Hypergraph generators (XGI `xgi/generators/*.py` port).
//!
//! Every randomized generator takes an explicit `seed: Option<u64>` and builds
//! a `rand_pcg::Pcg64Mcg` from it. Determinism is per-call (III.7 parity); the
//! conformance gate asserts structural properties, not numpy-sequence parity.

pub mod classic;
pub mod lattice;
pub mod random;
pub mod rng;
pub mod simple;
pub mod simplicial;
pub mod uniform;

pub use classic::{
    complement, complete_hypergraph, empty_hypergraph, trivial_hypergraph, GeneratorError,
};
pub use lattice::ring_lattice;
pub use simple::{star_clique, sunflower};
pub use random::{
    chung_lu_hypergraph, fast_random_hypergraph, random_hypergraph, watts_strogatz_hypergraph,
};
// TASK 8: uncomment when uniform.rs lands
// pub use uniform::{uniform_erdos_renyi_hypergraph, uniform_hypergraph_configuration_model};
// TASK 8: uncomment when simplicial.rs lands
// pub use simplicial::random_simplicial_complex;
