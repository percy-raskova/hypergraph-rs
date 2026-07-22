//! XGIError/IDNotFound error mapping (Task 3). Task 1 only needs a
//! `register` entry point that the `_hypergraph_rs_core` pymodule fn can call
//! at every task without recompiling the registration hub each time; Task 3
//! fills this in with the real exception-class registration + core-error
//! translation helpers.

use pyo3::prelude::*;

pub fn register(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
