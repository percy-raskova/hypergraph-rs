//! PyO3 bindings for hypergraph-rs — the `_core` extension module.
//!
//! The pure-Python `hypergraph_rs/` package (mirroring XGI's `__init__.py`) imports from this
//! extension. Deliberate core deviations (spec Sec3.1) are shimmed back to XGI behavior HERE.

use pyo3::prelude::*;

mod algorithms; // Task 9
mod communities; // Task 12
mod convert; // Task 12
mod dihypergraph; // Task 6
mod dynamics; // Task 12
mod errors;
mod generators; // Task 10
mod hypergraph;
mod layout; // Task 13
mod linalg; // Task 8
mod pyid;
#[cfg(feature = "cells3d")]
mod raster; // Phase I Task 8
mod readwrite; // Task 12
mod simplicial; // Task 7
mod stats; // Task 11
mod utils; // Task 12
mod views; // Task 5

#[pymodule]
fn _hypergraph_rs_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::register(m)?;
    hypergraph::register(m)?;
    views::register(m)?;
    dihypergraph::register(m)?;
    simplicial::register(m)?;
    linalg::register(m)?;
    algorithms::register(m)?;
    generators::register(m)?;
    stats::register(m)?;
    convert::register(m)?;
    readwrite::register(m)?;
    utils::register(m)?;
    dynamics::register(m)?;
    communities::register(m)?;
    layout::register(m)?;
    #[cfg(feature = "cells3d")]
    raster::register(m)?;
    Ok(())
}
