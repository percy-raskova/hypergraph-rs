//! `stats`: the general `Stat` proxy objects (Task 11) — `H.nodes.degree`, `H.edges.size`,
//! `H.edges.order`, `H.nodes.attrs(key)` / `H.edges.attrs(key)`. Replaces Task 5's per-kind
//! stand-in stats (`views::PyNodeDegreeStat` / `PyEdgeSizeStat` / `PyEdgeOrderStat`) with a single
//! parametrized `NodeStat`/`EdgeStat` pyclass, mirroring XGI's `xgi/stats/nodestats.py` /
//! `edgestats.py` `Stat` machinery over Phase 4's `node_degree`/`edge_size` core methods.
//!
//! XGI's `stats/__init__.py` decorator machinery (`__all__` — `nodestat_func`, `edgestat_func`,
//! `dispatch_stat`, …) is unbound decorator plumbing, not observable state; `tests/stats/
//! test_core_stats_functions.py` (which tests that machinery directly) is a documented exclusion
//! (Task 15 triage) — the binding only needs the observable `.asdict()`/`.aslist()`/subscript
//! surface.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

use crate::errors::id_not_found;
use crate::hypergraph::PyHypergraph;
use crate::pyid::json_to_py;

#[derive(Clone)]
enum NodeStatKind {
    /// `order`/`weight` mirror `xgi/stats/nodestats.py::degree` exactly: `order=None` counts all
    /// memberships; `Some(k)` restricts to edges of size `k+1`; `weight=Some(attr)` sums that
    /// edge-attr (default `1` when absent) instead of counting `1` per edge.
    Degree {
        order: Option<usize>,
        weight: Option<String>,
    },
    Attr(String),
}

#[pyclass(name = "NodeStat", module = "hypergraph_rs")]
pub struct PyNodeStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
    kind: NodeStatKind,
}

impl PyNodeStat {
    pub fn degree(
        py: Python<'_>,
        hg: Py<PyHypergraph>,
        ids: Vec<String>,
        order: Option<usize>,
        weight: Option<String>,
    ) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::Degree { order, weight },
        }
    }

    pub fn attr(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>, key: String) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::Attr(key),
        }
    }

    fn values(&self, py: Python<'_>) -> Vec<(String, Value)> {
        let hg = self.hg.borrow(py);
        match &self.kind {
            NodeStatKind::Degree { order, weight } => self
                .ids
                .iter()
                .map(|c| {
                    let memberships = hg.inner.memberships(c).unwrap_or_default();
                    let val = match (order, weight) {
                        (None, None) => memberships.len() as f64,
                        (None, Some(w)) => memberships
                            .iter()
                            .map(|e| {
                                hg.inner
                                    .edge_attrs(e)
                                    .and_then(|a| a.get(w))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(1.0)
                            })
                            .sum(),
                        (Some(k), None) => memberships
                            .iter()
                            .filter(|e| hg.inner.edge_size(e).map(|s| s == k + 1).unwrap_or(false))
                            .count() as f64,
                        (Some(k), Some(w)) => memberships
                            .iter()
                            .filter(|e| hg.inner.edge_size(e).map(|s| s == k + 1).unwrap_or(false))
                            .map(|e| {
                                hg.inner
                                    .edge_attrs(e)
                                    .and_then(|a| a.get(w))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(1.0)
                            })
                            .sum(),
                    };
                    let jv = if val.fract() == 0.0 {
                        Value::from(val as i64)
                    } else {
                        Value::from(val)
                    };
                    (c.clone(), jv)
                })
                .collect(),
            NodeStatKind::Attr(key) => self
                .ids
                .iter()
                .map(|c| {
                    let v = hg
                        .inner
                        .node_attrs(c)
                        .and_then(|a| a.get(key))
                        .cloned()
                        .unwrap_or(Value::Null);
                    (c.clone(), v)
                })
                .collect(),
        }
    }
}

#[pymethods]
impl PyNodeStat {
    /// `H.nodes.degree(order=None, weight=None)` — re-parametrize a `Degree` stat.
    /// XGI parity: `Stat.__call__` re-dispatches with new args (stats/__init__.py).
    #[pyo3(signature = (order=None, weight=None))]
    fn __call__(
        &self,
        py: Python<'_>,
        order: Option<usize>,
        weight: Option<String>,
    ) -> PyResult<PyNodeStat> {
        match &self.kind {
            NodeStatKind::Degree { .. } => Ok(PyNodeStat {
                hg: self.hg.clone_ref(py),
                ids: self.ids.clone(),
                kind: NodeStatKind::Degree { order, weight },
            }),
            NodeStatKind::Attr(_) => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "this Stat does not accept parameters",
            )),
        }
    }

    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.values(py) {
            if let Some(obj) = hg.node_ids.get_obj(py, &core) {
                d.set_item(obj, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (_, v) in self.values(py) {
            list.append(json_to_py(py, &v)?)?;
        }
        Ok(list.unbind())
    }

    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let hg = self.hg.borrow(py);
        let repr = key.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg
            .node_ids
            .get_key(key)?
            .ok_or_else(|| id_not_found(&repr))?;
        drop(hg);
        let v = self
            .values(py)
            .into_iter()
            .find(|(c, _)| c == &core)
            .map(|(_, v)| v)
            .unwrap_or(Value::Null);
        Ok(json_to_py(py, &v)?.unbind())
    }
}

#[derive(Clone)]
enum EdgeStatKind {
    Order,
    Size,
    Attr(String),
}

#[pyclass(name = "EdgeStat", module = "hypergraph_rs")]
pub struct PyEdgeStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
    kind: EdgeStatKind,
}

impl PyEdgeStat {
    pub fn order(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: EdgeStatKind::Order,
        }
    }

    pub fn size(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: EdgeStatKind::Size,
        }
    }

    pub fn attr(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>, key: String) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: EdgeStatKind::Attr(key),
        }
    }

    fn values(&self, py: Python<'_>) -> Vec<(String, Value)> {
        let hg = self.hg.borrow(py);
        match &self.kind {
            EdgeStatKind::Size | EdgeStatKind::Order => self
                .ids
                .iter()
                .map(|c| {
                    let size = hg.inner.edge_size(c).unwrap_or(0);
                    let v = match self.kind {
                        EdgeStatKind::Size => size,
                        EdgeStatKind::Order => size.saturating_sub(1),
                        EdgeStatKind::Attr(_) => unreachable!(),
                    };
                    (c.clone(), Value::from(v as i64))
                })
                .collect(),
            EdgeStatKind::Attr(key) => self
                .ids
                .iter()
                .map(|c| {
                    let v = hg
                        .inner
                        .edge_attrs(c)
                        .and_then(|a| a.get(key))
                        .cloned()
                        .unwrap_or(Value::Null);
                    (c.clone(), v)
                })
                .collect(),
        }
    }
}

#[pymethods]
impl PyEdgeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.values(py) {
            if let Some(obj) = hg.edge_ids.get_obj(py, &core) {
                d.set_item(obj, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (_, v) in self.values(py) {
            list.append(json_to_py(py, &v)?)?;
        }
        Ok(list.unbind())
    }

    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let hg = self.hg.borrow(py);
        let repr = key.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg
            .edge_ids
            .get_key(key)?
            .ok_or_else(|| id_not_found(&repr))?;
        drop(hg);
        let v = self
            .values(py)
            .into_iter()
            .find(|(c, _)| c == &core)
            .map(|(_, v)| v)
            .unwrap_or(Value::Null);
        Ok(json_to_py(py, &v)?.unbind())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNodeStat>()?;
    m.add_class::<PyEdgeStat>()?;
    Ok(())
}
