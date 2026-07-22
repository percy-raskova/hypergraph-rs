//! PyO3 bindings for hypergraph-rs — the `_core` extension module.
//!
//! The pure-Python `hypergraph_rs/` package (mirroring XGI's `__init__.py`) imports from this
//! extension. Deliberate core deviations (spec Sec3.1) are shimmed back to XGI behavior HERE.

use pyo3::prelude::*;

mod errors;
mod hypergraph;
mod pyid;
// mod views;        // Task 5
// mod dihypergraph; // Task 6
// mod simplicial;   // Task 7
// mod linalg;       // Task 8
// mod algorithms;   // Task 9
// mod generators;   // Task 10
// mod stats;        // Task 11
// mod convert; mod readwrite; mod utils; mod dynamics; mod communities; // Task 12
// mod layout;       // Task 13

#[pymodule]
fn _hypergraph_rs_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register(m)?;
    hypergraph::register(m)?;
    Ok(())
}
