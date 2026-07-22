//! Bridge core `EdgeError`/`NodeError`/`SwapError` → XGI-compatible Python exceptions.
//!
//! Tests catch `xgi.exception.XGIError` and `xgi.exception.IDNotFound`
//! (`tests/core/test_hypergraph.py:7`). `IDNotFound` subclasses KeyError (`xgi/exception.py:31`),
//! so it also satisfies `pytest.raises(KeyError)`.

use pyo3::prelude::*;

// pyo3 0.29 has NO `import_exception_bound!` (verified in the cached crate source — only
// `import_exception!` exists, and it already yields Bound-compatible types with `::new_err(...)`).
pyo3::import_exception!(hypergraph_rs.exception, XGIError);
pyo3::import_exception!(hypergraph_rs.exception, IDNotFound);
pyo3::import_exception!(hypergraph_rs.exception, XGIException);

pub fn id_not_found(id: &str) -> PyErr {
    IDNotFound::new_err(format!("{id} is not in the network"))
}
pub fn xgi_error(msg: impl Into<String>) -> PyErr {
    XGIError::new_err(msg.into())
}

/// Registrar (nothing to add to `_core`; the classes live in pure Python). Kept for uniformity.
pub fn register(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
