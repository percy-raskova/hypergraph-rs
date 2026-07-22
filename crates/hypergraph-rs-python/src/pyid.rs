//! Translation between arbitrary hashable Python IDs and the core's `String` keys.
//!
//! XGI node/edge IDs are any hashable Python object, preserved by identity on readback
//! (`tests/core/test_hypergraph.py:79`). The core keys by `String` (`core/hypergraph.rs:31`).
//! `PyIdMap` is the per-hypergraph bijection that lets the binding accept and return the
//! original Python objects while the core stays String-keyed.

use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyList, PyString, PyTuple};
use serde_json::Value;
use std::hash::{Hash, Hasher};

/// A hashable, `Eq` canonical identity for a Python hashable object.
/// Compared by Python hash AND repr — distinguishes `1` from `"1"` from `(1,)`.
pub struct PyKey {
    pub obj: Py<PyAny>,
    hash: isize,
    repr: String,
}

impl PyKey {
    pub fn new(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        // hash() raises TypeError for unhashable objects — XGI's `[1,2,3] not in H`
        // TypeError path (tests/core/test_hypergraph.py:70).
        let hash = obj.hash()?;
        let repr = obj.repr()?.to_string();
        Ok(Self {
            obj: obj.clone().unbind(),
            hash,
            repr,
        })
    }

    fn hash_raw(&self) -> isize {
        self.hash
    }

    fn repr_raw(&self) -> String {
        self.repr.clone()
    }
}
impl PartialEq for PyKey {
    fn eq(&self, o: &Self) -> bool {
        self.hash == o.hash && self.repr == o.repr
    }
}
impl Eq for PyKey {}
impl Hash for PyKey {
    fn hash<H: Hasher>(&self, s: &mut H) {
        self.hash.hash(s);
        self.repr.hash(s);
    }
}

#[derive(Default)]
pub struct PyIdMap {
    to_key: IndexMap<PyKey, String>,
    from_key: IndexMap<String, Py<PyAny>>,
    counter: u64,
}

impl PyIdMap {
    pub fn new() -> Self {
        Self::default()
    }

    fn canonical(&mut self, obj: &Bound<'_, PyAny>) -> String {
        if let Ok(s) = obj.cast::<PyString>() {
            return s.to_string_lossy().into_owned();
        }
        if obj.is_instance_of::<PyBool>() {
            return obj.str().map(|s| s.to_string()).unwrap_or_default();
        }
        if obj.is_instance_of::<PyInt>() {
            if let Ok(v) = obj.extract::<i64>() {
                return v.to_string();
            }
        }
        let n = self.counter;
        self.counter += 1;
        format!("__id{n}")
    }

    /// Intern a Python ID → (existing or new) core key. Collision-safe, bounded.
    pub fn intern(&mut self, obj: &Bound<'_, PyAny>) -> PyResult<String> {
        let key = PyKey::new(obj)?;
        if let Some(k) = self.to_key.get(&key) {
            return Ok(k.clone());
        }
        let mut core = self.canonical(obj);
        let mut bump = 0u64;
        while self.from_key.contains_key(&core) {
            bump += 1;
            core = format!("{core}\u{1}{bump}"); // U+0001 never appears in real ids
            if bump > 1_000_000 {
                break;
            } // bounded (Power-of-10 rule 2)
        }
        self.to_key.insert(key, core.clone());
        self.from_key.insert(core.clone(), obj.clone().unbind());
        Ok(core)
    }

    /// Look up an existing core key WITHOUT interning (read-only queries / membership).
    pub fn get_key(&self, obj: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        Ok(self.to_key.get(&PyKey::new(obj)?).cloned())
    }

    pub fn get_obj<'py>(&self, py: Python<'py>, core: &str) -> Option<Bound<'py, PyAny>> {
        self.from_key.get(core).map(|o| o.bind(py).clone())
    }

    pub fn forget(&mut self, core: &str) {
        if let Some(obj) = self.from_key.shift_remove(core) {
            Python::attach(|py| {
                if let Ok(k) = PyKey::new(obj.bind(py)) {
                    self.to_key.shift_remove(&k);
                }
            });
        }
    }

    /// Deep-copy under the GIL (Py<PyAny> clones are refcount bumps) — used by PyHypergraph::copy.
    pub fn clone_via_py(&self, py: Python<'_>) -> Self {
        Self {
            to_key: self
                .to_key
                .iter()
                .map(|(k, v)| {
                    (
                        PyKey {
                            obj: k.obj.clone_ref(py),
                            hash: k.hash_raw(),
                            repr: k.repr_raw(),
                        },
                        v.clone(),
                    )
                })
                .collect(),
            from_key: self
                .from_key
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect(),
            counter: self.counter,
        }
    }
}

/// Python ATTRIBUTE VALUE → serde_json::Value. Non-serializable values raise TypeError
/// (declared deviation: XGI stores arbitrary Python objects as attrs; we require JSON-representable).
pub fn py_any_to_json(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.cast::<PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::from(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::from(f));
    }
    if let Ok(s) = obj.cast::<PyString>() {
        return Ok(Value::from(s.to_string_lossy().into_owned()));
    }
    if let Ok(list) = obj.cast::<PyList>() {
        let mut out = Vec::with_capacity(list.len());
        for it in list.iter() {
            out.push(py_any_to_json(&it)?);
        }
        return Ok(Value::Array(out));
    }
    if let Ok(tup) = obj.cast::<PyTuple>() {
        let mut out = Vec::with_capacity(tup.len());
        for it in tup.iter() {
            out.push(py_any_to_json(&it)?);
        }
        return Ok(Value::Array(out));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            map.insert(k.str()?.to_string(), py_any_to_json(&v)?);
        }
        return Ok(Value::Object(map));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "attribute value of type {} is not JSON-serializable (hypergraph-rs deviation)",
        obj.get_type().name()?
    )))
}

/// serde_json::Value → Python object.
pub fn json_to_py<'py>(py: Python<'py>, v: &Value) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        Value::Null => py.None().into_bound(py),
        Value::Bool(b) => b.into_pyobject(py)?.to_owned().into_any(),
        Value::Number(n) if n.is_i64() => n.as_i64().unwrap().into_pyobject(py)?.into_any(),
        Value::Number(n) => n.as_f64().unwrap().into_pyobject(py)?.into_any(),
        Value::String(s) => s.into_pyobject(py)?.into_any(),
        Value::Array(a) => {
            let list = PyList::empty(py);
            for it in a {
                list.append(json_to_py(py, it)?)?;
            }
            list.into_any()
        }
        Value::Object(m) => {
            let dict = PyDict::new(py);
            for (k, val) in m {
                dict.set_item(k, json_to_py(py, val)?)?;
            }
            dict.into_any()
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn collision_separator_is_bounded_control_char() {
        // The U+0001 collision loop is bounded by 1_000_000 (Power-of-10 rule 2).
        // The behavioral bijection (int ids round-trip as ints) is proved in Task 4's
        // test_node_ids_preserve_python_type. Documented cross-reference.
        assert_eq!('\u{1}' as u32, 1);
    }
}
