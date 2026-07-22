//! `readwrite` bindings (plan Task 12a) — file I/O over Phase 4's `readwrite/` core module.
//!
//! XGI parity note (verified against the installed package): `xgi.readwrite.json.read_json`/
//! `write_json` and `xgi.readwrite.hif.read_hif`/`write_hif` default `nodetype`/`edgetype` to
//! `None`, which means node/edge ids round-trip as **plain strings**, NOT the `int`-if-numeric
//! heuristic `PyHypergraph::from_core_with_int_ids` uses for generator output (verified:
//! `xgi/convert/hypergraph_dict.py:112-121` only casts via `nodetype(idx)` when `nodetype` is
//! given). This module therefore builds its own id maps (plain `str`) rather than reusing
//! `from_core_with_int_ids`, and supports `nodetype`/`edgetype` callables where XGI does.
//!
//! **Deviation (documented):** the plan's Task 12a representative snippet reads
//! `h.inner.to_hypergraph_dict_json()` — no such inherent method exists; the core exposes
//! `readwrite::json::{to_hypergraph_dict, from_hypergraph_dict}` as free functions instead, and
//! (Task 4 finding) `PyHypergraph` keeps network-level attrs in its own `net_attr` field rather
//! than `inner`'s `M` generic (which this binding never populates). Both functions below bridge
//! `net_attr` into/out of the `"hypergraph-data"` field explicitly so graph-level attrs survive
//! the round trip; the plan's snippet is followed for the file-I/O shape otherwise.
//!
//! `load_bigg_data` is gated behind the core's `network` feature (OFF in this workspace — spec
//! `#C`, no network reads in the gate); it is intentionally left unbound here. The Python skeleton
//! (`python/hypergraph_rs/readwrite/__init__.py`) already falls back to `make_deferred` for any
//! `_BOUND` name this module does not register, so `load_bigg_data` degrades gracefully and
//! `test_bigg_data.py` stays a documented D7 exclusion.

use std::path::Path;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use serde_json::Value;

use hypergraph_rs::readwrite as rw;

use crate::errors::xgi_error;
use crate::hypergraph::PyHypergraph;
use crate::pyid::PyIdMap;

/// Cast a core string id to a Python object: `cast(idx)` if a cast callable is given, else a
/// plain `str` (XGI default — verified `from_hypergraph_dict`/`read_hif` keep ids as strings
/// when `nodetype`/`edgetype` is `None`).
fn cast_id<'py>(
    py: Python<'py>,
    core_id: &str,
    cast: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    match cast {
        Some(f) => f.call1((core_id,)),
        None => Ok(PyString::new(py, core_id).into_any()),
    }
}

/// Build a `PyHypergraph` from a freshly-parsed core `Hypergraph`, casting every node/edge id
/// via `nodetype`/`edgetype` (or plain `str`), and seeding `net_attr` from `hypergraph_data`.
fn wrap_core(
    py: Python<'_>,
    core: hypergraph_rs::Hypergraph,
    nodetype: Option<&Bound<'_, PyAny>>,
    edgetype: Option<&Bound<'_, PyAny>>,
    hypergraph_data: Option<&serde_json::Map<String, Value>>,
) -> PyResult<PyHypergraph> {
    let mut node_ids = PyIdMap::new();
    for nid in core.node_ids() {
        node_ids.intern(&cast_id(py, &nid, nodetype)?)?;
    }
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        edge_ids.intern(&cast_id(py, &eid, edgetype)?)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: hypergraph_data.cloned().unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------------------------
// JSON (deep-import only: `hypergraph_rs.readwrite.json`, not part of `xgi.readwrite.__all__`).
// ---------------------------------------------------------------------------------------------

/// `hypergraph_rs.readwrite.json.write_json(H, path)`. XGI parity: `xgi.write_json`
/// (`readwrite/json.py`, itself `json.dumps(to_hypergraph_dict(H))`).
#[pyfunction]
#[pyo3(signature = (h, path))]
fn write_json(h: PyRef<'_, PyHypergraph>, path: &str) -> PyResult<()> {
    let mut data = rw::to_hypergraph_dict(&h.inner);
    data["hypergraph-data"] = Value::Object(h.net_attr.clone());
    let s = serde_json::to_string_pretty(&data).map_err(|e| xgi_error(e.to_string()))?;
    std::fs::write(path, s).map_err(|e| xgi_error(e.to_string()))
}

/// `hypergraph_rs.readwrite.json.read_json(path, nodetype=None, edgetype=None)`.
#[pyfunction]
#[pyo3(signature = (path, nodetype=None, edgetype=None))]
fn read_json(
    py: Python<'_>,
    path: &str,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let s = std::fs::read_to_string(path).map_err(|e| xgi_error(e.to_string()))?;
    let data: Value = serde_json::from_str(&s).map_err(|e| xgi_error(e.to_string()))?;
    let core = rw::from_hypergraph_dict(&data).map_err(|e| xgi_error(e.to_string()))?;
    let hgdata = data.get("hypergraph-data").and_then(|v| v.as_object());
    wrap_core(py, core, nodetype.as_ref(), edgetype.as_ref(), hgdata)
}

// ---------------------------------------------------------------------------------------------
// HIF (`xgi.readwrite.hif`; `write_hif`/`read_hif` ARE part of `xgi.readwrite.__all__`).
// ---------------------------------------------------------------------------------------------

/// `xgi.write_hif(H, path)`. XGI parity: `xgi.write_hif` (`readwrite/hif.py`, itself
/// `json.dumps(to_hif_dict(H))`).
#[pyfunction]
#[pyo3(signature = (h, path, **_kwargs))]
fn write_hif(
    h: PyRef<'_, PyHypergraph>,
    path: &str,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<()> {
    let mut data = rw::to_hif_dict(&h.inner);
    data["metadata"] = Value::Object(h.net_attr.clone());
    let s = serde_json::to_string_pretty(&data).map_err(|e| xgi_error(e.to_string()))?;
    std::fs::write(path, s).map_err(|e| xgi_error(e.to_string()))
}

/// `xgi.read_hif(path, nodetype=None, edgetype=None)`.
#[pyfunction]
#[pyo3(signature = (path, nodetype=None, edgetype=None))]
fn read_hif(
    py: Python<'_>,
    path: &str,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let s = std::fs::read_to_string(path).map_err(|e| xgi_error(e.to_string()))?;
    let data: Value = serde_json::from_str(&s).map_err(|e| xgi_error(e.to_string()))?;
    let core = rw::from_hif_dict(&data).map_err(|e| xgi_error(e.to_string()))?;
    let metadata = data.get("metadata").and_then(|v| v.as_object());
    wrap_core(py, core, nodetype.as_ref(), edgetype.as_ref(), metadata)
}

// ---------------------------------------------------------------------------------------------
// Edgelist.
// ---------------------------------------------------------------------------------------------

fn first_char(s: Option<&str>) -> Option<char> {
    s.and_then(|s| s.chars().next())
}

/// `xgi.parse_edgelist(lines, comments="#", delimiter=None, nodetype=None)`.
#[pyfunction]
#[pyo3(signature = (lines, comments="#", delimiter=None, nodetype=None))]
fn parse_edgelist(
    py: Python<'_>,
    lines: Bound<'_, PyAny>,
    comments: &str,
    delimiter: Option<&str>,
    nodetype: Option<Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let mut raw = Vec::new();
    for line in lines.try_iter()? {
        raw.push(line?.extract::<String>()?);
    }
    let core = rw::parse_edgelist(raw, first_char(delimiter), comments)
        .map_err(|e| xgi_error(e.to_string()))?;
    wrap_core(py, core, nodetype.as_ref(), None, None)
}

/// `xgi.read_edgelist(path, comments="#", delimiter=None, nodetype=None)`.
#[pyfunction]
#[pyo3(signature = (path, comments="#", delimiter=None, nodetype=None))]
fn read_edgelist(
    py: Python<'_>,
    path: &str,
    comments: &str,
    delimiter: Option<&str>,
    nodetype: Option<Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let core = rw::read_edgelist(Path::new(path), first_char(delimiter), comments)
        .map_err(|e| xgi_error(e.to_string()))?;
    wrap_core(py, core, nodetype.as_ref(), None, None)
}

/// `xgi.write_edgelist(H, path, delimiter=" ")`.
#[pyfunction]
#[pyo3(signature = (h, path, delimiter=" "))]
fn write_edgelist(h: PyRef<'_, PyHypergraph>, path: &str, delimiter: &str) -> PyResult<()> {
    let d = first_char(Some(delimiter)).unwrap_or(' ');
    rw::write_edgelist(&h.inner, Path::new(path), d).map_err(|e| xgi_error(e.to_string()))
}

// ---------------------------------------------------------------------------------------------
// Bipartite edgelist.
// ---------------------------------------------------------------------------------------------

/// `xgi.parse_bipartite_edgelist(lines, comments="#", delimiter=None, nodetype=None,
/// edgetype=None, dual=False)`.
#[pyfunction]
#[pyo3(signature = (lines, comments="#", delimiter=None, nodetype=None, edgetype=None, dual=false))]
#[allow(clippy::too_many_arguments)]
fn parse_bipartite_edgelist(
    py: Python<'_>,
    lines: Bound<'_, PyAny>,
    comments: &str,
    delimiter: Option<&str>,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
    dual: bool,
) -> PyResult<PyHypergraph> {
    let mut raw = Vec::new();
    for line in lines.try_iter()? {
        raw.push(line?.extract::<String>()?);
    }
    let core = rw::parse_bipartite_edgelist(raw, first_char(delimiter), comments, dual)
        .map_err(|e| xgi_error(e.to_string()))?;
    wrap_core(py, core, nodetype.as_ref(), edgetype.as_ref(), None)
}

/// `xgi.read_bipartite_edgelist(path, comments="#", delimiter=None, nodetype=None,
/// edgetype=None, dual=False)`.
#[pyfunction]
#[pyo3(signature = (path, comments="#", delimiter=None, nodetype=None, edgetype=None, dual=false))]
#[allow(clippy::too_many_arguments)]
fn read_bipartite_edgelist(
    py: Python<'_>,
    path: &str,
    comments: &str,
    delimiter: Option<&str>,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
    dual: bool,
) -> PyResult<PyHypergraph> {
    let core = rw::read_bipartite_edgelist(Path::new(path), first_char(delimiter), comments, dual)
        .map_err(|e| xgi_error(e.to_string()))?;
    wrap_core(py, core, nodetype.as_ref(), edgetype.as_ref(), None)
}

/// `xgi.write_bipartite_edgelist(H, path, delimiter=" ")`.
#[pyfunction]
#[pyo3(signature = (h, path, delimiter=" "))]
fn write_bipartite_edgelist(
    h: PyRef<'_, PyHypergraph>,
    path: &str,
    delimiter: &str,
) -> PyResult<()> {
    let d = first_char(Some(delimiter)).unwrap_or(' ');
    rw::write_bipartite_edgelist(&h.inner, Path::new(path), d).map_err(|e| xgi_error(e.to_string()))
}

// ---------------------------------------------------------------------------------------------
// Incidence matrix.
// ---------------------------------------------------------------------------------------------

/// `xgi.read_incidence_matrix(path, comments="#", delimiter=None)`. Positional integer ids
/// (`"0".."k-1"`) — matches `from_incidence_matrix` (core doc: `readwrite/incidence.rs`).
#[pyfunction]
#[pyo3(signature = (path, comments="#", delimiter=None))]
fn read_incidence_matrix(
    py: Python<'_>,
    path: &str,
    comments: &str,
    delimiter: Option<&str>,
) -> PyResult<PyHypergraph> {
    let _ = comments; // core reader has no comment-stripping (dense-matrix format); accepted for signature parity
    let core = rw::read_incidence_matrix(Path::new(path), first_char(delimiter))
        .map_err(|e| xgi_error(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.write_incidence_matrix(H, path, delimiter=" ")`.
#[pyfunction]
#[pyo3(signature = (h, path, delimiter=" "))]
fn write_incidence_matrix(h: PyRef<'_, PyHypergraph>, path: &str, delimiter: &str) -> PyResult<()> {
    let d = first_char(Some(delimiter)).unwrap_or(' ');
    rw::write_incidence_matrix(&h.inner, Path::new(path), d).map_err(|e| xgi_error(e.to_string()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(write_json, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(read_json, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_hif, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(read_hif, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(read_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_bipartite_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(read_bipartite_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_bipartite_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(read_incidence_matrix, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(write_incidence_matrix, m)?)?;
    // `_hypergraph_rs_core.write_json`/`read_json` are picked up by the deep-import shim
    // `python/hypergraph_rs/readwrite/json.py`, not by `readwrite/__init__.py`'s `_BOUND` list
    // (neither is part of `xgi.readwrite.__all__` — verified).
    Ok(())
}
