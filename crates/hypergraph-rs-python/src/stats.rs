//! `stats`: the general `Stat` proxy objects (Task 11) — `H.nodes.degree`, `H.edges.size`,
//! `H.edges.order`. Replaces Task 5's per-kind stand-in stats (`views::PyNodeDegreeStat` /
//! `PyEdgeSizeStat` / `PyEdgeOrderStat`) with a single parametrized `NodeStat`/`EdgeStat` pyclass,
//! mirroring XGI's `xgi/stats/nodestats.py` / `edgestats.py` `Stat` machinery over Phase 4's
//! `node_degree`/`edge_size` core methods.
//!
//! XGI's `stats/__init__.py` decorator machinery (`__all__` — `nodestat_func`, `edgestat_func`,
//! `dispatch_stat`, …) is unbound decorator plumbing, not observable state; `tests/stats/
//! test_core_stats_functions.py` (which tests that machinery directly) is a documented exclusion
//! (Task 15 triage) — the binding only needs the observable `.asdict()`/`.aslist()` surface.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::hypergraph::PyHypergraph;

#[derive(Clone, Copy)]
enum NodeStatKind {
    Degree,
}

#[pyclass(name = "NodeStat", module = "hypergraph_rs")]
pub struct PyNodeStat {
    hg: Py<PyHypergraph>,
    ids: Vec<String>,
    kind: NodeStatKind,
}

impl PyNodeStat {
    pub fn degree(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::Degree,
        }
    }

    fn values(&self, py: Python<'_>) -> Vec<(String, i64)> {
        let hg = self.hg.borrow(py);
        match self.kind {
            NodeStatKind::Degree => self
                .ids
                .iter()
                .map(|c| (c.clone(), hg.inner.node_degree(c, None).unwrap_or(0) as i64))
                .collect(),
        }
    }
}

#[pymethods]
impl PyNodeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.values(py) {
            if let Some(obj) = hg.node_ids.get_obj(py, &core) {
                d.set_item(obj, v)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> Vec<i64> {
        self.values(py).into_iter().map(|(_, v)| v).collect()
    }
}

#[derive(Clone, Copy)]
enum EdgeStatKind {
    Order,
    Size,
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

    fn values(&self, py: Python<'_>) -> Vec<(String, i64)> {
        let hg = self.hg.borrow(py);
        self.ids
            .iter()
            .map(|c| {
                let size = hg.inner.edge_size(c).unwrap_or(0);
                let v = match self.kind {
                    EdgeStatKind::Size => size,
                    EdgeStatKind::Order => size.saturating_sub(1),
                };
                (c.clone(), v as i64)
            })
            .collect()
    }
}

#[pymethods]
impl PyEdgeStat {
    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.values(py) {
            if let Some(obj) = hg.edge_ids.get_obj(py, &core) {
                d.set_item(obj, v)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> Vec<i64> {
        self.values(py).into_iter().map(|(_, v)| v).collect()
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNodeStat>()?;
    m.add_class::<PyEdgeStat>()?;
    Ok(())
}
