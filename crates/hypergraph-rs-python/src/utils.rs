//! `utils` bindings (plan Task 12c) — over Phase 6's `utils/` core module: `dual_dict`,
//! `powerset`, `subfaces`, `get_network_type`, `convert_labels_to_integers`. `IDDict` lives in
//! pure Python (`python/hypergraph_rs/utils/__init__.py`, Task 1). Deferred: the ~10
//! numeric/network utils + `crest_r` + `Trie` (Python-side stubs, unchanged by this task).
//!
//! `dual_dict`/`powerset`/`subfaces` operate on arbitrary hashables that are NOT necessarily
//! tied to any `PyHypergraph` (e.g. `xgi.powerset([1, 2, 3])`), so each call builds a
//! throwaway `PyIdMap` purely to bridge Python ids through the core's `String`-keyed
//! implementation for the duration of the call.
//!
//! **Signature note (Task 6 finding, still true):** XGI's positional order is
//! `powerset(iterable, include_empty, include_full, include_singletons, max_size)`; the core
//! Rust signature is `(items, include_empty, include_singletons, include_full, max_size)` —
//! `include_full`/`include_singletons` swapped. This binding accepts XGI's order and remaps.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3::{PyRefMut, Python};

use hypergraph_rs::utils as core_utils;

use crate::errors::xgi_error;
use crate::hypergraph::PyHypergraph;
use crate::pyid::PyIdMap;

/// `xgi.dual_dict(edge_dict)`.
#[pyfunction]
fn dual_dict(py: Python<'_>, edge_dict: Bound<'_, PyDict>) -> PyResult<Py<PyDict>> {
    let mut ids = PyIdMap::new();
    let mut core_dict: indexmap::IndexMap<String, Vec<String>> = indexmap::IndexMap::new();
    for (k, v) in edge_dict.iter() {
        let ek = ids.intern(&k)?;
        let mut members = Vec::new();
        for m in v.try_iter()? {
            members.push(ids.intern(&m?)?);
        }
        core_dict.insert(ek, members);
    }
    let node_dict = core_utils::dual_dict(&core_dict);
    let out = PyDict::new(py);
    for (node_key, edge_keys) in node_dict {
        let Some(node_obj) = ids.get_obj(py, &node_key) else {
            continue;
        };
        let list = PyList::empty(py);
        for ek in edge_keys {
            if let Some(obj) = ids.get_obj(py, &ek) {
                list.append(obj)?;
            }
        }
        out.set_item(node_obj, list)?;
    }
    Ok(out.unbind())
}

/// `xgi.powerset(iterable, include_empty=False, include_full=False, include_singletons=True,
/// max_size=None)` — returns a `list` of `tuple`s (core `chain.from_iterable(combinations(...))`
/// is XGI's lazy equivalent; a materialized `list` is an accepted, documented deviation).
#[pyfunction]
#[pyo3(signature = (iterable, include_empty=false, include_full=false, include_singletons=true, max_size=None))]
fn powerset(
    py: Python<'_>,
    iterable: Bound<'_, PyAny>,
    include_empty: bool,
    include_full: bool,
    include_singletons: bool,
    max_size: Option<usize>,
) -> PyResult<Py<PyList>> {
    let mut ids = PyIdMap::new();
    let mut items = Vec::new();
    for it in iterable.try_iter()? {
        items.push(ids.intern(&it?)?);
    }
    // Core signature order is (items, include_empty, include_singletons, include_full, max_size)
    // — swapped from XGI's positional order (see module doc).
    let subsets = core_utils::powerset(
        &items,
        include_empty,
        include_singletons,
        include_full,
        max_size,
    );
    let out = PyList::empty(py);
    for subset in subsets {
        let mut items_py = Vec::with_capacity(subset.len());
        for k in &subset {
            if let Some(obj) = ids.get_obj(py, k) {
                items_py.push(obj);
            }
        }
        out.append(PyTuple::new(py, items_py)?)?;
    }
    Ok(out.unbind())
}

/// `xgi.subfaces(edges, order=None)`.
#[pyfunction]
#[pyo3(signature = (edges, order=None))]
fn subfaces(py: Python<'_>, edges: Bound<'_, PyAny>, order: Option<i64>) -> PyResult<Py<PyList>> {
    let mut ids = PyIdMap::new();
    let mut core_edges: Vec<Vec<String>> = Vec::new();
    for edge in edges.try_iter()? {
        let edge = edge?;
        let mut members = Vec::new();
        for m in edge.try_iter()? {
            members.push(ids.intern(&m?)?);
        }
        core_edges.push(members);
    }
    let faces = core_utils::subfaces(&core_edges, order).map_err(|e| xgi_error(e.to_string()))?;
    let out = PyList::empty(py);
    for face in faces {
        let mut items_py = Vec::with_capacity(face.len());
        for k in &face {
            if let Some(obj) = ids.get_obj(py, k) {
                items_py.push(obj);
            }
        }
        out.append(PyTuple::new(py, items_py)?)?;
    }
    Ok(out.unbind())
}

/// `xgi.get_network_type(H)`. Implemented via Python type-name reflection (`type(H).__name__
/// .lower()`), matching `xgi/utils/utilities.py:560-561`'s `str(type(H))...lower()` exactly —
/// strictly more general than the core `get_network_type` stub (which hardcodes
/// `"hypergraph"` regardless of input; see `utils/mod.rs` doc), so this also gets
/// `DiHypergraph`/`SimplicialComplex` right without needing per-type dispatch here.
#[pyfunction]
fn get_network_type(obj: Bound<'_, PyAny>) -> PyResult<String> {
    Ok(obj.get_type().name()?.to_string().to_lowercase())
}

/// `xgi.convert_labels_to_integers(net, label_attribute="label", in_place=False)`.
#[pyfunction]
#[pyo3(signature = (net, label_attribute="label", in_place=false))]
fn convert_labels_to_integers(
    py: Python<'_>,
    net: Bound<'_, PyAny>,
    label_attribute: &str,
    in_place: bool,
) -> PyResult<Py<PyAny>> {
    let mut h: PyRefMut<'_, PyHypergraph> = net.extract()?;
    let new_core = core_utils::convert_labels_to_integers(&h.inner, label_attribute);
    let new_hg = PyHypergraph::from_core_with_int_ids(py, new_core)?;
    if in_place {
        h.inner = new_hg.inner;
        h.node_ids = new_hg.node_ids;
        h.edge_ids = new_hg.edge_ids;
        Ok(py.None())
    } else {
        Ok(Py::new(py, new_hg)?.into_any())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(dual_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(powerset, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(subfaces, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(get_network_type, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(convert_labels_to_integers, m)?)?;
    Ok(())
}
