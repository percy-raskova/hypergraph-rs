//! PyO3 binding for the core `DiHypergraph` (Task 6). Mirrors Task 4's `PyHypergraph` CRUD +
//! Task 5's `PyNodeView`/`PyEdgeView` proxy pattern, adapted for directed `(tail, head)` edges.
//! XGI parity: `xgi.DiHypergraph` (dihypergraph.py). Directed `add_edge` takes a
//! `(tail_set, head_set)` pair (`conftest.py:diedgelist1` = `({1,2,3},{4})`).

use hypergraph_rs::core::error::EdgeError;
use hypergraph_rs::DiHypergraph as CoreDiHg;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyTuple};
use serde_json::Value;

use crate::errors::{id_not_found, xgi_error};
use crate::pyid::{json_to_py, py_any_to_json, PyIdMap};

#[pyclass(name = "DiHypergraph", module = "hypergraph_rs")]
pub struct PyDiHypergraph {
    pub inner: CoreDiHg<Value, Value, Value>,
    pub node_ids: PyIdMap,
    pub edge_ids: PyIdMap,
    pub net_attr: serde_json::Map<String, Value>,
}

#[pymethods]
impl PyDiHypergraph {
    #[new]
    #[pyo3(signature = (incoming_data=None, **attr))]
    fn new(
        py: Python<'_>,
        incoming_data: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut dh = Self {
            inner: CoreDiHg::new(),
            node_ids: PyIdMap::new(),
            edge_ids: PyIdMap::new(),
            net_attr: serde_json::Map::new(),
        };
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                dh.net_attr
                    .insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        if let Some(data) = incoming_data {
            dh.ingest(py, &data)?;
        }
        Ok(dh)
    }

    /// `DH["name"]` — network-level attr; missing -> XGIError.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.net_attr.get(key) {
            Some(v) => Ok(json_to_py(py, v)?.unbind()),
            None => Err(xgi_error(format!("{key} is not a network attribute"))),
        }
    }

    #[getter]
    fn num_nodes(&self) -> usize {
        self.inner.num_nodes()
    }
    #[getter]
    fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    #[getter]
    fn _net_attr(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.net_attr {
            d.set_item(k, json_to_py(py, v)?)?;
        }
        Ok(d.unbind())
    }

    /// `DH.add_edge((tail, head), idx=None, **attr)`. Shims: duplicate idx -> warn+no-op
    /// (same deviation shim as `PyHypergraph::add_edge_impl`, Task 4).
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_edge(
        &mut self,
        py: Python<'_>,
        members: Bound<'_, PyAny>,
        idx: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        self.add_edge_impl(py, &members, idx.as_ref(), attr.as_ref())
    }

    fn __contains__(&self, n: &Bound<'_, PyAny>) -> bool {
        matches!(self.node_ids.get_key(n), Ok(Some(_)))
    }

    fn has_node(&self, n: &Bound<'_, PyAny>) -> bool {
        self.__contains__(n)
    }

    fn has_edge(&self, e: &Bound<'_, PyAny>) -> bool {
        matches!(self.edge_ids.get_key(e), Ok(Some(_)))
    }

    /// `DH.add_node(n, **attr)` — merges the incoming dict into any existing attrs
    /// (core `add_node` REPLACES; the binding composes the merge on top, as Task 4's D6 shim).
    #[pyo3(signature = (n, **attr))]
    fn add_node(&mut self, n: &Bound<'_, PyAny>, attr: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        let core_id = self.node_ids.intern(n)?;
        let mut merged = match self.inner.node_attrs(&core_id) {
            Some(Value::Object(m)) => m.clone(),
            _ => serde_json::Map::new(),
        };
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                merged.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        self.inner.add_node(&core_id, Value::Object(merged));
        Ok(())
    }

    /// `DH.remove_node(n, strong=False, remove_empty=True)`.
    #[pyo3(signature = (n, strong=false, remove_empty=true))]
    fn remove_node(
        &mut self,
        n: &Bound<'_, PyAny>,
        strong: bool,
        remove_empty: bool,
    ) -> PyResult<()> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .node_ids
            .get_key(n)?
            .ok_or_else(|| id_not_found(&repr))?;
        self.inner
            .remove_node(&core, strong, remove_empty)
            .map_err(|_| id_not_found(&core))?;
        self.node_ids.forget(&core);
        Ok(())
    }

    /// `DH.remove_edge(e)`.
    fn remove_edge(&mut self, e: &Bound<'_, PyAny>) -> PyResult<()> {
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .edge_ids
            .get_key(e)?
            .ok_or_else(|| id_not_found(&repr))?;
        self.inner
            .remove_edge(&core)
            .map_err(|_| id_not_found(&core))?;
        self.edge_ids.forget(&core);
        Ok(())
    }

    /// Single-node degree (edges the node belongs to, tail or head).
    fn degree(&self, n: &Bound<'_, PyAny>) -> PyResult<usize> {
        let core = self.require_node(n)?;
        Ok(self.inner.degree(&core))
    }

    /// Single-node in-degree (edges where the node is a head/receiver).
    fn in_degree(&self, n: &Bound<'_, PyAny>) -> PyResult<usize> {
        let core = self.require_node(n)?;
        Ok(self.inner.in_degree(&core))
    }

    /// Single-node out-degree (edges where the node is a tail/sender).
    fn out_degree(&self, n: &Bound<'_, PyAny>) -> PyResult<usize> {
        let core = self.require_node(n)?;
        Ok(self.inner.out_degree(&core))
    }

    /// `DH.nodes` — the directed `DiNodeView` proxy (Task 6, mirrors Task 5's `NodeView`).
    #[getter]
    fn nodes(slf: Py<Self>) -> PyDiNodeView {
        PyDiNodeView { dh: slf }
    }

    /// `DH.edges` — the directed `DiEdgeView` proxy (Task 6, mirrors Task 5's `EdgeView`).
    #[getter]
    fn edges(slf: Py<Self>) -> PyDiEdgeView {
        PyDiEdgeView { dh: slf }
    }
}

impl PyDiHypergraph {
    fn require_node(&self, n: &Bound<'_, PyAny>) -> PyResult<String> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        self.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))
    }

    /// Accepts a list of `(tail, head)` pairs (`diedgelist1`-style) or a dict of
    /// `idx -> (tail, head)` (`dihypergraph1`-style, `conftest.py`).
    fn ingest(&mut self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(list) = data.cast::<PyList>() {
            for edge in list.iter() {
                self.add_edge_impl(py, &edge, None, None)?;
            }
            return Ok(());
        }
        if let Ok(dict) = data.cast::<PyDict>() {
            for (idx, members) in dict.iter() {
                self.add_edge_impl(py, &members, Some(&idx), None)?;
            }
            return Ok(());
        }
        if let Ok(other) = data.extract::<PyRef<PyDiHypergraph>>() {
            self.inner = other.inner.copy();
            self.node_ids = other.node_ids.clone_via_py(py);
            self.edge_ids = other.edge_ids.clone_via_py(py);
            return Ok(());
        }
        Err(xgi_error("could not interpret data as a dihypergraph"))
    }

    fn add_edge_impl(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        attr: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let pair: (Bound<'_, PyAny>, Bound<'_, PyAny>) = members.extract().map_err(|_| {
            xgi_error("a directed edge must be a (tail, head) pair (conftest.py:diedgelist1)")
        })?;
        let mut tail = Vec::new();
        for m in pair.0.try_iter()? {
            tail.push(self.node_ids.intern(&m?)?);
        }
        let mut head = Vec::new();
        for m in pair.1.try_iter()? {
            head.push(self.node_ids.intern(&m?)?);
        }

        let mut e_attrs = serde_json::Map::new();
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                e_attrs.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }

        let requested_core = match idx {
            Some(o) => {
                if self.edge_ids.get_key(o)?.is_some() {
                    // DEVIATION SHIM (Task 4 pattern): duplicate idx -> warn + no-op.
                    let warnings = py.import("warnings")?;
                    warnings.call_method1(
                        "warn",
                        (
                            format!("edge {} already exists, ignoring", o.repr()?),
                            py.get_type::<pyo3::exceptions::PyUserWarning>(),
                        ),
                    )?;
                    return Ok(());
                }
                Some(self.edge_ids.intern(o)?)
            }
            None => None,
        };

        match self
            .inner
            .add_edge(tail, head, requested_core.clone(), Value::Object(e_attrs))
        {
            Ok(core_id) => {
                if requested_core.is_none() {
                    self.edge_ids.intern(&int_or_str_pyobj(py, &core_id)?)?;
                }
                Ok(())
            }
            Err(EdgeError::AlreadyExists { .. }) => Ok(()), // shimmed above; unreachable
            Err(e) => Err(xgi_error(format!("{e:?}"))),
        }
    }
}

/// Auto-assigned core ids are numeric strings ("0", "1", …) — register them as Python `int`
/// (XGI auto-ids are ints), falling back to `str` for the (unreachable in practice) non-numeric case.
fn int_or_str_pyobj<'py>(py: Python<'py>, core_id: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(n) = core_id.parse::<i64>() {
        return Ok(n.into_pyobject(py)?.into_any());
    }
    Ok(core_id.into_pyobject(py)?.into_any())
}

// ---------------------------------------------------------------------------------------------
// DiNodeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "DiNodeView", module = "hypergraph_rs")]
pub struct PyDiNodeView {
    pub dh: Py<PyDiHypergraph>,
}

#[pymethods]
impl PyDiNodeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        self.dh.borrow(py).inner.num_nodes()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let dh = self.dh.borrow(py);
        let items: Vec<Py<PyAny>> = dh
            .inner
            .node_ids()
            .iter()
            .filter_map(|c| dh.node_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(dh);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> bool {
        let dh = self.dh.borrow(py);
        matches!(dh.node_ids.get_key(n), Ok(Some(_)))
    }

    /// `DH.nodes[n]` -> attribute dict.
    fn __getitem__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let dh = self.dh.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = dh.inner.node_attrs(&core).cloned() {
            for (k, v) in m {
                d.set_item(k, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    /// `DH.nodes.memberships(n)` -> a SET of edge ids the node belongs to (head or tail).
    fn memberships(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let dh = self.dh.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for e in dh.inner.memberships(&core).unwrap_or_default() {
            if let Some(o) = dh.edge_ids.get_obj(py, &e) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    /// `DH.nodes.dimemberships(n)` -> `(in_edges, out_edges)` as a pair of SETs.
    fn dimemberships(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PyTuple>> {
        let dh = self.dh.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let (in_e, out_e) = dh.inner.dimemberships(&core).unwrap_or_default();
        let in_set = PySet::empty(py)?;
        for e in in_e {
            if let Some(o) = dh.edge_ids.get_obj(py, &e) {
                in_set.add(o)?;
            }
        }
        let out_set = PySet::empty(py)?;
        for e in out_e {
            if let Some(o) = dh.edge_ids.get_obj(py, &e) {
                out_set.add(o)?;
            }
        }
        Ok(PyTuple::new(py, [in_set.into_any(), out_set.into_any()])?.unbind())
    }
}

// ---------------------------------------------------------------------------------------------
// DiEdgeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "DiEdgeView", module = "hypergraph_rs")]
pub struct PyDiEdgeView {
    pub dh: Py<PyDiHypergraph>,
}

#[pymethods]
impl PyDiEdgeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        self.dh.borrow(py).inner.num_edges()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let dh = self.dh.borrow(py);
        let items: Vec<Py<PyAny>> = dh
            .inner
            .edge_ids()
            .iter()
            .filter_map(|c| dh.edge_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(dh);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> bool {
        let dh = self.dh.borrow(py);
        matches!(dh.edge_ids.get_key(e), Ok(Some(_)))
    }

    /// `DH.edges[e]` -> attribute dict.
    fn __getitem__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let dh = self.dh.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = dh.inner.edge_attrs(&core).cloned() {
            for (k, v) in m {
                d.set_item(k, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    /// `DH.edges.dimembers(e)` -> `(tail, head)` as a pair of SETs (XGI parity; XGI's are
    /// hash-unordered sets too, so `==` against a literal `(set, set)` is order-independent).
    fn dimembers(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PyTuple>> {
        let dh = self.dh.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let (tail, head) = dh.inner.dimembers(&core).unwrap_or_default();
        let tail_set = PySet::empty(py)?;
        for n in tail {
            if let Some(o) = dh.node_ids.get_obj(py, &n) {
                tail_set.add(o)?;
            }
        }
        let head_set = PySet::empty(py)?;
        for n in head {
            if let Some(o) = dh.node_ids.get_obj(py, &n) {
                head_set.add(o)?;
            }
        }
        Ok(PyTuple::new(py, [tail_set.into_any(), head_set.into_any()])?.unbind())
    }

    /// `DH.edges.tail(e)` -> the tail (sender) node ids as a SET.
    fn tail(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let dh = self.dh.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for n in dh.inner.tail(&core).unwrap_or_default() {
            if let Some(o) = dh.node_ids.get_obj(py, &n) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    /// `DH.edges.head(e)` -> the head (receiver) node ids as a SET.
    fn head(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let dh = self.dh.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for n in dh.inner.head(&core).unwrap_or_default() {
            if let Some(o) = dh.node_ids.get_obj(py, &n) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    /// `DH.edges.members(e)` -> `tail ∪ head` node ids as a SET.
    fn members(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let dh = self.dh.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for n in dh.inner.members(&core).unwrap_or_default() {
            if let Some(o) = dh.node_ids.get_obj(py, &n) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDiHypergraph>()?;
    m.add_class::<PyDiNodeView>()?;
    m.add_class::<PyDiEdgeView>()?;
    Ok(())
}
