//! `generators` bindings (plan Task 10) — seeded, integer node ids.
//!
//! `tests/generators/`. XGI generators take `seed=` and produce **integer** node ids
//! (`xgi.random_hypergraph(10, [0.2], seed=1)`). Every generator that assigns its own node
//! set (classic/lattice/simple/random/uniform-Erdos-Renyi/simplicial) returns a fresh
//! `PyHypergraph`/`PySimplicialComplex` built via `from_core_with_int_ids` (node keys
//! `"0".."n-1"` -> Python ints). `complement` reuses the input hypergraph's existing node
//! id map (same node set, XGI parity) and mints fresh int ids only for the new edges.
//! `chung_lu_hypergraph` / `uniform_configuration_model` take arbitrary-hashable
//! node-id-keyed `dict`s (XGI parity) — those user ids are interned directly, not remapped
//! to ints.
//!
//! **Deferred stubs** (no Phase 0-6 core support): `empty_simplicial_complex`,
//! `empty_dihypergraph`, `node_swap`, `shuffle_hyperedges`, `flag_complex`, `flag_complex_d2`,
//! `random_flag_complex`, `random_flag_complex_d2`, `dcsbm_hypergraph`, `uniform_HSBM`,
//! `uniform_HPPM` — left unbound; the Python skeleton (`python/hypergraph_rs/generators/__init__.py`)
//! already falls back to `make_deferred` for any symbol this module does not register.

use indexmap::IndexMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use hypergraph_rs::generators as gen;
use hypergraph_rs::generators::uniform::PType;

use crate::hypergraph::{int_or_str_pyobj, PyHypergraph};
use crate::pyid::PyIdMap;
use crate::simplicial::PySimplicialComplex;

fn parse_p_type(p_type: &str) -> PyResult<PType> {
    match p_type {
        "prob" => Ok(PType::Prob),
        "degree" => Ok(PType::Degree),
        _ => Err(PyValueError::new_err(format!(
            "invalid p_type {p_type:?}; must be \"prob\" or \"degree\""
        ))),
    }
}

/// `xgi.empty_hypergraph()`.
#[pyfunction]
fn empty_hypergraph(py: Python<'_>) -> PyResult<PyHypergraph> {
    PyHypergraph::from_core_with_int_ids(py, gen::empty_hypergraph())
}

/// `xgi.trivial_hypergraph(n=1)`.
#[pyfunction]
#[pyo3(signature = (n=1))]
fn trivial_hypergraph(py: Python<'_>, n: usize) -> PyResult<PyHypergraph> {
    PyHypergraph::from_core_with_int_ids(py, gen::trivial_hypergraph(n))
}

/// `xgi.complete_hypergraph(N, order=None, max_order=None, include_singletons=False)`. XGI's
/// own parameter is literally `N` (capitalized) — `generators/classic.py`'s
/// `complete_hypergraph(N, order=None, ...)` — so the oracle tests call it as a keyword
/// (`complete_hypergraph(N=N, order=1)`); the Rust arg name matches for parity.
#[pyfunction]
#[pyo3(signature = (N, order=None, max_order=None, include_singletons=false))]
#[allow(non_snake_case)]
fn complete_hypergraph(
    py: Python<'_>,
    N: usize,
    order: Option<usize>,
    max_order: Option<usize>,
    include_singletons: bool,
) -> PyResult<PyHypergraph> {
    let core = gen::complete_hypergraph(N, order, max_order, include_singletons)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.complement(H)` — same node set as `H` (XGI parity), fresh int edge ids.
#[pyfunction]
fn complement(py: Python<'_>, h: &PyHypergraph) -> PyResult<PyHypergraph> {
    let core = gen::complement(&h.inner);
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids: h.node_ids.clone_via_py(py),
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.ring_lattice(n, d, k, l, with_positions=False)`. XGI parity (`generators/lattice.py:15-54`): `k < 0` raises
/// `ValueError`; `k < 2` warns "This creates a completely disconnected hypergraph!" (and the
/// core's `k / 2 == 0` loop naturally produces zero edges); XGI additionally warns for odd `k`
/// (not evenly splittable into left/right neighbors) — reproduced here since the core has no
/// warnings concept.
#[pyfunction]
#[pyo3(signature = (n, d, k, l, with_positions=false))]
fn ring_lattice(
    py: Python<'_>,
    n: usize,
    d: usize,
    k: i64,
    l: usize,
    with_positions: bool,
) -> PyResult<PyHypergraph> {
    if k < 0 {
        return Err(PyValueError::new_err("Invalid k value!"));
    }
    let k = k as usize;
    if k < 2 {
        py.import("warnings")?.call_method1(
            "warn",
            ("This creates a completely disconnected hypergraph!",),
        )?;
    } else if !k.is_multiple_of(2) {
        py.import("warnings")?
            .call_method1("warn", ("k should be a multiple of 2.",))?;
    }
    let core = gen::ring_lattice(n, d, k, l, with_positions)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.generators.lattice._ring_lattice_edges(n, d, k, l)`: the lattice's edges as lists of
/// node numbers, in edge-id order. Like XGI's comprehension, `n == 0` gives an empty list (the
/// loop over `range(0)` never reaches `% n`).
#[pyfunction]
fn _ring_lattice_edges(n: usize, d: usize, k: usize, l: usize) -> Vec<Vec<usize>> {
    gen::ring_lattice_edges(n, d, k, l)
}

/// `xgi.star_clique(n_star, n_clique, d_max)`. XGI parity (`generators/simple.py:50-57`):
/// negative inputs raise `ValueError` (not a Rust-level `usize` overflow) — `i64` params so a
/// negative Python `int` converts cleanly instead of erroring in argument extraction.
#[pyfunction]
fn star_clique(py: Python<'_>, n_star: i64, n_clique: i64, d_max: i64) -> PyResult<PyHypergraph> {
    if n_star <= 0 {
        return Err(PyValueError::new_err("n_star must be an integer > 0."));
    }
    if n_clique <= 0 {
        return Err(PyValueError::new_err("n_clique must be an integer > 0."));
    }
    if d_max < 0 {
        return Err(PyValueError::new_err("d_max must be an integer >= 0."));
    }
    if d_max > n_clique - 1 {
        return Err(PyValueError::new_err("d_max must be <= n_clique - 1."));
    }
    let core = gen::star_clique(n_star as usize, n_clique as usize, d_max as usize)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.sunflower(l, c, m)`. XGI parity: `generators/simple.py:107-108` (`m < c` -> `ValueError`).
#[pyfunction]
fn sunflower(py: Python<'_>, l: usize, c: usize, m: usize) -> PyResult<PyHypergraph> {
    let core = gen::sunflower(l, c, m).map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `ps` accepts a list OR a bare float (XGI parity: `ps : list of float, or float`,
/// `random.py:112-117`) — a bare float means "generate a uniform hypergraph", valid only
/// paired with a scalar `order`.
fn parse_ps_arg(ps: &Bound<'_, PyAny>) -> PyResult<(Vec<f64>, bool)> {
    if let Ok(v) = ps.extract::<Vec<f64>>() {
        return Ok((v, false));
    }
    let scalar: f64 = ps
        .extract()
        .map_err(|_| PyValueError::new_err("ps must be a list of float, or a float"))?;
    Ok((vec![scalar], true))
}

/// XGI parity: `generators/random.py::_check_input_args`. `order=None` requires `ps` to be a
/// list; a bare int `order` requires a bare-float `ps` (`"If order is an int, ps must be a
/// float"`); a sequence `order` must match `ps`'s length (`"The length ps must match the
/// length of order"`).
fn parse_order_arg(
    order: Option<&Bound<'_, PyAny>>,
    ps_len: usize,
    ps_is_scalar: bool,
) -> PyResult<Option<Vec<usize>>> {
    let Some(o) = order else {
        if ps_is_scalar {
            return Err(PyValueError::new_err(
                "If order is not specified, ps must be a list or numpy array!",
            ));
        }
        return Ok(None);
    };
    if let Ok(scalar) = o.extract::<usize>() {
        if !ps_is_scalar {
            return Err(PyValueError::new_err(
                "If order is an int, ps must be a float",
            ));
        }
        return Ok(Some(vec![scalar]));
    }
    if ps_is_scalar {
        return Err(PyValueError::new_err("Invalid entries!"));
    }
    let v: Vec<usize> = o
        .extract()
        .map_err(|_| PyValueError::new_err("Invalid entries!"))?;
    if v.len() != ps_len {
        return Err(PyValueError::new_err(
            "The length ps must match the length of order",
        ));
    }
    Ok(Some(v))
}

/// `xgi.fast_random_hypergraph(n, ps, order=None, seed=None)`.
#[pyfunction]
#[pyo3(signature = (n, ps, order=None, seed=None))]
fn fast_random_hypergraph(
    py: Python<'_>,
    n: usize,
    ps: &Bound<'_, PyAny>,
    order: Option<&Bound<'_, PyAny>>,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let (ps, ps_scalar) = parse_ps_arg(ps)?;
    let order = parse_order_arg(order, ps.len(), ps_scalar)?;
    let core = gen::fast_random_hypergraph(n, &ps, order.as_deref(), seed)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.random_hypergraph(n, ps, order=None, seed=None)`.
#[pyfunction]
#[pyo3(signature = (n, ps, order=None, seed=None))]
fn random_hypergraph(
    py: Python<'_>,
    n: usize,
    ps: &Bound<'_, PyAny>,
    order: Option<&Bound<'_, PyAny>>,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let (ps, ps_scalar) = parse_ps_arg(ps)?;
    let order = parse_order_arg(order, ps.len(), ps_scalar)?;
    let core = gen::random_hypergraph(n, &ps, order.as_deref(), seed)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.chung_lu_hypergraph(k1, k2, seed=None)` — `k1`/`k2` are `{node/edge_id: value}`
/// dicts with arbitrary hashable keys (XGI parity); those ids are interned directly.
#[pyfunction]
#[pyo3(signature = (k1, k2, seed=None))]
fn chung_lu_hypergraph(
    py: Python<'_>,
    k1: &Bound<'_, PyDict>,
    k2: &Bound<'_, PyDict>,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let mut node_ids = PyIdMap::new();
    let mut k1_map: IndexMap<String, f64> = IndexMap::new();
    for (k, v) in k1.iter() {
        let core_id = node_ids.intern(&k)?;
        k1_map.insert(core_id, v.extract::<f64>()?);
    }
    let mut tmp_edge_ids = PyIdMap::new();
    let mut k2_map: IndexMap<String, f64> = IndexMap::new();
    for (k, v) in k2.iter() {
        let core_id = tmp_edge_ids.intern(&k)?;
        k2_map.insert(core_id, v.extract::<f64>()?);
    }
    // XGI parity: `generators/random.py:254-257` warns when the degree/size sequence sums
    // disagree (the algorithm still runs — the core has no warnings concept, so it's raised
    // binding-side before delegating).
    let sum1: f64 = k1_map.values().sum();
    let sum2: f64 = k2_map.values().sum();
    if (sum1 - sum2).abs() > f64::EPSILON {
        py.import("warnings")?.call_method1(
            "warn",
            ("The sum of the degree sequence does not match the sum of the size sequence",),
        )?;
    }
    let core = gen::chung_lu_hypergraph(&k1_map, &k2_map, seed);
    // Only the k2 entries that actually gained a member became real edges.
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        if let Some(obj) = tmp_edge_ids.get_obj(py, &eid) {
            edge_ids.intern(&obj)?;
        }
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.watts_strogatz_hypergraph(n, d, k, l, p, seed=None)`.
#[pyfunction]
#[pyo3(signature = (n, d, k, l, p, seed=None))]
fn watts_strogatz_hypergraph(
    py: Python<'_>,
    n: usize,
    d: usize,
    k: usize,
    l: usize,
    p: f64,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let core = gen::watts_strogatz_hypergraph(n, d, k, l, p, seed)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.uniform_erdos_renyi_model(n, m, p, p_type="prob", multiedges=False, seed=None)` (XGI 1.0's
/// new name for `uniform_erdos_renyi_hypergraph`).
#[pyfunction]
#[pyo3(name = "uniform_erdos_renyi_model")]
#[pyo3(signature = (n, m, p, p_type="prob".to_string(), multiedges=false, seed=None))]
fn uniform_erdos_renyi_hypergraph(
    py: Python<'_>,
    n: usize,
    m: usize,
    p: f64,
    p_type: String,
    multiedges: bool,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let p_type = parse_p_type(&p_type)?;
    let core = gen::uniform_erdos_renyi_hypergraph(n, m, p, p_type, multiedges, seed)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PyHypergraph::from_core_with_int_ids(py, core)
}

/// `xgi.uniform_configuration_model(k, m, seed=None)` (XGI 1.0's new name for
/// `uniform_hypergraph_configuration_model`) — `k` is a `{node_id: degree}` dict with arbitrary
/// hashable keys (XGI parity); edges are freshly auto-id'd (Python ints).
#[pyfunction]
#[pyo3(name = "uniform_configuration_model")]
#[pyo3(signature = (k, m, seed=None))]
fn uniform_hypergraph_configuration_model(
    py: Python<'_>,
    k: &Bound<'_, PyDict>,
    m: usize,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyHypergraph> {
    let seed = crate::seed::extract_seed(seed)?;
    let mut node_ids = PyIdMap::new();
    let mut k_map: IndexMap<String, usize> = IndexMap::new();
    for (key, val) in k.iter() {
        let core_id = node_ids.intern(&key)?;
        k_map.insert(core_id, val.extract::<usize>()?);
    }
    let core = gen::uniform_hypergraph_configuration_model(&k_map, m, seed);
    let mut edge_ids = PyIdMap::new();
    for eid in core.edge_ids() {
        edge_ids.intern(&int_or_str_pyobj(py, &eid)?)?;
    }
    Ok(PyHypergraph {
        inner: core,
        node_ids,
        edge_ids,
        net_attr: serde_json::Map::new(),
    })
}

/// `xgi.random_simplicial_complex(N, ps, seed=None)`.
#[pyfunction]
#[pyo3(signature = (n, ps, seed=None))]
fn random_simplicial_complex(
    py: Python<'_>,
    n: usize,
    ps: Vec<f64>,
    seed: Option<&Bound<'_, PyAny>>,
) -> PyResult<PySimplicialComplex> {
    let seed = crate::seed::extract_seed(seed)?;
    let core = gen::random_simplicial_complex(n, &ps, seed)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    PySimplicialComplex::from_core_with_int_ids(py, core)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(empty_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(trivial_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(complete_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(complement, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(ring_lattice, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(_ring_lattice_edges, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(star_clique, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(sunflower, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(fast_random_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(random_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(chung_lu_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(watts_strogatz_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(uniform_erdos_renyi_hypergraph, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        uniform_hypergraph_configuration_model,
        m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(random_simplicial_complex, m)?)?;
    Ok(())
}
