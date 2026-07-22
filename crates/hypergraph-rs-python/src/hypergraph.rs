//! PyO3 binding for the core `Hypergraph`. Deviation shims (spec §3.1) live here.

use hypergraph_rs::core::error::EdgeError;
use hypergraph_rs::Hypergraph as CoreHg;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use serde_json::Value;

use crate::errors::{id_not_found, xgi_error};
use crate::pyid::{json_to_py, py_any_to_json, PyIdMap};

#[pyclass(name = "Hypergraph", module = "hypergraph_rs")]
pub struct PyHypergraph {
    pub inner: CoreHg<Value, Value, Value>,
    pub node_ids: PyIdMap,
    pub edge_ids: PyIdMap,
    pub net_attr: serde_json::Map<String, Value>,
}

#[pymethods]
impl PyHypergraph {
    #[new]
    #[pyo3(signature = (incoming_data=None, **attr))]
    fn new(
        py: Python<'_>,
        incoming_data: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut hg = Self {
            inner: CoreHg::new(),
            node_ids: PyIdMap::new(),
            edge_ids: PyIdMap::new(),
            net_attr: serde_json::Map::new(),
        };
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                hg.net_attr
                    .insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        if let Some(data) = incoming_data {
            hg.ingest(py, &data)?;
        }
        Ok(hg)
    }

    /// `H["name"]` — network-level attr; missing → XGIError (tests/core/test_hypergraph.py:55-56).
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

    /// `H.add_edge(members, idx=None, **attr)`. Shims: duplicate idx → warn+no-op.
    /// (Empty-members deviation shim is NOT needed: verified the core `add_edge` already accepts
    /// empty `members` and succeeds directly — see the Task 4 execution-time deviation note.)
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

    /// `n in H` — unhashable objects return False, not error (XGI test_contains,
    /// tests/core/test_hypergraph.py:70).
    fn __contains__(&self, n: &Bound<'_, PyAny>) -> bool {
        matches!(self.node_ids.get_key(n), Ok(Some(_)))
    }

    fn has_node(&self, n: &Bound<'_, PyAny>) -> bool {
        self.__contains__(n)
    }

    fn has_edge(&self, e: &Bound<'_, PyAny>) -> bool {
        matches!(self.edge_ids.get_key(e), Ok(Some(_)))
    }

    /// `H.add_node(n, **attr)` — merges the incoming dict into any existing attrs
    /// (core `add_node` REPLACES; the binding composes the merge on top, per core D6).
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

    /// `H.add_nodes_from(nodes_for_adding, **attr)` — each item is either a plain hashable
    /// id or a `(id, attr_dict)` pair (XGI parity).
    #[pyo3(signature = (nodes_for_adding, **attr))]
    fn add_nodes_from(
        &mut self,
        py: Python<'_>,
        nodes_for_adding: Bound<'_, PyAny>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        for item in nodes_for_adding.try_iter()? {
            let item = item?;
            let (id_obj, item_attr) = match item.cast::<PyTuple>() {
                Ok(t) if t.len() == 2 && t.get_item(1)?.cast::<PyDict>().is_ok() => {
                    let d = t.get_item(1)?.cast::<PyDict>().unwrap().clone();
                    (t.get_item(0)?, Some(d))
                }
                _ => (item.clone(), None),
            };
            let core_id = self.node_ids.intern(&id_obj)?;
            let mut merged = match self.inner.node_attrs(&core_id) {
                Some(Value::Object(m)) => m.clone(),
                _ => serde_json::Map::new(),
            };
            if let Some(a) = &attr {
                for (k, v) in a.iter() {
                    merged.insert(k.str()?.to_string(), py_any_to_json(&v)?);
                }
            }
            if let Some(a) = &item_attr {
                for (k, v) in a.iter() {
                    merged.insert(k.str()?.to_string(), py_any_to_json(&v)?);
                }
            }
            self.inner.add_node(&core_id, Value::Object(merged));
        }
        let _ = py;
        Ok(())
    }

    /// `H.remove_node(n, strong=False, remove_empty=True)`.
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

    /// `H.remove_edge(e)`.
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

    /// `H.nodes.memberships(n)` — the edge ids `n` belongs to, translated back to Python ids.
    fn memberships(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PyList>> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .node_ids
            .get_key(n)?
            .ok_or_else(|| id_not_found(&repr))?;
        let ids = self.inner.memberships(&core).unwrap_or_default();
        let list = PyList::empty(py);
        for eid in ids {
            if let Some(obj) = self.edge_ids.get_obj(py, &eid) {
                list.append(obj)?;
            }
        }
        Ok(list.unbind())
    }

    /// `H.edges.members(e)` — the node ids belonging to edge `e`, translated back to Python ids.
    fn members(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PyList>> {
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .edge_ids
            .get_key(e)?
            .ok_or_else(|| id_not_found(&repr))?;
        let ids = self.inner.members(&core).unwrap_or_default();
        let list = PyList::empty(py);
        for nid in ids {
            if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                list.append(obj)?;
            }
        }
        Ok(list.unbind())
    }

    /// Single-node degree (the full `H.degree()` Stat-dict proxy lands in Task 11).
    fn degree(&self, n: &Bound<'_, PyAny>) -> PyResult<usize> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .node_ids
            .get_key(n)?
            .ok_or_else(|| id_not_found(&repr))?;
        Ok(self.inner.memberships(&core).map(|m| m.len()).unwrap_or(0))
    }

    /// `H.nodes` — the full `NodeView` proxy (Task 5; replaces the Task 4 plain-list stub).
    #[getter]
    fn nodes(slf: Py<Self>) -> crate::views::PyNodeView {
        crate::views::PyNodeView { hg: slf, ids: None }
    }

    /// `H.edges` — the full `EdgeView` proxy (Task 5; replaces the Task 4 plain-list stub).
    #[getter]
    fn edges(slf: Py<Self>) -> crate::views::PyEdgeView {
        crate::views::PyEdgeView { hg: slf, ids: None }
    }
}

impl PyHypergraph {
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
        if let Ok(other) = data.extract::<PyRef<PyHypergraph>>() {
            self.inner = other.inner.copy();
            self.node_ids = other.node_ids.clone_via_py(py);
            self.edge_ids = other.edge_ids.clone_via_py(py);
            return Ok(());
        }
        // numpy/pandas/int → XGIError (matches pytest.raises(XGIError): xgi.Hypergraph(1),
        // tests/core/test_hypergraph.py:41). numpy-incidence + pandas constructors are documented
        // exclusions (no phase built from_incidence_matrix / the pandas slice).
        Err(xgi_error("could not interpret data as a hypergraph"))
    }

    fn add_edge_impl(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        attr: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let mut core_members = Vec::new();
        for m in members.try_iter()? {
            core_members.push(self.node_ids.intern(&m?)?);
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
                    // DEVIATION SHIM 1: duplicate idx → warn + no-op (XGI add_edge).
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

        // DEVIATION-SHIM-2 NOTE (Task 4, execution-time correction of the plan's premise): the plan
        // expected core `add_edge` to return `Err(EdgeError::EmptyMembers)` for empty `members`, needing
        // a shim into Task 0's `add_empty_edge`. Verified against the actual core
        // (crates/hypergraph-rs/src/core/hypergraph.rs): no `EmptyMembers` variant exists and
        // `add_edge(vec![], ...)` already succeeds directly (probed: `Ok("0")`, 0 members, 0 nodes) —
        // XGI empty-edge parity is already native. So there is no `Err(EdgeError::EmptyMembers)` arm
        // here; `add_empty_edge` remains landed (Task 0, harmless/additive) but unused by this shim.
        match self
            .inner
            .add_edge(core_members, requested_core.clone(), Value::Object(e_attrs))
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

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHypergraph>()?;
    Ok(())
}
