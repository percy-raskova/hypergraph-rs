//! `linalg` bindings — numpy dense / scipy sparse returns over the 8 matrix operators
//! Phase 3 built (spec §9.1 Task 8). Deferred (Phase 3 "Deferred"): `boundary_matrix`,
//! `hodge_laplacian`, `adjacency_tensor` — those stay Python-side `make_deferred` stubs
//! (`python/hypergraph_rs/linalg/__init__.py`).
//!
//! XGI parity signatures verified against the installed package
//! (`xgi/linalg/hypergraph_matrix.py`, `xgi/linalg/laplacian_matrix.py`).
//!
//! **D11 (plan Task 8):** core `ndarray` (0.17, pinned) is never handed straight to
//! `numpy::IntoPyArray` — every dense crossing goes through a flat row-major `Vec<f64>` + shape,
//! reshaped on the numpy side. `cargo tree -i ndarray` (execution-time check) shows a single
//! unified 0.17.2 in this workspace, so the crossing would in fact typecheck today, but the plan's
//! rule is followed regardless — it is the durable-safe contract, not a today-only workaround.
//!
//! **`incidence_matrix` weight callable:** the core computes an unweighted (binary) incidence
//! structure; the per-(node, edge) `weight` callable from XGI's signature is applied binding-side
//! by calling it once per nonzero incidence entry (in CSR row-major order, matching XGI's own
//! `for edge in edge_ids: for node in members: data.append(weight(node, edge, H))` iteration —
//! `hypergraph_matrix.py:118-126`) and substituting the resulting values for the core's `1.0`s.

use numpy::{IntoPyArray, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::errors::xgi_error;
use crate::hypergraph::PyHypergraph;
use crate::pyid::PyIdMap;

/// Build `{index: original_python_id}` for one axis (node or edge ids, in core order).
fn id_dict<'py>(py: Python<'py>, ids: &[String], map: &PyIdMap) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    for (i, cid) in ids.iter().enumerate() {
        if let Some(obj) = map.get_obj(py, cid) {
            d.set_item(i, obj)?;
        }
    }
    Ok(d)
}

/// `(matrix, rowdict)` when `index`, else bare `matrix`.
fn with_index1<'py>(
    py: Python<'py>,
    matrix: Bound<'py, PyAny>,
    ids: &[String],
    map: &PyIdMap,
    index: bool,
) -> PyResult<Py<PyAny>> {
    if !index {
        return Ok(matrix.unbind());
    }
    let d = id_dict(py, ids, map)?;
    Ok(PyTuple::new(py, [matrix, d.into_any()])?
        .into_any()
        .unbind())
}

/// `(matrix, rowdict, coldict)` when `index`, else bare `matrix`.
fn with_index2<'py>(
    py: Python<'py>,
    matrix: Bound<'py, PyAny>,
    row_ids: &[String],
    row_map: &PyIdMap,
    col_ids: &[String],
    col_map: &PyIdMap,
    index: bool,
) -> PyResult<Py<PyAny>> {
    if !index {
        return Ok(matrix.unbind());
    }
    let rd = id_dict(py, row_ids, row_map)?;
    let cd = id_dict(py, col_ids, col_map)?;
    Ok(PyTuple::new(py, [matrix, rd.into_any(), cd.into_any()])?
        .into_any()
        .unbind())
}

/// Dense `Array2<f64>` → row-major numpy `ndarray` (D11: flat `Vec<f64>` + reshape crossing).
fn dense2_to_pyarray<'py>(
    py: Python<'py>,
    arr: ndarray::Array2<f64>,
) -> PyResult<Bound<'py, PyAny>> {
    let (rows, cols) = arr.dim();
    let flat = arr.into_raw_vec_and_offset().0;
    Ok(flat.into_pyarray(py).reshape((rows, cols))?.into_any())
}

/// Dense `Array1<f64>` → 1-D numpy `ndarray` (no reshape needed; `Vec<f64>` is already 1-D).
fn dense1_to_pyarray<'py>(
    py: Python<'py>,
    arr: ndarray::Array1<f64>,
) -> PyResult<Bound<'py, PyAny>> {
    let flat = arr.into_raw_vec_and_offset().0;
    Ok(flat.into_pyarray(py).into_any())
}

/// Build `scipy.sparse.csr_array((data, indices, indptr), shape=…)` from a core `sprs::CsMat`,
/// with `data` supplied explicitly (lets callers substitute weighted values for the core's
/// binary `1.0`s while keeping the CSR index/indptr structure).
fn to_scipy_csr_data<'py>(
    py: Python<'py>,
    m: &sprs::CsMat<f64>,
    data: &[f64],
) -> PyResult<Bound<'py, PyAny>> {
    let sp = py.import("scipy.sparse")?;
    let data_arr = data.to_vec().into_pyarray(py);
    let indices: Vec<i32> = m.indices().iter().map(|&i| i as i32).collect();
    let indptr: Vec<i32> = m
        .indptr()
        .as_slice()
        .unwrap_or(&[])
        .iter()
        .map(|&i| i as i32)
        .collect();
    let kwargs = PyDict::new(py);
    kwargs.set_item("shape", (m.rows(), m.cols()))?;
    sp.getattr("csr_array")?.call(
        ((data_arr, indices.into_pyarray(py), indptr.into_pyarray(py)),),
        Some(&kwargs),
    )
}

/// `to_scipy_csr_data` using the matrix's own (unweighted) data.
fn to_scipy_csr<'py>(py: Python<'py>, m: &sprs::CsMat<f64>) -> PyResult<Bound<'py, PyAny>> {
    to_scipy_csr_data(py, m, m.data())
}

/// `scipy.sparse.csr_array(dense_ndarray)` — used where the core only computes a dense matrix
/// (adjacency/intersection/clique-motif/laplacian family) but `sparse=True` was requested.
fn dense_to_scipy_csr<'py>(
    py: Python<'py>,
    dense: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sp = py.import("scipy.sparse")?;
    sp.getattr("csr_array")?.call1((dense,))
}

/// Apply an XGI `weight(node, edge, H)` callable to every nonzero entry of `m`, in CSR row-major
/// order, returning the substituted data vector (same length/order as `m.data()`).
fn weighted_incidence_data(
    py: Python<'_>,
    m: &sprs::CsMat<f64>,
    rows: &[String],
    cols: &[String],
    node_map: &PyIdMap,
    edge_map: &PyIdMap,
    h_obj: &Bound<'_, PyAny>,
    weight: &Bound<'_, PyAny>,
) -> PyResult<Vec<f64>> {
    let indptr_owned = m.indptr();
    let indptr = indptr_owned.as_slice().unwrap_or(&[]);
    let indices = m.indices();
    let mut out = Vec::with_capacity(indices.len());
    for r in 0..m.rows() {
        let node_obj = node_map
            .get_obj(py, &rows[r])
            .expect("row id present in node map");
        for pos in indptr[r]..indptr[r + 1] {
            let c = indices[pos];
            let edge_obj = edge_map
                .get_obj(py, &cols[c])
                .expect("col id present in edge map");
            let val: f64 = weight
                .call1((node_obj.clone(), edge_obj.clone(), h_obj.clone()))?
                .extract()?;
            out.push(val);
        }
    }
    Ok(out)
}

/// Scatter a weighted CSR data vector into a dense `Array2<f64>` (zeros elsewhere) — matches
/// XGI's dense path `I = np.zeros(...); I[rows, cols] = data` (`hypergraph_matrix.py:127-129`).
fn weighted_incidence_dense(m: &sprs::CsMat<f64>, data: &[f64]) -> ndarray::Array2<f64> {
    let mut dense = ndarray::Array2::<f64>::zeros((m.rows(), m.cols()));
    let indptr_owned = m.indptr();
    let indptr = indptr_owned.as_slice().unwrap_or(&[]);
    let indices = m.indices();
    for r in 0..m.rows() {
        for pos in indptr[r]..indptr[r + 1] {
            dense[[r, indices[pos]]] = data[pos];
        }
    }
    dense
}

/// `xgi.incidence_matrix(H, order=None, sparse=True, index=False, weight=None)`.
#[pyfunction]
#[pyo3(signature = (h, order=None, sparse=true, index=false, weight=None))]
fn incidence_matrix(
    py: Python<'_>,
    h: Bound<'_, PyHypergraph>,
    order: Option<usize>,
    sparse: bool,
    index: bool,
    weight: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let (m, rows, cols) = {
        let h_ref = h.borrow();
        hypergraph_rs::linalg::incidence_matrix_sparse(&h_ref.inner, order)
    };
    let h_any = h.clone().into_any();
    let data: Option<Vec<f64>> = match &weight {
        Some(w) => {
            let h_ref = h.borrow();
            Some(weighted_incidence_data(
                py,
                &m,
                &rows,
                &cols,
                &h_ref.node_ids,
                &h_ref.edge_ids,
                &h_any,
                w,
            )?)
        }
        None => None,
    };
    let h_ref = h.borrow();
    if sparse {
        let arr = match &data {
            Some(d) => to_scipy_csr_data(py, &m, d)?,
            None => to_scipy_csr(py, &m)?,
        };
        with_index2(
            py,
            arr,
            &rows,
            &h_ref.node_ids,
            &cols,
            &h_ref.edge_ids,
            index,
        )
    } else {
        let dense = match &data {
            Some(d) => weighted_incidence_dense(&m, d),
            None => hypergraph_rs::linalg::incidence_matrix(&h_ref.inner, order).0,
        };
        let arr = dense2_to_pyarray(py, dense)?;
        with_index2(
            py,
            arr,
            &rows,
            &h_ref.node_ids,
            &cols,
            &h_ref.edge_ids,
            index,
        )
    }
}

/// `xgi.adjacency_matrix(H, order=None, sparse=True, s=1, weighted=False, index=False)`.
#[pyfunction]
#[pyo3(signature = (h, order=None, sparse=true, s=1, weighted=false, index=false))]
fn adjacency_matrix(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    order: Option<usize>,
    sparse: bool,
    s: usize,
    weighted: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (m, rows) = hypergraph_rs::linalg::adjacency_matrix(&h.inner, order, s, weighted);
    let dense = dense2_to_pyarray(py, m)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &rows, &h.node_ids, index)
}

/// `xgi.intersection_profile(H, order=None, sparse=True, index=False)`. Row/col ids = edge ids.
#[pyfunction]
#[pyo3(signature = (h, order=None, sparse=true, index=false))]
fn intersection_profile(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    order: Option<usize>,
    sparse: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (m, cols) = hypergraph_rs::linalg::intersection_profile(&h.inner, order);
    let dense = dense2_to_pyarray(py, m)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &cols, &h.edge_ids, index)
}

/// `xgi.degree_matrix(H, order=None, index=False)`. Always a 1-D dense numpy array (no `sparse`
/// parameter in XGI's own signature).
#[pyfunction]
#[pyo3(signature = (h, order=None, index=false))]
fn degree_matrix(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    order: Option<usize>,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (k, rows) = hypergraph_rs::linalg::degree_matrix(&h.inner, order);
    let arr = dense1_to_pyarray(py, k)?;
    with_index1(py, arr, &rows, &h.node_ids, index)
}

/// `xgi.clique_motif_matrix(H, sparse=True, index=False)` = `adjacency_matrix(H, None, 1, True)`.
#[pyfunction]
#[pyo3(signature = (h, sparse=true, index=false))]
fn clique_motif_matrix(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    sparse: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (m, rows) = hypergraph_rs::linalg::clique_motif_matrix(&h.inner);
    let dense = dense2_to_pyarray(py, m)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &rows, &h.node_ids, index)
}

/// `xgi.laplacian(H, order=1, sparse=False, rescale_per_node=False, index=False)`.
#[pyfunction]
#[pyo3(signature = (h, order=1, sparse=false, rescale_per_node=false, index=false))]
fn laplacian(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    order: usize,
    sparse: bool,
    rescale_per_node: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (l, rows) = hypergraph_rs::linalg::laplacian(&h.inner, order, rescale_per_node);
    let dense = dense2_to_pyarray(py, l)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &rows, &h.node_ids, index)
}

/// `xgi.multiorder_laplacian(H, orders, weights, sparse=False, rescale_per_node=False, index=False)`.
/// XGI raises `ValueError` on mismatched lengths (`laplacian_matrix.py:156-157`); the core panics
/// via `assert_eq!`, so the length check is duplicated here to surface a catchable `ValueError`
/// instead of a Rust-panic `PanicException`.
#[pyfunction]
#[pyo3(signature = (h, orders, weights, sparse=false, rescale_per_node=false, index=false))]
fn multiorder_laplacian(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    orders: Vec<usize>,
    weights: Vec<f64>,
    sparse: bool,
    rescale_per_node: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    if orders.len() != weights.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "orders and weights must have the same length.",
        ));
    }
    // XGI parity: warn (not error) for each order whose degree matrix is all-zero — its
    // contribution is silently dropped rather than dividing by a zero mean (laplacian_matrix.py:172-179).
    let warnings = py.import("warnings")?;
    for &d in &orders {
        let (k, _) = hypergraph_rs::linalg::degree_matrix(&h.inner, Some(d));
        if k.iter().all(|&v| v == 0.0) {
            let msg = format!(
                "No edges of order {d}. Contribution of that order is zero. Its weight is effectively zero."
            );
            warnings.call_method1("warn", (msg,))?;
        }
    }
    let (l, rows) =
        hypergraph_rs::linalg::multiorder_laplacian(&h.inner, &orders, &weights, rescale_per_node);
    let dense = dense2_to_pyarray(py, l)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &rows, &h.node_ids, index)
}

/// `xgi.normalized_hypergraph_laplacian(H, weighted=False, sparse=True, index=False)`. Isolated
/// nodes → `XGIError` (core `LinalgError::IsolatedNodes`).
#[pyfunction]
#[pyo3(signature = (h, weighted=false, sparse=true, index=false))]
fn normalized_hypergraph_laplacian(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    weighted: bool,
    sparse: bool,
    index: bool,
) -> PyResult<Py<PyAny>> {
    let (l, rows) = hypergraph_rs::linalg::normalized_hypergraph_laplacian(&h.inner, weighted)
        .map_err(|e| xgi_error(e.to_string()))?;
    let dense = dense2_to_pyarray(py, l)?;
    let matrix = if sparse {
        dense_to_scipy_csr(py, &dense)?
    } else {
        dense
    };
    with_index1(py, matrix, &rows, &h.node_ids, index)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(incidence_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(adjacency_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(intersection_profile, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(degree_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(clique_motif_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(laplacian, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(multiorder_laplacian, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(normalized_hypergraph_laplacian, m)?)?;
    Ok(())
}
