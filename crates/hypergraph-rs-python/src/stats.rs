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

use std::collections::HashMap;

use numpy::IntoPyArray;
use pyo3::exceptions::{PyTypeError, PyValueError};
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
    /// `""` = the whole per-node attr dict (`H.nodes.attrs.asdict()`); non-empty = a single key.
    Attr(String),
    /// `H.nodes.pos`: the attribute `"pos"`. XGI's `pos(net, bunch)` takes no argument, so calling
    /// the stat with one makes a stat that raises `TypeError` when it is evaluated, not when it
    /// is called (`IDStat.__call__` only stores the arguments). `error` is that message.
    Pos { error: Option<String> },
    /// A stat whose values were computed once, up front, by an existing `algorithms::*`
    /// function (e.g. `clustering_coefficient`, `clique_eigenvector_centrality`) — used for
    /// stats that take no further parametrization.
    Precomputed(HashMap<String, f64>),
    /// `two_node_clustering_coefficient(kind)` — recomputed on each `values()` call (the core
    /// call is cheap) so `H.nodes.two_node_clustering_coefficient(kind="min")` re-parametrizes
    /// correctly via `__call__`, unlike `Precomputed`.
    TwoNodeClustering(String),
    /// `local_edit_simpliciality`/`local_face_edit_simpliciality`/`local_simplicial_fraction`
    /// (`min_size`, `exclude_min_size`) — recomputed on each `values()` call via
    /// `algorithms::local_simpliciality_stat_map` so `H.nodes.local_edit_simpliciality(min_size=1)`
    /// re-parametrizes correctly via `__call__`, like `TwoNodeClustering`.
    LocalSimpliciality {
        min_size: usize,
        exclude_min_size: bool,
        func: fn(&hypergraph_rs::Hypergraph<Value, Value, Value>, usize, bool) -> f64,
    },
}

/// A bound argument that is there and is not `None`.
fn present<'a, 'py>(slot: &'a Option<Bound<'py, PyAny>>) -> Option<&'a Bound<'py, PyAny>> {
    slot.as_ref().filter(|v| !v.is_none())
}

/// The five parameters `NodeStat.__call__` takes, in order (`order`, `weight`, `kind`, `min_size`,
/// `exclude_min_size`), bound from `*args, **kwargs` the way Python binds them: a repeated or
/// unknown name, or a sixth positional, is a `TypeError`.
fn bind_stat_arguments<'py>(
    args: &Bound<'py, pyo3::types::PyTuple>,
    kwargs: Option<&Bound<'py, PyDict>>,
) -> PyResult<[Option<Bound<'py, PyAny>>; 5]> {
    const NAMES: [&str; 5] = ["order", "weight", "kind", "min_size", "exclude_min_size"];
    let mut slots: [Option<Bound<'py, PyAny>>; 5] = [None, None, None, None, None];
    if args.len() > NAMES.len() {
        return Err(PyTypeError::new_err(format!(
            "__call__() takes from 1 to {} positional arguments but {} were given",
            NAMES.len() + 1,
            args.len() + 1
        )));
    }
    for (i, a) in args.iter().enumerate() {
        slots[i] = Some(a);
    }
    if let Some(kwargs) = kwargs {
        for (k, v) in kwargs.iter() {
            let name: String = k.extract()?;
            let Some(i) = NAMES.iter().position(|n| *n == name) else {
                return Err(PyTypeError::new_err(format!(
                    "__call__() got an unexpected keyword argument '{name}'"
                )));
            };
            if slots[i].is_some() {
                return Err(PyTypeError::new_err(format!(
                    "__call__() got multiple values for argument '{name}'"
                )));
            }
            slots[i] = Some(v);
        }
    }
    Ok(slots)
}

/// What Python says when `pos(net, bunch)` is called with the arguments of a `pos(...)` stat:
/// too many positionals first, then an unknown keyword; `None` for no arguments.
pub(crate) fn pos_call_error(
    args: &Bound<'_, pyo3::types::PyTuple>,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<Option<String>> {
    if !args.is_empty() {
        return Ok(Some(format!(
            "pos() takes 2 positional arguments but {} were given",
            2 + args.len()
        )));
    }
    if let Some(kwargs) = kwargs {
        if let Some((k, _)) = kwargs.iter().next() {
            return Ok(Some(format!(
                "pos() got an unexpected keyword argument '{}'",
                k.str()?
            )));
        }
    }
    Ok(None)
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

    pub fn pos(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::Pos { error: None },
        }
    }

    /// The values, or the `TypeError` a `pos` stat called with arguments raises when evaluated.
    fn checked_values(&self, py: Python<'_>) -> PyResult<Vec<(String, Value)>> {
        if let NodeStatKind::Pos { error: Some(msg) } = &self.kind {
            return Err(PyTypeError::new_err(msg.clone()));
        }
        Ok(self.values(py))
    }

    pub fn precomputed(
        py: Python<'_>,
        hg: Py<PyHypergraph>,
        ids: Vec<String>,
        values: HashMap<String, f64>,
    ) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::Precomputed(values),
        }
    }

    pub fn two_node_clustering(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::TwoNodeClustering("union".to_string()),
        }
    }

    pub fn local_simpliciality(
        py: Python<'_>,
        hg: Py<PyHypergraph>,
        ids: Vec<String>,
        func: fn(&hypergraph_rs::Hypergraph<Value, Value, Value>, usize, bool) -> f64,
    ) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: NodeStatKind::LocalSimpliciality {
                min_size: 2,
                exclude_min_size: true,
                func,
            },
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
            NodeStatKind::Attr(_) | NodeStatKind::Pos { .. } => self
                .ids
                .iter()
                .map(|c| {
                    let key: &str = match &self.kind {
                        NodeStatKind::Attr(key) => key,
                        _ => "pos",
                    };
                    let v = if key.is_empty() {
                        // `""` = the whole attr dict for this node (`H.nodes.attrs.asdict()`).
                        hg.inner
                            .node_attrs(c)
                            .cloned()
                            .unwrap_or(Value::Object(Default::default()))
                    } else {
                        hg.inner
                            .node_attrs(c)
                            .and_then(|a| a.get(key))
                            .cloned()
                            .unwrap_or(Value::Null)
                    };
                    (c.clone(), v)
                })
                .collect(),
            NodeStatKind::Precomputed(values) => self
                .ids
                .iter()
                .map(|c| {
                    let val = values.get(c).copied().unwrap_or(0.0);
                    let jv = if val.is_finite() && val.fract() == 0.0 {
                        Value::from(val as i64)
                    } else {
                        Value::from(val)
                    };
                    (c.clone(), jv)
                })
                .collect(),
            NodeStatKind::TwoNodeClustering(kind) => {
                let values = hypergraph_rs::algorithms::clustering::two_node_clustering_coefficient(
                    &hg.inner, kind,
                );
                self.ids
                    .iter()
                    .map(|c| {
                        let val = values.get(c).copied().unwrap_or(0.0);
                        let jv = if val.is_finite() && val.fract() == 0.0 {
                            Value::from(val as i64)
                        } else {
                            Value::from(val)
                        };
                        (c.clone(), jv)
                    })
                    .collect()
            }
            NodeStatKind::LocalSimpliciality {
                min_size,
                exclude_min_size,
                func,
            } => {
                let values = crate::algorithms::local_simpliciality_stat_map(
                    &hg,
                    *min_size,
                    *exclude_min_size,
                    *func,
                );
                self.ids
                    .iter()
                    .map(|c| {
                        let val = values.get(c).copied().unwrap_or(f64::NAN);
                        (c.clone(), Value::from(val))
                    })
                    .collect()
            }
        }
    }
}

#[pymethods]
impl PyNodeStat {
    /// Re-parametrize a `Stat`. XGI parity: `Stat.__call__` re-dispatches with new args
    /// (`stats/__init__.py`). `order` is the first positional slot for BOTH callers:
    /// `H.nodes.degree(order=2)` (keyword) and `H.nodes.attrs("color")` /
    /// `H.nodes.attrs(-1)` (positional; the `Attr` branch reinterprets it as the attr key and
    /// raises `ValueError` for a non-str/None value, matching `stats/nodestats.py:95-96`'s
    /// `raise ValueError('"attr" must be str or None')`). `Precomputed` stats take no
    /// parameters — calling one is a same-value no-op, mirroring XGI's parameterless nodestats.
    #[pyo3(signature = (*args, **kwargs))]
    fn __call__(
        &self,
        py: Python<'_>,
        args: &Bound<'_, pyo3::types::PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyNodeStat> {
        if let NodeStatKind::Pos { .. } = &self.kind {
            return Ok(PyNodeStat {
                hg: self.hg.clone_ref(py),
                ids: self.ids.clone(),
                kind: NodeStatKind::Pos {
                    error: pos_call_error(args, kwargs)?,
                },
            });
        }
        let slots = bind_stat_arguments(args, kwargs)?;
        let order = slots[0].as_ref();
        let weight: Option<String> = present(&slots[1]).map(|v| v.extract()).transpose()?;
        let kind: Option<String> = present(&slots[2]).map(|v| v.extract()).transpose()?;
        let min_size: Option<usize> = present(&slots[3]).map(|v| v.extract()).transpose()?;
        let exclude_min_size: Option<bool> = present(&slots[4]).map(|v| v.extract()).transpose()?;
        match &self.kind {
            NodeStatKind::Degree {
                order: cur_order,
                weight: cur_weight,
            } => {
                let order = match order {
                    Some(o) if !o.is_none() => Some(o.extract::<usize>()?),
                    _ => *cur_order,
                };
                let weight = weight.or_else(|| cur_weight.clone());
                Ok(PyNodeStat {
                    hg: self.hg.clone_ref(py),
                    ids: self.ids.clone(),
                    kind: NodeStatKind::Degree { order, weight },
                })
            }
            NodeStatKind::Attr(cur) => {
                let attr = match order {
                    None => cur.clone(),
                    Some(v) if v.is_none() => cur.clone(),
                    Some(v) => v
                        .extract::<String>()
                        .map_err(|_| PyValueError::new_err("\"attr\" must be str or None"))?,
                };
                Ok(PyNodeStat {
                    hg: self.hg.clone_ref(py),
                    ids: self.ids.clone(),
                    kind: NodeStatKind::Attr(attr),
                })
            }
            NodeStatKind::Precomputed(_) | NodeStatKind::Pos { .. } => Ok(PyNodeStat {
                hg: self.hg.clone_ref(py),
                ids: self.ids.clone(),
                kind: self.kind.clone(),
            }),
            NodeStatKind::TwoNodeClustering(cur) => {
                let k = kind.unwrap_or_else(|| cur.clone());
                if !matches!(k.as_str(), "union" | "min" | "max") {
                    return Err(crate::errors::xgi_error(format!(
                        "invalid kind of clustering: {k}"
                    )));
                }
                Ok(PyNodeStat {
                    hg: self.hg.clone_ref(py),
                    ids: self.ids.clone(),
                    kind: NodeStatKind::TwoNodeClustering(k),
                })
            }
            NodeStatKind::LocalSimpliciality {
                min_size: cur_min_size,
                exclude_min_size: cur_exclude_min_size,
                func,
            } => Ok(PyNodeStat {
                hg: self.hg.clone_ref(py),
                ids: self.ids.clone(),
                kind: NodeStatKind::LocalSimpliciality {
                    min_size: min_size.unwrap_or(*cur_min_size),
                    exclude_min_size: exclude_min_size.unwrap_or(*cur_exclude_min_size),
                    func: *func,
                },
            }),
        }
    }

    /// `H.nodes.<stat>.asnumpy()` — dense `numpy.ndarray` of values, in id order.
    fn asnumpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let vals: Vec<f64> = self
            .checked_values(py)?
            .into_iter()
            .map(|(_, v)| v.as_f64().unwrap_or(f64::NAN))
            .collect();
        Ok(vals.into_pyarray(py).into_any())
    }

    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let hg = self.hg.borrow(py);
        let d = PyDict::new(py);
        for (core, v) in self.checked_values(py)? {
            if let Some(obj) = hg.node_ids.get_obj(py, &core) {
                d.set_item(obj, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (_, v) in self.checked_values(py)? {
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
            .checked_values(py)?
            .into_iter()
            .find(|(c, _)| c == &core)
            .map(|(_, v)| v)
            .unwrap_or(Value::Null);
        Ok(json_to_py(py, &v)?.unbind())
    }
}

#[derive(Clone)]
enum EdgeStatKind {
    /// `degree`, if given, counts only member nodes whose own node degree equals it (XGI
    /// parity: `stats/edgestats.py::order`/`size`) — mirrors `PyHypergraph::order`/`size`.
    Order(Option<usize>),
    Size(Option<usize>),
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
            kind: EdgeStatKind::Order(None),
        }
    }

    pub fn size(py: Python<'_>, hg: Py<PyHypergraph>, ids: Vec<String>) -> Self {
        let _ = py;
        Self {
            hg,
            ids,
            kind: EdgeStatKind::Size(None),
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
        let member_count = |c: &str, degree: Option<usize>| -> usize {
            match degree {
                None => hg.inner.edge_size(c).unwrap_or(0),
                Some(target) => hg
                    .inner
                    .members(c)
                    .unwrap_or_default()
                    .iter()
                    .filter(|n| hg.inner.node_degree(n, None).unwrap_or(0) == target)
                    .count(),
            }
        };
        match &self.kind {
            EdgeStatKind::Size(degree) => self
                .ids
                .iter()
                .map(|c| (c.clone(), Value::from(member_count(c, *degree) as i64)))
                .collect(),
            EdgeStatKind::Order(degree) => self
                .ids
                .iter()
                .map(|c| (c.clone(), Value::from(member_count(c, *degree) as i64 - 1)))
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
    /// Re-parametrize a `Size`/`Order` `EdgeStat` by member-node `degree` (XGI parity:
    /// `Stat.__call__` / `stats/edgestats.py::order`/`size`); `Attr` stats are parameterless.
    #[pyo3(signature = (degree=None))]
    fn __call__(&self, py: Python<'_>, degree: Option<usize>) -> PyResult<PyEdgeStat> {
        let kind = match &self.kind {
            EdgeStatKind::Size(_) => EdgeStatKind::Size(degree),
            EdgeStatKind::Order(_) => EdgeStatKind::Order(degree),
            EdgeStatKind::Attr(_) => {
                return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                    "this Stat does not accept parameters",
                ))
            }
        };
        Ok(PyEdgeStat {
            hg: self.hg.clone_ref(py),
            ids: self.ids.clone(),
            kind,
        })
    }

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

    /// `H.edges.<stat>.asnumpy()` — dense `numpy.ndarray` of values, in id order.
    fn asnumpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let vals: Vec<f64> = self
            .values(py)
            .into_iter()
            .map(|(_, v)| v.as_f64().unwrap_or(f64::NAN))
            .collect();
        Ok(vals.into_pyarray(py).into_any())
    }

    /// `H.edges.<stat>.max()` — XGI parity: `stats/__init__.py::Stat.max`.
    fn max(&self, py: Python<'_>) -> Option<f64> {
        self.values(py)
            .into_iter()
            .filter_map(|(_, v)| v.as_f64())
            .fold(None, |acc, x| Some(acc.map_or(x, |a: f64| a.max(x))))
    }

    /// `H.edges.<stat>.min()` — XGI parity: `stats/__init__.py::Stat.min`.
    fn min(&self, py: Python<'_>) -> Option<f64> {
        self.values(py)
            .into_iter()
            .filter_map(|(_, v)| v.as_f64())
            .fold(None, |acc, x| Some(acc.map_or(x, |a: f64| a.min(x))))
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
