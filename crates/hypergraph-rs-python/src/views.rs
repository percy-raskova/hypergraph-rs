//! `PyNodeView` / `PyEdgeView` proxy objects (Task 5) + minimal degree/size/order `Stat` getters.
//!
//! XGI parity: `H.nodes` / `H.edges` are lazy, insertion-ordered, iterable/filterable views —
//! never materialized lists (Task 4's stub getters). Wraps Phase 2's `NodeView`/`EdgeView` +
//! `FilterMode` (`hypergraph_rs::core::views`) for the filtering logic; the binding re-borrows the
//! parent `PyHypergraph` under the GIL per call (`#[pyclass]` cannot hold a Rust borrow, so views
//! hold a `Py<PyHypergraph>` back-reference — standard PyO3 pattern).
//!
//! DEVIATION (documented, not a plan violation): the plan's Step 1 snippet forward-references
//! `crate::stats::PyNodeStat` ("Task 11 owns PyNodeStat") — that module does not exist yet (Task 11
//! is a later task). `.degree`/`.size`/`.order` getters here return small local stand-in stat
//! proxies (`PyNodeDegreeStat`/`PyEdgeSizeStat`/`PyEdgeOrderStat`) that support exactly the
//! `.asdict()`/`.aslist()` surface `test_views_smoke.py` and `tests/stats/test_nodestats.py`
//! exercise. Task 11 replaces these with the general multi-stat `Stat` proxy (same pattern Task 4's
//! `nodes`/`edges` stub getters were replaced by this task).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};
use serde_json::Value;

use hypergraph_rs::FilterMode;

use crate::errors::id_not_found;
use crate::hypergraph::PyHypergraph;
use crate::pyid::json_to_py;

fn parse_mode(mode: &str) -> PyResult<FilterMode> {
    match mode {
        "eq" => Ok(FilterMode::Eq),
        "neq" => Ok(FilterMode::Neq),
        "lt" => Ok(FilterMode::Lt),
        "gt" => Ok(FilterMode::Gt),
        "leq" => Ok(FilterMode::Leq),
        "geq" => Ok(FilterMode::Geq),
        other => Err(PyValueError::new_err(format!(
            "unrecognized filterby mode: {other}"
        ))),
    }
}

// ---------------------------------------------------------------------------------------------
// NodeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "NodeView", module = "hypergraph_rs")]
pub struct PyNodeView {
    pub hg: Py<PyHypergraph>,
    /// `None` = the full node set (the `H.nodes` root view); `Some` = a filtered sub-view.
    pub ids: Option<Vec<String>>,
}

impl PyNodeView {
    fn effective_ids(&self, hg: &PyHypergraph) -> Vec<String> {
        self.ids.clone().unwrap_or_else(|| hg.inner.node_ids())
    }

    fn sub(&self, py: Python<'_>, ids: Vec<String>) -> PyNodeView {
        PyNodeView {
            hg: self.hg.clone_ref(py),
            ids: Some(ids),
        }
    }
}

#[pymethods]
impl PyNodeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        let hg = self.hg.borrow(py);
        self.effective_ids(&hg).len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let hg = self.hg.borrow(py);
        let items: Vec<Py<PyAny>> = self
            .effective_ids(&hg)
            .iter()
            .filter_map(|c| hg.node_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(hg);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> bool {
        let hg = self.hg.borrow(py);
        match hg.node_ids.get_key(n) {
            Ok(Some(core)) => self.effective_ids(&hg).iter().any(|i| i == &core),
            _ => false,
        }
    }

    /// `H.nodes[n]` -> attribute dict (tests/core/test_hypergraph.py:87).
    fn __getitem__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = hg.inner.node_attrs(&core).cloned() {
            for (k, v) in m {
                d.set_item(k, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    /// `H.nodes.memberships(n)` -> a SET of edge ids (deviation: core `Vec` -> set).
    fn memberships(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for e in hg.inner.memberships(&core).unwrap_or_default() {
            if let Some(o) = hg.edge_ids.get_obj(py, &e) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    #[getter]
    fn degree(&self, py: Python<'_>) -> PyNodeDegreeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        PyNodeDegreeStat {
            hg: self.hg.clone_ref(py),
            ids,
        }
    }

    /// Sub-view of degree-0 nodes. XGI parity: `H.nodes.isolates()`.
    #[pyo3(signature = (ignore_singletons=false))]
    fn isolates(&self, py: Python<'_>, ignore_singletons: bool) -> PyNodeView {
        let hg = self.hg.borrow(py);
        let full = hg.inner.nodes();
        let sub_ids: Vec<String> = if ignore_singletons {
            full.isolates_ignoring_singletons().ids().to_vec()
        } else {
            full.isolates().ids().to_vec()
        };
        let ids: Vec<String> = sub_ids
            .into_iter()
            .filter(|i| self.effective_ids(&hg).contains(i))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    /// `H.nodes.filterby("degree", val, mode="eq")` — only the `"degree"` stat is available at
    /// this task (Task 11 generalizes to arbitrary stats); other names raise `NotImplementedError`.
    #[pyo3(signature = (stat, val, mode="eq".to_string()))]
    fn filterby(
        &self,
        py: Python<'_>,
        stat: &str,
        val: usize,
        mode: String,
    ) -> PyResult<PyNodeView> {
        if stat != "degree" {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(format!(
                "filterby(\"{stat}\", ...) deferred: only \"degree\" is wired before Task 11"
            )));
        }
        let m = parse_mode(&mode)?;
        let hg = self.hg.borrow(py);
        let full = hg.inner.nodes();
        let filtered_ids: Vec<String> = full.filterby_degree(val, m).ids().to_vec();
        let ids: Vec<String> = filtered_ids
            .into_iter()
            .filter(|i| self.effective_ids(&hg).contains(i))
            .collect();
        drop(hg);
        Ok(self.sub(py, ids))
    }
}

// ---------------------------------------------------------------------------------------------
// EdgeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "EdgeView", module = "hypergraph_rs")]
pub struct PyEdgeView {
    pub hg: Py<PyHypergraph>,
    /// `None` = the full edge set (the `H.edges` root view); `Some` = a filtered sub-view.
    pub ids: Option<Vec<String>>,
}

impl PyEdgeView {
    fn effective_ids(&self, hg: &PyHypergraph) -> Vec<String> {
        self.ids.clone().unwrap_or_else(|| hg.inner.edge_ids())
    }

    fn sub(&self, py: Python<'_>, ids: Vec<String>) -> PyEdgeView {
        PyEdgeView {
            hg: self.hg.clone_ref(py),
            ids: Some(ids),
        }
    }

    /// Intersect a core-view filter result (computed over ALL edges) with this view's current
    /// scope, preserving this view's insertion order.
    fn narrow(&self, hg: &PyHypergraph, filtered: Vec<String>) -> Vec<String> {
        let scope = self.effective_ids(hg);
        scope.into_iter().filter(|i| filtered.contains(i)).collect()
    }
}

#[pymethods]
impl PyEdgeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        let hg = self.hg.borrow(py);
        self.effective_ids(&hg).len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let hg = self.hg.borrow(py);
        let items: Vec<Py<PyAny>> = self
            .effective_ids(&hg)
            .iter()
            .filter_map(|c| hg.edge_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(hg);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> bool {
        let hg = self.hg.borrow(py);
        match hg.edge_ids.get_key(e) {
            Ok(Some(core)) => self.effective_ids(&hg).iter().any(|i| i == &core),
            _ => false,
        }
    }

    /// `H.edges[e]` -> attribute dict.
    fn __getitem__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = hg.inner.edge_attrs(&core).cloned() {
            for (k, v) in m {
                d.set_item(k, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    /// `H.edges.members(e)` -> a SET of node ids (deviation: core `Vec` -> set;
    /// tests/core/test_hypergraph.py:83).
    fn members(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let set = PySet::empty(py)?;
        for n in hg.inner.members(&core).unwrap_or_default() {
            if let Some(o) = hg.node_ids.get_obj(py, &n) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    #[getter]
    fn size(&self, py: Python<'_>) -> PyEdgeSizeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        PyEdgeSizeStat {
            hg: self.hg.clone_ref(py),
            ids,
        }
    }

    #[getter]
    fn order(&self, py: Python<'_>) -> PyEdgeOrderStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        PyEdgeOrderStat {
            hg: self.hg.clone_ref(py),
            ids,
        }
    }

    /// Sub-view of size-1 edges. XGI parity: `H.edges.singletons()`.
    fn singletons(&self, py: Python<'_>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().singletons().ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        self.sub(py, ids)
    }

    /// Sub-view of size-0 edges. XGI parity: `H.edges.empty()`.
    fn empty(&self, py: Python<'_>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().empty().ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        self.sub(py, ids)
    }

    #[pyo3(signature = (val, mode="eq".to_string()))]
    fn filterby_size(&self, py: Python<'_>, val: usize, mode: String) -> PyResult<PyEdgeView> {
        let m = parse_mode(&mode)?;
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().filterby_size(val, m).ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        Ok(self.sub(py, ids))
    }

    #[pyo3(signature = (val, mode="eq".to_string()))]
    fn filterby_order(&self, py: Python<'_>, val: usize, mode: String) -> PyResult<PyEdgeView> {
        let m = parse_mode(&mode)?;
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().filterby_order(val, m).ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// Sub-view of edges that duplicate an earlier one (same member-set). XGI parity:
    /// `H.edges.duplicates()`.
    fn duplicates(&self, py: Python<'_>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().duplicates().ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        self.sub(py, ids)
    }

    /// Sub-view of maximal edges (not a subset of any other edge). XGI parity: `H.edges.maximal()`.
    #[pyo3(signature = (strict=true))]
    fn maximal(&self, py: Python<'_>, strict: bool) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let filtered = hg.inner.edges().maximal(strict).ids().to_vec();
        let ids = self.narrow(&hg, filtered);
        drop(hg);
        self.sub(py, ids)
    }
}

// ---------------------------------------------------------------------------------------------
// Minimal stand-in Stat proxies (Task 5 stopgap — Task 11 owns the general `Stat` proxy).
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "NodeStat", module = "hypergraph_rs")]
pub struct PyNodeDegreeStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
}

#[pymethods]
impl PyNodeDegreeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for id in &self.ids {
            let deg = hg.inner.memberships(id).map(|m| m.len()).unwrap_or(0);
            if let Some(obj) = hg.node_ids.get_obj(py, id) {
                d.set_item(obj, deg)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let hg = self.hg.borrow(py);
        let list = PyList::empty(py);
        for id in &self.ids {
            list.append(hg.inner.memberships(id).map(|m| m.len()).unwrap_or(0))?;
        }
        Ok(list.unbind())
    }
}

#[pyclass(name = "EdgeSizeStat", module = "hypergraph_rs")]
pub struct PyEdgeSizeStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
}

#[pymethods]
impl PyEdgeSizeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for id in &self.ids {
            let size = hg.inner.members(id).map(|m| m.len()).unwrap_or(0);
            if let Some(obj) = hg.edge_ids.get_obj(py, id) {
                d.set_item(obj, size)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let hg = self.hg.borrow(py);
        let list = PyList::empty(py);
        for id in &self.ids {
            list.append(hg.inner.members(id).map(|m| m.len()).unwrap_or(0))?;
        }
        Ok(list.unbind())
    }
}

#[pyclass(name = "EdgeOrderStat", module = "hypergraph_rs")]
pub struct PyEdgeOrderStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
}

#[pymethods]
impl PyEdgeOrderStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for id in &self.ids {
            let order = hg
                .inner
                .members(id)
                .map(|m| m.len().saturating_sub(1))
                .unwrap_or(0);
            if let Some(obj) = hg.edge_ids.get_obj(py, id) {
                d.set_item(obj, order)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let hg = self.hg.borrow(py);
        let list = PyList::empty(py);
        for id in &self.ids {
            list.append(
                hg.inner
                    .members(id)
                    .map(|m| m.len().saturating_sub(1))
                    .unwrap_or(0),
            )?;
        }
        Ok(list.unbind())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNodeView>()?;
    m.add_class::<PyEdgeView>()?;
    m.add_class::<PyNodeDegreeStat>()?;
    m.add_class::<PyEdgeSizeStat>()?;
    m.add_class::<PyEdgeOrderStat>()?;
    Ok(())
}
