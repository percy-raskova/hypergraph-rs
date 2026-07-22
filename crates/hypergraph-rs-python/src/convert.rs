//! `convert` bindings (plan Task 12b) — over Phase 6's `convert/` core module.
//!
//! Bound (matches `python/hypergraph_rs/convert/__init__.py`'s `_BOUND` list): the hyperedge
//! list/dict pair, the bipartite-edgelist pair, `to_bipartite_graph`/`from_bipartite_graph`
//! (**networkx** on the Python side — spec §6.3/D-style optional import, matching the plan's
//! Task 12b representative snippet), `to_graph` (1-skeleton → `nx.Graph`), the standardized
//! `to_hypergraph_dict`/`from_hypergraph_dict`, and `cut_to_order`. `to_hif_dict`/`from_hif_dict`
//! stay `make_deferred` here (§A: the "convert (~14)" deferred list) even though the core `hif`
//! module exists — Task 12a binds it under `readwrite` instead (`write_hif`/`read_hif`), which is
//! where `xgi.readwrite.hif` actually re-exports it; the plan's convert-namespace deferral is
//! about the *symbol location*, not the underlying capability.
//!
//! **Id typing:** `to_hyperedge_list`/`to_hyperedge_dict`/`to_bipartite_edgelist` preserve the
//! *exact* original Python ids (via `PyIdMap`, not the `int`-if-numeric heuristic) — these
//! round-trip through a hypergraph that was itself built with real ids. `to_hypergraph_dict`/
//! `from_hypergraph_dict` follow XGI's own contract instead: ids serialize as `str(idx)`
//! (`xgi/convert/hypergraph_dict.py:41-43`) and deserialize back to plain `str` unless
//! `nodetype`/`edgetype` is given (`hypergraph_dict.py:112-121`) — verified against the
//! installed package.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyString, PyTuple};
use rustworkx_core::petgraph::stable_graph::StableDiGraph;
use rustworkx_core::petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde_json::Value;

use hypergraph_rs::convert as conv;
use hypergraph_rs::convert::BipartiteNode;

use crate::errors::xgi_error;
use crate::hypergraph::{int_or_str_pyobj, PyHypergraph};
use crate::pyid::{json_to_py, py_any_to_json, PyIdMap};

// ---------------------------------------------------------------------------------------------
// Hyperedge list / dict.
// ---------------------------------------------------------------------------------------------

/// `xgi.from_hyperedge_list(d)`. Auto-assigned int edge ids (XGI parity); node ids are
/// interned exactly as given (arbitrary hashables).
#[pyfunction]
fn from_hyperedge_list(py: Python<'_>, d: Bound<'_, PyAny>) -> PyResult<PyHypergraph> {
    let mut node_ids = PyIdMap::new();
    let mut edges: Vec<Vec<String>> = Vec::new();
    for edge in d.try_iter()? {
        let edge = edge?;
        let mut members = Vec::new();
        for m in edge.try_iter()? {
            members.push(node_ids.intern(&m?)?);
        }
        edges.push(members);
    }
    let core = conv::from_hyperedge_list(edges);
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.to_hyperedge_list(H)` — a `list` of `set`s (XGI parity: `H.edges.members()`).
#[pyfunction]
fn to_hyperedge_list(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyList>> {
    let out = PyList::empty(py);
    for members in conv::to_hyperedge_list(&h.inner) {
        let set = PySet::empty(py)?;
        for m in &members {
            if let Some(obj) = h.node_ids.get_obj(py, m) {
                set.add(obj)?;
            }
        }
        out.append(set)?;
    }
    Ok(out.unbind())
}

/// `xgi.from_hyperedge_dict(d)`. Edge ids taken from the dict keys (arbitrary hashables).
#[pyfunction]
fn from_hyperedge_dict(d: Bound<'_, PyDict>) -> PyResult<PyHypergraph> {
    let mut node_ids = PyIdMap::new();
    let mut edge_ids = PyIdMap::new();
    let mut pairs: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in d.iter() {
        let ek = edge_ids.intern(&k)?;
        let mut members = Vec::new();
        for m in v.try_iter()? {
            members.push(node_ids.intern(&m?)?);
        }
        pairs.push((ek, members));
    }
    let core = conv::from_hyperedge_dict(pairs);
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.to_hyperedge_dict(H)` — a `dict` of edge id -> `set` of members.
#[pyfunction]
fn to_hyperedge_dict(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let out = PyDict::new(py);
    for (eid, members) in conv::to_hyperedge_dict(&h.inner) {
        let Some(eobj) = h.edge_ids.get_obj(py, &eid) else {
            continue;
        };
        let set = PySet::empty(py)?;
        for m in &members {
            if let Some(obj) = h.node_ids.get_obj(py, m) {
                set.add(obj)?;
            }
        }
        out.set_item(eobj, set)?;
    }
    Ok(out.unbind())
}

// ---------------------------------------------------------------------------------------------
// Bipartite edgelist.
// ---------------------------------------------------------------------------------------------

/// `xgi.from_bipartite_edgelist(edges)` — an iterable of `(node, edge)` pairs.
#[pyfunction]
fn from_bipartite_edgelist(edges: Bound<'_, PyAny>) -> PyResult<PyHypergraph> {
    let mut node_ids = PyIdMap::new();
    let mut edge_ids = PyIdMap::new();
    let mut core_pairs: Vec<(String, String)> = Vec::new();
    for pair in edges.try_iter()? {
        let pair = pair?;
        let node = pair.get_item(0)?;
        let edge = pair.get_item(1)?;
        let nk = node_ids.intern(&node)?;
        let ek = edge_ids.intern(&edge)?;
        core_pairs.push((nk, ek));
    }
    let core = conv::from_bipartite_edgelist(core_pairs);
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.to_bipartite_edgelist(H)` — a `list` of `(node, edge)` pairs.
#[pyfunction]
fn to_bipartite_edgelist(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyList>> {
    let out = PyList::empty(py);
    for (n, e) in conv::to_bipartite_edgelist(&h.inner) {
        let (Some(nobj), Some(eobj)) = (h.node_ids.get_obj(py, &n), h.edge_ids.get_obj(py, &e))
        else {
            continue;
        };
        out.append(PyTuple::new(py, [nobj, eobj])?)?;
    }
    Ok(out.unbind())
}

// ---------------------------------------------------------------------------------------------
// Bipartite graph (networkx, optional runtime import — spec §6.3 pattern).
// ---------------------------------------------------------------------------------------------

/// `xgi.to_bipartite_graph(H)` -> `nx.Graph` with a `bipartite` node attribute (0 = node,
/// 1 = hyperedge), matching `xgi/convert/bipartite_graph.py:110-185`.
#[pyfunction]
fn to_bipartite_graph(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyAny>> {
    let nx = py.import("networkx").map_err(|_| {
        xgi_error("to_bipartite_graph requires networkx to be installed (soft dependency)")
    })?;
    let g = nx.getattr("Graph")?.call0()?;
    let core_g = conv::to_bipartite_graph(&h.inner);
    for w in core_g.node_weights() {
        let ids = if w.bipartite == 0 {
            &h.node_ids
        } else {
            &h.edge_ids
        };
        if let Some(obj) = ids.get_obj(py, &w.id) {
            let kwargs = PyDict::new(py);
            kwargs.set_item("bipartite", w.bipartite)?;
            g.call_method("add_node", (obj,), Some(&kwargs))?;
        }
    }
    for edge in core_g.edge_references() {
        let src = &core_g[edge.source()];
        let dst = &core_g[edge.target()];
        let src_map = if src.bipartite == 0 {
            &h.node_ids
        } else {
            &h.edge_ids
        };
        let dst_map = if dst.bipartite == 0 {
            &h.node_ids
        } else {
            &h.edge_ids
        };
        if let (Some(a), Some(b)) = (src_map.get_obj(py, &src.id), dst_map.get_obj(py, &dst.id)) {
            g.call_method1("add_edge", (a, b))?;
        }
    }
    Ok(g.unbind())
}

/// `xgi.from_bipartite_graph(G, dual=False)`. `G` is an `nx.Graph` whose nodes carry a
/// `bipartite` attribute (0/1); `dual=True` swaps which layer is nodes vs. hyperedges.
#[pyfunction]
#[pyo3(signature = (g, dual=false))]
fn from_bipartite_graph(py: Python<'_>, g: Bound<'_, PyAny>, dual: bool) -> PyResult<PyHypergraph> {
    let mut sg: StableDiGraph<BipartiteNode, ()> = StableDiGraph::new();
    let mut ids = PyIdMap::new();
    let mut idx_of: std::collections::HashMap<String, _> = std::collections::HashMap::new();

    let kwargs = PyDict::new(py);
    kwargs.set_item("data", true)?;
    let nodes_view = g.call_method("nodes", (), Some(&kwargs))?;
    for item in nodes_view.try_iter()? {
        let item = item?;
        let node_obj = item.get_item(0)?;
        let data = item.get_item(1)?;
        let raw_bipartite: i64 = data.get_item("bipartite")?.extract()?;
        let bipartite = if dual {
            1 - raw_bipartite
        } else {
            raw_bipartite
        };
        if !(0..=1).contains(&bipartite) {
            return Err(xgi_error(format!(
                "node {} has an invalid bipartite marker (expected 0 or 1)",
                node_obj.repr()?
            )));
        }
        let key = ids.intern(&node_obj)?;
        let idx = sg.add_node(BipartiteNode {
            id: key.clone(),
            bipartite: bipartite as u8,
        });
        idx_of.insert(key, idx);
    }
    for item in g.call_method0("edges")?.try_iter()? {
        let item = item?;
        let a = item.get_item(0)?;
        let b = item.get_item(1)?;
        let ak = ids
            .get_key(&a)?
            .ok_or_else(|| xgi_error("edge references an unknown node"))?;
        let bk = ids
            .get_key(&b)?
            .ok_or_else(|| xgi_error("edge references an unknown node"))?;
        sg.add_edge(idx_of[&ak], idx_of[&bk], ());
    }

    let core = conv::from_bipartite_graph(&sg).map_err(|e| xgi_error(e.to_string()))?;
    let mut node_ids = PyIdMap::new();
    for nid in core.node_ids() {
        if let Some(obj) = ids.get_obj(py, &nid) {
            node_ids.intern(&obj)?;
        }
    }
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        match ids.get_obj(py, &eid) {
            Some(obj) => {
                edge_ids.intern(&obj)?;
            }
            None => {
                edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
            }
        }
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

// ---------------------------------------------------------------------------------------------
// 1-skeleton graph (networkx).
// ---------------------------------------------------------------------------------------------

/// `xgi.to_graph(H)` -> the unweighted 1-skeleton as an `nx.Graph`.
#[pyfunction]
fn to_graph(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyAny>> {
    let nx = py
        .import("networkx")
        .map_err(|_| xgi_error("to_graph requires networkx to be installed (soft dependency)"))?;
    let g = nx.getattr("Graph")?.call0()?;
    let core_g = conv::to_graph(&h.inner);
    for idx in core_g.node_indices() {
        let core_id = &core_g[idx];
        if let Some(obj) = h.node_ids.get_obj(py, core_id) {
            g.call_method1("add_node", (obj,))?;
        }
    }
    for edge in core_g.edge_references() {
        let a = &core_g[edge.source()];
        let b = &core_g[edge.target()];
        if let (Some(oa), Some(ob)) = (h.node_ids.get_obj(py, a), h.node_ids.get_obj(py, b)) {
            g.call_method1("add_edge", (oa, ob))?;
        }
    }
    Ok(g.unbind())
}

// ---------------------------------------------------------------------------------------------
// Standardized hypergraph dict (ids serialize as `str(idx)` — XGI parity, see module doc).
// ---------------------------------------------------------------------------------------------

/// `xgi.to_hypergraph_dict(H)`.
#[pyfunction]
fn to_hypergraph_dict(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let mut data = conv::to_hypergraph_dict(&h.inner);
    data["hypergraph-data"] = Value::Object(h.net_attr.clone());
    let obj = json_to_py(py, &data)?;
    obj.cast_into::<PyDict>()
        .map(Bound::unbind)
        .map_err(|e| xgi_error(e.to_string()))
}

/// `xgi.from_hypergraph_dict(data, nodetype=None, edgetype=None, max_order=None)`.
#[pyfunction]
#[pyo3(signature = (data, nodetype=None, edgetype=None, max_order=None))]
fn from_hypergraph_dict(
    py: Python<'_>,
    data: Bound<'_, PyDict>,
    nodetype: Option<Bound<'_, PyAny>>,
    edgetype: Option<Bound<'_, PyAny>>,
    max_order: Option<i64>,
) -> PyResult<PyHypergraph> {
    let mut value = py_any_to_json(data.as_any())?;
    if let Some(max_order) = max_order {
        if let Some(edge_dict) = value.get_mut("edge-dict").and_then(|v| v.as_object_mut()) {
            edge_dict.retain(|_, members| {
                members
                    .as_array()
                    .map(|a| a.len() as i64 <= max_order + 1)
                    .unwrap_or(true)
            });
        }
    }
    let core = conv::from_hypergraph_dict(&value).map_err(|e| xgi_error(e.to_string()))?;
    let hgdata = value
        .get("hypergraph-data")
        .and_then(|v| v.as_object())
        .cloned();

    let mut node_ids = PyIdMap::new();
    for nid in core.node_ids() {
        let obj = match &nodetype {
            Some(f) => f.call1((nid.as_str(),))?,
            None => PyString::new(py, &nid).into_any(),
        };
        node_ids.intern(&obj)?;
    }
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        let obj = match &edgetype {
            Some(f) => f.call1((eid.as_str(),))?,
            None => PyString::new(py, &eid).into_any(),
        };
        edge_ids.intern(&obj)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: hgdata.unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------------------------
// cut_to_order (existing node/edge id maps are preserved — only edges are filtered).
// ---------------------------------------------------------------------------------------------

/// `xgi.cut_to_order(H, order)`.
#[pyfunction]
fn cut_to_order(py: Python<'_>, h: PyRef<'_, PyHypergraph>, order: i64) -> PyResult<PyHypergraph> {
    let core = conv::cut_to_order(&h.inner, order).map_err(|e| xgi_error(e.to_string()))?;
    Ok(PyHypergraph {
        inner: core,
        node_ids: h.node_ids.clone_via_py(py),
        edge_ids: h.edge_ids.clone_via_py(py),
        net_attr: h.net_attr.clone(),
    })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(from_hyperedge_list, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_hyperedge_list, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(from_hyperedge_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_hyperedge_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(from_bipartite_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_bipartite_edgelist, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_bipartite_graph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(from_bipartite_graph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_graph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(to_hypergraph_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(from_hypergraph_dict, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(cut_to_order, m)?)?;
    Ok(())
}
