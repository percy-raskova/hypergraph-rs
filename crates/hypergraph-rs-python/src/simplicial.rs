//! PyO3 binding for the core `SimplicialComplex` (Task 7) + the `subhypergraph` global view.
//!
//! XGI parity: `xgi.SimplicialComplex` (simplicialcomplex.py) — a `Hypergraph` variant closed
//! under faces; `add_edge`/`add_edges_from`/`remove_edge`/`remove_edges_from` are blocked (raise
//! `XGIError`); `add_simplex`/`add_simplices_from`/`close` are the mutators. `xgi.subhypergraph`
//! (globalviews.py:19) wraps `core::globalviews::subhypergraph`.
//!
//! Node/edge id auto-assignment for simplices closes over faces inside the core (`add_simplex`,
//! `close`) and does not report which ids it minted. The binding recovers the full post-call edge
//! id set (`self.inner.edge_ids()`) and registers any id not yet known to `self.edge_ids` — this
//! covers both the requested top simplex id (already interned before the call) and every
//! auto-assigned face id the core created underneath it.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyTuple};
use serde_json::Value;

use hypergraph_rs::SimplicialComplex as CoreSC;

use crate::errors::{id_not_found, xgi_error};
use crate::hypergraph::PyHypergraph;
use crate::pyid::{json_to_py, py_any_to_json, PyIdMap};

/// Auto-assigned core ids are numeric strings ("0", "1", …) — register them as Python `int`
/// (XGI auto-ids are ints), falling back to `str` for the (unreachable in practice) non-numeric case.
fn int_or_str_pyobj<'py>(py: Python<'py>, core_id: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(n) = core_id.parse::<i64>() {
        return Ok(n.into_pyobject(py)?.into_any());
    }
    Ok(core_id.into_pyobject(py)?.into_any())
}

// `unsendable`: the core `SimplicialComplex` caches its face lattice in a `std::cell::OnceCell`
// (interior mutability, not `Sync`) — pyo3 pyclasses are already GIL-single-threaded per instance,
// so this only opts out of pyo3's stricter (unneeded) cross-thread-Send guarantee.
#[pyclass(name = "SimplicialComplex", module = "hypergraph_rs", unsendable)]
pub struct PySimplicialComplex {
    pub inner: CoreSC<Value, Value, Value>,
    pub node_ids: PyIdMap,
    pub edge_ids: PyIdMap,
    pub net_attr: serde_json::Map<String, Value>,
}

#[pymethods]
impl PySimplicialComplex {
    #[new]
    #[pyo3(signature = (incoming_simplices=None, **attr))]
    fn new(
        py: Python<'_>,
        incoming_simplices: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut sc = Self {
            inner: CoreSC::new(),
            node_ids: PyIdMap::new(),
            edge_ids: PyIdMap::new(),
            net_attr: serde_json::Map::new(),
        };
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                sc.net_attr
                    .insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        if let Some(data) = incoming_simplices {
            sc.ingest(py, &data)?;
        }
        Ok(sc)
    }

    #[getter]
    fn num_nodes(&self) -> usize {
        self.inner.num_nodes()
    }
    #[getter]
    fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    fn has_node(&self, n: &Bound<'_, PyAny>) -> bool {
        matches!(self.node_ids.get_key(n), Ok(Some(_)))
    }

    fn __contains__(&self, n: &Bound<'_, PyAny>) -> bool {
        self.has_node(n)
    }

    fn __len__(&self) -> usize {
        self.inner.num_nodes()
    }

    /// `for n in S` iterates NODES (XGI parity: `Hypergraph.__iter__`, inherited unchanged).
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let items: Vec<Py<PyAny>> = self
            .inner
            .node_ids()
            .iter()
            .filter_map(|c| self.node_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    /// `S["name"]` — network-level attr; missing -> XGIError.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.net_attr.get(key) {
            Some(v) => Ok(json_to_py(py, v)?.unbind()),
            None => Err(xgi_error(format!("{key} is not a network attribute"))),
        }
    }

    fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.net_attr
            .insert(key.to_string(), py_any_to_json(value)?);
        Ok(())
    }

    #[getter]
    fn _net_attr(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.net_attr {
            d.set_item(k, json_to_py(py, v)?)?;
        }
        Ok(d.unbind())
    }

    /// `S._edge` — `{edge_id: {member_ids...}}` (XGI's `_edge` values are `frozenset`s; a plain
    /// `set` compares equal to a `frozenset` with the same elements, so a `set` suffices here).
    #[getter]
    fn _edge(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for eid in self.inner.edge_ids() {
            let Some(obj) = self.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            let set = PySet::empty(py)?;
            for nid in self.inner.members(&eid).unwrap_or_default() {
                if let Some(o) = self.node_ids.get_obj(py, &nid) {
                    set.add(o)?;
                }
            }
            d.set_item(obj, set)?;
        }
        Ok(d.unbind())
    }

    fn __str__(&self, py: Python<'_>) -> PyResult<String> {
        let cls = "SimplicialComplex";
        match self.net_attr.get("name") {
            Some(v) => {
                let s = json_to_py(py, v)?.str()?.to_string();
                Ok(format!(
                    "{cls} named '{s}' with {} nodes and {} simplices",
                    self.inner.num_nodes(),
                    self.inner.num_edges()
                ))
            }
            None => Ok(format!(
                "Unnamed {cls} with {} nodes and {} simplices",
                self.inner.num_nodes(),
                self.inner.num_edges()
            )),
        }
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        let Ok(o) = other.extract::<PyRef<PySimplicialComplex>>() else {
            return false;
        };
        let mut n1 = self.inner.node_ids();
        n1.sort();
        let mut n2 = o.inner.node_ids();
        n2.sort();
        if n1 != n2 {
            return false;
        }
        let mut e1 = self.inner.edge_ids();
        e1.sort();
        let mut e2 = o.inner.edge_ids();
        e2.sort();
        if e1 != e2 {
            return false;
        }
        for eid in &e1 {
            let mut m1 = self.inner.members(eid).unwrap_or_default();
            m1.sort();
            let mut m2 = o.inner.members(eid).unwrap_or_default();
            m2.sort();
            if m1 != m2 {
                return false;
            }
        }
        self.net_attr == o.net_attr
    }

    #[getter]
    fn is_frozen(&self) -> bool {
        self.inner.is_frozen()
    }

    fn freeze(&mut self) {
        self.inner.freeze();
    }

    fn copy(&self, py: Python<'_>) -> PySimplicialComplex {
        PySimplicialComplex {
            inner: self.inner.copy(),
            node_ids: self.node_ids.clone_via_py(py),
            edge_ids: self.edge_ids.clone_via_py(py),
            net_attr: self.net_attr.clone(),
        }
    }

    fn __copy__(&self, py: Python<'_>) -> PySimplicialComplex {
        self.copy(py)
    }

    #[pyo3(signature = (_memo=None))]
    fn __deepcopy__(&self, py: Python<'_>, _memo: Option<Bound<'_, PyAny>>) -> PySimplicialComplex {
        self.copy(py)
    }

    /// `S.add_node(n, **attr)` — merges the incoming dict into any existing attrs (Task 4 D6
    /// pattern, mirrored from `PyHypergraph::add_node`).
    #[pyo3(signature = (n, **attr))]
    fn add_node(&mut self, n: &Bound<'_, PyAny>, attr: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        self.frozen_check()?;
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

    /// `S.add_nodes_from(nodes_for_adding, **attr)`.
    #[pyo3(signature = (nodes_for_adding, **attr))]
    fn add_nodes_from(
        &mut self,
        nodes_for_adding: Bound<'_, PyAny>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        self.frozen_check()?;
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
        Ok(())
    }

    /// `S.remove_simplex_id(idx)`.
    fn remove_simplex_id(&mut self, idx: &Bound<'_, PyAny>) -> PyResult<()> {
        self.frozen_check()?;
        let repr = idx.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = self
            .edge_ids
            .get_key(idx)?
            .ok_or_else(|| id_not_found(&repr))?;
        self.inner
            .remove_simplex_id(&core)
            .map_err(|e| xgi_error(format!("{e:?}")))?;
        self.edge_ids.forget(&core);
        Ok(())
    }

    /// `S.remove_simplex_ids_from(ids)`.
    fn remove_simplex_ids_from(&mut self, ids: &Bound<'_, PyAny>) -> PyResult<()> {
        self.frozen_check()?;
        let mut cores = Vec::new();
        for id in ids.try_iter()? {
            let id = id?;
            let repr = id.repr().map(|r| r.to_string()).unwrap_or_default();
            cores.push(
                self.edge_ids
                    .get_key(&id)?
                    .ok_or_else(|| id_not_found(&repr))?,
            );
        }
        self.inner.remove_simplex_ids_from(&cores);
        for c in &cores {
            self.edge_ids.forget(c);
        }
        Ok(())
    }

    /// `S.add_edge(...)` is not available on a simplicial complex.
    /// XGI parity: raises `XGIError` (simplicialcomplex.py:135-139).
    #[pyo3(signature = (*_args, **_kwargs))]
    fn add_edge(
        &self,
        _args: &Bound<'_, PyTuple>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        Err(xgi_error(
            "Cannot add_edge to a SimplicialComplex; use add_simplex/add_simplices_from",
        ))
    }

    /// `S.remove_edge(...)` is not available on a simplicial complex.
    /// XGI parity: raises `XGIError` (simplicialcomplex.py:153-157).
    #[pyo3(signature = (*_args, **_kwargs))]
    fn remove_edge(
        &self,
        _args: &Bound<'_, PyTuple>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        Err(xgi_error(
            "Cannot remove_edge from a SimplicialComplex; use remove_simplex_id",
        ))
    }

    /// `S.has_simplex(members)` — unknown members (never interned) cannot match, so this is a
    /// read-only lookup (`get_key`, not `intern`).
    fn has_simplex(&self, members: Bound<'_, PyAny>) -> PyResult<bool> {
        let mut core_members = Vec::new();
        for m in members.try_iter()? {
            match self.node_ids.get_key(&m?)? {
                Some(c) => core_members.push(c),
                None => return Ok(false),
            }
        }
        Ok(self.inner.has_simplex(&core_members))
    }

    /// `S.add_simplex(members, idx=None, **attr)` — closes under faces (core `add_simplex`).
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_simplex(
        &mut self,
        py: Python<'_>,
        members: Bound<'_, PyAny>,
        idx: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        self.add_simplex_impl(py, &members, idx.as_ref(), attr.as_ref())
    }

    /// `S.add_simplices_from(ebunch, **attr)` — Formats 1-5, mirroring
    /// `PyHypergraph::add_edges_from`'s format-detection exactly (a simplex's `members` are a flat
    /// node set, same shape as an undirected edge's).
    #[pyo3(signature = (ebunch_to_add, **attr))]
    fn add_simplices_from(
        &mut self,
        py: Python<'_>,
        ebunch_to_add: Bound<'_, PyAny>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let mut base_attr = serde_json::Map::new();
        if let Some(a) = &attr {
            for (k, v) in a.iter() {
                base_attr.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }

        if let Ok(dict) = ebunch_to_add.cast::<PyDict>() {
            for (idx, members) in dict.iter() {
                self.add_simplex_impl_attrs(py, &members, Some(&idx), base_attr.clone())?;
            }
            return Ok(());
        }

        let mut iter = ebunch_to_add.try_iter()?;
        let Some(first_res) = iter.next() else {
            return Ok(());
        };
        let first = first_res?;

        let elem: Option<Bound<PyAny>> = match first.try_iter() {
            Ok(mut it) => match it.next() {
                Some(r) => Some(r?),
                None => None,
            },
            Err(_) => None,
        };
        let elem_is_iterable = match &elem {
            Some(e) => e.try_iter().is_ok(),
            None => false,
        };

        #[derive(Clone, Copy, PartialEq)]
        enum Fmt {
            One,
            Two,
            Three,
            Four,
        }
        let fmt = if elem_is_iterable {
            let mut all_str = true;
            for it in first.try_iter()? {
                if it?.cast::<pyo3::types::PyString>().is_err() {
                    all_str = false;
                    break;
                }
            }
            if all_str {
                Fmt::One
            } else {
                let len = first.len()?;
                if len == 2 {
                    let second = first.get_item(1)?;
                    if second.cast::<PyDict>().is_ok() {
                        Fmt::Three
                    } else {
                        Fmt::Two
                    }
                } else if len == 3 {
                    Fmt::Four
                } else {
                    return Err(xgi_error("Invalid ebunch format"));
                }
            }
        } else {
            Fmt::One
        };

        let elem_is_str = elem
            .as_ref()
            .map(|e| e.cast::<pyo3::types::PyString>().is_ok())
            .unwrap_or(false);
        if (fmt == Fmt::One && first.cast::<pyo3::types::PyString>().is_ok())
            || (fmt != Fmt::One && elem_is_str)
        {
            return Err(xgi_error("Members cannot be specified as a string"));
        }

        let mut current = first;
        loop {
            let (members, idx, extra_attrs): (
                Bound<PyAny>,
                Option<Bound<PyAny>>,
                serde_json::Map<String, Value>,
            ) = match fmt {
                Fmt::One => (current.clone(), None, serde_json::Map::new()),
                Fmt::Two => (
                    current.get_item(0)?,
                    Some(current.get_item(1)?),
                    serde_json::Map::new(),
                ),
                Fmt::Three => {
                    let d = current
                        .get_item(1)?
                        .cast_into::<PyDict>()
                        .map_err(|_| xgi_error("Invalid ebunch format"))?;
                    let mut m = serde_json::Map::new();
                    for (k, v) in d.iter() {
                        m.insert(k.str()?.to_string(), py_any_to_json(&v)?);
                    }
                    (current.get_item(0)?, None, m)
                }
                Fmt::Four => {
                    let d = current
                        .get_item(2)?
                        .cast_into::<PyDict>()
                        .map_err(|_| xgi_error("Invalid ebunch format"))?;
                    let mut m = serde_json::Map::new();
                    for (k, v) in d.iter() {
                        m.insert(k.str()?.to_string(), py_any_to_json(&v)?);
                    }
                    (current.get_item(0)?, Some(current.get_item(1)?), m)
                }
            };
            let mut merged = base_attr.clone();
            for (k, v) in extra_attrs {
                merged.insert(k, v);
            }
            self.add_simplex_impl_attrs(py, &members, idx.as_ref(), merged)?;

            match iter.next() {
                Some(r) => current = r?,
                None => break,
            }
        }
        Ok(())
    }

    /// `S.close()` — adds every missing subface of every existing simplex.
    fn close(&mut self, py: Python<'_>) -> PyResult<()> {
        self.inner.close();
        self.register_new_edge_ids(py)?;
        Ok(())
    }

    /// `S.nodes` — the `NodeView` proxy (mirrors Task 5's `PyNodeView`, scoped to this complex).
    #[getter]
    fn nodes(slf: Py<Self>) -> PySCNodeView {
        PySCNodeView { sc: slf }
    }

    /// `S.edges` — the `EdgeView` proxy (mirrors Task 5's `PyEdgeView`, scoped to this complex).
    #[getter]
    fn edges(slf: Py<Self>) -> PySCEdgeView {
        PySCEdgeView { sc: slf, ids: None }
    }
}

impl PySimplicialComplex {
    /// The core's `assert_not_frozen()` PANICS rather than returning a `Result` — check here
    /// first so a frozen-complex mutation raises `XGIError` instead of crashing the process
    /// (same pattern as `PyHypergraph::frozen_check`).
    fn frozen_check(&self) -> PyResult<()> {
        if self.inner.is_frozen() {
            Err(xgi_error("Frozen higher-order network can't be modified"))
        } else {
            Ok(())
        }
    }

    /// Build a `PySimplicialComplex` from a core complex whose String node/edge keys are
    /// decimal integers (as produced by `random_simplicial_complex` — Task 10) — registers
    /// them as Python `int`s, preserving insertion order.
    pub fn from_core_with_int_ids(
        py: Python<'_>,
        core: CoreSC<Value, Value, Value>,
    ) -> PyResult<Self> {
        let mut node_ids = PyIdMap::new();
        for nid in core.node_ids() {
            node_ids.intern(&int_or_str_pyobj(py, &nid)?)?;
        }
        let mut edge_ids = PyIdMap::new();
        for eid in core.edge_ids() {
            edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
        }
        Ok(Self {
            inner: core,
            node_ids,
            edge_ids,
            net_attr: serde_json::Map::new(),
        })
    }

    /// Accepts a simplex list, a `{idx: members}` dict, another `SimplicialComplex`, or a
    /// `Hypergraph` (each of the hypergraph's edges becomes a simplex, closing under faces —
    /// XGI parity: `convert.to_simplicial_complex`). numpy/pandas inputs are a documented
    /// exclusion (matches `PyHypergraph::ingest`'s deviation).
    fn ingest(&mut self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(list) = data.cast::<PyList>() {
            for simplex in list.iter() {
                self.add_simplex_impl(py, &simplex, None, None)?;
            }
            return Ok(());
        }
        if let Ok(dict) = data.cast::<PyDict>() {
            for (idx, members) in dict.iter() {
                self.add_simplex_impl(py, &members, Some(&idx), None)?;
            }
            return Ok(());
        }
        if let Ok(other) = data.extract::<PyRef<PySimplicialComplex>>() {
            self.inner = other.inner.copy();
            self.node_ids = other.node_ids.clone_via_py(py);
            self.edge_ids = other.edge_ids.clone_via_py(py);
            self.net_attr = other.net_attr.clone();
            return Ok(());
        }
        if let Ok(hg) = data.extract::<PyRef<PyHypergraph>>() {
            for eid in hg.inner.edge_ids() {
                let member_objs: Vec<Bound<PyAny>> = hg
                    .inner
                    .members(&eid)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|nid| hg.node_ids.get_obj(py, nid))
                    .collect();
                let members_list = PyList::new(py, &member_objs)?;
                self.add_simplex_impl(py, members_list.as_any(), None, None)?;
            }
            self.net_attr = hg.net_attr.clone();
            return Ok(());
        }
        Err(xgi_error(
            "could not interpret data as a simplicial complex",
        ))
    }

    fn add_simplex_impl(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        attr: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let mut e_attrs = serde_json::Map::new();
        if let Some(a) = attr {
            for (k, v) in a.iter() {
                e_attrs.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        self.add_simplex_impl_attrs2(py, members, idx, e_attrs, false)
    }

    /// `add_simplices_from`'s per-item entry point: `attr` is already a merged JSON map (base
    /// kwargs + per-item attrs), and the duplicate-uid warning gets the trailing period XGI's
    /// `add_simplices_from` uses (vs. `add_simplex`'s, which omits it).
    fn add_simplex_impl_attrs(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        attr: serde_json::Map<String, Value>,
    ) -> PyResult<()> {
        self.add_simplex_impl_attrs2(py, members, idx, attr, true)
    }

    fn add_simplex_impl_attrs2(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        e_attrs: serde_json::Map<String, Value>,
        warn_with_period: bool,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let mut member_objs: Vec<Bound<PyAny>> = Vec::new();
        match members.try_iter() {
            Ok(it) => {
                for m in it {
                    member_objs.push(m?);
                }
            }
            Err(_) => return Err(xgi_error("Invalid ebunch format")),
        }
        let mut core_members = Vec::new();
        for m in &member_objs {
            core_members.push(self.node_ids.intern(m)?);
        }

        let requested_core = match idx {
            Some(o) => {
                if self.edge_ids.get_key(o)?.is_some() {
                    // XGI parity: duplicate simplex uid -> warn + no-op
                    // (simplicialcomplex.py:314: "uid {idx} already exists, cannot add simplex {members}").
                    let set = PySet::new(py, &member_objs)?;
                    let msg = if warn_with_period {
                        format!(
                            "uid {} already exists, cannot add simplex {}.",
                            o.str()?,
                            set.repr()?
                        )
                    } else {
                        format!(
                            "uid {} already exists, cannot add simplex {}",
                            o.str()?,
                            set.repr()?
                        )
                    };
                    let warnings = py.import("warnings")?;
                    warnings.call_method1(
                        "warn",
                        (msg, py.get_type::<pyo3::exceptions::PyUserWarning>()),
                    )?;
                    return Ok(());
                }
                Some(self.edge_ids.intern(o)?)
            }
            None => None,
        };

        self.inner
            .add_simplex(core_members, requested_core, Value::Object(e_attrs))
            .map_err(|e| xgi_error(format!("{e:?}")))?;
        self.register_new_edge_ids(py)
    }

    /// Register every core edge id not yet known to `self.edge_ids` as a fresh Python `int` id —
    /// covers auto-assigned top-simplex ids and every auto-assigned face id `add_simplex`/`close`
    /// minted underneath (the core does not report which ids it created).
    fn register_new_edge_ids(&mut self, py: Python<'_>) -> PyResult<()> {
        for eid in self.inner.edge_ids() {
            if self.edge_ids.get_obj(py, &eid).is_none() {
                self.edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// NodeView / EdgeView (scoped to a SimplicialComplex)
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "SCNodeView", module = "hypergraph_rs")]
pub struct PySCNodeView {
    pub sc: Py<PySimplicialComplex>,
}

#[pymethods]
impl PySCNodeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        self.sc.borrow(py).inner.num_nodes()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let sc = self.sc.borrow(py);
        let items: Vec<Py<PyAny>> = sc
            .inner
            .node_ids()
            .iter()
            .filter_map(|c| sc.node_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(sc);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> bool {
        matches!(self.sc.borrow(py).node_ids.get_key(n), Ok(Some(_)))
    }

    /// Sub-view of nodes that duplicate an earlier one (same edge-membership set).
    fn duplicates(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let sc = self.sc.borrow(py);
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for nid in sc.inner.node_ids() {
            let mut memberships: Vec<String> = sc
                .inner
                .edge_ids()
                .into_iter()
                .filter(|eid| sc.inner.members(eid).unwrap_or_default().contains(&nid))
                .collect();
            memberships.sort();
            groups.entry(memberships).or_default().push(nid);
        }
        let mut dups: Vec<String> = Vec::new();
        for group in groups.values() {
            if group.len() > 1 {
                let mut sorted = group.clone();
                sorted.sort();
                dups.extend(sorted.into_iter().skip(1));
            }
        }
        let list = PyList::empty(py);
        for nid in dups {
            if let Some(obj) = sc.node_ids.get_obj(py, &nid) {
                list.append(obj)?;
            }
        }
        Ok(list.unbind())
    }
}

#[pyclass(name = "SCEdgeView", module = "hypergraph_rs")]
pub struct PySCEdgeView {
    pub sc: Py<PySimplicialComplex>,
    pub ids: Option<Vec<String>>,
}

impl PySCEdgeView {
    fn effective_ids(&self, sc: &PySimplicialComplex) -> Vec<String> {
        self.ids.clone().unwrap_or_else(|| sc.inner.edge_ids())
    }
}

#[pymethods]
impl PySCEdgeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        let sc = self.sc.borrow(py);
        self.effective_ids(&sc).len()
    }

    /// `S.edges(bunch)` — a sub-view scoped to `bunch`, ordered by the global edge order.
    fn __call__(&self, py: Python<'_>, bunch: &Bound<'_, PyAny>) -> PyResult<PySCEdgeView> {
        let sc = self.sc.borrow(py);
        let mut wanted: Vec<String> = Vec::new();
        for item in bunch.try_iter()? {
            let item = item?;
            let repr = item.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = sc
                .edge_ids
                .get_key(&item)?
                .ok_or_else(|| id_not_found(&repr))?;
            wanted.push(core);
        }
        let ids: Vec<String> = sc
            .inner
            .edge_ids()
            .into_iter()
            .filter(|i| wanted.contains(i))
            .collect();
        Ok(PySCEdgeView {
            sc: self.sc.clone_ref(py),
            ids: Some(ids),
        })
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let sc = self.sc.borrow(py);
        let items: Vec<Py<PyAny>> = self
            .effective_ids(&sc)
            .iter()
            .filter_map(|c| sc.edge_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(sc);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> bool {
        let sc = self.sc.borrow(py);
        match sc.edge_ids.get_key(e) {
            Ok(Some(core)) => self.effective_ids(&sc).contains(&core),
            _ => false,
        }
    }

    /// `S.edges.members(e=None, dtype=list)` — node ids of simplex `e` (a SET), or all simplices
    /// as a list/dict of sets when `e` is omitted.
    #[pyo3(signature = (e=None, dtype=None))]
    fn members(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let sc = self.sc.borrow(py);
        // XGI parity: SimplicialComplex simplex-member sets are `frozenset`s (immutable, hashable
        // — `set(S.edges.members())` must work), unlike Hypergraph's mutable-`set` edges.
        let set_for = |py: Python<'_>, core: &str| -> PyResult<Py<pyo3::types::PyFrozenSet>> {
            let members: Vec<Py<PyAny>> = sc
                .inner
                .members(core)
                .unwrap_or_default()
                .iter()
                .filter_map(|nid| sc.node_ids.get_obj(py, nid).map(|o| o.unbind()))
                .collect();
            Ok(pyo3::types::PyFrozenSet::new(py, &members)?.unbind())
        };
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = sc.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                if !self.effective_ids(&sc).contains(&core) {
                    return Err(id_not_found(&repr));
                }
                Ok(set_for(py, &core)?.into_any())
            }
            None => {
                let is_dict = match dtype {
                    Some(d) => d.is(&py.get_type::<PyDict>()),
                    None => false,
                };
                if is_dict {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&sc) {
                        let Some(obj) = sc.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        d.set_item(obj, set_for(py, &eid)?)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&sc) {
                        list.append(set_for(py, &eid)?)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    /// Sub-view of edges that duplicate an earlier one (same member-set).
    fn duplicates(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let sc = self.sc.borrow(py);
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for eid in sc.inner.edge_ids() {
            let mut mem = sc.inner.members(&eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid);
        }
        let mut dups: Vec<String> = Vec::new();
        for group in groups.values() {
            if group.len() > 1 {
                let mut sorted = group.clone();
                sorted.sort();
                dups.extend(sorted.into_iter().skip(1));
            }
        }
        let list = PyList::empty(py);
        for eid in dups {
            if let Some(obj) = sc.edge_ids.get_obj(py, &eid) {
                list.append(obj)?;
            }
        }
        Ok(list.unbind())
    }

    /// Sub-view of maximal simplices (not a face of any other simplex).
    #[pyo3(signature = (strict=true))]
    fn maximal(&self, py: Python<'_>, strict: bool) -> PyResult<Py<PyList>> {
        let sc = self.sc.borrow(py);
        let mut member_sets: Vec<(String, std::collections::BTreeSet<String>)> = sc
            .inner
            .edge_ids()
            .into_iter()
            .map(|eid| {
                let m: std::collections::BTreeSet<String> = sc
                    .inner
                    .members(&eid)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                (eid, m)
            })
            .collect();
        member_sets.sort_by(|a, b| a.0.cmp(&b.0));
        let ids: Vec<String> = member_sets
            .iter()
            .filter(|(eid, m)| {
                !member_sets.iter().any(|(other_eid, other_m)| {
                    if other_eid == eid {
                        return false;
                    }
                    let is_subset = m.is_subset(other_m);
                    if strict {
                        is_subset && m != other_m
                    } else {
                        is_subset
                    }
                })
            })
            .map(|(eid, _)| eid.clone())
            .collect();
        let list = PyList::empty(py);
        for eid in sc.inner.edge_ids() {
            if ids.contains(&eid) {
                if let Some(obj) = sc.edge_ids.get_obj(py, &eid) {
                    list.append(obj)?;
                }
            }
        }
        Ok(list.unbind())
    }

    /// `S.edges[e]` -> attribute dict.
    fn __getitem__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let sc = self.sc.borrow(py);
        let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
        let core = sc
            .edge_ids
            .get_key(e)?
            .ok_or_else(|| crate::errors::id_not_found(&repr))?;
        let d = PyDict::new(py);
        if let Some(Value::Object(m)) = sc.inner.edge_attrs(&core).cloned() {
            for (k, v) in m {
                d.set_item(k, json_to_py(py, &v)?)?;
            }
        }
        Ok(d.unbind())
    }
}

// ---------------------------------------------------------------------------------------------
// subhypergraph (module-level global view; works over PyHypergraph)
// ---------------------------------------------------------------------------------------------

/// `xgi.subhypergraph(H, nodes=None, edges=None, keep_isolates=True)`.
/// XGI parity: globalviews.py:19-74. Unknown ids in `nodes`/`edges` are simply not present in the
/// selection (never interned — read-only, matches `has_node`/`has_edge` id lookups elsewhere).
#[pyfunction]
#[pyo3(signature = (h, nodes=None, edges=None, keep_isolates=true))]
fn subhypergraph(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    nodes: Option<Bound<'_, PyAny>>,
    edges: Option<Bound<'_, PyAny>>,
    keep_isolates: bool,
) -> PyResult<PyHypergraph> {
    let node_core: Option<Vec<String>> = match nodes {
        Some(ns) => {
            let mut v = Vec::new();
            for n in ns.try_iter()? {
                if let Some(c) = h.node_ids.get_key(&n?)? {
                    v.push(c);
                }
            }
            Some(v)
        }
        None => None,
    };
    let edge_core: Option<Vec<String>> = match edges {
        Some(es) => {
            let mut v = Vec::new();
            for e in es.try_iter()? {
                if let Some(c) = h.edge_ids.get_key(&e?)? {
                    v.push(c);
                }
            }
            Some(v)
        }
        None => None,
    };

    let new_inner = hypergraph_rs::subhypergraph(
        &h.inner,
        node_core.as_deref(),
        edge_core.as_deref(),
        keep_isolates,
    );

    // Rebuild id maps over the retained ids only, reusing the parent's Python objects
    // (so `nodes`/`edges` on the sub-hypergraph still yield the original ints/strings/tuples).
    let mut node_ids = PyIdMap::new();
    for nid in new_inner.node_ids() {
        if let Some(obj) = h.node_ids.get_obj(py, &nid) {
            node_ids.intern(&obj)?;
        }
    }
    let mut edge_ids = PyIdMap::new();
    for eid in new_inner.edge_ids() {
        if let Some(obj) = h.edge_ids.get_obj(py, &eid) {
            edge_ids.intern(&obj)?;
        }
    }

    Ok(PyHypergraph {
        inner: new_inner,
        node_ids,
        edge_ids,
        net_attr: h.net_attr.clone(),
    })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimplicialComplex>()?;
    m.add_class::<PySCNodeView>()?;
    m.add_class::<PySCEdgeView>()?;
    m.add_function(pyo3::wrap_pyfunction!(subhypergraph, m)?)?;
    Ok(())
}
