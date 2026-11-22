//! `convert` bindings (plan Task 12b) — over Phase 6's `convert/` core module.
//!
//! Bound (matches `python/hypergraph_rs/convert/__init__.py`'s `_BOUND` list): the hyperedge
//! list/dict pair, the bipartite-edgelist pair, `to_bipartite_graph`/`from_bipartite_graph`
//! (**networkx** on the Python side — spec §6.3/D-style optional import, matching the plan's
//! Task 12b representative snippet), `to_graph` (1-skeleton → `nx.Graph`), and `cut_to_order`. `to_hif_dict`/`from_hif_dict`
//! stay `make_deferred` here (§A: the "convert (~14)" deferred list) even though the core `hif`
//! module exists — Task 12a binds it under `readwrite` instead (`write_hif`/`read_hif`), which is
//! where `xgi.readwrite.hif` actually re-exports it; the plan's convert-namespace deferral is
//! about the *symbol location*, not the underlying capability.
//!
//! **Id typing:** `to_hyperedge_list`/`to_hyperedge_dict`/`to_bipartite_edgelist` preserve the
//! *exact* original Python ids (via `PyIdMap`, not the `int`-if-numeric heuristic) — these
//! round-trip through a hypergraph that was itself built with real ids. XGI 1.0 dropped
//! `to_hypergraph_dict`/`from_hypergraph_dict`; so did this library.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyString, PyTuple};
use rustworkx_core::petgraph::stable_graph::StableDiGraph;
use rustworkx_core::petgraph::visit::EdgeRef;

use hypergraph_rs::convert as conv;
use hypergraph_rs::convert::BipartiteNode;

use crate::errors::xgi_error;
use crate::hypergraph::{int_or_str_pyobj, PyHypergraph};
use crate::pyid::PyIdMap;

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

/// `xgi.from_bipartite_edgelist(edges)` — an iterable of `(node, edge)` pairs (undirected) or
/// `(node, edge, "in"|"out")` triples (directed) — dispatched on the FIRST element's length,
/// XGI parity (`convert/bipartite_edges.py:9-42`).
#[pyfunction]
fn from_bipartite_edgelist(py: Python<'_>, edges: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut rows: Vec<Bound<'_, PyAny>> = Vec::new();
    for row in edges.try_iter()? {
        rows.push(row?);
    }
    let width = match rows.first() {
        Some(r) => r.len()?,
        None => {
            return Err(xgi_error(
                "Each list element must have two entries for undirected and three entries for directed.",
            ))
        }
    };
    if width == 2 {
        let mut node_ids = PyIdMap::new();
        let mut edge_ids = PyIdMap::new();
        let mut core_pairs: Vec<(String, String)> = Vec::new();
        for pair in &rows {
            let node = pair.get_item(0)?;
            let edge = pair.get_item(1)?;
            let nk = node_ids.intern(&node)?;
            let ek = edge_ids.intern(&edge)?;
            core_pairs.push((nk, ek));
        }
        let core = conv::from_bipartite_edgelist(core_pairs);
        return Ok(Py::new(
            py,
            PyHypergraph {
                inner: core,
                node_ids,
                edge_ids,
                net_attr: serde_json::Map::new(),
            },
        )?
        .into_any());
    }
    if width == 3 {
        let mut dh = crate::dihypergraph::PyDiHypergraph::new(py, None, None)?;
        for row in &rows {
            let node = row.get_item(0)?;
            let edge = row.get_item(1)?;
            let dir: String = row.get_item(2)?.extract()?;
            dh.add_node_to_edge(&edge, &node, &dir)?;
        }
        return Ok(Py::new(py, dh)?.into_any());
    }
    Err(xgi_error(
        "Each list element must have two entries for undirected and three entries for directed.",
    ))
}

/// `xgi.to_bipartite_edgelist(H)` — a `list` of `(node, edge)` pairs (undirected `Hypergraph`)
/// or `(node, edge, "in"|"out")` triples (`DiHypergraph`) — XGI parity
/// (`convert/bipartite_edges.py:45-71`).
#[pyfunction]
fn to_bipartite_edgelist(py: Python<'_>, h: &Bound<'_, PyAny>) -> PyResult<Py<PyList>> {
    if let Ok(dh) = h.extract::<PyRef<crate::dihypergraph::PyDiHypergraph>>() {
        let out = PyList::empty(py);
        for eid in dh.inner.edge_ids() {
            let Some(eobj) = dh.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            let (tail, head) = dh.inner.dimembers(&eid).unwrap_or_default();
            for n in tail {
                if let Some(nobj) = dh.node_ids.get_obj(py, &n) {
                    out.append(PyTuple::new(
                        py,
                        [nobj, eobj.clone(), PyString::new(py, "in").into_any()],
                    )?)?;
                }
            }
            let Some(eobj) = dh.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            for n in head {
                if let Some(nobj) = dh.node_ids.get_obj(py, &n) {
                    out.append(PyTuple::new(
                        py,
                        [nobj, eobj.clone(), PyString::new(py, "out").into_any()],
                    )?)?;
                }
            }
        }
        return Ok(out.unbind());
    }
    let h: PyRef<'_, PyHypergraph> = h.extract()?;
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

/// `xgi.to_bipartite_graph(H, index=False)` -> `nx.Graph`/`nx.DiGraph` with a `bipartite` node
/// attribute (0 = node, 1 = hyperedge). XGI parity (`convert/bipartite_graph.py:110-185`): the
/// graph's own node labels are POSITIONAL INTEGERS (`0..n` for hypergraph nodes, `n..n+m` for
/// hyperedges) — NOT the original ids; `index=True` additionally returns the
/// `{int_label: original_id}` maps. `H` may be a `Hypergraph` (undirected `nx.Graph`) or a
/// `DiHypergraph` (`nx.DiGraph`, edges `tail -> edge_label -> head`).
#[pyfunction]
#[pyo3(signature = (h, index=false))]
fn to_bipartite_graph(py: Python<'_>, h: &Bound<'_, PyAny>, index: bool) -> PyResult<Py<PyAny>> {
    let nx = py.import("networkx").map_err(|_| {
        xgi_error("to_bipartite_graph requires networkx to be installed (soft dependency)")
    })?;

    let build = |g: &Bound<'_, PyAny>,
                 node_objs: Vec<Bound<'_, PyAny>>,
                 edge_objs: Vec<Bound<'_, PyAny>>,
                 edges: Vec<(usize, usize)>|
     -> PyResult<()> {
        let n = node_objs.len();
        for i in 0..n {
            let kwargs = PyDict::new(py);
            kwargs.set_item("bipartite", 0)?;
            g.call_method("add_node", (i,), Some(&kwargs))?;
        }
        for (j, _) in edge_objs.iter().enumerate() {
            let kwargs = PyDict::new(py);
            kwargs.set_item("bipartite", 1)?;
            g.call_method("add_node", (n + j,), Some(&kwargs))?;
        }
        for (a, b) in edges {
            g.call_method1("add_edge", (a, b))?;
        }
        Ok(())
    };

    if let Ok(dh) = h.extract::<PyRef<crate::dihypergraph::PyDiHypergraph>>() {
        let g = nx.getattr("DiGraph")?.call0()?;
        let node_ids = dh.inner.node_ids();
        let edge_ids = dh.inner.edge_ids();
        let n = node_ids.len();
        let node_objs: Vec<Bound<'_, PyAny>> = node_ids
            .iter()
            .filter_map(|nid| dh.node_ids.get_obj(py, nid))
            .collect();
        let edge_objs: Vec<Bound<'_, PyAny>> = edge_ids
            .iter()
            .filter_map(|eid| dh.edge_ids.get_obj(py, eid))
            .collect();
        let node_pos: std::collections::HashMap<&String, usize> =
            node_ids.iter().enumerate().map(|(i, n)| (n, i)).collect();
        let mut edges = Vec::new();
        for (j, eid) in edge_ids.iter().enumerate() {
            for v in dh.inner.tail(eid).unwrap_or_default() {
                if let Some(&i) = node_pos.get(&v) {
                    edges.push((i, n + j));
                }
            }
            for v in dh.inner.head(eid).unwrap_or_default() {
                if let Some(&i) = node_pos.get(&v) {
                    edges.push((n + j, i));
                }
            }
        }
        build(&g, node_objs.clone(), edge_objs.clone(), edges)?;
        if !index {
            return Ok(g.unbind());
        }
        let node_dict = PyDict::new(py);
        for (i, obj) in node_objs.into_iter().enumerate() {
            node_dict.set_item(i, obj)?;
        }
        let edge_dict = PyDict::new(py);
        for (j, obj) in edge_objs.into_iter().enumerate() {
            edge_dict.set_item(n + j, obj)?;
        }
        return Ok(
            PyTuple::new(py, [g, node_dict.into_any(), edge_dict.into_any()])?
                .into_any()
                .unbind(),
        );
    }

    let h: PyRef<'_, PyHypergraph> = h.extract()?;
    let g = nx.getattr("Graph")?.call0()?;
    let node_ids = h.inner.node_ids();
    let edge_ids = h.inner.edge_ids();
    let n = node_ids.len();
    let node_objs: Vec<Bound<'_, PyAny>> = node_ids
        .iter()
        .filter_map(|nid| h.node_ids.get_obj(py, nid))
        .collect();
    let edge_objs: Vec<Bound<'_, PyAny>> = edge_ids
        .iter()
        .filter_map(|eid| h.edge_ids.get_obj(py, eid))
        .collect();
    let node_pos: std::collections::HashMap<&String, usize> =
        node_ids.iter().enumerate().map(|(i, n)| (n, i)).collect();
    let mut edges = Vec::new();
    for (j, eid) in edge_ids.iter().enumerate() {
        for v in h.inner.members(eid).unwrap_or_default() {
            if let Some(&i) = node_pos.get(&v) {
                edges.push((i, n + j));
            }
        }
    }
    build(&g, node_objs.clone(), edge_objs.clone(), edges)?;
    if !index {
        return Ok(g.unbind());
    }
    let node_dict = PyDict::new(py);
    for (i, obj) in node_objs.into_iter().enumerate() {
        node_dict.set_item(i, obj)?;
    }
    let edge_dict = PyDict::new(py);
    for (j, obj) in edge_objs.into_iter().enumerate() {
        edge_dict.set_item(n + j, obj)?;
    }
    Ok(
        PyTuple::new(py, [g, node_dict.into_any(), edge_dict.into_any()])?
            .into_any()
            .unbind(),
    )
}

/// `xgi.from_bipartite_graph(G, dual=False)`. `G` an `nx.Graph` -> `Hypergraph` (`dual=True`
/// swaps which layer is nodes vs. hyperedges, via `Hypergraph.dual()`); `G` an `nx.DiGraph` ->
/// `DiHypergraph` (XGI parity: `convert/bipartite_graph.py:57-95` — edges into a `bipartite=1`
/// node are "in" (tail), edges out of one are "out" (head); `dual` is untested/unsupported for
/// the directed case in the oracle corpus — `DiHypergraph` has no `.dual()` in XGI itself).
#[pyfunction]
#[pyo3(signature = (g, dual=false))]
fn from_bipartite_graph(py: Python<'_>, g: Bound<'_, PyAny>, dual: bool) -> PyResult<Py<PyAny>> {
    let nx = py.import("networkx").map_err(|_| {
        xgi_error("from_bipartite_graph requires networkx to be installed (soft dependency)")
    })?;
    if g.is_instance(&nx.getattr("DiGraph")?)? {
        let kwargs = PyDict::new(py);
        kwargs.set_item("data", true)?;
        let nodes_view = g.call_method("nodes", (), Some(&kwargs))?;
        let mut node_objs: Vec<Bound<'_, PyAny>> = Vec::new();
        let mut edge_objs: Vec<Bound<'_, PyAny>> = Vec::new();
        for item in nodes_view.try_iter()? {
            let item = item?;
            let node_obj = item.get_item(0)?;
            let data = item.get_item(1)?;
            let bipartite: i64 = data
                .get_item("bipartite")
                .map_err(|_| xgi_error("bipartite property not set"))?
                .extract()
                .map_err(|_| xgi_error("bipartite property not set"))?;
            match bipartite {
                0 => node_objs.push(node_obj),
                1 => edge_objs.push(node_obj),
                _ => return Err(xgi_error("Invalid type specifier")),
            }
        }
        let mut dh = crate::dihypergraph::PyDiHypergraph::new(py, None, None)?;
        for n in &node_objs {
            dh.add_node(n, None)?;
        }
        for item in g.call_method0("edges")?.try_iter()? {
            let item = item?;
            let u = item.get_item(0)?;
            let v = item.get_item(1)?;
            let v_is_edge = edge_objs.iter().any(|e| e.eq(&v).unwrap_or(false));
            if v_is_edge {
                dh.add_node_to_edge(&v, &u, "in")?;
            } else {
                dh.add_node_to_edge(&u, &v, "out")?;
            }
        }
        return Ok(Py::new(py, dh)?.into_any());
    }
    Ok(Py::new(py, from_bipartite_graph_undirected(py, g, dual)?)?.into_any())
}

fn from_bipartite_graph_undirected(
    py: Python<'_>,
    g: Bound<'_, PyAny>,
    dual: bool,
) -> PyResult<PyHypergraph> {
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
        let raw_bipartite: i64 = data
            .get_item("bipartite")
            .map_err(|_| {
                xgi_error(format!(
                    "node {} is missing the 'bipartite' attribute",
                    node_obj.repr().map(|r| r.to_string()).unwrap_or_default()
                ))
            })?
            .extract()
            .map_err(|_| {
                xgi_error(format!(
                    "node {} has a non-integer 'bipartite' attribute",
                    node_obj.repr().map(|r| r.to_string()).unwrap_or_default()
                ))
            })?;
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
    m.add_function(pyo3::wrap_pyfunction!(cut_to_order, m)?)?;
    Ok(())
}
