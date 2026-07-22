//! PyO3 binding for the core `Hypergraph`. Deviation shims (spec §3.1) live here.

use hypergraph_rs::algorithms::connected::largest_connected_component;
use hypergraph_rs::core::error::EdgeError;
use hypergraph_rs::layout::{BipartiteAlign, LayoutStrategy};
use hypergraph_rs::Hypergraph as CoreHg;
use hypergraph_rs::{MembershipError, SwapError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyString, PyTuple};
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

fn warn_user(py: Python<'_>, msg: &str) -> PyResult<()> {
    py.import("warnings")?.call_method1(
        "warn",
        (msg, py.get_type::<pyo3::exceptions::PyUserWarning>()),
    )?;
    Ok(())
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

    /// `iter(H)` — XGI parity: `Hypergraph.__iter__` = `iter(self._node)` (node ids, insertion
    /// order). Without this, Python falls back to the legacy `__getitem__(0), __getitem__(1), …`
    /// iteration protocol, which would instead call the network-attr `__getitem__` below with
    /// integer indices — a `TypeError`, not node iteration.
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let list = PyList::empty(py);
        for nid in self.inner.node_ids() {
            if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                list.append(obj)?;
            }
        }
        Ok(list.into_any().try_iter()?.unbind())
    }

    /// `H["name"]` — network-level attr; missing → XGIError (tests/core/test_hypergraph.py:55-56).
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.net_attr.get(key) {
            Some(v) => Ok(json_to_py(py, v)?.unbind()),
            None => Err(xgi_error(format!("{key} is not a network attribute"))),
        }
    }

    /// `H["name"] = value` — write a network-level attr.
    fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.net_attr
            .insert(key.to_string(), py_any_to_json(value)?);
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.inner.num_nodes()
    }

    fn __str__(&self, py: Python<'_>) -> PyResult<String> {
        let cls = "Hypergraph";
        match self.net_attr.get("name") {
            Some(v) => {
                let s = json_to_py(py, v)?.str()?.to_string();
                Ok(format!(
                    "{cls} named {s} with {} nodes and {} hyperedges",
                    self.inner.num_nodes(),
                    self.inner.num_edges()
                ))
            }
            None => Ok(format!(
                "Unnamed {cls} with {} nodes and {} hyperedges",
                self.inner.num_nodes(),
                self.inner.num_edges()
            )),
        }
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let list = PyList::empty(py);
        for eid in self.inner.edge_ids() {
            let set = PySet::empty(py)?;
            for nid in self.inner.members(&eid).unwrap_or_default() {
                if let Some(o) = self.node_ids.get_obj(py, &nid) {
                    set.add(o)?;
                }
            }
            list.append(set)?;
        }
        Ok(format!("Hypergraph({})", list.repr()?))
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.extract::<PyRef<PyHypergraph>>() {
            Ok(o) => self.structurally_equal(&o),
            Err(_) => false,
        }
    }

    /// `H1 << H2` — union into a fresh hypergraph; H2's node/edge attrs take precedence,
    /// edges are relabeled sequentially. XGI parity: `Hypergraph.__lshift__` (hypergraph.py:268-317).
    fn __lshift__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyHypergraph>> {
        let h2 = other.extract::<PyRef<PyHypergraph>>()?;
        let mut result = PyHypergraph {
            inner: CoreHg::new(),
            node_ids: PyIdMap::new(),
            edge_ids: PyIdMap::new(),
            net_attr: self.net_attr.clone(),
        };
        for src in [self, &*h2] {
            for nid in src.inner.node_ids() {
                let obj = src.node_ids.get_obj(py, &nid).unwrap();
                let attrs = src.inner.node_attrs(&nid).cloned().unwrap_or(Value::Null);
                let d = PyDict::new(py);
                if let Value::Object(m) = &attrs {
                    for (k, v) in m {
                        d.set_item(k, json_to_py(py, v)?)?;
                    }
                }
                result.add_node(&obj, Some(d))?;
            }
        }
        for src in [self, &*h2] {
            for eid in src.inner.edge_ids() {
                let member_objs: Vec<Bound<PyAny>> = src
                    .inner
                    .members(&eid)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|nid| src.node_ids.get_obj(py, nid))
                    .collect();
                let members_list = PyList::new(py, &member_objs)?;
                let attrs = match src.inner.edge_attrs(&eid).cloned() {
                    Some(Value::Object(m)) => m,
                    _ => serde_json::Map::new(),
                };
                result.try_add_one_edge(py, members_list.as_any(), None, attrs, false)?;
            }
        }
        for (k, v) in &h2.net_attr {
            result.net_attr.insert(k.clone(), v.clone());
        }
        Py::new(py, result)
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

    /// `H._edge` — `{edge_id: {member_ids...}}`, mirroring XGI's internal `_edge` IDDict.
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

    /// `H._node` — `{node_id: {edge_ids...}}`, mirroring XGI's internal `_node` IDDict.
    #[getter]
    fn _node(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let Some(obj) = self.node_ids.get_obj(py, &nid) else {
                continue;
            };
            let set = PySet::empty(py)?;
            for eid in self.inner.memberships(&nid).unwrap_or_default() {
                if let Some(o) = self.edge_ids.get_obj(py, &eid) {
                    set.add(o)?;
                }
            }
            d.set_item(obj, set)?;
        }
        Ok(d.unbind())
    }

    #[getter]
    fn _node_attr(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let Some(obj) = self.node_ids.get_obj(py, &nid) else {
                continue;
            };
            let attrs = match self.inner.node_attrs(&nid).cloned() {
                Some(Value::Object(m)) => Value::Object(m),
                _ => Value::Object(serde_json::Map::new()),
            };
            d.set_item(obj, json_to_py(py, &attrs)?)?;
        }
        Ok(d.unbind())
    }

    #[getter]
    fn _edge_attr(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for eid in self.inner.edge_ids() {
            let Some(obj) = self.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            let attrs = match self.inner.edge_attrs(&eid).cloned() {
                Some(Value::Object(m)) => Value::Object(m),
                _ => Value::Object(serde_json::Map::new()),
            };
            d.set_item(obj, json_to_py(py, &attrs)?)?;
        }
        Ok(d.unbind())
    }

    #[getter]
    fn is_frozen(&self) -> bool {
        self.inner.is_frozen()
    }

    fn freeze(&mut self) {
        self.inner.freeze();
    }

    /// `H.add_edge(members, idx=None, **attr)`.
    #[pyo3(signature = (members, idx=None, **attr))]
    fn add_edge(
        &mut self,
        py: Python<'_>,
        members: Bound<'_, PyAny>,
        idx: Option<Bound<'_, PyAny>>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let mut e_attrs = serde_json::Map::new();
        if let Some(a) = &attr {
            for (k, v) in a.iter() {
                e_attrs.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }
        self.try_add_one_edge(py, &members, idx.as_ref(), e_attrs, false)
    }

    /// `H.add_edges_from(ebunch_to_add, **attr)` — Formats 1-5 (hypergraph.py:633-819).
    #[pyo3(signature = (ebunch_to_add, **attr))]
    fn add_edges_from(
        &mut self,
        py: Python<'_>,
        ebunch_to_add: Bound<'_, PyAny>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let mut base_attr = serde_json::Map::new();
        if let Some(a) = &attr {
            for (k, v) in a.iter() {
                base_attr.insert(k.str()?.to_string(), py_any_to_json(&v)?);
            }
        }

        // Format 5: dict of {idx: members}.
        if let Ok(dict) = ebunch_to_add.cast::<PyDict>() {
            for (idx, members) in dict.iter() {
                self.try_add_one_edge(py, &members, Some(&idx), base_attr.clone(), true)?;
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
        let elem_is_str = elem
            .as_ref()
            .map(|e| e.cast::<PyString>().is_ok())
            .unwrap_or(false);

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
                if it?.cast::<PyString>().is_err() {
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

        if (fmt == Fmt::One && first.cast::<PyString>().is_ok()) || (fmt != Fmt::One && elem_is_str)
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
            self.try_add_one_edge(py, &members, idx.as_ref(), merged, true)?;

            match iter.next() {
                Some(r) => current = r?,
                None => break,
            }
        }
        Ok(())
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

    /// `H.add_nodes_from(nodes_for_adding, **attr)` — each item is either a plain hashable
    /// id or a `(id, attr_dict)` pair (XGI parity).
    #[pyo3(signature = (nodes_for_adding, **attr))]
    fn add_nodes_from(
        &mut self,
        py: Python<'_>,
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
        self.frozen_check()?;
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

    /// `H.remove_nodes_from(nodes, strong=False, remove_empty=True)` — warns and continues
    /// on a missing id (hypergraph.py:480-500).
    ///
    /// Takes `slf: Py<Self>` (not `&mut self`): `nodes` may itself be a `NodeView` aliasing this
    /// same hypergraph (e.g. `H.remove_nodes_from(H.nodes.isolates())`) — holding a live `&mut
    /// self` borrow across the call into `nodes.__iter__` (which immutably borrows the same
    /// `PyRefCell`) panics with `PyBorrowError`. Materializing `nodes` into owned Python objects
    /// FIRST, with no live borrow of `slf` held, then resolving/mutating per item avoids that.
    #[pyo3(signature = (nodes, strong=false, remove_empty=true))]
    fn remove_nodes_from(
        slf: Py<Self>,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        strong: bool,
        remove_empty: bool,
    ) -> PyResult<()> {
        {
            let hg = slf.borrow(py);
            hg.frozen_check()?;
        }
        let mut items: Vec<Py<PyAny>> = Vec::new();
        for n in nodes.try_iter()? {
            items.push(n?.unbind());
        }
        for item in items {
            let n = item.bind(py);
            let core = {
                let hg = slf.borrow(py);
                hg.node_ids.get_key(n)?
            };
            match core {
                None => {
                    warn_user(py, &format!("Node {} not in hypergraph", n.repr()?))?;
                }
                Some(core) => {
                    let mut hg = slf.borrow_mut(py);
                    hg.inner
                        .remove_node(&core, strong, remove_empty)
                        .map_err(|_| id_not_found(&core))?;
                    hg.node_ids.forget(&core);
                }
            }
        }
        Ok(())
    }

    /// `H.remove_edge(e)`.
    fn remove_edge(&mut self, e: &Bound<'_, PyAny>) -> PyResult<()> {
        self.frozen_check()?;
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

    /// `H.remove_edges_from(ebunch)` — raises on the first missing id (partial effects
    /// preserved), matching XGI's `remove_edge`-in-a-loop semantics (hypergraph.py:1142-1165).
    ///
    /// Takes `slf: Py<Self>` (not `&mut self`) for the same aliasing reason as
    /// `remove_nodes_from` — `ebunch` is routinely `H.edges.singletons()` etc., a view over this
    /// same hypergraph.
    fn remove_edges_from(slf: Py<Self>, py: Python<'_>, ebunch: &Bound<'_, PyAny>) -> PyResult<()> {
        {
            let hg = slf.borrow(py);
            hg.frozen_check()?;
        }
        let mut items: Vec<Py<PyAny>> = Vec::new();
        for e in ebunch.try_iter()? {
            items.push(e?.unbind());
        }
        for item in items {
            let e = item.bind(py);
            let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = {
                let hg = slf.borrow(py);
                hg.edge_ids.get_key(e)?
            }
            .ok_or_else(|| id_not_found(&repr))?;
            let mut hg = slf.borrow_mut(py);
            hg.inner
                .remove_edge(&core)
                .map_err(|_| id_not_found(&core))?;
            hg.edge_ids.forget(&core);
        }
        Ok(())
    }

    /// `H.add_node_to_edge(edge, node)` — auto-creates both if missing.
    fn add_node_to_edge(
        &mut self,
        edge: &Bound<'_, PyAny>,
        node: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let ecore = self.edge_ids.intern(edge)?;
        let ncore = self.node_ids.intern(node)?;
        self.inner
            .add_node_to_edge(&ecore, &ncore)
            .map_err(|e| xgi_error(format!("{e:?}")))
    }

    /// `H.remove_node_from_edge(edge, node, remove_empty=True)`.
    #[pyo3(signature = (edge, node, remove_empty=true))]
    fn remove_node_from_edge(
        &mut self,
        edge: &Bound<'_, PyAny>,
        node: &Bound<'_, PyAny>,
        remove_empty: bool,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let edge_repr = edge.repr()?.to_string();
        let node_repr = node.repr()?.to_string();
        let ecore = self
            .edge_ids
            .get_key(edge)?
            .ok_or_else(|| xgi_error(format!("Edge {edge_repr} not in the hypergraph")))?;
        let ncore = self
            .node_ids
            .get_key(node)?
            .ok_or_else(|| xgi_error(format!("Node {node_repr} not in the hypergraph")))?;
        match self
            .inner
            .remove_node_from_edge(&ecore, &ncore, remove_empty)
        {
            Ok(()) => {
                if remove_empty && !self.inner.has_edge(&ecore) {
                    self.edge_ids.forget(&ecore);
                }
                Ok(())
            }
            Err(MembershipError::EdgeNotFound { edge_id }) => {
                Err(xgi_error(format!("Edge {edge_id} not in the hypergraph")))
            }
            Err(MembershipError::NodeNotFound { node_id }) => {
                Err(xgi_error(format!("Node {node_id} not in the hypergraph")))
            }
            Err(MembershipError::NotAMember { node_id, edge_id }) => Err(xgi_error(format!(
                "Edge {edge_id} does not contain node {node_id}"
            ))),
        }
    }

    /// `H.clear_edges()`.
    fn clear_edges(&mut self) -> PyResult<()> {
        self.frozen_check()?;
        for eid in self.inner.edge_ids() {
            self.edge_ids.forget(&eid);
        }
        self.inner.clear_edges();
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

    /// `H.degree(node=None, order=None)` — XGI parity: `Hypergraph.__getattr__`'s
    /// `H.degree(...)` proxy (`func(node=None, *args, **kwargs)`, hypergraph.py:246-267): the full
    /// dict when `node` is omitted, a single scalar (raising `KeyError` for a missing node)
    /// otherwise.
    #[pyo3(signature = (node=None, order=None))]
    fn degree(
        &self,
        py: Python<'_>,
        node: Option<&Bound<'_, PyAny>>,
        order: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let deg = self.inner.node_degree(&nid, order).unwrap_or(0);
            if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                d.set_item(obj, deg)?;
            }
        }
        match node {
            None => Ok(d.into_any().unbind()),
            Some(n) => {
                let repr = n.repr()?.to_string();
                d.get_item(n)?
                    .map(|v| v.unbind())
                    .ok_or_else(|| id_not_found(&repr))
            }
        }
    }

    /// `H.size(degree=None)` — XGI parity: `Hypergraph.__getattr__` dispatch to the `size`
    /// edgestat (`stats/edgestats.py:143-176`). `degree`, if given, counts only member nodes
    /// whose own (order-agnostic) node degree equals it.
    #[pyo3(signature = (degree=None))]
    fn size(&self, py: Python<'_>, degree: Option<usize>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for eid in self.inner.edge_ids() {
            let members = self.inner.members(&eid).unwrap_or_default();
            let count = match degree {
                None => members.len(),
                Some(target) => members
                    .iter()
                    .filter(|n| self.inner.node_degree(n, None).unwrap_or(0) == target)
                    .count(),
            };
            if let Some(obj) = self.edge_ids.get_obj(py, &eid) {
                d.set_item(obj, count)?;
            }
        }
        Ok(d.unbind())
    }

    /// `H.order(degree=None)` — XGI parity: `Hypergraph.__getattr__` dispatch to the `order`
    /// edgestat (`stats/edgestats.py:106-142`); `order = size - 1`.
    #[pyo3(signature = (degree=None))]
    fn order(&self, py: Python<'_>, degree: Option<usize>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for eid in self.inner.edge_ids() {
            let members = self.inner.members(&eid).unwrap_or_default();
            let count: i64 = match degree {
                None => members.len() as i64,
                Some(target) => members
                    .iter()
                    .filter(|n| self.inner.node_degree(n, None).unwrap_or(0) == target)
                    .count() as i64,
            };
            if let Some(obj) = self.edge_ids.get_obj(py, &eid) {
                d.set_item(obj, count - 1)?;
            }
        }
        Ok(d.unbind())
    }

    /// `H.clustering_coefficient()` — `Hypergraph.__getattr__` dispatch to the
    /// `clustering_coefficient` nodestat.
    fn clustering_coefficient(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let m = hypergraph_rs::algorithms::clustering::clustering_coefficient(&self.inner);
        crate::algorithms::node_f64_dict(py, m, &self.node_ids)
    }

    /// `H.local_clustering_coefficient()` — `Hypergraph.__getattr__` dispatch to the
    /// `local_clustering_coefficient` nodestat.
    fn local_clustering_coefficient(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let m = hypergraph_rs::algorithms::clustering::local_clustering_coefficient(&self.inner);
        crate::algorithms::node_f64_dict(py, m, &self.node_ids)
    }

    /// `H.two_node_clustering_coefficient(kind="union")` — `Hypergraph.__getattr__` dispatch to
    /// the `two_node_clustering_coefficient` nodestat.
    #[pyo3(signature = (kind="union"))]
    fn two_node_clustering_coefficient(&self, py: Python<'_>, kind: &str) -> PyResult<Py<PyDict>> {
        if !matches!(kind, "union" | "min" | "max") {
            return Err(xgi_error(format!("invalid kind of clustering: {kind}")));
        }
        let m =
            hypergraph_rs::algorithms::clustering::two_node_clustering_coefficient(
                &self.inner,
                kind,
            );
        crate::algorithms::node_f64_dict(py, m, &self.node_ids)
    }

    /// `H.dual()`.
    fn dual(&self, py: Python<'_>) -> PyResult<PyHypergraph> {
        let mut node_ids = PyIdMap::new();
        for eid in self.inner.edge_ids() {
            if let Some(obj) = self.edge_ids.get_obj(py, &eid) {
                node_ids.intern(&obj)?;
            }
        }
        let mut edge_ids = PyIdMap::new();
        for nid in self.inner.node_ids() {
            if self
                .inner
                .memberships(&nid)
                .map(|m| !m.is_empty())
                .unwrap_or(false)
            {
                if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                    edge_ids.intern(&obj)?;
                }
            }
        }
        Ok(PyHypergraph {
            inner: self.inner.dual(),
            node_ids,
            edge_ids,
            net_attr: self.net_attr.clone(),
        })
    }

    /// `H.copy()` — an independent deep copy (hypergraph.py:1421-1445).
    fn copy(&self, py: Python<'_>) -> PyHypergraph {
        PyHypergraph {
            inner: self.inner.copy(),
            node_ids: self.node_ids.clone_via_py(py),
            edge_ids: self.edge_ids.clone_via_py(py),
            net_attr: self.net_attr.clone(),
        }
    }

    fn __copy__(&self, py: Python<'_>) -> PyHypergraph {
        self.copy(py)
    }

    #[pyo3(signature = (_memo=None))]
    fn __deepcopy__(&self, py: Python<'_>, _memo: Option<Bound<'_, PyAny>>) -> PyHypergraph {
        self.copy(py)
    }

    /// `H.double_edge_swap(n1, n2, e1, e2)`.
    fn double_edge_swap(
        &mut self,
        n1: &Bound<'_, PyAny>,
        n2: &Bound<'_, PyAny>,
        e1: &Bound<'_, PyAny>,
        e2: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let n1c = self.node_ids.get_key(n1)?;
        let n2c = self.node_ids.get_key(n2)?;
        let e1c = self.edge_ids.get_key(e1)?;
        let e2c = self.edge_ids.get_key(e2)?;
        let (Some(n1c), Some(n2c), Some(e1c), Some(e2c)) = (n1c, n2c, e1c, e2c) else {
            return Err(id_not_found(
                "One of the nodes specified doesn't belong to the specified edge",
            ));
        };
        self.inner
            .double_edge_swap(&n1c, &n2c, &e1c, &e2c)
            .map_err(|e| match e {
                SwapError::IdNotFound { id } => id_not_found(&id),
                SwapError::SizeNotPreserved => xgi_error("This swap does not preserve edge sizes."),
            })
    }

    /// `H.random_edge_shuffle(e_id1=None, e_id2=None, seed=None)`.
    #[pyo3(signature = (e_id1=None, e_id2=None, seed=None))]
    fn random_edge_shuffle(
        &mut self,
        e_id1: Option<&Bound<'_, PyAny>>,
        e_id2: Option<&Bound<'_, PyAny>>,
        seed: Option<u64>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let e1c = match e_id1 {
            Some(o) => Some(self.edge_ids.get_key(o)?.ok_or_else(|| {
                id_not_found(&o.repr().map(|r| r.to_string()).unwrap_or_default())
            })?),
            None => None,
        };
        let e2c = match e_id2 {
            Some(o) => Some(self.edge_ids.get_key(o)?.ok_or_else(|| {
                id_not_found(&o.repr().map(|r| r.to_string()).unwrap_or_default())
            })?),
            None => None,
        };
        let seed = seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
        });
        self.inner
            .random_edge_shuffle(e1c.as_deref(), e2c.as_deref(), seed)
            .map_err(|e| match e {
                EdgeError::TooFewEdges => {
                    PyValueError::new_err("Hypergraph must have at least two edges.")
                }
                EdgeError::NotFound { edge_id } => id_not_found(&edge_id),
                other => xgi_error(format!("{other:?}")),
            })
    }

    /// `H.set_node_attributes(values, name=None)` (hypergraph.py:503-561).
    #[pyo3(signature = (values, name=None))]
    fn set_node_attributes(
        &mut self,
        py: Python<'_>,
        values: Bound<'_, PyAny>,
        name: Option<&str>,
    ) -> PyResult<()> {
        self.set_attrs_impl(py, true, values, name)
    }

    /// `H.set_edge_attributes(values, name=None)`.
    #[pyo3(signature = (values, name=None))]
    fn set_edge_attributes(
        &mut self,
        py: Python<'_>,
        values: Bound<'_, PyAny>,
        name: Option<&str>,
    ) -> PyResult<()> {
        self.set_attrs_impl(py, false, values, name)
    }

    /// `H.merge_duplicate_edges(rename="first", merge_rule="first", multiplicity=None)`
    /// (hypergraph.py:1257-1419). Implemented entirely at the binding level (not delegating to
    /// core's narrower `merge_duplicate_edges`, which only covers `rename` first/new and
    /// `merge_rule` first/union/intersection with JSON-array union output) so `rename="tuple"`
    /// (a Python tuple edge id) is reachable without adding a core enum variant.
    #[pyo3(signature = (rename="first".to_string(), merge_rule="first".to_string(), multiplicity=None))]
    fn merge_duplicate_edges(
        &mut self,
        py: Python<'_>,
        rename: String,
        merge_rule: String,
        multiplicity: Option<String>,
    ) -> PyResult<()> {
        self.frozen_check()?;
        self.merge_duplicate_edges_impl(py, &rename, &merge_rule, multiplicity.as_deref())
    }

    /// `H.cleanup(isolates=False, singletons=False, multiedges=False, connected=True,
    /// relabel=True, in_place=True)` (hypergraph.py:1468-1518).
    #[pyo3(signature = (isolates=false, singletons=false, multiedges=false, connected=true, relabel=true, in_place=true))]
    #[allow(clippy::too_many_arguments)]
    fn cleanup(
        &mut self,
        py: Python<'_>,
        isolates: bool,
        singletons: bool,
        multiedges: bool,
        connected: bool,
        relabel: bool,
        in_place: bool,
    ) -> PyResult<Option<Py<PyHypergraph>>> {
        if in_place {
            self.frozen_check()?;
        }
        let cleaned =
            self.cleanup_computed(py, isolates, singletons, multiedges, connected, relabel)?;
        if in_place {
            self.inner = cleaned.inner;
            self.node_ids = cleaned.node_ids;
            self.edge_ids = cleaned.edge_ids;
            self.net_attr = cleaned.net_attr;
            Ok(None)
        } else {
            Ok(Some(Py::new(py, cleaned)?))
        }
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

    /// `H.render(strategy)` — the matplotlib-free scene-graph (spec §5.3/§6.3, Task 13). `strategy`
    /// is one of `"spring"`, `"bipartite"`, `"hyperedge_aware"` (default), `"radial"`; returns the
    /// `SceneGraph` (`nodes`, `hyperedges`, `bounding_box`, `metadata`) as a JSON-shaped dict. Node
    /// and hyperedge glyph `id`s stay core `String`s (opaque render coordinates), not translated back
    /// through `PyIdMap` — the scene-graph is a rendering artifact, not an XGI-parity surface.
    #[pyo3(signature = (strategy=None))]
    fn render(&self, py: Python<'_>, strategy: Option<&str>) -> PyResult<Py<PyDict>> {
        let strategy = parse_layout_strategy(strategy.unwrap_or("hyperedge_aware"))?;
        let scene = self.inner.render(strategy);
        let value = serde_json::to_value(&scene)
            .map_err(|e| xgi_error(format!("scene-graph serialization failed: {e}")))?;
        Ok(json_to_py(py, &value)?.cast_into::<PyDict>()?.unbind())
    }

    /// Pickle support (OQ-4, orchestrator ruling 2026-07-19 — implement, not exclude).
    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let nodes = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let Some(obj) = self.node_ids.get_obj(py, &nid) else {
                continue;
            };
            // Nodes auto-created by `add_edge` for a not-yet-seen member carry `N::default()` =
            // `Value::Null` (core, not `Value::Object`) — normalize so the reconstruction hook's
            // `attrs must be a dict` check always succeeds.
            let attrs = match self.inner.node_attrs(&nid).cloned() {
                Some(Value::Object(m)) => Value::Object(m),
                _ => Value::Object(serde_json::Map::new()),
            };
            nodes.set_item(obj, json_to_py(py, &attrs)?)?;
        }
        let edges = PyList::empty(py);
        for eid in self.inner.edge_ids() {
            let Some(obj) = self.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            let members_list = PyList::empty(py);
            for nid in self.inner.members(&eid).unwrap_or_default() {
                if let Some(o) = self.node_ids.get_obj(py, &nid) {
                    members_list.append(o)?;
                }
            }
            let attrs = self
                .inner
                .edge_attrs(&eid)
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));
            edges.append(PyTuple::new(
                py,
                [obj, members_list.into_any(), json_to_py(py, &attrs)?],
            )?)?;
        }
        let net = PyDict::new(py);
        for (k, v) in &self.net_attr {
            net.set_item(k, json_to_py(py, v)?)?;
        }
        let state = PyDict::new(py);
        state.set_item("nodes", nodes)?;
        state.set_item("edges", edges)?;
        state.set_item("net_attr", net)?;

        let func = py
            .import("hypergraph_rs")?
            .getattr("_reconstruct_hypergraph")?;
        Ok(PyTuple::new(py, [func.into_any(), PyTuple::new(py, [state])?.into_any()])?.unbind())
    }
}

/// Parse `H.render(strategy=...)`'s strategy name into a `LayoutStrategy` (spec §5.2 defaults).
fn parse_layout_strategy(name: &str) -> PyResult<LayoutStrategy> {
    match name {
        "spring" => Ok(LayoutStrategy::Spring {
            iterations: 50,
            seed: None,
        }),
        "bipartite" => Ok(LayoutStrategy::Bipartite {
            align: BipartiteAlign::Vertical,
        }),
        "hyperedge_aware" => Ok(LayoutStrategy::HyperedgeAware {
            iterations: 50,
            halo_padding: 0.05,
        }),
        "radial" => Ok(LayoutStrategy::Radial { rings: 4 }),
        other => Err(xgi_error(format!(
            "unknown render strategy {other:?}; expected one of \"spring\", \"bipartite\", \"hyperedge_aware\", \"radial\""
        ))),
    }
}

impl PyHypergraph {
    fn frozen_check(&self) -> PyResult<()> {
        if self.inner.is_frozen() {
            Err(xgi_error("Frozen higher-order network can't be modified"))
        } else {
            Ok(())
        }
    }

    /// Structural equality used by `__eq__` — XGI parity: `algorithms.equal(H1, H2)`
    /// (compares `_edge`, `_node_attr`, `_edge_attr`, `_net_attr`).
    fn structurally_equal(&self, other: &PyHypergraph) -> bool {
        let mut n1 = self.inner.node_ids();
        n1.sort();
        let mut n2 = other.inner.node_ids();
        n2.sort();
        if n1 != n2 {
            return false;
        }
        for nid in &n1 {
            if self.inner.node_attrs(nid) != other.inner.node_attrs(nid) {
                return false;
            }
        }
        let mut e1 = self.inner.edge_ids();
        e1.sort();
        let mut e2 = other.inner.edge_ids();
        e2.sort();
        if e1 != e2 {
            return false;
        }
        for eid in &e1 {
            if self.inner.edge_attrs(eid) != other.inner.edge_attrs(eid) {
                return false;
            }
            let mut m1 = self.inner.members(eid).unwrap_or_default();
            m1.sort();
            let mut m2 = other.inner.members(eid).unwrap_or_default();
            m2.sort();
            if m1 != m2 {
                return false;
            }
        }
        self.net_attr == other.net_attr
    }

    /// Build a `PyHypergraph` from a core graph whose String node/edge keys are
    /// decimal integers (as produced by the `generators` — Task 10) — registers them
    /// as Python `int`s (XGI generator ids are ints), preserving insertion order.
    pub fn from_core_with_int_ids(
        py: Python<'_>,
        core: CoreHg<Value, Value, Value>,
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

    fn ingest(&mut self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(list) = data.cast::<PyList>() {
            for edge in list.iter() {
                self.try_add_one_edge(py, &edge, None, serde_json::Map::new(), false)?;
            }
            return Ok(());
        }
        if let Ok(dict) = data.cast::<PyDict>() {
            for (idx, members) in dict.iter() {
                self.try_add_one_edge(py, &members, Some(&idx), serde_json::Map::new(), false)?;
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

    /// Attempt to add ONE edge, replicating XGI's per-edge `add_edge`/`add_edges_from` duplicate-id
    /// warning (hypergraph.py:566-632, 633-819). `warn_with_period` selects the exact message XGI
    /// uses (`add_edge` omits the trailing period; `add_edges_from` includes it).
    fn try_add_one_edge(
        &mut self,
        py: Python<'_>,
        members_obj: &Bound<'_, PyAny>,
        idx_obj: Option<&Bound<'_, PyAny>>,
        attrs: serde_json::Map<String, Value>,
        warn_with_period: bool,
    ) -> PyResult<()> {
        let mut member_pyobjs: Vec<Bound<PyAny>> = Vec::new();
        match members_obj.try_iter() {
            Ok(it) => {
                for m in it {
                    member_pyobjs.push(m?);
                }
            }
            Err(_) => return Err(xgi_error("Invalid ebunch format")),
        }

        if let Some(idx) = idx_obj {
            if self.edge_ids.get_key(idx)?.is_some() {
                let set = PySet::new(py, &member_pyobjs)?;
                let msg = if warn_with_period {
                    format!(
                        "uid {} already exists, cannot add edge {}.",
                        idx.str()?,
                        set.repr()?
                    )
                } else {
                    format!(
                        "uid {} already exists, cannot add edge {}",
                        idx.str()?,
                        set.repr()?
                    )
                };
                warn_user(py, &msg)?;
                return Ok(());
            }
        }

        let mut core_members = Vec::new();
        for m in &member_pyobjs {
            core_members.push(self.node_ids.intern(m)?);
        }
        let requested_core = match idx_obj {
            Some(o) => Some(self.edge_ids.intern(o)?),
            None => None,
        };
        match self
            .inner
            .add_edge(core_members, requested_core.clone(), Value::Object(attrs))
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

    /// `set_node_attributes`/`set_edge_attributes` shared implementation
    /// (hypergraph.py:503-561, 863-917).
    fn set_attrs_impl(
        &mut self,
        py: Python<'_>,
        is_node: bool,
        values: Bound<'_, PyAny>,
        name: Option<&str>,
    ) -> PyResult<()> {
        let kind = if is_node { "Node" } else { "Edge" };
        if let Some(name) = name {
            if let Ok(dict) = values.cast::<PyDict>() {
                for (k, v) in dict.iter() {
                    let core = if is_node {
                        self.node_ids.get_key(&k)?
                    } else {
                        self.edge_ids.get_key(&k)?
                    };
                    match core {
                        Some(c) => self.assign_attr(&c, is_node, name, py_any_to_json(&v)?),
                        None => warn_user(py, &format!("{kind} {} does not exist!", k.str()?))?,
                    }
                }
            } else {
                let jv = py_any_to_json(&values)?;
                let ids = if is_node {
                    self.inner.node_ids()
                } else {
                    self.inner.edge_ids()
                };
                for c in ids {
                    self.assign_attr(&c, is_node, name, jv.clone());
                }
            }
            Ok(())
        } else {
            let Ok(dict) = values.cast::<PyDict>() else {
                return Err(xgi_error("Must pass a dictionary of dictionaries"));
            };
            for (k, v) in dict.iter() {
                let core = if is_node {
                    self.node_ids.get_key(&k)?
                } else {
                    self.edge_ids.get_key(&k)?
                };
                match core {
                    None => warn_user(py, &format!("{kind} {} does not exist!", k.str()?))?,
                    Some(c) => {
                        let Ok(inner_dict) = v.cast::<PyDict>() else {
                            return Err(xgi_error("Must pass a dictionary of dictionaries"));
                        };
                        for (kk, vv) in inner_dict.iter() {
                            self.assign_attr(
                                &c,
                                is_node,
                                &kk.str()?.to_string(),
                                py_any_to_json(&vv)?,
                            );
                        }
                    }
                }
            }
            Ok(())
        }
    }

    fn assign_attr(&mut self, core_id: &str, is_node: bool, key: &str, value: Value) {
        let attrs = if is_node {
            self.inner.node_attrs_mut(core_id)
        } else {
            self.inner.edge_attrs_mut(core_id)
        };
        if let Some(attrs) = attrs {
            if !attrs.is_object() {
                *attrs = Value::Object(serde_json::Map::new());
            }
            if let Some(obj) = attrs.as_object_mut() {
                obj.insert(key.to_string(), value);
            }
        }
    }

    /// `H.merge_duplicate_edges(...)` core algorithm, implemented at the binding level (see the
    /// `#[pymethods]` doc comment above for why).
    fn merge_duplicate_edges_impl(
        &mut self,
        py: Python<'_>,
        rename: &str,
        merge_rule: &str,
        multiplicity: Option<&str>,
    ) -> PyResult<()> {
        if !["first", "tuple", "new"].contains(&rename) {
            return Err(xgi_error("Invalid ID renaming scheme!"));
        }
        if !["first", "union", "intersection"].contains(&merge_rule) {
            return Err(xgi_error("Invalid merge rule!"));
        }

        let mut groups: indexmap::IndexMap<Vec<String>, Vec<String>> = indexmap::IndexMap::new();
        for eid in self.inner.edge_ids() {
            let mut mem = self.inner.members(&eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid);
        }

        struct NewEdge {
            members: Vec<String>,
            id_obj: Py<PyAny>,
            attrs: Value,
        }
        let mut dups_to_remove: Vec<String> = Vec::new();
        let mut new_edges: Vec<NewEdge> = Vec::new();

        for (members, dup_ids) in groups.iter() {
            if dup_ids.len() <= 1 {
                continue;
            }
            dups_to_remove.extend(dup_ids.iter().cloned());

            let dup_objs: Vec<Bound<PyAny>> = dup_ids
                .iter()
                .filter_map(|c| self.edge_ids.get_obj(py, c))
                .collect();
            let sort_list = PyList::new(py, &dup_objs)?;
            sort_list.sort()?;
            let sorted_objs: Vec<Bound<PyAny>> = sort_list.iter().collect();
            let sorted_core: Vec<String> = sorted_objs
                .iter()
                .filter_map(|o| self.edge_ids.get_key(o).ok().flatten())
                .collect();

            let id_obj: Bound<PyAny> = match rename {
                "first" => sorted_objs[0].clone(),
                "tuple" => PyTuple::new(py, &sorted_objs)?.into_any(),
                "new" => {
                    let placeholder = self.inner.add_empty_edge(None, Value::Null);
                    let _ = self.inner.remove_edge(&placeholder);
                    int_or_str_pyobj(py, &placeholder)?
                }
                _ => unreachable!(),
            };

            let min_core = &sorted_core[0];
            let attrs: Value = match merge_rule {
                "first" => self
                    .inner
                    .edge_attrs(min_core)
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new())),
                "union" | "intersection" => {
                    use std::collections::BTreeSet;
                    let mut keys: BTreeSet<String> = BTreeSet::new();
                    for id in dup_ids {
                        if let Some(obj) = self.inner.edge_attrs(id).and_then(|a| a.as_object()) {
                            keys.extend(obj.keys().cloned());
                        }
                    }
                    let mut out = serde_json::Map::new();
                    for k in keys {
                        let mut distinct: Vec<Value> = Vec::new();
                        for id in dup_ids {
                            let val = self
                                .inner
                                .edge_attrs(id)
                                .and_then(|a| a.get(&k))
                                .cloned()
                                .unwrap_or(Value::Null);
                            if !distinct.contains(&val) {
                                distinct.push(val);
                            }
                        }
                        // DEVIATION (documented, cited test_merge_duplicate_edges exclusion):
                        // real XGI's merge_rule="union" stores a Python `set` per key; the JSON
                        // attr channel (serde_json::Value) has no Set variant, so a dedup'd array
                        // stands in. merge_rule="intersection" needs no set (scalar-or-null) and
                        // is fully correct.
                        let merged = if merge_rule == "union" {
                            Value::Array(distinct)
                        } else if distinct.len() == 1 {
                            distinct.into_iter().next().unwrap()
                        } else {
                            Value::Null
                        };
                        out.insert(k, merged);
                    }
                    Value::Object(out)
                }
                _ => unreachable!(),
            };
            let mut attrs_obj = match attrs {
                Value::Object(m) => m,
                _ => serde_json::Map::new(),
            };
            if let Some(mkey) = multiplicity {
                attrs_obj.insert(mkey.to_string(), serde_json::json!(dup_ids.len()));
            }
            new_edges.push(NewEdge {
                members: members.clone(),
                id_obj: id_obj.unbind(),
                attrs: Value::Object(attrs_obj),
            });
        }

        for eid in &dups_to_remove {
            let _ = self.inner.remove_edge(eid);
            self.edge_ids.forget(eid);
        }
        for ne in new_edges {
            let obj = ne.id_obj.bind(py);
            let core_id = self.edge_ids.intern(obj)?;
            let _ = self.inner.add_edge(ne.members, Some(core_id), ne.attrs);
        }

        if merge_rule == "union" {
            warn_user(
                py,
                "You will not be able to color/draw by merged attributes with xgi.draw()!",
            )?;
        }
        Ok(())
    }

    /// Compute a cleaned-up COPY (never mutates `self`); `cleanup()` decides whether to swap it
    /// back into `self` (`in_place=True`) or return it fresh.
    fn cleanup_computed(
        &self,
        py: Python<'_>,
        isolates: bool,
        singletons: bool,
        multiedges: bool,
        connected: bool,
        relabel: bool,
    ) -> PyResult<PyHypergraph> {
        let mut work = PyHypergraph {
            inner: self.inner.copy(),
            node_ids: self.node_ids.clone_via_py(py),
            edge_ids: self.edge_ids.clone_via_py(py),
            net_attr: self.net_attr.clone(),
        };
        if !multiedges {
            work.merge_duplicate_edges_impl(py, "first", "first", None)?;
        }
        if !singletons {
            let s = work.inner.singleton_edge_ids();
            let _ = work.inner.remove_edges_from(&s);
            for e in &s {
                work.edge_ids.forget(e);
            }
        }
        if !isolates {
            let i = work.inner.isolate_node_ids();
            let _ = work.inner.remove_nodes_from(&i, false, true);
            for n in &i {
                work.node_ids.forget(n);
            }
        }
        if connected {
            let giant: std::collections::HashSet<String> = largest_connected_component(&work.inner)
                .into_iter()
                .collect();
            let nodes_before = work.inner.node_ids();
            let edges_before = work.inner.edge_ids();
            let to_remove: Vec<String> = nodes_before
                .iter()
                .filter(|n| !giant.contains(*n))
                .cloned()
                .collect();
            let _ = work.inner.remove_nodes_from(&to_remove, false, true);
            for n in &nodes_before {
                if !work.inner.has_node(n) {
                    work.node_ids.forget(n);
                }
            }
            for e in &edges_before {
                if !work.inner.has_edge(e) {
                    work.edge_ids.forget(e);
                }
            }
        }
        if relabel {
            work = work.relabel_to_integers(py)?;
        }
        Ok(work)
    }

    /// Relabel nodes/edges to sequential integers `0..N`/`0..M` in current iteration order.
    /// XGI parity: `xgi.utils.convert_labels_to_integers` (utils/utilities.py:377+) minus the
    /// `label` provenance attribute (not exercised by the P0 conformance tests; documented
    /// deviation).
    fn relabel_to_integers(&self, py: Python<'_>) -> PyResult<PyHypergraph> {
        let mut out = PyHypergraph {
            inner: CoreHg::new(),
            node_ids: PyIdMap::new(),
            edge_ids: PyIdMap::new(),
            net_attr: self.net_attr.clone(),
        };
        let mut node_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (i, nid) in self.inner.node_ids().into_iter().enumerate() {
            let new_core = i.to_string();
            let attrs = self.inner.node_attrs(&nid).cloned().unwrap_or(Value::Null);
            out.inner.add_node(&new_core, attrs);
            let obj = (i as i64).into_pyobject(py)?.into_any();
            out.node_ids.intern(&obj)?;
            node_map.insert(nid, new_core);
        }
        for (i, eid) in self.inner.edge_ids().into_iter().enumerate() {
            let members: Vec<String> = self
                .inner
                .members(&eid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| node_map.get(&n).cloned().unwrap_or(n))
                .collect();
            let attrs = self.inner.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            let _ = out
                .inner
                .add_edge(members, Some(i.to_string()), attrs)
                .map_err(|e| xgi_error(format!("{e:?}")))?;
            let obj = (i as i64).into_pyobject(py)?.into_any();
            out.edge_ids.intern(&obj)?;
        }
        Ok(out)
    }
}

/// Auto-assigned core ids are numeric strings ("0", "1", …) — register them as Python `int`
/// (XGI auto-ids are ints), falling back to `str` for the (unreachable in practice) non-numeric case.
pub(crate) fn int_or_str_pyobj<'py>(py: Python<'py>, core_id: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(n) = core_id.parse::<i64>() {
        return Ok(n.into_pyobject(py)?.into_any());
    }
    Ok(core_id.into_pyobject(py)?.into_any())
}

/// `hypergraph_rs._reconstruct_hypergraph(state)` — the pickle `__reduce__` reconstruction hook.
/// `state` is `{"nodes": {obj: attrs}, "edges": [(idx, members, attrs), ...], "net_attr": {...}}`.
#[pyfunction(name = "_reconstruct_hypergraph")]
fn reconstruct_hypergraph(py: Python<'_>, state: Bound<'_, PyDict>) -> PyResult<PyHypergraph> {
    let mut hg = PyHypergraph {
        inner: CoreHg::new(),
        node_ids: PyIdMap::new(),
        edge_ids: PyIdMap::new(),
        net_attr: serde_json::Map::new(),
    };
    let nodes: Bound<PyDict> = state
        .get_item("nodes")?
        .ok_or_else(|| xgi_error("missing 'nodes' in pickled state"))?
        .cast_into()
        .map_err(|_| xgi_error("'nodes' must be a dict"))?;
    for (k, v) in nodes.iter() {
        let attrd: Bound<PyDict> = v
            .cast_into()
            .map_err(|_| xgi_error("node attrs must be a dict"))?;
        hg.add_node(&k, Some(attrd))?;
    }
    let edges: Bound<PyList> = state
        .get_item("edges")?
        .ok_or_else(|| xgi_error("missing 'edges' in pickled state"))?
        .cast_into()
        .map_err(|_| xgi_error("'edges' must be a list"))?;
    for item in edges.iter() {
        let t: Bound<PyTuple> = item
            .cast_into()
            .map_err(|_| xgi_error("edge entry must be a tuple"))?;
        let idx = t.get_item(0)?;
        let members = t.get_item(1)?;
        let attrd: Bound<PyDict> = t
            .get_item(2)?
            .cast_into()
            .map_err(|_| xgi_error("edge attrs must be a dict"))?;
        let mut m = serde_json::Map::new();
        for (k, v) in attrd.iter() {
            m.insert(k.str()?.to_string(), py_any_to_json(&v)?);
        }
        hg.try_add_one_edge(py, &members, Some(&idx), m, false)?;
    }
    let net: Bound<PyDict> = state
        .get_item("net_attr")?
        .ok_or_else(|| xgi_error("missing 'net_attr' in pickled state"))?
        .cast_into()
        .map_err(|_| xgi_error("'net_attr' must be a dict"))?;
    for (k, v) in net.iter() {
        hg.net_attr
            .insert(k.str()?.to_string(), py_any_to_json(&v)?);
    }
    Ok(hg)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHypergraph>()?;
    let func = pyo3::wrap_pyfunction!(reconstruct_hypergraph, m)?;
    func.setattr("__module__", "hypergraph_rs")?;
    m.add_function(func)?;
    Ok(())
}
