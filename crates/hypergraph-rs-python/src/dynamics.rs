//! `dynamics` bindings (plan Task 12d) — Kuramoto synchronization over Phase 6's `dynamics/`
//! core module. Deferred: `simulate_simplicial_kuramoto`, `compute_simplicial_order_parameter`
//! (no Phase 0-6 Hodge-boundary-matrix support; §A).
//!
//! **§9.4 / OQ-3 ruling (kept split, per plan):** `tests/dynamics/test_synchronization.py`
//! asserts a hard-coded `output` trajectory array to ~1e-8 — not a legitimate cross-language
//! contract (libm `sin`/`exp` transcendentals don't reproduce bit-for-bit across
//! implementations, unlike basic IEEE-754 ops; CLAUDE.md float policy). The exact-trajectory
//! asserts are a documented exclusion; only the structural shape/`times` asserts are ported
//! (Task 15). This binding itself is behaviorally complete either way — the exclusion is a
//! test-triage decision, not a binding gap.

use numpy::{IntoPyArray, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use hypergraph_rs::dynamics as core_dyn;

use crate::hypergraph::PyHypergraph;

/// Convert an array-like Python object (list or numpy array) to `Vec<f64>` via
/// `numpy.asarray` (matches XGI's flexible acceptance of "numpy array of real values").
fn as_vec1(py: Python<'_>, obj: Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
    let np = py.import("numpy")?;
    let arr_obj = np.call_method1("asarray", (obj,))?;
    let arr: PyReadonlyArray1<'_, f64> = arr_obj.extract()?;
    Ok(arr.as_array().iter().copied().collect())
}

/// `xgi.simulate_kuramoto(H, k2, k3, omega=None, theta=None, timesteps=10000, dt=0.002,
/// seed=None)`. Returns `(theta_time, times)` as numpy arrays.
#[pyfunction]
#[pyo3(signature = (h, k2, k3, omega=None, theta=None, timesteps=10000, dt=0.002, seed=None))]
#[allow(clippy::too_many_arguments)]
fn simulate_kuramoto(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    k2: f64,
    k3: f64,
    omega: Option<Bound<'_, PyAny>>,
    theta: Option<Bound<'_, PyAny>>,
    timesteps: usize,
    dt: f64,
    seed: Option<u64>,
) -> PyResult<Py<PyTuple>> {
    let omega_vec = omega.map(|o| as_vec1(py, o)).transpose()?;
    let theta_vec = theta.map(|t| as_vec1(py, t)).transpose()?;
    let (theta_time, times) =
        core_dyn::simulate_kuramoto(&h.inner, k2, k3, omega_vec, theta_vec, timesteps, dt, seed);
    let rows = theta_time.len();
    let cols = theta_time.first().map(|r| r.len()).unwrap_or(0);
    let flat: Vec<f64> = theta_time.into_iter().flatten().collect();
    let theta_arr = flat.into_pyarray(py).reshape((rows, cols))?;
    let times_arr = times.into_pyarray(py);
    Ok(PyTuple::new(py, [theta_arr.into_any(), times_arr.into_any()])?.unbind())
}

/// `xgi.compute_kuramoto_order_parameter(theta_time)`.
#[pyfunction]
fn compute_kuramoto_order_parameter(
    py: Python<'_>,
    theta_time: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let np = py.import("numpy")?;
    let arr_obj = np.call_method1("asarray", (theta_time,))?;
    let arr: PyReadonlyArray2<'_, f64> = arr_obj.extract()?;
    let rows: Vec<Vec<f64>> = arr.as_array().outer_iter().map(|r| r.to_vec()).collect();
    let out = core_dyn::compute_kuramoto_order_parameter(&rows);
    Ok(out.into_pyarray(py).into_any().unbind())
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(simulate_kuramoto, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_kuramoto_order_parameter, m)?)?;
    Ok(())
}
