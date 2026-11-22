//! `PyNodeView` / `PyEdgeView` proxy objects (Task 5) + minimal degree/size/order `Stat` getters.
//!
//! XGI parity: `H.nodes` / `H.edges` are lazy, insertion-ordered, iterable/filterable views —
//! never materialized lists (Task 4's stub getters). Wraps Phase 2's `NodeView`/`EdgeView` +
//! `FilterMode` (`hypergraph_rs::core::views`) for the filtering logic; the binding re-borrows the
//! parent `PyHypergraph` under the GIL per call (`#[pyclass]` cannot hold a Rust borrow, so views
//! hold a `Py<PyHypergraph>` back-reference — standard PyO3 pattern).
//!
//! `.degree`/`.size`/`.order` getters delegate to `crate::stats::PyNodeStat`/`PyEdgeStat` (Task 11's
//! general multi-stat `Stat` proxy — same pattern Task 4's `nodes`/`edges` stub getters were
//! replaced by this task).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyString};
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

/// `IDView.filterby`/`filterby_attr`'s `mode` — a string (`"eq"`/`"neq"`/`"lt"`/`"gt"`/`"leq"`/
/// `"geq"`/`"between"`) or an arbitrary callable `mode(value, val) -> bool` (views.py:289-364).
pub(crate) fn eval_mode(
    mode: &Bound<'_, PyAny>,
    value: &Bound<'_, PyAny>,
    target: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    use pyo3::basic::CompareOp;
    if let Ok(s) = mode.cast::<PyString>() {
        let s = s.to_string_lossy();
        let op = match s.as_ref() {
            "eq" => CompareOp::Eq,
            "neq" => CompareOp::Ne,
            "lt" => CompareOp::Lt,
            "gt" => CompareOp::Gt,
            "leq" => CompareOp::Le,
            "geq" => CompareOp::Ge,
            "between" => {
                let lo = target.get_item(0)?;
                let hi = target.get_item(1)?;
                let ge_lo = value.rich_compare(&lo, CompareOp::Ge)?.is_truthy()?;
                let le_hi = value.rich_compare(&hi, CompareOp::Le)?.is_truthy()?;
                return Ok(ge_lo && le_hi);
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unrecognized mode {other}. mode must be one of 'eq', 'neq', 'lt', 'gt', \
                     'leq', 'geq', or 'between'."
                )))
            }
        };
        return value.rich_compare(target, op)?.is_truthy();
    }
    if mode.is_callable() {
        return mode.call1((value, target))?.is_truthy();
    }
    Err(PyValueError::new_err(
        "mode must be a string or a callable mode(value, val) -> bool",
    ))
}

// ---------------------------------------------------------------------------------------------
// NodeView
// ---------------------------------------------------------------------------------------------

/// The `dtype` of a node view's `memberships`-like method: `None` and `dict` give a dict (`true`),
/// `list` a list (`false`), anything else raises `XGIError("Unrecognized dtype ...")`.
pub(crate) fn node_dtype_is_dict(
    py: Python<'_>,
    dtype: Option<&Bound<'_, PyAny>>,
) -> PyResult<bool> {
    match dtype {
        None => Ok(true),
        Some(d) if d.is_none() || d.is(py.get_type::<PyDict>()) => Ok(true),
        Some(d) if d.is(py.get_type::<PyList>()) => Ok(false),
        Some(d) => Err(crate::errors::xgi_error(format!(
            "Unrecognized dtype {}",
            d.str()?
        ))),
    }
}

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

    fn __bool__(&self, py: Python<'_>) -> bool {
        self.__len__(py) > 0
    }

    /// `H.nodes == H2.nodes` — XGI's `IDView` is a `Set`, so equality compares the ID sets
    /// (views.py: `IDView(Mapping, Set)`), not attrs.
    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(other_view) = other.extract::<PyRef<PyNodeView>>() else {
            return Ok(false);
        };
        let hg1 = self.hg.borrow(py);
        let ids1 = hg1.node_ids.clone_via_py(py);
        let a = reprs_of(py, ids1, &self.effective_ids(&hg1))?;
        drop(hg1);
        let hg2 = other_view.hg.borrow(py);
        let ids2 = hg2.node_ids.clone_via_py(py);
        let b = reprs_of(py, ids2, &other_view.effective_ids(&hg2))?;
        Ok(a == b)
    }

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

    /// `H.nodes(bunch)` — a view scoped to `bunch`, IDs ordered by the GLOBAL node order
    /// (views.py `from_view`: `[i for i in view._id_dict if i in bunch]`, not `bunch`'s own
    /// order). Every item of `bunch` must be a currently-known node id or this raises
    /// `IDNotFound` (views.py:512-545: `wrong = bunch - all_ids`).
    fn __call__(&self, py: Python<'_>, bunch: &Bound<'_, PyAny>) -> PyResult<PyNodeView> {
        let hg = self.hg.borrow(py);
        let mut wanted: Vec<String> = Vec::new();
        for item in bunch.try_iter()? {
            let item = item?;
            let repr = item.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = hg
                .node_ids
                .get_key(&item)?
                .ok_or_else(|| id_not_found(&repr))?;
            wanted.push(core);
        }
        let ids: Vec<String> = hg
            .inner
            .node_ids()
            .into_iter()
            .filter(|i| wanted.contains(i))
            .collect();
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// `View.from_view(view, bunch=None)` (views.py:512-545) — reachable as an instance method
    /// (`H.edges.from_view(H.edges, bunch=[...])`, matching the oracle test's call shape) even
    /// though XGI declares it a `@classmethod`; `self` is unused.
    #[pyo3(signature = (view, bunch=None))]
    // XGI API-surface constraint: `from_view` is a deliberate port of XGI's Python-facing
    // classmethod name (views.py:512-545); renaming would break conformance.
    #[allow(clippy::wrong_self_convention)]
    fn from_view(
        &self,
        py: Python<'_>,
        view: PyRef<'_, PyNodeView>,
        bunch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyNodeView> {
        match bunch {
            Some(b) => view.__call__(py, b),
            None => Ok(PyNodeView {
                hg: view.hg.clone_ref(py),
                ids: None,
            }),
        }
    }

    #[getter]
    fn ids(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let set = PySet::empty(py)?;
        for c in self.effective_ids(&hg) {
            if let Some(o) = hg.node_ids.get_obj(py, &c) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    fn __sub__(&self, py: Python<'_>, other: PyRef<'_, PyNodeView>) -> PyNodeView {
        let hg = self.hg.borrow(py);
        let ob: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let ids: Vec<String> = self
            .effective_ids(&hg)
            .into_iter()
            .filter(|i| !ob.contains(i))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    fn __and__(&self, py: Python<'_>, other: PyRef<'_, PyNodeView>) -> PyNodeView {
        let hg = self.hg.borrow(py);
        let ob: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let ids: Vec<String> = self
            .effective_ids(&hg)
            .into_iter()
            .filter(|i| ob.contains(i))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    fn __or__(&self, py: Python<'_>, other: PyRef<'_, PyNodeView>) -> PyNodeView {
        let hg = self.hg.borrow(py);
        let mut ids = self.effective_ids(&hg);
        let existing: std::collections::HashSet<String> = ids.iter().cloned().collect();
        for i in other.effective_ids(&hg) {
            if !existing.contains(&i) {
                ids.push(i);
            }
        }
        drop(hg);
        self.sub(py, ids)
    }

    fn __xor__(&self, py: Python<'_>, other: PyRef<'_, PyNodeView>) -> PyNodeView {
        let hg = self.hg.borrow(py);
        let a: std::collections::HashSet<String> = self.effective_ids(&hg).into_iter().collect();
        let b: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let mut ids: Vec<String> = hg.inner.node_ids();
        ids.retain(|i| a.contains(i) ^ b.contains(i));
        drop(hg);
        self.sub(py, ids)
    }

    /// `H.nodes.memberships(n=None, dtype=dict)` -> a SET of edge ids for `n`, or, when `n` is
    /// omitted, the sets of all nodes in this view as a dict (`dtype=dict`, the default) or a list
    /// in view order (`dtype=list`). Anything else raises `XGIError`.
    #[pyo3(signature = (n=None, dtype=None))]
    fn memberships(
        &self,
        py: Python<'_>,
        n: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let hg = self.hg.borrow(py);
        match n {
            Some(n) => {
                let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = hg.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
                let set = PySet::empty(py)?;
                for e in hg.inner.memberships(&core).unwrap_or_default() {
                    if let Some(o) = hg.edge_ids.get_obj(py, &e) {
                        set.add(o)?;
                    }
                }
                Ok(set.into_any().unbind())
            }
            None => {
                let as_dict = node_dtype_is_dict(py, dtype)?;
                let d = PyDict::new(py);
                for nid in self.effective_ids(&hg) {
                    let Some(obj) = hg.node_ids.get_obj(py, &nid) else {
                        continue;
                    };
                    let set = PySet::empty(py)?;
                    for e in hg.inner.memberships(&nid).unwrap_or_default() {
                        if let Some(o) = hg.edge_ids.get_obj(py, &e) {
                            set.add(o)?;
                        }
                    }
                    d.set_item(obj, set)?;
                }
                if as_dict {
                    Ok(d.into_any().unbind())
                } else {
                    Ok(d.values().into_any().unbind())
                }
            }
        }
    }

    /// `H.nodes.neighbors(n)` — nodes sharing at least one edge with `n` (views.py:364-408).
    fn neighbors(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
        let mut result: std::collections::HashSet<String> = std::collections::HashSet::new();
        for eid in hg.inner.memberships(&core).unwrap_or_default() {
            for m in hg.inner.members(&eid).unwrap_or_default() {
                if m != core {
                    result.insert(m);
                }
            }
        }
        let set = PySet::empty(py)?;
        for c in result {
            if let Some(o) = hg.node_ids.get_obj(py, &c) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    /// `H.nodes.lookup(neighbors)` — nodes whose exact membership set equals `neighbors`
    /// (views.py:467-509).
    fn lookup(&self, py: Python<'_>, neighbors: &Bound<'_, PyAny>) -> PyResult<PyNodeView> {
        let hg = self.hg.borrow(py);
        let mut sought: Vec<String> = Vec::new();
        for m in neighbors.try_iter()? {
            let m = m?;
            match hg.edge_ids.get_key(&m)? {
                Some(c) => sought.push(c),
                None => {
                    drop(hg);
                    return Ok(self.sub(py, Vec::new()));
                }
            }
        }
        sought.sort();
        sought.dedup();
        let mut ids = Vec::new();
        for nid in self.effective_ids(&hg) {
            let mut mem = hg.inner.memberships(&nid).unwrap_or_default();
            mem.sort();
            mem.dedup();
            if mem == sought {
                ids.push(nid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }

    #[getter]
    fn degree(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::degree(py, self.hg.clone_ref(py), ids, None, None)
    }

    /// `H.nodes.attrs` — the general node attribute `Stat` proxy (`""` key = whole attr dict).
    /// Dispatched exactly like `degree`: attribute access alone yields the full-attrs `Stat`;
    /// `H.nodes.attrs("color")` / `H.nodes.attrs(attr="color")` re-parametrizes it via
    /// `Stat.__call__` (XGI parity: `attrs` is a `nodestat_func` like any other).
    #[getter]
    fn attrs(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::attr(py, self.hg.clone_ref(py), ids, String::new())
    }

    /// `H.nodes.pos` — the node attribute `"pos"` as a `Stat` (`None` for a node without one).
    /// XGI parity: `stats/nodestats.py::pos`.
    #[getter]
    fn pos(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::attr(py, self.hg.clone_ref(py), ids, "pos".to_string())
    }

    /// `H.nodes.average_neighbor_degree` (bare/called) — XGI parity:
    /// `stats/nodestats.py::average_neighbor_degree`.
    #[getter]
    fn average_neighbor_degree(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        let values: std::collections::HashMap<String, f64> =
            crate::algorithms::average_neighbor_degree_map(&hg)
                .into_iter()
                .collect();
        drop(hg);
        crate::stats::PyNodeStat::precomputed(py, self.hg.clone_ref(py), ids, values)
    }

    /// `H.nodes.local_edit_simpliciality` (bare/called, `min_size`/`exclude_min_size`
    /// re-parametrizable) — XGI parity: `stats/nodestats.py::local_edit_simpliciality`.
    #[getter]
    fn local_edit_simpliciality(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::local_simpliciality(
            py,
            self.hg.clone_ref(py),
            ids,
            hypergraph_rs::algorithms::simpliciality::edit_simpliciality,
        )
    }

    /// `H.nodes.local_face_edit_simpliciality` (bare/called) — XGI parity:
    /// `stats/nodestats.py::local_face_edit_simpliciality`.
    #[getter]
    fn local_face_edit_simpliciality(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::local_simpliciality(
            py,
            self.hg.clone_ref(py),
            ids,
            hypergraph_rs::algorithms::simpliciality::face_edit_simpliciality,
        )
    }

    /// `H.nodes.local_simplicial_fraction` (bare/called) — XGI parity:
    /// `stats/nodestats.py::local_simplicial_fraction`.
    #[getter]
    fn local_simplicial_fraction(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::local_simpliciality(
            py,
            self.hg.clone_ref(py),
            ids,
            hypergraph_rs::algorithms::simpliciality::simplicial_fraction,
        )
    }

    /// `H.nodes.clustering_coefficient` (bare) / `H.nodes.clustering_coefficient()` (called) —
    /// XGI parity: `algorithms/clustering.py:clustering_coefficient`, wired via the
    /// `nodestat_func` registry (attribute access alone yields the realized `Stat`; a further
    /// call just re-dispatches to an equivalent `Stat`, see `PyNodeStat::__call__`'s
    /// `Precomputed` branch).
    #[getter]
    fn clustering_coefficient(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        let values: std::collections::HashMap<String, f64> =
            hypergraph_rs::algorithms::clustering::clustering_coefficient(&hg.inner)
                .into_iter()
                .collect();
        drop(hg);
        crate::stats::PyNodeStat::precomputed(py, self.hg.clone_ref(py), ids, values)
    }

    /// `H.nodes.local_clustering_coefficient` (bare/called) — XGI parity:
    /// `clustering.py:local_clustering_coefficient`.
    #[getter]
    fn local_clustering_coefficient(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        let values: std::collections::HashMap<String, f64> =
            hypergraph_rs::algorithms::clustering::local_clustering_coefficient(&hg.inner)
                .into_iter()
                .collect();
        drop(hg);
        crate::stats::PyNodeStat::precomputed(py, self.hg.clone_ref(py), ids, values)
    }

    /// `H.nodes.two_node_clustering_coefficient` (bare, `kind="union"`) /
    /// `H.nodes.two_node_clustering_coefficient(kind=...)` (called) — XGI parity:
    /// `clustering.py:two_node_clustering_coefficient`. Re-parametrizable via
    /// `PyNodeStat::__call__`'s `TwoNodeClustering` branch.
    #[getter]
    fn two_node_clustering_coefficient(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyNodeStat::two_node_clustering(py, self.hg.clone_ref(py), ids)
    }

    /// `H.nodes.clique_eigenvector_centrality` — XGI parity: `centrality.py:clique_eigenvector_centrality`
    /// (accessed as a bare property, e.g. `H.nodes.clique_eigenvector_centrality.asnumpy()`).
    #[getter]
    fn clique_eigenvector_centrality(&self, py: Python<'_>) -> crate::stats::PyNodeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        let values: std::collections::HashMap<String, f64> =
            hypergraph_rs::algorithms::centrality::clique_eigenvector_centrality(
                &hg.inner, 1e-6, 1000,
            )
            .into_iter()
            .collect();
        drop(hg);
        crate::stats::PyNodeStat::precomputed(py, self.hg.clone_ref(py), ids, values)
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

    /// `H.nodes.duplicate_groups()`: groups of nodes with the same edge memberships. Like XGI,
    /// groups the whole network whatever this view filters, one view per group.
    fn duplicate_groups(&self, py: Python<'_>) -> Vec<PyNodeView> {
        let groups = self.hg.borrow(py).inner.nodes().duplicate_groups();
        groups.into_iter().map(|g| self.sub(py, g)).collect()
    }

    /// Sub-view of nodes that duplicate an earlier one (same edge-membership set). XGI parity:
    /// `H.nodes.duplicates()` (views.py:409-465, mirrored for nodes).
    fn duplicates(&self, py: Python<'_>) -> PyNodeView {
        let hg = self.hg.borrow(py);
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for nid in self.effective_ids(&hg) {
            let mut m = hg.inner.memberships(&nid).unwrap_or_default();
            m.sort();
            groups.entry(m).or_default().push(nid);
        }
        let mut dups: Vec<String> = Vec::new();
        for group in groups.values() {
            if group.len() > 1 {
                let mut sorted = group.clone();
                sorted.sort();
                dups.extend(sorted.into_iter().skip(1));
            }
        }
        let ids: Vec<String> = self
            .effective_ids(&hg)
            .into_iter()
            .filter(|n| dups.contains(n))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    /// `H.nodes.filterby(stat, val, mode="eq")` (views.py:181-288). `"degree"` is fast-pathed
    /// via the core; any other stat name falls back to a plain node-attr lookup. `mode` accepts
    /// a string or `mode(value, val) -> bool` callable.
    #[pyo3(signature = (stat, val, mode=None))]
    fn filterby(
        &self,
        py: Python<'_>,
        stat: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyNodeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        if stat == "degree" {
            if let Ok(s) = mode_obj.cast::<PyString>() {
                if let Ok(m) = parse_mode(&s.to_string_lossy()) {
                    let target: usize = val.extract()?;
                    let hg = self.hg.borrow(py);
                    let full = hg.inner.nodes();
                    let filtered_ids: Vec<String> = full.filterby_degree(target, m).ids().to_vec();
                    let ids: Vec<String> = filtered_ids
                        .into_iter()
                        .filter(|i| self.effective_ids(&hg).contains(i))
                        .collect();
                    drop(hg);
                    return Ok(self.sub(py, ids));
                }
            }
        }
        let hg = self.hg.borrow(py);
        let mut ids = Vec::new();
        for nid in self.effective_ids(&hg) {
            let pv: Bound<'_, PyAny> = if stat == "degree" {
                (hg.inner.node_degree(&nid, None).unwrap_or(0) as i64)
                    .into_pyobject(py)?
                    .into_any()
            } else {
                match hg.inner.node_attrs(&nid).and_then(|a| a.get(stat)).cloned() {
                    Some(v) => json_to_py(py, &v)?,
                    None => continue,
                }
            };
            if eval_mode(&mode_obj, &pv, &val)? {
                ids.push(nid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// `H.nodes.filterby_attr(attr, val, mode="eq", missing=None)` (views.py:289-364).
    #[pyo3(signature = (attr, val, mode=None, missing=None))]
    fn filterby_attr(
        &self,
        py: Python<'_>,
        attr: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
        missing: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyNodeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let hg = self.hg.borrow(py);
        let mut ids = Vec::new();
        for nid in self.effective_ids(&hg) {
            let v = hg.inner.node_attrs(&nid).and_then(|a| a.get(attr)).cloned();
            let pv = match v {
                Some(v) => json_to_py(py, &v)?,
                None => match &missing {
                    Some(m) => m.clone(),
                    None => continue,
                },
            };
            if eval_mode(&mode_obj, &pv, &val)? {
                ids.push(nid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }
}

fn reprs_of(py: Python<'_>, ids: crate::pyid::PyIdMap, cores: &[String]) -> PyResult<Vec<String>> {
    let mut out: Vec<String> = cores
        .iter()
        .filter_map(|c| ids.get_obj(py, c))
        .filter_map(|o| o.repr().ok())
        .map(|r| r.to_string())
        .collect();
    out.sort();
    Ok(out)
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

    fn dtype_is_valid(py: Python<'_>, dtype: Option<&Bound<'_, PyAny>>) -> PyResult<bool> {
        match dtype {
            None => Ok(false),
            Some(d) => {
                if d.is(py.get_type::<PyDict>()) {
                    Ok(true)
                } else if d.is(py.get_type::<PyList>()) {
                    Ok(false)
                } else {
                    Err(crate::errors::xgi_error(format!(
                        "Unrecognized dtype {}",
                        d.repr()?
                    )))
                }
            }
        }
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

    fn __bool__(&self, py: Python<'_>) -> bool {
        self.__len__(py) > 0
    }

    /// `H.edges == H2.edges` — id-set equality (see `PyNodeView::__eq__`).
    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(other_view) = other.extract::<PyRef<PyEdgeView>>() else {
            return Ok(false);
        };
        let hg1 = self.hg.borrow(py);
        let ids1 = hg1.edge_ids.clone_via_py(py);
        let a = reprs_of(py, ids1, &self.effective_ids(&hg1))?;
        drop(hg1);
        let hg2 = other_view.hg.borrow(py);
        let ids2 = hg2.edge_ids.clone_via_py(py);
        let b = reprs_of(py, ids2, &other_view.effective_ids(&hg2))?;
        Ok(a == b)
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

    /// `H.edges(bunch)` — see `PyNodeView::__call__`.
    fn __call__(&self, py: Python<'_>, bunch: &Bound<'_, PyAny>) -> PyResult<PyEdgeView> {
        let hg = self.hg.borrow(py);
        let mut wanted: Vec<String> = Vec::new();
        for item in bunch.try_iter()? {
            let item = item?;
            let repr = item.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = hg
                .edge_ids
                .get_key(&item)?
                .ok_or_else(|| id_not_found(&repr))?;
            wanted.push(core);
        }
        let ids: Vec<String> = hg
            .inner
            .edge_ids()
            .into_iter()
            .filter(|i| wanted.contains(i))
            .collect();
        drop(hg);
        Ok(self.sub(py, ids))
    }

    #[pyo3(signature = (view, bunch=None))]
    // XGI API-surface constraint: `from_view` is a deliberate port of XGI's Python-facing
    // classmethod name (views.py:512-545); renaming would break conformance.
    #[allow(clippy::wrong_self_convention)]
    fn from_view(
        &self,
        py: Python<'_>,
        view: PyRef<'_, PyEdgeView>,
        bunch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyEdgeView> {
        match bunch {
            Some(b) => view.__call__(py, b),
            None => Ok(PyEdgeView {
                hg: view.hg.clone_ref(py),
                ids: None,
            }),
        }
    }

    #[getter]
    fn ids(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let set = PySet::empty(py)?;
        for c in self.effective_ids(&hg) {
            if let Some(o) = hg.edge_ids.get_obj(py, &c) {
                set.add(o)?;
            }
        }
        Ok(set.unbind())
    }

    fn __sub__(&self, py: Python<'_>, other: PyRef<'_, PyEdgeView>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let ob: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let ids: Vec<String> = self
            .effective_ids(&hg)
            .into_iter()
            .filter(|i| !ob.contains(i))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    fn __and__(&self, py: Python<'_>, other: PyRef<'_, PyEdgeView>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let ob: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let ids: Vec<String> = self
            .effective_ids(&hg)
            .into_iter()
            .filter(|i| ob.contains(i))
            .collect();
        drop(hg);
        self.sub(py, ids)
    }

    fn __or__(&self, py: Python<'_>, other: PyRef<'_, PyEdgeView>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let mut ids = self.effective_ids(&hg);
        let existing: std::collections::HashSet<String> = ids.iter().cloned().collect();
        for i in other.effective_ids(&hg) {
            if !existing.contains(&i) {
                ids.push(i);
            }
        }
        drop(hg);
        self.sub(py, ids)
    }

    fn __xor__(&self, py: Python<'_>, other: PyRef<'_, PyEdgeView>) -> PyEdgeView {
        let hg = self.hg.borrow(py);
        let a: std::collections::HashSet<String> = self.effective_ids(&hg).into_iter().collect();
        let b: std::collections::HashSet<String> = other.effective_ids(&hg).into_iter().collect();
        let mut ids: Vec<String> = hg.inner.edge_ids();
        ids.retain(|i| a.contains(i) ^ b.contains(i));
        drop(hg);
        self.sub(py, ids)
    }

    /// `H.edges.neighbors(e, s=1)` — edges sharing at least `s` members with `e`.
    #[pyo3(signature = (e, s=1))]
    fn neighbors(&self, py: Python<'_>, e: &Bound<'_, PyAny>, s: usize) -> PyResult<Py<PySet>> {
        let hg = self.hg.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = hg.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
        let members: std::collections::HashSet<String> = hg
            .inner
            .members(&core)
            .unwrap_or_default()
            .into_iter()
            .collect();
        let set = PySet::empty(py)?;
        for other in hg.inner.edge_ids() {
            if other == core {
                continue;
            }
            let om: std::collections::HashSet<String> = hg
                .inner
                .members(&other)
                .unwrap_or_default()
                .into_iter()
                .collect();
            if members.intersection(&om).count() >= s {
                if let Some(o) = hg.edge_ids.get_obj(py, &other) {
                    set.add(o)?;
                }
            }
        }
        Ok(set.unbind())
    }

    /// `H.edges.lookup(members)` — edges whose exact member-set equals `members`.
    fn lookup(&self, py: Python<'_>, members: &Bound<'_, PyAny>) -> PyResult<PyEdgeView> {
        let hg = self.hg.borrow(py);
        let mut sought: Vec<String> = Vec::new();
        for m in members.try_iter()? {
            let m = m?;
            match hg.node_ids.get_key(&m)? {
                Some(c) => sought.push(c),
                None => {
                    drop(hg);
                    return Ok(self.sub(py, Vec::new()));
                }
            }
        }
        sought.sort();
        sought.dedup();
        let mut ids = Vec::new();
        for eid in self.effective_ids(&hg) {
            let mut mem = hg.inner.members(&eid).unwrap_or_default();
            mem.sort();
            mem.dedup();
            if mem == sought {
                ids.push(eid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// `H.edges.members(e=None, dtype=list)` (views.py:706-745).
    #[pyo3(signature = (e=None, dtype=None))]
    fn members(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let hg = self.hg.borrow(py);
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = hg.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                if !self.effective_ids(&hg).contains(&core) {
                    return Err(id_not_found(&repr));
                }
                let set = PySet::empty(py)?;
                for n in hg.inner.members(&core).unwrap_or_default() {
                    if let Some(o) = hg.node_ids.get_obj(py, &n) {
                        set.add(o)?;
                    }
                }
                Ok(set.into_any().unbind())
            }
            None => {
                let is_dict = Self::dtype_is_valid(py, dtype)?;
                if is_dict {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&hg) {
                        let Some(obj) = hg.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        let set = PySet::empty(py)?;
                        for n in hg.inner.members(&eid).unwrap_or_default() {
                            if let Some(o) = hg.node_ids.get_obj(py, &n) {
                                set.add(o)?;
                            }
                        }
                        d.set_item(obj, set)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&hg) {
                        let set = PySet::empty(py)?;
                        for n in hg.inner.members(&eid).unwrap_or_default() {
                            if let Some(o) = hg.node_ids.get_obj(py, &n) {
                                set.add(o)?;
                            }
                        }
                        list.append(set)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    #[getter]
    fn size(&self, py: Python<'_>) -> crate::stats::PyEdgeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyEdgeStat::size(py, self.hg.clone_ref(py), ids)
    }

    #[getter]
    fn order(&self, py: Python<'_>) -> crate::stats::PyEdgeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyEdgeStat::order(py, self.hg.clone_ref(py), ids)
    }

    /// `H.edges.attrs(attr=None)` — the general edge attribute `Stat` proxy.
    #[pyo3(signature = (attr=None))]
    fn attrs(&self, py: Python<'_>, attr: Option<String>) -> crate::stats::PyEdgeStat {
        let hg = self.hg.borrow(py);
        let ids = self.effective_ids(&hg);
        drop(hg);
        crate::stats::PyEdgeStat::attr(py, self.hg.clone_ref(py), ids, attr.unwrap_or_default())
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

    /// `H.edges.filterby(stat, val, mode="eq")` — `"size"`/`"order"` use the fast core path when
    /// `mode` is a plain string; any other stat name (or a callable `mode`) falls back to a
    /// generic edge-attr/size/order lookup.
    #[pyo3(signature = (stat, val, mode=None))]
    fn filterby(
        &self,
        py: Python<'_>,
        stat: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyEdgeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        if let Ok(s) = mode_obj.cast::<PyString>() {
            let mode_str = s.to_string_lossy().to_string();
            if stat == "size" {
                return self.filterby_size(py, val.extract()?, mode_str);
            }
            if stat == "order" {
                return self.filterby_order(py, val.extract()?, mode_str);
            }
        }
        let hg = self.hg.borrow(py);
        let mut ids = Vec::new();
        for eid in self.effective_ids(&hg) {
            let pv: Bound<'_, PyAny> = match stat {
                "size" => (hg.inner.edge_size(&eid).unwrap_or(0) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                "order" => ((hg.inner.edge_size(&eid).unwrap_or(0) as i64) - 1)
                    .into_pyobject(py)?
                    .into_any(),
                other => match hg
                    .inner
                    .edge_attrs(&eid)
                    .and_then(|a| a.get(other))
                    .cloned()
                {
                    Some(v) => json_to_py(py, &v)?,
                    None => continue,
                },
            };
            if eval_mode(&mode_obj, &pv, &val)? {
                ids.push(eid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// `H.edges.filterby_attr(attr, val, mode="eq", missing=None)`.
    #[pyo3(signature = (attr, val, mode=None, missing=None))]
    fn filterby_attr(
        &self,
        py: Python<'_>,
        attr: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
        missing: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyEdgeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let hg = self.hg.borrow(py);
        let mut ids = Vec::new();
        for eid in self.effective_ids(&hg) {
            let v = hg.inner.edge_attrs(&eid).and_then(|a| a.get(attr)).cloned();
            let pv = match v {
                Some(v) => json_to_py(py, &v)?,
                None => match &missing {
                    Some(m) => m.clone(),
                    None => continue,
                },
            };
            if eval_mode(&mode_obj, &pv, &val)? {
                ids.push(eid);
            }
        }
        drop(hg);
        Ok(self.sub(py, ids))
    }

    /// `H.edges.duplicate_groups()`: groups of edges with the same members. Like XGI, groups the
    /// whole network whatever this view filters, one view per group.
    fn duplicate_groups(&self, py: Python<'_>) -> Vec<PyEdgeView> {
        let groups = self.hg.borrow(py).inner.edges().duplicate_groups();
        groups.into_iter().map(|g| self.sub(py, g)).collect()
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

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyNodeView>()?;
    m.add_class::<PyEdgeView>()?;
    Ok(())
}
