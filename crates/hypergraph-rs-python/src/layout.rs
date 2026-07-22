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

use std::f64::consts::PI;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use hypergraph_rs::layout::{
    bipartite_layout, spring_layout, BipartiteAlign, HyperedgeGeometry, LayoutResult,
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

/// `xgi.bipartite_spring_layout(H, seed=None)` — two-column agent/hyperedge placement (Phase 5's
/// deterministic `bipartite_layout`, not force-directed — declared deviation; `seed` is accepted
/// for XGI signature parity and ignored since the placement has no RNG to seed). Returns the XGI
/// shape: a `(node_positions, edge_positions)` tuple of dicts (`xgi/drawing/layout.py:186-226`).
#[pyfunction]
#[pyo3(signature = (h, seed=None))]
fn bipartite_spring_layout(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    seed: Option<u64>,
) -> PyResult<(Py<PyDict>, Py<PyDict>)> {
    let _ = seed;
    let result = bipartite_layout(&h.inner, BipartiteAlign::Vertical);
    let nodes = PyDict::new(py);
    for (core_id, (x, y)) in &result.node_positions {
        if let Some(obj) = h.node_ids.get_obj(py, core_id) {
            nodes.set_item(obj, (*x, *y))?;
        }
    }
    let edges = PyDict::new(py);
    for (core_id, geom) in &result.edge_geometries {
        if let HyperedgeGeometry::Stripe { x, y_min, y_max } = geom {
            if let Some(obj) = h.edge_ids.get_obj(py, core_id) {
                edges.set_item(obj, (*x, (*y_min + *y_max) / 2.0))?;
            }
        }
    }
    Ok((nodes.unbind(), edges.unbind()))
}

/// `xgi.circular_layout(H, center=None, radius=None)` — every node evenly spaced on a circle
/// (`xgi/drawing/layout.py:470-506`, ported directly rather than reusing Phase 5's `radial_layout`
/// so `center`/`radius` match XGI exactly: `theta = linspace(0, 1, n+1)[:-1] * 2*pi`,
/// `pos = radius * (cos theta, sin theta) + center`; `n == 0` -> `{}`, `n == 1` -> `{node: center}`
/// (the *same* center object, matching XGI's `pos = {node: center}` identity).
#[pyfunction]
#[pyo3(signature = (h, center=None, radius=None))]
fn circular_layout<'py>(
    py: Python<'py>,
    h: PyRef<'_, PyHypergraph>,
    center: Option<Bound<'py, PyAny>>,
    radius: Option<f64>,
) -> PyResult<Py<PyDict>> {
    let (cx, cy): (f64, f64) = match &center {
        Some(c) => {
            let v: Vec<f64> = c.extract()?;
            let x = *v
                .first()
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("center needs 2 coords"))?;
            let y = *v
                .get(1)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("center needs 2 coords"))?;
            (x, y)
        }
        None => (0.0, 0.0),
    };
    let r = radius.unwrap_or(1.0);
    let node_ids = h.inner.node_ids();
    let n = node_ids.len();
    let d = PyDict::new(py);
    if n == 0 {
        return Ok(d.unbind());
    }
    if n == 1 {
        if let Some(obj) = h.node_ids.get_obj(py, &node_ids[0]) {
            match &center {
                Some(c) => d.set_item(obj, c.clone())?,
                None => d.set_item(obj, (cx, cy))?,
            }
        }
        return Ok(d.unbind());
    }
    for (i, nid) in node_ids.iter().enumerate() {
        let theta = (i as f64 / n as f64) * 2.0 * PI;
        let x = r * theta.cos() + cx;
        let y = r * theta.sin() + cy;
        if let Some(obj) = h.node_ids.get_obj(py, nid) {
            d.set_item(obj, (x, y))?;
        }
    }
    Ok(d.unbind())
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(random_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(pairwise_spring_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(bipartite_spring_layout, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(circular_layout, m)?)?;
    Ok(())
}
