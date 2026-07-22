//! `layout` bindings (plan Task 13) — XGI `xgi.drawing.layout` positioning functions, mapped onto
//! Phase 5's `spring_layout` / `bipartite_layout` / `radial_layout`. Every function returns
//! `dict[id -> (x, y)]` keyed by the *original* Python node ids (translated through `PyIdMap`).
//!
//! Mapping (10 XGI names, verified against `xgi/drawing/layout.py:__all__`):
//! - `random_layout` -> `spring_layout(h, 0, seed)` (zero FR iterations == the raw seeded
//!   uniform-random initial positions XGI's `random_layout` produces — verified
//!   `crates/hypergraph-rs/src/layout/spring.rs:fruchterman_reingold` returns the RNG-seeded
//!   initial `pos` unmodified when `iterations == 0`).
//! - `pairwise_spring_layout` -> `spring_layout(h, 50, seed)` (the 1-skeleton FR layout).
//! - `bipartite_spring_layout` -> `bipartite_layout(h, Vertical)` (nearest Phase-5 analogue;
//!   deterministic two-column placement, not force-directed — declared deviation).
//! - `circular_layout` -> `radial_layout(h, 1)` (a single ring == every node on one circle).
//!
//! **Deferred** (no Phase-5 analogue — barycenter/PCA/Kamada-Kawai/spiral math was never built):
//! `barycenter_spring_layout`, `weighted_barycenter_spring_layout`, `pca_transform`,
//! `spiral_layout`, `barycenter_kamada_kawai_layout`, `edge_positions_from_barycenters` — left
//! unbound; the Python skeleton (`python/hypergraph_rs/drawing/__init__.py`) falls back to
//! `make_deferred` for any symbol this module does not register.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use hypergraph_rs::layout::{
    bipartite_layout, radial_layout, spring_layout, BipartiteAlign, LayoutResult,
};

use crate::hypergraph::PyHypergraph;

/// The default FR iteration count for the non-`random` spring-family layouts.
const PAIRWISE_SPRING_ITERATIONS: usize = 50;

/// Translate a `LayoutResult`'s node positions into `{original_id: (x, y)}`, skipping any core id
/// with no `PyIdMap` entry (defensive; every node id reaching a `PyHypergraph` is interned).
fn positions_to_dict(
    py: Python<'_>,
    h: &PyHypergraph,
    result: &LayoutResult,
) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    for (core_id, (x, y)) in &result.node_positions {
        if let Some(obj) = h.node_ids.get_obj(py, core_id) {
            d.set_item(obj, (*x, *y))?;
        }
    }
    Ok(d.unbind())
}

/// `xgi.random_layout(H, seed=None)` — uniformly random positions in the unit square.
#[pyfunction]
#[pyo3(signature = (h, seed=None))]
fn random_layout(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    seed: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let result = spring_layout(&h.inner, 0, seed);
    positions_to_dict(py, &h, &result)
}

/// `xgi.pairwise_spring_layout(H, seed=None)` — Fruchterman-Reingold on the pairwise 1-skeleton.
#[pyfunction]
#[pyo3(signature = (h, seed=None))]
fn pairwise_spring_layout(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    seed: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let result = spring_layout(&h.inner, PAIRWISE_SPRING_ITERATIONS, seed);
    positions_to_dict(py, &h, &result)
}

/// `xgi.bipartite_spring_layout(H)` — two-column agent/hyperedge placement (Phase 5's
/// deterministic `bipartite_layout`, not force-directed — declared deviation).
#[pyfunction]
fn bipartite_spring_layout(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let result = bipartite_layout(&h.inner, BipartiteAlign::Vertical);
    positions_to_dict(py, &h, &result)
}

/// `xgi.circular_layout(H)` — every node on a single circle (Phase 5's `radial_layout` with one
/// ring collapses to a circular placement).
#[pyfunction]
fn circular_layout(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let result = radial_layout(&h.inner, 1);
    positions_to_dict(py, &h, &result)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(random_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(pairwise_spring_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(bipartite_spring_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(circular_layout, m)?)?;
    Ok(())
}
