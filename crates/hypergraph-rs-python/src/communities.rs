//! `communities` bindings (plan Task 12e) — `spectral_clustering` over Phase 6's
//! `communities/` core module.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use hypergraph_rs::communities as core_comm;

use crate::errors::xgi_error;
use crate::hypergraph::PyHypergraph;

/// `xgi.spectral_clustering(H, k=2, max_iter=1_000, seed=None)`. Returns a `dict` mapping
/// node ids to their cluster (clusters begin at 0). Deterministic under `seed`; structural
/// (not exact-XGI) parity per §9.4 (eigendecomposition/k-means, not a bit-for-bit contract).
#[pyfunction]
#[pyo3(signature = (h, k=2, max_iter=1_000, seed=None))]
fn spectral_clustering(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    k: usize,
    max_iter: usize,
    seed: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let labels = core_comm::spectral_clustering(&h.inner, k, max_iter, seed)
        .map_err(|e| xgi_error(e.to_string()))?;
    let out = PyDict::new(py);
    for (nid, cluster) in labels {
        if let Some(obj) = h.node_ids.get_obj(py, &nid) {
            out.set_item(obj, cluster)?;
        }
    }
    Ok(out.unbind())
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(spectral_clustering, m)?)?;
    Ok(())
}
