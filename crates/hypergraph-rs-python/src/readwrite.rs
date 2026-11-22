//! `readwrite` bindings (plan Task 12a) — file I/O over Phase 4's `readwrite/` core module.
//!
//! XGI parity note (verified against the installed package): `xgi.readwrite.hif.read_hif`/
//! `write_hif` default `nodetype`/`edgetype` to `None`, which means node/edge ids round-trip as
//! **plain strings**, NOT the `int`-if-numeric heuristic `PyHypergraph::from_core_with_int_ids`
//! uses for generator output. This module therefore builds its own id maps (plain `str`) rather
//! than reusing `from_core_with_int_ids`, and supports `nodetype`/`edgetype` callables where XGI
//! does. XGI 1.1 dropped its own JSON format (`read_json`, `write_json`, `to_hypergraph_dict`);
//! so did this library.
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

/// Cast a core string id to a Python object: `cast(idx)` if a cast callable is given; else,
/// when `default_int` is set, an `int` if the id parses cleanly as one (else `str`) — HIF's
/// default (`nodetype=None`) preserves the JSON-native type of incidence-array entries (numbers
/// stay numbers); else a plain `str` (`json.py`'s default — dict-key ids are always JSON
/// strings, so there is no native-int case to preserve).
fn cast_id<'py>(
    py: Python<'py>,
    core_id: &str,
    cast: Option<&Bound<'py, PyAny>>,
    default_int: bool,
) -> PyResult<Bound<'py, PyAny>> {
    match cast {
        // XGI parity: a `ValueError` from the cast callable (e.g. `int("test")`) is converted
        // to `TypeError` (`edgelist.py:156-160`, `hypergraph_dict.py:114-120` — both catch
        // `ValueError` and re-raise `TypeError`).
        Some(f) => f.call1((core_id,)).map_err(|e| {
            if e.is_instance_of::<pyo3::exceptions::PyValueError>(py) {
                pyo3::exceptions::PyTypeError::new_err("Failed to convert id to the given type.")
            } else {
                e
            }
        }),
        None if default_int => match core_id.parse::<i64>() {
            Ok(n) => Ok(n.into_pyobject(py)?.into_any()),
            Err(_) => Ok(PyString::new(py, core_id).into_any()),
        },
        None => Ok(PyString::new(py, core_id).into_any()),
    }
}

/// Build a `PyHypergraph` from a freshly-parsed core `Hypergraph`, casting every node/edge id
/// via `nodetype`/`edgetype` (or the `default_int` rule — see [`cast_id`]), and seeding
/// `net_attr` from `hypergraph_data`.
fn wrap_core(
    py: Python<'_>,
    core: hypergraph_rs::Hypergraph,
    nodetype: Option<&Bound<'_, PyAny>>,
    edgetype: Option<&Bound<'_, PyAny>>,
    hypergraph_data: Option<&serde_json::Map<String, Value>>,
) -> PyResult<PyHypergraph> {
    wrap_core_ex(py, core, nodetype, edgetype, hypergraph_data, false)
}

fn wrap_core_ex(
    py: Python<'_>,
    core: hypergraph_rs::Hypergraph,
    nodetype: Option<&Bound<'_, PyAny>>,
    edgetype: Option<&Bound<'_, PyAny>>,
    hypergraph_data: Option<&serde_json::Map<String, Value>>,
    default_int: bool,
) -> PyResult<PyHypergraph> {
    // `bind`, not `intern`: the cast object must map onto THIS exact core id (the one the
    // freshly-parsed `core::Hypergraph` actually uses internally) — `intern` would instead
    // compute its own `canonical()` key for the object, which only matches by coincidence for
    // int/str casts of numeric strings (see `PyIdMap::bind`'s doc).
    let mut node_ids = PyIdMap::new();
    for nid in core.node_ids() {
        let obj = cast_id(py, &nid, nodetype, default_int)?;
        node_ids.bind(&obj, nid)?;
    }
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        let obj = cast_id(py, &eid, edgetype, default_int)?;
        edge_ids.bind(&obj, eid)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: hypergraph_data.cloned().unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------------------------
// HIF (`xgi.readwrite.hif`; `write_hif`/`read_hif` ARE part of `xgi.readwrite.__all__`).
// ---------------------------------------------------------------------------------------------

/// Recursively substitute the core `String` id at every HIF `"node"`/`"edge"` field with the
/// original Python id's native JSON representation (int stays int, str stays str) — XGI parity:
/// `to_hif_dict` writes the network's actual node/edge ids verbatim, never `str()`-cast
/// (`convert/hif_dict.py`, contrast with the always-`str` keys of XGI's removed JSON format).
fn nativize_hif_ids(data: &mut Value, node_ids: &PyIdMap, edge_ids: &PyIdMap, py: Python<'_>) {
    let native = |map: &PyIdMap, core: &str| -> Value {
        map.get_obj(py, core)
            .and_then(|o| crate::pyid::py_any_to_json(&o).ok())
            .unwrap_or_else(|| Value::String(core.to_string()))
    };
    for key in ["nodes", "edges", "incidences"] {
        if let Some(arr) = data.get_mut(key).and_then(|v| v.as_array_mut()) {
            for rec in arr {
                if let Some(Value::String(s)) = rec.get("node").cloned() {
                    rec["node"] = native(node_ids, &s);
                }
                if let Some(Value::String(s)) = rec.get("edge").cloned() {
                    rec["edge"] = native(edge_ids, &s);
                }
            }
        }
    }
}

/// gzip-compress `bytes` (flate2, default compression) — used when `path` ends in `.gz`, the
/// only compression format supported (XGI's `xopen(**kwargs)` also does `.xz`/`.bz2`/`.zst`, not
/// reproduced here — no system-linked compression crate was added for this pass).
fn maybe_gzip_write(path: &str, bytes: &[u8]) -> PyResult<()> {
    if path.ends_with(".gz") {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let file = std::fs::File::create(path).map_err(|e| xgi_error(e.to_string()))?;
        let mut enc = GzEncoder::new(file, Compression::default());
        enc.write_all(bytes).map_err(|e| xgi_error(e.to_string()))?;
        enc.finish().map_err(|e| xgi_error(e.to_string()))?;
        Ok(())
    } else {
        std::fs::write(path, bytes).map_err(|e| xgi_error(e.to_string()))
    }
}

/// Read `path`'s bytes, gzip-decompressing first if it ends in `.gz` (mirrors [`maybe_gzip_write`]).
fn maybe_gzip_read(path: &str) -> PyResult<String> {
    if path.ends_with(".gz") {
        use flate2::read::GzDecoder;
        use std::io::Read;
        let file = std::fs::File::open(path).map_err(|e| xgi_error(e.to_string()))?;
        let mut dec = GzDecoder::new(file);
        let mut s = String::new();
        dec.read_to_string(&mut s)
            .map_err(|e| xgi_error(e.to_string()))?;
        Ok(s)
    } else {
        std::fs::read_to_string(path).map_err(|e| xgi_error(e.to_string()))
    }
}

/// `xgi.write_hif(H, path, **kwargs)`. XGI parity: `xgi.write_hif` (`readwrite/hif.py`, itself
/// `json.dumps(to_hif_dict(H))`). `**kwargs` (`compresslevel`, `format`, …) are XGI's `xopen`
/// compression options; only gzip-by-extension (`.gz`) is reproduced (see [`maybe_gzip_write`]).
#[pyfunction]
#[pyo3(signature = (h, path, **_kwargs))]
fn write_hif(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    path: &str,
    _kwargs: Option<Bound<'_, PyDict>>,
) -> PyResult<()> {
    let mut data = rw::to_hif_dict(&h.inner);
    data["metadata"] = Value::Object(h.net_attr.clone());
    nativize_hif_ids(&mut data, &h.node_ids, &h.edge_ids, py);
    let s = serde_json::to_string_pretty(&data).map_err(|e| xgi_error(e.to_string()))?;
    maybe_gzip_write(path, s.as_bytes())
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
    let s = maybe_gzip_read(path)?;
    let data: Value = serde_json::from_str(&s).map_err(|e| xgi_error(e.to_string()))?;
    let core = rw::from_hif_dict(&data).map_err(|e| xgi_error(e.to_string()))?;
    let metadata = data.get("metadata").and_then(|v| v.as_object());
    wrap_core_ex(
        py,
        core,
        nodetype.as_ref(),
        edgetype.as_ref(),
        metadata,
        true,
    )
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
    // XGI parity: `parse_edgelist` has NO `edgetype` parameter at all — edges get the
    // hypergraph's normal auto-assigned `int` ids (`edgelist.py:120-162`), unaffected by
    // `nodetype`. `default_int=true` gives that (edges have no cast, so they fall to the
    // int-if-numeric default; nodes still honor `nodetype` when given).
    wrap_core_ex(py, core, nodetype.as_ref(), None, None, true)
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
    // Same edge-id parity note as `parse_edgelist` above.
    wrap_core_ex(py, core, nodetype.as_ref(), None, None, true)
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
    // XGI parity: `read_incidence_matrix` strips lines starting with `comments` before parsing
    // rows. The core reader has no comment-stripping (dense-matrix format only), so it's applied
    // binding-side: filter to a temp file, hand that to the core reader, then clean up.
    let raw = std::fs::read_to_string(path).map_err(|e| xgi_error(e.to_string()))?;
    let filtered: String = raw
        .lines()
        .filter(|line| !comments.is_empty() && !line.trim_start().starts_with(comments))
        .collect::<Vec<_>>()
        .join("\n");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!(
        "hypergraph_rs_read_incidence_{}_{nanos}.tmp",
        std::process::id()
    ));
    std::fs::write(&tmp, filtered).map_err(|e| xgi_error(e.to_string()))?;
    let result = rw::read_incidence_matrix(&tmp, first_char(delimiter));
    let _ = std::fs::remove_file(&tmp);
    let core = result.map_err(|e| xgi_error(e.to_string()))?;
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
    Ok(())
}
