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

/// Raise `XGIError` if casting the original Python ids (via `map`) to `str` would collapse two
/// distinct ids onto the same string — XGI parity: `to_hypergraph_dict`
/// (`convert/hypergraph_dict.py:42-64`).
pub(crate) fn check_no_str_collision(
    py: Python<'_>,
    map: &PyIdMap,
    core_ids: &[String],
    kind: &str,
) -> PyResult<()> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, Vec<String>> = HashMap::new();
    for cid in core_ids {
        if let Some(obj) = map.get_obj(py, cid) {
            let s = obj.str()?.to_string();
            seen.entry(s).or_default().push(obj.repr()?.to_string());
        }
    }
    let dups: Vec<String> = seen
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| k)
        .collect();
    if !dups.is_empty() {
        return Err(xgi_error(format!(
            "When casting {kind} IDs to strings, ID(s) {} have conflicting IDs!",
            dups.join(", ")
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// JSON (deep-import only: `hypergraph_rs.readwrite.json`, not part of `xgi.readwrite.__all__`).
// ---------------------------------------------------------------------------------------------

/// Write a single (non-collection) hypergraph to `path` as the standard JSON dict.
fn write_json_single(py: Python<'_>, h: &PyHypergraph, path: &str) -> PyResult<()> {
    // XGI parity: `to_hypergraph_dict` raises `XGIError` when casting the ORIGINAL Python
    // node/edge ids to `str` would collapse two distinct ids onto the same string (e.g. the
    // int `2` and the string `"2"`) — `convert/hypergraph_dict.py:42-64`. The core's own ids
    // are already collision-free `String`s (via `PyIdMap`'s bump-suffix), so this check must
    // run binding-side against the original Python objects.
    check_no_str_collision(py, &h.node_ids, &h.inner.node_ids(), "node")?;
    check_no_str_collision(py, &h.edge_ids, &h.inner.edge_ids(), "edge")?;
    let mut data = rw::to_hypergraph_dict(&h.inner);
    data["hypergraph-data"] = Value::Object(h.net_attr.clone());
    let s = serde_json::to_string_pretty(&data).map_err(|e| xgi_error(e.to_string()))?;
    std::fs::write(path, s).map_err(|e| xgi_error(e.to_string()))
}

/// `hypergraph_rs.readwrite.json.write_json(H, path, collection_name="")`. XGI parity:
/// `xgi.write_json` (`readwrite/json.py:14-70`) — `H` a single hypergraph writes one JSON file
/// to `path`; a `list`/`dict` of hypergraphs writes `path` as a directory of per-dataset JSON
/// files plus a `<collection_name>collection_information.json` index.
#[pyfunction]
#[pyo3(signature = (h, path, collection_name=""))]
fn write_json(
    py: Python<'_>,
    h: &Bound<'_, PyAny>,
    path: &str,
    collection_name: &str,
) -> PyResult<()> {
    let prefix = if collection_name.is_empty() {
        String::new()
    } else {
        format!("{collection_name}_")
    };
    if let Ok(list) = h.cast::<pyo3::types::PyList>() {
        let datasets = PyDict::new(py);
        for (i, item) in list.iter().enumerate() {
            let fname = format!("{path}/{prefix}{i}.json");
            let hg: PyRef<PyHypergraph> = item.extract()?;
            write_json_single(py, &hg, &fname)?;
            let entry = PyDict::new(py);
            entry.set_item("relative-path", format!("{prefix}{i}.json"))?;
            datasets.set_item(i, entry)?;
        }
        let collection_data = PyDict::new(py);
        collection_data.set_item("datasets", datasets)?;
        collection_data.set_item("type", "collection")?;
        let json_mod = py.import("json")?;
        let s: String = json_mod
            .call_method1("dumps", (collection_data,))?
            .extract()?;
        std::fs::write(format!("{path}/{prefix}collection_information.json"), s)
            .map_err(|e| xgi_error(e.to_string()))?;
        return Ok(());
    }
    if let Ok(dict) = h.cast::<PyDict>() {
        let datasets = PyDict::new(py);
        for (name, item) in dict.iter() {
            let name_str = name.str()?.to_string();
            let fname = format!("{path}/{prefix}{name_str}.json");
            let hg: PyRef<PyHypergraph> = item.extract()?;
            write_json_single(py, &hg, &fname)?;
            let entry = PyDict::new(py);
            entry.set_item("relative-path", format!("{prefix}{name_str}.json"))?;
            datasets.set_item(&name, entry)?;
        }
        let collection_data = PyDict::new(py);
        collection_data.set_item("datasets", datasets)?;
        collection_data.set_item("type", "collection")?;
        let json_mod = py.import("json")?;
        let s: String = json_mod
            .call_method1("dumps", (collection_data,))?
            .extract()?;
        std::fs::write(format!("{path}/{prefix}collection_information.json"), s)
            .map_err(|e| xgi_error(e.to_string()))?;
        return Ok(());
    }
    let hg: PyRef<PyHypergraph> = h.extract()?;
    write_json_single(py, &hg, path)
}

fn read_json_single(
    py: Python<'_>,
    data: &Value,
    nodetype: Option<&Bound<'_, PyAny>>,
    edgetype: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    // XGI parity: `from_hypergraph_dict` converts node-data keys via `nodetype` FIRST — before
    // even looking at "edge-dict" — raising `TypeError` on a failed cast
    // (`convert/hypergraph_dict.py:112-120`). The core validates "edge-dict" presence while
    // parsing (a binding-agnostic structural check), which would otherwise fire first when
    // "edge-dict" happens to be absent too; pre-check the node-type cast here to match XGI's
    // ordering without needing "edge-dict" to exist yet.
    if let Some(cast) = nodetype {
        if let Some(node_data) = data.get("node-data").and_then(|v| v.as_object()) {
            for k in node_data.keys() {
                cast_id(py, k, Some(cast), false)?;
            }
        }
    }
    let core = rw::from_hypergraph_dict(data).map_err(|e| xgi_error(e.to_string()))?;
    let hgdata = data.get("hypergraph-data").and_then(|v| v.as_object());
    wrap_core(py, core, nodetype, edgetype, hgdata)
}

/// `hypergraph_rs.readwrite.json.read_json(path, nodetype=None, edgetype=None)`. Returns a
/// single `Hypergraph`, or (XGI parity, `readwrite/json.py:106-118`) a `dict` of them when the
/// file has `"type": "collection"`.
#[pyfunction]
#[pyo3(signature = (path, nodetype=None, edgetype=None))]
fn read_json(
    py: Python<'_>,
    path: &str,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let s = std::fs::read_to_string(path).map_err(|e| xgi_error(e.to_string()))?;
    let data: Value = serde_json::from_str(&s).map_err(|e| xgi_error(e.to_string()))?;
    if data.get("type").and_then(|v| v.as_str()) == Some("collection") {
        let os_path = py.import("os.path")?;
        let dirname: String = os_path.call_method1("dirname", (path,))?.extract()?;
        let datasets = data
            .get("datasets")
            .and_then(|v| v.as_object())
            .ok_or_else(|| xgi_error("missing datasets"))?;
        let out = PyDict::new(py);
        for (name, entry) in datasets {
            let relpath = entry
                .get("relative-path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| xgi_error("missing relative-path"))?;
            let joined: String = os_path
                .call_method1("join", (&dirname, relpath))?
                .extract()?;
            let sub_s = std::fs::read_to_string(&joined).map_err(|e| xgi_error(e.to_string()))?;
            let sub_data: Value =
                serde_json::from_str(&sub_s).map_err(|e| xgi_error(e.to_string()))?;
            let hg = read_json_single(py, &sub_data, nodetype.as_ref(), edgetype.as_ref())?;
            out.set_item(name, Py::new(py, hg)?)?;
        }
        return Ok(out.into_any().unbind());
    }
    let hg = read_json_single(py, &data, nodetype.as_ref(), edgetype.as_ref())?;
    Ok(Py::new(py, hg)?.into_any())
}

// ---------------------------------------------------------------------------------------------
// HIF (`xgi.readwrite.hif`; `write_hif`/`read_hif` ARE part of `xgi.readwrite.__all__`).
// ---------------------------------------------------------------------------------------------

/// Recursively substitute the core `String` id at every HIF `"node"`/`"edge"` field with the
/// original Python id's native JSON representation (int stays int, str stays str) — XGI parity:
/// `to_hif_dict` writes the network's actual node/edge ids verbatim, never `str()`-cast
/// (`convert/hif_dict.py`, contrast with `to_hypergraph_dict`'s always-`str` dict keys).
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
