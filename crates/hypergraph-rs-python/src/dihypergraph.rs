//! PyO3 binding for the core `DiHypergraph` (Task 6). Mirrors Task 4's `PyHypergraph` CRUD +
//! Task 5's `PyNodeView`/`PyEdgeView` proxy pattern, adapted for directed `(tail, head)` edges.
//! XGI parity: `xgi.DiHypergraph` (dihypergraph.py). Directed `add_edge` takes a
//! `(tail_set, head_set)` pair (`conftest.py:diedgelist1` = `({1,2,3},{4})`).

use hypergraph_rs::core::error::EdgeError;
use hypergraph_rs::core::kinds::DiRole;
use hypergraph_rs::DiHypergraph as CoreDiHg;
use hypergraph_rs::MembershipError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet, PyString, PyTuple};
use serde_json::Value;

use crate::errors::{id_not_found, xgi_error};
use crate::pyid::{json_to_py, py_any_to_json, PyIdMap};

fn warn_user(py: Python<'_>, msg: &str) -> PyResult<()> {
    py.import("warnings")?.call_method1(
        "warn",
        (msg, py.get_type::<pyo3::exceptions::PyUserWarning>()),
    )?;
    Ok(())
}

fn parse_direction(direction: &str) -> PyResult<DiRole> {
    match direction {
        "in" => Ok(DiRole::Tail),
        "out" => Ok(DiRole::Head),
        _ => Err(xgi_error("Invalid direction!")),
    }
}

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
    pub(crate) fn new(
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

    fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.net_attr
            .insert(key.to_string(), py_any_to_json(value)?);
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.inner.num_nodes()
    }

    fn __str__(&self, py: Python<'_>) -> PyResult<String> {
        let cls = "DiHypergraph";
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
            let (tail, head) = self.inner.dimembers(&eid).unwrap_or_default();
            let tail_set = PySet::empty(py)?;
            for n in tail {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    tail_set.add(o)?;
                }
            }
            let head_set = PySet::empty(py)?;
            for n in head {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    head_set.add(o)?;
                }
            }
            list.append(PyTuple::new(
                py,
                [tail_set.into_any(), head_set.into_any()],
            )?)?;
        }
        Ok(format!("DiHypergraph({})", list.repr()?))
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.extract::<PyRef<PyDiHypergraph>>() {
            Ok(o) => self.structurally_equal(&o),
            Err(_) => false,
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

    /// `DH._edge` — `{edge_id: {"in": tail_set, "out": head_set}}` (XGI internal layout;
    /// tests read it directly, e.g. `H2._edge.items()`).
    #[getter]
    fn _edge(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for eid in self.inner.edge_ids() {
            let Some(obj) = self.edge_ids.get_obj(py, &eid) else {
                continue;
            };
            let (tail, head) = self.inner.dimembers(&eid).unwrap_or_default();
            let tail_set = PySet::empty(py)?;
            for n in tail {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    tail_set.add(o)?;
                }
            }
            let head_set = PySet::empty(py)?;
            for n in head {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    head_set.add(o)?;
                }
            }
            let entry = PyDict::new(py);
            entry.set_item("in", tail_set)?;
            entry.set_item("out", head_set)?;
            d.set_item(obj, entry)?;
        }
        Ok(d.unbind())
    }

    /// `DH._node` — `{node_id: {"in": in_edge_ids, "out": out_edge_ids}}` (mirrors `_edge`;
    /// `"in"` = edges where the node is a head/receiver, `"out"` = edges where it's a
    /// tail/sender — the same `dimemberships()` pair).
    #[getter]
    fn _node(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let Some(obj) = self.node_ids.get_obj(py, &nid) else {
                continue;
            };
            let (in_e, out_e) = self.inner.dimemberships(&nid).unwrap_or_default();
            let in_set = PySet::empty(py)?;
            for e in in_e {
                if let Some(o) = self.edge_ids.get_obj(py, &e) {
                    in_set.add(o)?;
                }
            }
            let out_set = PySet::empty(py)?;
            for e in out_e {
                if let Some(o) = self.edge_ids.get_obj(py, &e) {
                    out_set.add(o)?;
                }
            }
            let entry = PyDict::new(py);
            entry.set_item("in", in_set)?;
            entry.set_item("out", out_set)?;
            d.set_item(obj, entry)?;
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

    /// `DH.add_edge((tail, head), idx=None, **attr)`. Shims: duplicate idx -> warn+no-op
    /// (same deviation shim as `PyHypergraph`, Task 4).
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

    /// `DH.add_edges_from(ebunch_to_add, **attr)` — Formats 1,2,3,4,5 (dihypergraph.py:587-786).
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

        // second_elem = list(first_edge)[1] (dihypergraph.py:727).
        let second_elem: Option<Bound<PyAny>> = match first.try_iter() {
            Ok(mut it) => {
                let _ = it.next();
                match it.next() {
                    Some(r) => Some(r?),
                    None => None,
                }
            }
            Err(_) => None,
        };
        let second_is_edge_shaped = match &second_elem {
            Some(e) => {
                e.try_iter().is_ok() && e.cast::<PyString>().is_err() && e.cast::<PyDict>().is_err()
            }
            None => false,
        };

        #[derive(Clone, Copy, PartialEq)]
        enum Fmt {
            One,
            Two,
            Three,
            Four,
        }
        let fmt = if second_is_edge_shaped {
            Fmt::One
        } else {
            let len = first.len()?;
            if len == 3 {
                Fmt::Four
            } else if len == 2 {
                let second = first.get_item(1)?;
                if second.cast::<PyDict>().is_ok() {
                    Fmt::Three
                } else {
                    Fmt::Two
                }
            } else {
                return Err(xgi_error("Invalid ebunch format"));
            }
        };

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
    pub(crate) fn add_node(
        &mut self,
        n: &Bound<'_, PyAny>,
        attr: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
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

    /// `DH.add_nodes_from(nodes_for_adding, **attr)`.
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

    /// `DH.remove_node(n, strong=False, remove_empty=True)`.
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

    /// `DH.remove_nodes_from(nodes, strong=False, remove_empty=True)` — warns and continues
    /// on a missing id. `slf: Py<Self>` for the same view-aliasing reason as
    /// `PyHypergraph::remove_nodes_from`.
    #[pyo3(signature = (nodes, strong=false, remove_empty=true))]
    fn remove_nodes_from(
        slf: Py<Self>,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        strong: bool,
        remove_empty: bool,
    ) -> PyResult<()> {
        {
            let dh = slf.borrow(py);
            dh.frozen_check()?;
        }
        let mut items: Vec<Py<PyAny>> = Vec::new();
        for n in nodes.try_iter()? {
            items.push(n?.unbind());
        }
        for item in items {
            let n = item.bind(py);
            let core = {
                let dh = slf.borrow(py);
                dh.node_ids.get_key(n)?
            };
            match core {
                None => {
                    warn_user(py, &format!("Node {} not in hypergraph", n.repr()?))?;
                }
                Some(core) => {
                    let mut dh = slf.borrow_mut(py);
                    dh.inner
                        .remove_node(&core, strong, remove_empty)
                        .map_err(|_| id_not_found(&core))?;
                    dh.node_ids.forget(&core);
                }
            }
        }
        Ok(())
    }

    /// `DH.remove_edge(e)`.
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

    /// `DH.remove_edges_from(ebunch)` — `slf: Py<Self>` for the view-aliasing reason.
    fn remove_edges_from(slf: Py<Self>, py: Python<'_>, ebunch: &Bound<'_, PyAny>) -> PyResult<()> {
        {
            let dh = slf.borrow(py);
            dh.frozen_check()?;
        }
        let mut items: Vec<Py<PyAny>> = Vec::new();
        for e in ebunch.try_iter()? {
            items.push(e?.unbind());
        }
        for item in items {
            let e = item.bind(py);
            let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = {
                let dh = slf.borrow(py);
                dh.edge_ids.get_key(e)?
            }
            .ok_or_else(|| id_not_found(&repr))?;
            let mut dh = slf.borrow_mut(py);
            dh.inner
                .remove_edge(&core)
                .map_err(|_| id_not_found(&core))?;
            dh.edge_ids.forget(&core);
        }
        Ok(())
    }

    /// `DH.add_node_to_edge(edge, node, direction)` — `direction` is `"in"` (tail) or `"out"`
    /// (head).
    pub(crate) fn add_node_to_edge(
        &mut self,
        edge: &Bound<'_, PyAny>,
        node: &Bound<'_, PyAny>,
        direction: &str,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let role = parse_direction(direction)?;
        let ecore = self.edge_ids.intern(edge)?;
        let ncore = self.node_ids.intern(node)?;
        self.inner
            .add_node_to_edge(&ecore, &ncore, role)
            .map_err(|e| xgi_error(format!("{e:?}")))
    }

    /// `DH.remove_node_from_edge(edge, node, direction, remove_empty=True)`.
    #[pyo3(signature = (edge, node, direction, remove_empty=true))]
    fn remove_node_from_edge(
        &mut self,
        edge: &Bound<'_, PyAny>,
        node: &Bound<'_, PyAny>,
        direction: &str,
        remove_empty: bool,
    ) -> PyResult<()> {
        self.frozen_check()?;
        let role = parse_direction(direction)?;
        let ed = if direction == "in" { "in" } else { "out" };
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
            .remove_node_from_edge(&ecore, &ncore, role, remove_empty)
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
                "{ed}-edge {edge_id} does not contain node {node_id}"
            ))),
        }
    }

    /// `DH.set_node_attributes(values, name=None)`.
    #[pyo3(signature = (values, name=None))]
    fn set_node_attributes(
        &mut self,
        py: Python<'_>,
        values: Bound<'_, PyAny>,
        name: Option<&str>,
    ) -> PyResult<()> {
        self.set_attrs_impl(py, true, values, name)
    }

    /// `DH.set_edge_attributes(values, name=None)`.
    #[pyo3(signature = (values, name=None))]
    fn set_edge_attributes(
        &mut self,
        py: Python<'_>,
        values: Bound<'_, PyAny>,
        name: Option<&str>,
    ) -> PyResult<()> {
        self.set_attrs_impl(py, false, values, name)
    }

    /// `DH.copy()`.
    fn copy(&self, py: Python<'_>) -> PyDiHypergraph {
        PyDiHypergraph {
            inner: self.inner.copy(),
            node_ids: self.node_ids.clone_via_py(py),
            edge_ids: self.edge_ids.clone_via_py(py),
            net_attr: self.net_attr.clone(),
        }
    }

    fn __copy__(&self, py: Python<'_>) -> PyDiHypergraph {
        self.copy(py)
    }

    #[pyo3(signature = (_memo=None))]
    fn __deepcopy__(&self, py: Python<'_>, _memo: Option<Bound<'_, PyAny>>) -> PyDiHypergraph {
        self.copy(py)
    }

    /// `DH.cleanup(isolates=False, relabel=True, in_place=True)` (dihypergraph.py:1044-1058).
    #[pyo3(signature = (isolates=false, relabel=true, in_place=true))]
    fn cleanup(
        &mut self,
        py: Python<'_>,
        isolates: bool,
        relabel: bool,
        in_place: bool,
    ) -> PyResult<Option<Py<PyDiHypergraph>>> {
        if in_place {
            self.frozen_check()?;
        }
        let mut work = PyDiHypergraph {
            inner: self.inner.copy(),
            node_ids: self.node_ids.clone_via_py(py),
            edge_ids: self.edge_ids.clone_via_py(py),
            net_attr: self.net_attr.clone(),
        };
        if !isolates {
            let i = work.inner.isolate_node_ids();
            for n in &i {
                let _ = work.inner.remove_node(n, false, true);
            }
            for n in &i {
                work.node_ids.forget(n);
            }
        }
        if relabel {
            work = work.relabel_to_integers(py)?;
        }
        if in_place {
            self.inner = work.inner;
            self.node_ids = work.node_ids;
            self.edge_ids = work.edge_ids;
            self.net_attr = work.net_attr;
            Ok(None)
        } else {
            Ok(Some(Py::new(py, work)?))
        }
    }

    /// `DH.degree(node=None)` — the full dict when `node` is omitted, a single scalar (raising
    /// `KeyError`/`IDNotFound` for a missing node) otherwise (mirrors `PyHypergraph::degree`).
    #[pyo3(signature = (node=None, order=None))]
    fn degree(
        &self,
        py: Python<'_>,
        node: Option<&Bound<'_, PyAny>>,
        order: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let deg = match order {
                None => self.inner.degree(&nid),
                Some(target) => self
                    .inner
                    .memberships(&nid)
                    .unwrap_or_default()
                    .iter()
                    .filter(|e| self.inner.order(e) == target)
                    .count(),
            };
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

    /// Single-node in-degree (edges where the node is a head/receiver).
    #[pyo3(signature = (n=None, order=None))]
    fn in_degree(
        &self,
        py: Python<'_>,
        n: Option<&Bound<'_, PyAny>>,
        order: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        let deg_of = |nid: &str| -> usize {
            match order {
                None => self.inner.in_degree(nid),
                Some(target) => self
                    .inner
                    .dimemberships(nid)
                    .map(|(in_edges, _)| {
                        in_edges
                            .iter()
                            .filter(|e| self.inner.order(e) == target)
                            .count()
                    })
                    .unwrap_or(0),
            }
        };
        match n {
            Some(n) => {
                let core = self.require_node(n)?;
                Ok(deg_of(&core).into_pyobject(py)?.into_any().unbind())
            }
            None => {
                let d = PyDict::new(py);
                for nid in self.inner.node_ids() {
                    if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                        d.set_item(obj, deg_of(&nid))?;
                    }
                }
                Ok(d.into_any().unbind())
            }
        }
    }

    /// Out-degree (edges where the node is a tail/sender) — dict over all nodes when `n` is
    /// omitted, else a single scalar.
    #[pyo3(signature = (n=None, order=None))]
    fn out_degree(
        &self,
        py: Python<'_>,
        n: Option<&Bound<'_, PyAny>>,
        order: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        let deg_of = |nid: &str| -> usize {
            match order {
                None => self.inner.out_degree(nid),
                Some(target) => self
                    .inner
                    .dimemberships(nid)
                    .map(|(_, out_edges)| {
                        out_edges
                            .iter()
                            .filter(|e| self.inner.order(e) == target)
                            .count()
                    })
                    .unwrap_or(0),
            }
        };
        match n {
            Some(n) => {
                let core = self.require_node(n)?;
                Ok(deg_of(&core).into_pyobject(py)?.into_any().unbind())
            }
            None => {
                let d = PyDict::new(py);
                for nid in self.inner.node_ids() {
                    if let Some(obj) = self.node_ids.get_obj(py, &nid) {
                        d.set_item(obj, deg_of(&nid))?;
                    }
                }
                Ok(d.into_any().unbind())
            }
        }
    }

    /// `DH.nodes` — the directed `DiNodeView` proxy (Task 6, mirrors Task 5's `NodeView`).
    #[getter]
    fn nodes(slf: Py<Self>) -> PyDiNodeView {
        PyDiNodeView { dh: slf, ids: None }
    }

    /// `DH.edges` — the directed `DiEdgeView` proxy (Task 6, mirrors Task 5's `EdgeView`).
    #[getter]
    fn edges(slf: Py<Self>) -> PyDiEdgeView {
        PyDiEdgeView { dh: slf, ids: None }
    }

    /// Pickle support (OQ-4).
    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let nodes = PyDict::new(py);
        for nid in self.inner.node_ids() {
            let Some(obj) = self.node_ids.get_obj(py, &nid) else {
                continue;
            };
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
            let (tail, head) = self.inner.dimembers(&eid).unwrap_or_default();
            let tail_list = PyList::empty(py);
            for n in tail {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    tail_list.append(o)?;
                }
            }
            let head_list = PyList::empty(py);
            for n in head {
                if let Some(o) = self.node_ids.get_obj(py, &n) {
                    head_list.append(o)?;
                }
            }
            let attrs = match self.inner.edge_attrs(&eid).cloned() {
                Some(Value::Object(m)) => Value::Object(m),
                _ => Value::Object(serde_json::Map::new()),
            };
            edges.append(PyTuple::new(
                py,
                [
                    obj,
                    tail_list.into_any(),
                    head_list.into_any(),
                    json_to_py(py, &attrs)?,
                ],
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
            .getattr("_reconstruct_dihypergraph")?;
        Ok(PyTuple::new(py, [func.into_any(), PyTuple::new(py, [state])?.into_any()])?.unbind())
    }
}

impl PyDiHypergraph {
    fn frozen_check(&self) -> PyResult<()> {
        if self.inner.is_frozen() {
            Err(xgi_error("Frozen higher-order network can't be modified"))
        } else {
            Ok(())
        }
    }

    fn structurally_equal(&self, other: &PyDiHypergraph) -> bool {
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
            let (mut t1, mut h1) = self.inner.dimembers(eid).unwrap_or_default();
            let (mut t2, mut h2) = other.inner.dimembers(eid).unwrap_or_default();
            t1.sort();
            t2.sort();
            h1.sort();
            h2.sort();
            if t1 != t2 || h1 != h2 {
                return false;
            }
        }
        self.net_attr == other.net_attr
    }

    fn require_node(&self, n: &Bound<'_, PyAny>) -> PyResult<String> {
        let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
        self.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))
    }

    /// Accepts a list of `(tail, head)` pairs (`diedgelist1`-style) or a dict of
    /// `idx -> (tail, head)` (`dihypergraph1`-style, `conftest.py`).
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
        if let Ok(other) = data.extract::<PyRef<PyDiHypergraph>>() {
            self.inner = other.inner.copy();
            self.node_ids = other.node_ids.clone_via_py(py);
            self.edge_ids = other.edge_ids.clone_via_py(py);
            return Ok(());
        }
        Err(xgi_error("could not interpret data as a dihypergraph"))
    }

    fn try_add_one_edge(
        &mut self,
        py: Python<'_>,
        members: &Bound<'_, PyAny>,
        idx: Option<&Bound<'_, PyAny>>,
        attrs: serde_json::Map<String, Value>,
        warn_with_period: bool,
    ) -> PyResult<()> {
        // `members` must be a subscriptable (tail, head) pair — list or tuple, indexed `[0]`/`[1]`
        // (XGI: `isinstance(members, (tuple, list))`, dihypergraph.py:553). A bare `set` of two
        // items (`test_add_edge_rejects_set`) has no `__getitem__` and must raise, so this can't
        // be "iterate and take the first two items".
        let tail_iterable = members
            .get_item(0)
            .map_err(|_| xgi_error("Directed edge must be a list or tuple!"))?;
        let head_iterable = members
            .get_item(1)
            .map_err(|_| xgi_error("Directed edge must be a list or tuple!"))?;
        let mut tail_objs: Vec<Bound<PyAny>> = Vec::new();
        for m in tail_iterable.try_iter()? {
            tail_objs.push(m?);
        }
        let mut head_objs: Vec<Bound<PyAny>> = Vec::new();
        for m in head_iterable.try_iter()? {
            head_objs.push(m?);
        }

        if let Some(idx) = idx {
            if self.edge_ids.get_key(idx)?.is_some() {
                let members_set = PySet::empty(py)?;
                for o in tail_objs.iter().chain(head_objs.iter()) {
                    members_set.add(o)?;
                }
                let msg = if warn_with_period {
                    format!(
                        "uid {} already exists, cannot add edge {}.",
                        idx.str()?,
                        members_set.repr()?
                    )
                } else {
                    format!(
                        "uid {} already exists, cannot add edge {}",
                        idx.str()?,
                        members_set.repr()?
                    )
                };
                warn_user(py, &msg)?;
                return Ok(());
            }
        }

        let mut tail = Vec::new();
        for m in &tail_objs {
            tail.push(self.node_ids.intern(m)?);
        }
        let mut head = Vec::new();
        for m in &head_objs {
            head.push(self.node_ids.intern(m)?);
        }
        let requested_core = match idx {
            Some(o) => Some(self.edge_ids.intern(o)?),
            None => None,
        };
        match self
            .inner
            .add_edge(tail, head, requested_core.clone(), Value::Object(attrs))
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

    /// Core `DiHypergraph` has no `node_attrs_mut`/`edge_attrs_mut` (unlike `Hypergraph`) — merge
    /// via read-then-REPLACE instead: `add_node` already replaces-in-place by id (Task 4 D6
    /// pattern); edges have no bare "replace attrs" primitive, so remove+re-add the edge with the
    /// same id/members and the merged attrs (id reuse after removal is safe — `add_edge`'s counter
    /// bump only fires for a NEW id).
    fn assign_attr(&mut self, core_id: &str, is_node: bool, key: &str, value: Value) {
        if is_node {
            let mut merged = match self.inner.node_attrs(core_id) {
                Some(Value::Object(m)) => m.clone(),
                _ => serde_json::Map::new(),
            };
            merged.insert(key.to_string(), value);
            self.inner.add_node(core_id, Value::Object(merged));
        } else {
            let mut merged = match self.inner.edge_attrs(core_id) {
                Some(Value::Object(m)) => m.clone(),
                _ => serde_json::Map::new(),
            };
            merged.insert(key.to_string(), value);
            let Some((tail, head)) = self.inner.dimembers(core_id) else {
                return;
            };
            let _ = self.inner.remove_edge(core_id);
            let _ =
                self.inner
                    .add_edge(tail, head, Some(core_id.to_string()), Value::Object(merged));
        }
    }

    /// Relabel nodes/edges to sequential integers (see `PyHypergraph::relabel_to_integers`).
    fn relabel_to_integers(&self, py: Python<'_>) -> PyResult<PyDiHypergraph> {
        let mut out = PyDiHypergraph {
            inner: CoreDiHg::new(),
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
            let (tail, head) = self.inner.dimembers(&eid).unwrap_or_default();
            let tail: Vec<String> = tail
                .into_iter()
                .map(|n| node_map.get(&n).cloned().unwrap_or(n))
                .collect();
            let head: Vec<String> = head
                .into_iter()
                .map(|n| node_map.get(&n).cloned().unwrap_or(n))
                .collect();
            let attrs = self.inner.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            let _ = out
                .inner
                .add_edge(tail, head, Some(i.to_string()), attrs)
                .map_err(|e| xgi_error(format!("{e:?}")))?;
            let obj = (i as i64).into_pyobject(py)?.into_any();
            out.edge_ids.intern(&obj)?;
        }
        Ok(out)
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

/// `hypergraph_rs._reconstruct_dihypergraph(state)` — pickle `__reduce__` reconstruction hook.
#[pyfunction(name = "_reconstruct_dihypergraph")]
fn reconstruct_dihypergraph(py: Python<'_>, state: Bound<'_, PyDict>) -> PyResult<PyDiHypergraph> {
    let mut dh = PyDiHypergraph {
        inner: CoreDiHg::new(),
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
        dh.add_node(&k, Some(attrd))?;
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
        let tail = t.get_item(1)?;
        let head = t.get_item(2)?;
        let attrd: Bound<PyDict> = t
            .get_item(3)?
            .cast_into()
            .map_err(|_| xgi_error("edge attrs must be a dict"))?;
        let mut m = serde_json::Map::new();
        for (k, v) in attrd.iter() {
            m.insert(k.str()?.to_string(), py_any_to_json(&v)?);
        }
        let pair = PyTuple::new(py, [tail, head])?;
        dh.try_add_one_edge(py, pair.as_any(), Some(&idx), m, false)?;
    }
    let net: Bound<PyDict> = state
        .get_item("net_attr")?
        .ok_or_else(|| xgi_error("missing 'net_attr' in pickled state"))?
        .cast_into()
        .map_err(|_| xgi_error("'net_attr' must be a dict"))?;
    for (k, v) in net.iter() {
        dh.net_attr
            .insert(k.str()?.to_string(), py_any_to_json(&v)?);
    }
    Ok(dh)
}

// ---------------------------------------------------------------------------------------------
// DiNodeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "DiNodeView", module = "hypergraph_rs")]
pub struct PyDiNodeView {
    pub dh: Py<PyDiHypergraph>,
    pub ids: Option<Vec<String>>,
}

impl PyDiNodeView {
    fn effective_ids(&self, dh: &PyDiHypergraph) -> Vec<String> {
        self.ids.clone().unwrap_or_else(|| dh.inner.node_ids())
    }

    fn sub(&self, py: Python<'_>, ids: Vec<String>) -> PyDiNodeView {
        PyDiNodeView {
            dh: self.dh.clone_ref(py),
            ids: Some(ids),
        }
    }
}

#[pymethods]
impl PyDiNodeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        let dh = self.dh.borrow(py);
        self.effective_ids(&dh).len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let dh = self.dh.borrow(py);
        let items: Vec<Py<PyAny>> = self
            .effective_ids(&dh)
            .iter()
            .filter_map(|c| dh.node_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(dh);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, n: &Bound<'_, PyAny>) -> bool {
        let dh = self.dh.borrow(py);
        match dh.node_ids.get_key(n) {
            Ok(Some(core)) => self.effective_ids(&dh).iter().any(|i| i == &core),
            _ => false,
        }
    }

    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(other_view) = other.extract::<PyRef<PyDiNodeView>>() else {
            return Ok(false);
        };
        let dh1 = self.dh.borrow(py);
        let dh2 = other_view.dh.borrow(py);
        let mut a: Vec<String> = self
            .effective_ids(&dh1)
            .into_iter()
            .filter_map(|c| {
                dh1.node_ids
                    .get_obj(py, &c)
                    .and_then(|o| o.repr().ok())
                    .map(|r| r.to_string())
            })
            .collect();
        let mut b: Vec<String> = other_view
            .effective_ids(&dh2)
            .into_iter()
            .filter_map(|c| {
                dh2.node_ids
                    .get_obj(py, &c)
                    .and_then(|o| o.repr().ok())
                    .map(|r| r.to_string())
            })
            .collect();
        a.sort();
        b.sort();
        Ok(a == b)
    }

    /// `DH.nodes(bunch)` — a view scoped to `bunch`, ordered by the GLOBAL node order (see
    /// `PyNodeView::__call__`). Raises `IDNotFound` for any `bunch` item that isn't a currently
    /// known node id.
    fn __call__(&self, py: Python<'_>, bunch: &Bound<'_, PyAny>) -> PyResult<PyDiNodeView> {
        let dh = self.dh.borrow(py);
        let mut wanted: Vec<String> = Vec::new();
        for item in bunch.try_iter()? {
            let item = item?;
            let repr = item.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = dh
                .node_ids
                .get_key(&item)?
                .ok_or_else(|| id_not_found(&repr))?;
            wanted.push(core);
        }
        let ids: Vec<String> = dh
            .inner
            .node_ids()
            .into_iter()
            .filter(|i| wanted.contains(i))
            .collect();
        drop(dh);
        Ok(self.sub(py, ids))
    }

    #[pyo3(signature = (view, bunch=None))]
    // XGI API-surface constraint: `from_view` is a deliberate port of XGI's Python-facing
    // classmethod name (views.py:512-545); renaming would break conformance.
    #[allow(clippy::wrong_self_convention)]
    fn from_view(
        &self,
        py: Python<'_>,
        view: PyRef<'_, PyDiNodeView>,
        bunch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyDiNodeView> {
        match bunch {
            Some(b) => view.__call__(py, b),
            None => Ok(PyDiNodeView {
                dh: view.dh.clone_ref(py),
                ids: None,
            }),
        }
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

    /// `DH.nodes.memberships(n=None)` -> a SET of edge ids the node belongs to (head or tail),
    /// or a dict of sets for all nodes in this view when `n` is omitted.
    #[pyo3(signature = (n=None))]
    fn memberships(&self, py: Python<'_>, n: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        match n {
            Some(n) => {
                let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
                let set = PySet::empty(py)?;
                for e in dh.inner.memberships(&core).unwrap_or_default() {
                    if let Some(o) = dh.edge_ids.get_obj(py, &e) {
                        set.add(o)?;
                    }
                }
                Ok(set.into_any().unbind())
            }
            None => {
                let d = PyDict::new(py);
                for nid in self.effective_ids(&dh) {
                    let Some(obj) = dh.node_ids.get_obj(py, &nid) else {
                        continue;
                    };
                    let set = PySet::empty(py)?;
                    for e in dh.inner.memberships(&nid).unwrap_or_default() {
                        if let Some(o) = dh.edge_ids.get_obj(py, &e) {
                            set.add(o)?;
                        }
                    }
                    d.set_item(obj, set)?;
                }
                Ok(d.into_any().unbind())
            }
        }
    }

    /// `DH.nodes.dimemberships(n=None)` -> `(in_edges, out_edges)` as a pair of SETs, or a dict
    /// of pairs for all nodes in this view when `n` is omitted.
    #[pyo3(signature = (n=None))]
    fn dimemberships(&self, py: Python<'_>, n: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        let pair_for = |py: Python<'_>, core: &str| -> PyResult<Py<PyTuple>> {
            let (in_e, out_e) = dh.inner.dimemberships(core).unwrap_or_default();
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
        };
        match n {
            Some(n) => {
                let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))?;
                Ok(pair_for(py, &core)?.into_any())
            }
            None => {
                let d = PyDict::new(py);
                for nid in self.effective_ids(&dh) {
                    let Some(obj) = dh.node_ids.get_obj(py, &nid) else {
                        continue;
                    };
                    d.set_item(obj, pair_for(py, &nid)?)?;
                }
                Ok(d.into_any().unbind())
            }
        }
    }

    /// Sub-view of degree-0 nodes. XGI parity: `DH.nodes.isolates()`.
    fn isolates(&self, py: Python<'_>) -> PyDiNodeView {
        let dh = self.dh.borrow(py);
        let all_isolates = dh.inner.isolate_node_ids();
        let ids: Vec<String> = all_isolates
            .into_iter()
            .filter(|i| self.effective_ids(&dh).contains(i))
            .collect();
        drop(dh);
        self.sub(py, ids)
    }

    #[getter]
    fn degree(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for nid in &ids {
            if let Some(obj) = dh.node_ids.get_obj(py, nid) {
                items.push((obj.unbind(), dh.inner.degree(nid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: Some((self.dh.clone_ref(py), ids)),
            edge_source: None,
        })
    }

    /// `DH.nodes.in_degree` — XGI parity: `stats/dinodestats.py::in_degree`.
    #[getter]
    fn in_degree(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for nid in &ids {
            if let Some(obj) = dh.node_ids.get_obj(py, nid) {
                items.push((obj.unbind(), dh.inner.in_degree(nid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: None,
        })
    }

    /// `DH.nodes.out_degree` — XGI parity: `stats/dinodestats.py::out_degree`.
    #[getter]
    fn out_degree(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for nid in &ids {
            if let Some(obj) = dh.node_ids.get_obj(py, nid) {
                items.push((obj.unbind(), dh.inner.out_degree(nid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: None,
        })
    }

    /// `DH.nodes.filterby(stat, val, mode="eq")` — `"degree"`/`"in_degree"`/`"out_degree"` are
    /// built in; any other stat name falls back to a node-attr lookup. `mode` accepts a string or
    /// a `mode(value, val) -> bool` callable.
    #[pyo3(signature = (stat, val, mode=None))]
    fn filterby(
        &self,
        py: Python<'_>,
        stat: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyDiNodeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let dh = self.dh.borrow(py);
        let mut ids = Vec::new();
        for nid in self.effective_ids(&dh) {
            let pv: Bound<'_, PyAny> = match stat {
                "degree" => (dh.inner.degree(&nid) as i64).into_pyobject(py)?.into_any(),
                "in_degree" => (dh.inner.in_degree(&nid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                "out_degree" => (dh.inner.out_degree(&nid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                other => match dh
                    .inner
                    .node_attrs(&nid)
                    .and_then(|a| a.get(other))
                    .cloned()
                {
                    Some(v) => json_to_py(py, &v)?,
                    None => continue,
                },
            };
            if crate::views::eval_mode(&mode_obj, &pv, &val)? {
                ids.push(nid);
            }
        }
        drop(dh);
        Ok(self.sub(py, ids))
    }

    /// `DH.nodes.filterby_attr(attr, val, mode="eq", missing=None)`.
    #[pyo3(signature = (attr, val, mode=None, missing=None))]
    fn filterby_attr(
        &self,
        py: Python<'_>,
        attr: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
        missing: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyDiNodeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let dh = self.dh.borrow(py);
        let mut ids = Vec::new();
        for nid in self.effective_ids(&dh) {
            let v = dh.inner.node_attrs(&nid).and_then(|a| a.get(attr)).cloned();
            let pv = match v {
                Some(v) => json_to_py(py, &v)?,
                None => match &missing {
                    Some(m) => m.clone(),
                    None => continue,
                },
            };
            if crate::views::eval_mode(&mode_obj, &pv, &val)? {
                ids.push(nid);
            }
        }
        drop(dh);
        Ok(self.sub(py, ids))
    }
}

// ---------------------------------------------------------------------------------------------
// DiEdgeView
// ---------------------------------------------------------------------------------------------

#[pyclass(name = "DiEdgeView", module = "hypergraph_rs")]
pub struct PyDiEdgeView {
    pub dh: Py<PyDiHypergraph>,
    pub ids: Option<Vec<String>>,
}

impl PyDiEdgeView {
    fn effective_ids(&self, dh: &PyDiHypergraph) -> Vec<String> {
        self.ids.clone().unwrap_or_else(|| dh.inner.edge_ids())
    }

    fn sub(&self, py: Python<'_>, ids: Vec<String>) -> PyDiEdgeView {
        PyDiEdgeView {
            dh: self.dh.clone_ref(py),
            ids: Some(ids),
        }
    }

    fn is_dict_dtype(py: Python<'_>, dtype: Option<&Bound<'_, PyAny>>) -> PyResult<bool> {
        match dtype {
            None => Ok(false),
            Some(d) => {
                if d.is(py.get_type::<PyDict>()) {
                    Ok(true)
                } else if d.is(py.get_type::<PyList>()) {
                    Ok(false)
                } else {
                    Err(xgi_error(format!("Unrecognized dtype {}", d.repr()?)))
                }
            }
        }
    }
}

#[pymethods]
impl PyDiEdgeView {
    fn __len__(&self, py: Python<'_>) -> usize {
        let dh = self.dh.borrow(py);
        self.effective_ids(&dh).len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyIterator>> {
        let dh = self.dh.borrow(py);
        let items: Vec<Py<PyAny>> = self
            .effective_ids(&dh)
            .iter()
            .filter_map(|c| dh.edge_ids.get_obj(py, c).map(|o| o.unbind()))
            .collect();
        drop(dh);
        Ok(PyList::new(py, items)?.into_any().try_iter()?.unbind())
    }

    fn __contains__(&self, py: Python<'_>, e: &Bound<'_, PyAny>) -> bool {
        let dh = self.dh.borrow(py);
        match dh.edge_ids.get_key(e) {
            Ok(Some(core)) => self.effective_ids(&dh).iter().any(|i| i == &core),
            _ => false,
        }
    }

    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(other_view) = other.extract::<PyRef<PyDiEdgeView>>() else {
            return Ok(false);
        };
        let dh1 = self.dh.borrow(py);
        let dh2 = other_view.dh.borrow(py);
        let mut a: Vec<String> = self
            .effective_ids(&dh1)
            .into_iter()
            .filter_map(|c| {
                dh1.edge_ids
                    .get_obj(py, &c)
                    .and_then(|o| o.repr().ok())
                    .map(|r| r.to_string())
            })
            .collect();
        let mut b: Vec<String> = other_view
            .effective_ids(&dh2)
            .into_iter()
            .filter_map(|c| {
                dh2.edge_ids
                    .get_obj(py, &c)
                    .and_then(|o| o.repr().ok())
                    .map(|r| r.to_string())
            })
            .collect();
        a.sort();
        b.sort();
        Ok(a == b)
    }

    /// `DH.edges(bunch)` — see `PyDiNodeView::__call__`.
    fn __call__(&self, py: Python<'_>, bunch: &Bound<'_, PyAny>) -> PyResult<PyDiEdgeView> {
        let dh = self.dh.borrow(py);
        let mut wanted: Vec<String> = Vec::new();
        for item in bunch.try_iter()? {
            let item = item?;
            let repr = item.repr().map(|r| r.to_string()).unwrap_or_default();
            let core = dh
                .edge_ids
                .get_key(&item)?
                .ok_or_else(|| id_not_found(&repr))?;
            wanted.push(core);
        }
        let ids: Vec<String> = dh
            .inner
            .edge_ids()
            .into_iter()
            .filter(|i| wanted.contains(i))
            .collect();
        drop(dh);
        Ok(self.sub(py, ids))
    }

    #[pyo3(signature = (view, bunch=None))]
    // XGI API-surface constraint: `from_view` is a deliberate port of XGI's Python-facing
    // classmethod name (views.py:512-545); renaming would break conformance.
    #[allow(clippy::wrong_self_convention)]
    fn from_view(
        &self,
        py: Python<'_>,
        view: PyRef<'_, PyDiEdgeView>,
        bunch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyDiEdgeView> {
        match bunch {
            Some(b) => view.__call__(py, b),
            None => Ok(PyDiEdgeView {
                dh: view.dh.clone_ref(py),
                ids: None,
            }),
        }
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

    /// `DH.edges.dimembers(e=None, dtype=list)` -> `(tail, head)` pair(s) of SETs.
    #[pyo3(signature = (e=None, dtype=None))]
    fn dimembers(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        let pair_for = |py: Python<'_>, core: &str| -> PyResult<Py<PyTuple>> {
            let (tail, head) = dh.inner.dimembers(core).unwrap_or_default();
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
        };
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                Ok(pair_for(py, &core)?.into_any())
            }
            None => {
                if Self::is_dict_dtype(py, dtype)? {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&dh) {
                        let Some(obj) = dh.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        d.set_item(obj, pair_for(py, &eid)?)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&dh) {
                        list.append(pair_for(py, &eid)?)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    /// `DH.edges.members(e=None, dtype=list)` -> `tail ∪ head` set(s).
    #[pyo3(signature = (e=None, dtype=None))]
    fn members(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        let set_for = |py: Python<'_>, core: &str| -> PyResult<Py<PySet>> {
            let set = PySet::empty(py)?;
            for n in dh.inner.members(core).unwrap_or_default() {
                if let Some(o) = dh.node_ids.get_obj(py, &n) {
                    set.add(o)?;
                }
            }
            Ok(set.unbind())
        };
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                if !self.effective_ids(&dh).contains(&core) {
                    return Err(id_not_found(&repr));
                }
                Ok(set_for(py, &core)?.into_any())
            }
            None => {
                if Self::is_dict_dtype(py, dtype)? {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&dh) {
                        let Some(obj) = dh.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        d.set_item(obj, set_for(py, &eid)?)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&dh) {
                        list.append(set_for(py, &eid)?)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    /// `DH.edges.tail(e=None, dtype=list)` -> the tail (sender) node ids.
    #[pyo3(signature = (e=None, dtype=None))]
    fn tail(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        let set_for = |py: Python<'_>, core: &str| -> PyResult<Py<PySet>> {
            let set = PySet::empty(py)?;
            for n in dh.inner.tail(core).unwrap_or_default() {
                if let Some(o) = dh.node_ids.get_obj(py, &n) {
                    set.add(o)?;
                }
            }
            Ok(set.unbind())
        };
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                if !self.effective_ids(&dh).contains(&core) {
                    return Err(id_not_found(&repr));
                }
                Ok(set_for(py, &core)?.into_any())
            }
            None => {
                if Self::is_dict_dtype(py, dtype)? {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&dh) {
                        let Some(obj) = dh.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        d.set_item(obj, set_for(py, &eid)?)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&dh) {
                        list.append(set_for(py, &eid)?)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    /// `DH.edges.head(e=None, dtype=list)` -> the head (receiver) node ids.
    #[pyo3(signature = (e=None, dtype=None))]
    fn head(
        &self,
        py: Python<'_>,
        e: Option<&Bound<'_, PyAny>>,
        dtype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let dh = self.dh.borrow(py);
        let set_for = |py: Python<'_>, core: &str| -> PyResult<Py<PySet>> {
            let set = PySet::empty(py)?;
            for n in dh.inner.head(core).unwrap_or_default() {
                if let Some(o) = dh.node_ids.get_obj(py, &n) {
                    set.add(o)?;
                }
            }
            Ok(set.unbind())
        };
        match e {
            Some(e) => {
                let repr = e.repr().map(|r| r.to_string()).unwrap_or_default();
                let core = dh.edge_ids.get_key(e)?.ok_or_else(|| id_not_found(&repr))?;
                if !self.effective_ids(&dh).contains(&core) {
                    return Err(id_not_found(&repr));
                }
                Ok(set_for(py, &core)?.into_any())
            }
            None => {
                if Self::is_dict_dtype(py, dtype)? {
                    let d = PyDict::new(py);
                    for eid in self.effective_ids(&dh) {
                        let Some(obj) = dh.edge_ids.get_obj(py, &eid) else {
                            continue;
                        };
                        d.set_item(obj, set_for(py, &eid)?)?;
                    }
                    Ok(d.into_any().unbind())
                } else {
                    let list = PyList::empty(py);
                    for eid in self.effective_ids(&dh) {
                        list.append(set_for(py, &eid)?)?;
                    }
                    Ok(list.into_any().unbind())
                }
            }
        }
    }

    #[getter]
    fn size(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.size(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::Size)),
        })
    }

    #[getter]
    fn order(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.order(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::Order)),
        })
    }

    /// `DH.edges.tail_size` — XGI parity: `stats/diedgestats.py::tail_size`.
    #[getter]
    fn tail_size(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.tail_size(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::TailSize)),
        })
    }

    /// `DH.edges.tail_order` — XGI parity: `stats/diedgestats.py::tail_order`.
    #[getter]
    fn tail_order(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.tail_order(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::TailOrder)),
        })
    }

    /// `DH.edges.head_size` — XGI parity: `stats/diedgestats.py::head_size`.
    #[getter]
    fn head_size(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.head_size(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::HeadSize)),
        })
    }

    /// `DH.edges.head_order` — XGI parity: `stats/diedgestats.py::head_order`.
    #[getter]
    fn head_order(&self, py: Python<'_>) -> PyResult<DiStat> {
        let dh = self.dh.borrow(py);
        let ids = self.effective_ids(&dh);
        let mut items = Vec::new();
        for eid in &ids {
            if let Some(obj) = dh.edge_ids.get_obj(py, eid) {
                items.push((obj.unbind(), dh.inner.head_order(eid) as f64));
            }
        }
        Ok(DiStat {
            items,
            degree_source: None,
            edge_source: Some((self.dh.clone_ref(py), ids, DiEdgeStatKind::HeadOrder)),
        })
    }

    /// `DH.edges.filterby(stat, val, mode="eq")` — `"size"`/`"order"`/`"tail_size"`/`"head_size"`
    /// are built in; any other stat name falls back to an edge-attr lookup. `mode` accepts a
    /// string or a `mode(value, val) -> bool` callable.
    #[pyo3(signature = (stat, val, mode=None))]
    fn filterby(
        &self,
        py: Python<'_>,
        stat: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyDiEdgeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let dh = self.dh.borrow(py);
        let mut ids = Vec::new();
        for eid in self.effective_ids(&dh) {
            let pv: Bound<'_, PyAny> = match stat {
                "size" => (dh.inner.size(&eid) as i64).into_pyobject(py)?.into_any(),
                "order" => (dh.inner.order(&eid) as i64).into_pyobject(py)?.into_any(),
                "tail_size" => (dh.inner.tail_size(&eid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                "head_size" => (dh.inner.head_size(&eid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                "tail_order" => (dh.inner.tail_order(&eid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                "head_order" => (dh.inner.head_order(&eid) as i64)
                    .into_pyobject(py)?
                    .into_any(),
                other => match dh
                    .inner
                    .edge_attrs(&eid)
                    .and_then(|a| a.get(other))
                    .cloned()
                {
                    Some(v) => json_to_py(py, &v)?,
                    None => continue,
                },
            };
            if crate::views::eval_mode(&mode_obj, &pv, &val)? {
                ids.push(eid);
            }
        }
        drop(dh);
        Ok(self.sub(py, ids))
    }

    /// `DH.edges.filterby_attr(attr, val, mode="eq", missing=None)`.
    #[pyo3(signature = (attr, val, mode=None, missing=None))]
    fn filterby_attr(
        &self,
        py: Python<'_>,
        attr: &str,
        val: Bound<'_, PyAny>,
        mode: Option<Bound<'_, PyAny>>,
        missing: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyDiEdgeView> {
        let mode_obj = mode.unwrap_or_else(|| PyString::new(py, "eq").into_any());
        let dh = self.dh.borrow(py);
        let mut ids = Vec::new();
        for eid in self.effective_ids(&dh) {
            let v = dh.inner.edge_attrs(&eid).and_then(|a| a.get(attr)).cloned();
            let pv = match v {
                Some(v) => json_to_py(py, &v)?,
                None => match &missing {
                    Some(m) => m.clone(),
                    None => continue,
                },
            };
            if crate::views::eval_mode(&mode_obj, &pv, &val)? {
                ids.push(eid);
            }
        }
        drop(dh);
        Ok(self.sub(py, ids))
    }
}

/// Minimal `Stat`-like proxy for `DiHypergraph`'s `.nodes.degree` / `.edges.size` — mirrors
/// `stats::PyNodeStat`/`PyEdgeStat`'s `.asdict()`/`.aslist()`/subscript surface, but pre-computed
/// (not lazy) since `DiHypergraph`'s stat surface is small (Task 6 scope: just degree/size).
/// `degree_source`, when present, lets `__call__(order=, weight=)` re-parametrize a degree stat
/// (XGI parity: `Stat.__call__`, `stats/dinodestats.py::degree`), matching `stats::PyNodeStat`.
/// Which per-edge quantity an edge-side `DiStat` (`size`/`order`/`tail_size`/`tail_order`/
/// `head_size`/`head_order`) computes, so `__call__(degree=…)` (XGI parity:
/// `stats/diedgestats.py::order`/`size`, which filter by *member node degree*) can recompute it.
#[derive(Clone, Copy)]
enum DiEdgeStatKind {
    Size,
    Order,
    TailSize,
    TailOrder,
    HeadSize,
    HeadOrder,
}

#[pyclass(name = "DiStat", module = "hypergraph_rs")]
pub struct DiStat {
    items: Vec<(Py<PyAny>, f64)>,
    degree_source: Option<(Py<PyDiHypergraph>, Vec<String>)>,
    edge_source: Option<(Py<PyDiHypergraph>, Vec<String>, DiEdgeStatKind)>,
}

#[pymethods]
impl DiStat {
    #[pyo3(signature = (order=None, weight=None, degree=None))]
    fn __call__(
        &self,
        py: Python<'_>,
        order: Option<usize>,
        weight: Option<String>,
        degree: Option<usize>,
    ) -> PyResult<DiStat> {
        if let Some((dh, ids, kind)) = &self.edge_source {
            let dh_ref = dh.borrow(py);
            let mut items = Vec::new();
            for eid in ids {
                let Some(obj) = dh_ref.edge_ids.get_obj(py, eid) else {
                    continue;
                };
                // XGI parity: `stats/diedgestats.py::order`/`size` filter by *member node
                // degree* (not edge order) — `degree=None` is the plain size/order.
                let count_of = |members: Vec<String>| -> usize {
                    match degree {
                        None => members.len(),
                        Some(target) => members
                            .iter()
                            .filter(|n| dh_ref.inner.degree(n) == target)
                            .count(),
                    }
                };
                let val = match kind {
                    DiEdgeStatKind::Size => {
                        count_of(dh_ref.inner.members(eid).unwrap_or_default()) as f64
                    }
                    DiEdgeStatKind::Order => {
                        count_of(dh_ref.inner.members(eid).unwrap_or_default()) as f64 - 1.0
                    }
                    DiEdgeStatKind::TailSize => {
                        count_of(dh_ref.inner.tail(eid).unwrap_or_default()) as f64
                    }
                    DiEdgeStatKind::TailOrder => {
                        count_of(dh_ref.inner.tail(eid).unwrap_or_default()) as f64 - 1.0
                    }
                    DiEdgeStatKind::HeadSize => {
                        count_of(dh_ref.inner.head(eid).unwrap_or_default()) as f64
                    }
                    DiEdgeStatKind::HeadOrder => {
                        count_of(dh_ref.inner.head(eid).unwrap_or_default()) as f64 - 1.0
                    }
                };
                items.push((obj.unbind(), val));
            }
            return Ok(DiStat {
                items,
                degree_source: None,
                edge_source: Some((dh.clone_ref(py), ids.clone(), *kind)),
            });
        }
        let Some((dh, ids)) = &self.degree_source else {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "this Stat does not accept parameters",
            ));
        };
        let dh_ref = dh.borrow(py);
        let mut items = Vec::new();
        for nid in ids {
            let Some(obj) = dh_ref.node_ids.get_obj(py, nid) else {
                continue;
            };
            let memberships = dh_ref.inner.memberships(nid).unwrap_or_default();
            let val: f64 = match (order, &weight) {
                (None, None) => memberships.len() as f64,
                (None, Some(w)) => memberships
                    .iter()
                    .map(|e| {
                        dh_ref
                            .inner
                            .edge_attrs(e)
                            .and_then(|a| a.get(w))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0)
                    })
                    .sum(),
                (Some(k), None) => memberships
                    .iter()
                    .filter(|e| dh_ref.inner.size(e) == k + 1)
                    .count() as f64,
                (Some(k), Some(w)) => memberships
                    .iter()
                    .filter(|e| dh_ref.inner.size(e) == k + 1)
                    .map(|e| {
                        dh_ref
                            .inner
                            .edge_attrs(e)
                            .and_then(|a| a.get(w))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0)
                    })
                    .sum(),
            };
            items.push((obj.unbind(), val));
        }
        Ok(DiStat {
            items,
            degree_source: Some((dh.clone_ref(py), ids.clone())),
            edge_source: None,
        })
    }

    fn asdict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        for (k, v) in &self.items {
            d.set_item(k.clone_ref(py), numeric(py, *v)?)?;
        }
        Ok(d.unbind())
    }

    fn aslist(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let list = PyList::empty(py);
        for (_, v) in &self.items {
            list.append(numeric(py, *v)?)?;
        }
        Ok(list.unbind())
    }

    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let repr = key.repr().map(|r| r.to_string()).unwrap_or_default();
        for (k, v) in &self.items {
            if k.bind(py).eq(key).unwrap_or(false) {
                return numeric(py, *v);
            }
        }
        Err(id_not_found(&repr))
    }
}

/// `f64` -> Python `int` when integral, else `float` (mirrors `stats::PyNodeStat`'s degree
/// numeric coercion — `H.degree()` values are ints unless `weight=` introduces a fraction).
fn numeric(py: Python<'_>, v: f64) -> PyResult<Py<PyAny>> {
    if v.fract() == 0.0 {
        Ok((v as i64).into_pyobject(py)?.into_any().unbind())
    } else {
        Ok(v.into_pyobject(py)?.into_any().unbind())
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDiHypergraph>()?;
    m.add_class::<PyDiNodeView>()?;
    m.add_class::<PyDiEdgeView>()?;
    m.add_class::<DiStat>()?;
    let func = pyo3::wrap_pyfunction!(reconstruct_dihypergraph, m)?;
    func.setattr("__module__", "hypergraph_rs")?;
    m.add_function(func)?;
    Ok(())
}
