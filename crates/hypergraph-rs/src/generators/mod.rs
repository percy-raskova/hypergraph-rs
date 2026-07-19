//! Hypergraph generators (XGI `xgi/generators/*.py` port).
//!
//! Every randomized generator takes an explicit `seed: Option<u64>` and builds
//! a `rand_pcg::Pcg64Mcg` from it. Determinism is per-call (III.7 parity); the
//! conformance gate asserts structural properties, not numpy-sequence parity.

pub mod rng;
pub mod classic;
pub mod lattice;
pub mod simple;
pub mod random;
pub mod uniform;
pub mod simplicial;

pub use classic::{
    complement, complete_hypergraph, empty_hypergraph, trivial_hypergraph, GeneratorError,
};
// TASK 4: uncomment when lattice.rs lands
// pub use lattice::ring_lattice;
// TASK 5: uncomment when simple.rs lands
// pub use simple::{star_clique, sunflower};
// TASK 6: uncomment when random.rs lands
// pub use random::{
//     chung_lu_hypergraph, fast_random_hypergraph, random_hypergraph, watts_strogatz_hypergraph,
// };
// TASK 7: uncomment when uniform.rs lands
// pub use uniform::{uniform_erdos_renyi_hypergraph, uniform_hypergraph_configuration_model};
// TASK 8: uncomment when simplicial.rs lands
// pub use simplicial::random_simplicial_complex;
