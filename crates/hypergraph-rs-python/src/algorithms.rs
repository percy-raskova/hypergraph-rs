//! `algorithms` bindings — the non-tensor subset Phase 3 built (plan Task 9).
//!
//! `tests/algorithms/` (centrality, clustering, shortest_path, connected, assortativity,
//! properties, simpliciality). Node-keyed dict results translate keys back to original Python
//! ids via the `PyHypergraph::node_ids` bimap. Deferred (Phase 3 "Deferred", `make_deferred`
//! Python-side stubs — see `python/hypergraph_rs/algorithms/__init__.py`):
//! `h_eigenvector_centrality`, `z_eigenvector_centrality`, `uniform_h_eigenvector_centrality`,
//! `node_edge_centrality`, `line_vector_centrality`, `largest_connected_hypergraph`.
//!
//! **Deviations (documented):**
//! - `degree_assortativity(H, kind="uniform", exact=False, ...)`: the core only implements the
//!   exact (deterministic) path (`degree_assortativity_exact`) — the random-sampling path awaits
//!   the Phase-4 `rand_pcg` pin (`assortativity.rs` module doc). `exact=False` raises
//!   `NotImplementedError` instead of silently sampling.
//! - `clique_eigenvector_centrality(H, tol=1e-6)`: XGI has no `max_iter` (uses scipy `eigsh`'s
//!   unbounded default); the core's hand-rolled power iteration (deviation #5) needs a bound
//!   (Power-of-10 rule 2), so `max_iter` is accepted as an extra keyword, default `1000`.
//! - `equal(H1, H2, compare_edge_ids=True, compare_attrs=True)`: the core `PartialEq` always
//!   compares both edge structure/attrs and node/edge attrs together; the two flags are accepted
//!   for signature compatibility but not independently honored (deviation).
//! - `connected_components` returns a materialized `list[set]`, not XGI's generator of sets.
//! - `density`/`incidence_density` precondition checks (`ValueError`/`XGIError`) are duplicated
//!   here (mirroring the Task 8 `multiorder_laplacian` pattern) so violations raise catchable
//!   Python exceptions instead of an uncatchable `PanicException` from the core's `assert!`s.

use pyo3::exceptions::PyNotImplementedError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};

use hypergraph_rs::algorithms::assortativity::AssortKind;

use crate::errors::{id_not_found, xgi_error};
use crate::hypergraph::PyHypergraph;
use crate::pyid::PyIdMap;

/// Translate a core `IndexMap<String, f64>` (node-keyed) to a `{orig_id: value}` `PyDict`.
fn node_f64_dict<'py>(
    py: Python<'py>,
    map: indexmap::IndexMap<String, f64>,
    ids: &PyIdMap,
) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    for (core, val) in map {
        if let Some(o) = ids.get_obj(py, &core) {
            d.set_item(o, val)?;
        }
    }
    Ok(d.unbind())
}

/// Translate a `Vec<String>` of core node ids to a `PySet` of original Python ids.
fn node_set<'py>(py: Python<'py>, ids: &[String], map: &PyIdMap) -> PyResult<Bound<'py, PySet>> {
    let s = PySet::empty(py)?;
    for cid in ids {
        if let Some(o) = map.get_obj(py, cid) {
            s.add(o)?;
        }
    }
    Ok(s)
}

fn resolve_node(h: &PyHypergraph, n: &Bound<'_, PyAny>) -> PyResult<String> {
    let repr = n.repr().map(|r| r.to_string()).unwrap_or_default();
    h.node_ids.get_key(n)?.ok_or_else(|| id_not_found(&repr))
}

fn parse_assort_kind(kind: &str) -> PyResult<AssortKind> {
    match kind {
        "uniform" => Ok(AssortKind::Uniform),
        "top-2" => Ok(AssortKind::Top2),
        "top-bottom" => Ok(AssortKind::TopBottom),
        _ => Err(PyValueError::new_err(
            "Invalid type of degree assortativity!",
        )),
    }
}

/// `xgi.dynamical_assortativity(H)`.
#[pyfunction]
fn dynamical_assortativity(h: PyRef<'_, PyHypergraph>) -> PyResult<f64> {
    hypergraph_rs::algorithms::assortativity::dynamical_assortativity(&h.inner)
        .map_err(|e| xgi_error(e.to_string()))
}

/// `xgi.degree_assortativity(H, kind="uniform", exact=False, num_samples=1000, seed=None)`.
#[pyfunction]
#[pyo3(signature = (h, kind="uniform", exact=false, num_samples=1000, seed=None))]
fn degree_assortativity(
    h: PyRef<'_, PyHypergraph>,
    kind: &str,
    exact: bool,
    num_samples: usize,
    seed: Option<Bound<'_, PyAny>>,
) -> PyResult<f64> {
    let _ = (num_samples, seed); // only the exact path is ported (see module doc)
    if !exact {
        return Err(PyNotImplementedError::new_err(
            "degree_assortativity(exact=False) (random sampling) is not ported; call with exact=True",
        ));
    }
    let k = parse_assort_kind(kind)?;
    hypergraph_rs::algorithms::assortativity::degree_assortativity_exact(&h.inner, k)
        .map_err(|e| xgi_error(e.to_string()))
}

/// `xgi.clique_eigenvector_centrality(H, tol=1e-6, max_iter=1000)` (`max_iter` extra; see module doc).
#[pyfunction]
#[pyo3(signature = (h, tol=1e-6, max_iter=1000))]
fn clique_eigenvector_centrality(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    tol: f64,
    max_iter: usize,
) -> PyResult<Py<PyDict>> {
    let m = hypergraph_rs::algorithms::centrality::clique_eigenvector_centrality(
        &h.inner, tol, max_iter,
    );
    node_f64_dict(py, m, &h.node_ids)
}

/// `xgi.katz_centrality(H, cutoff=100)`.
#[pyfunction]
#[pyo3(signature = (h, cutoff=100))]
fn katz_centrality(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    cutoff: usize,
) -> PyResult<Py<PyDict>> {
    let m = hypergraph_rs::algorithms::centrality::katz_centrality(&h.inner, cutoff);
    node_f64_dict(py, m, &h.node_ids)
}

/// `xgi.clustering_coefficient(H)`.
#[pyfunction]
fn clustering_coefficient(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let m = hypergraph_rs::algorithms::clustering::clustering_coefficient(&h.inner);
    node_f64_dict(py, m, &h.node_ids)
}

/// `xgi.local_clustering_coefficient(H)`.
#[pyfunction]
fn local_clustering_coefficient(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
) -> PyResult<Py<PyDict>> {
    let m = hypergraph_rs::algorithms::clustering::local_clustering_coefficient(&h.inner);
    node_f64_dict(py, m, &h.node_ids)
}

/// `xgi.two_node_clustering_coefficient(H, kind="union")`.
#[pyfunction]
#[pyo3(signature = (h, kind="union"))]
fn two_node_clustering_coefficient(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    kind: &str,
) -> PyResult<Py<PyDict>> {
    if !matches!(kind, "union" | "min" | "max") {
        return Err(xgi_error(format!("invalid kind of clustering: {kind}")));
    }
    let m = hypergraph_rs::algorithms::clustering::two_node_clustering_coefficient(&h.inner, kind);
    node_f64_dict(py, m, &h.node_ids)
}

/// `xgi.is_connected(H)`.
#[pyfunction]
fn is_connected(h: PyRef<'_, PyHypergraph>) -> bool {
    hypergraph_rs::algorithms::connected::is_connected(&h.inner)
}

/// `xgi.connected_components(H)` — materialized `list[set]` (deviation: XGI yields a generator).
#[pyfunction]
fn connected_components(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyList>> {
    let comps = hypergraph_rs::algorithms::connected::connected_components(&h.inner);
    let out = PyList::empty(py);
    for c in comps {
        out.append(node_set(py, &c, &h.node_ids)?)?;
    }
    Ok(out.unbind())
}

/// `xgi.largest_connected_component(H)`.
#[pyfunction]
fn largest_connected_component<'py>(
    py: Python<'py>,
    h: PyRef<'_, PyHypergraph>,
) -> PyResult<Bound<'py, PySet>> {
    let c = hypergraph_rs::algorithms::connected::largest_connected_component(&h.inner);
    node_set(py, &c, &h.node_ids)
}

/// `xgi.number_connected_components(H)`.
#[pyfunction]
fn number_connected_components(h: PyRef<'_, PyHypergraph>) -> usize {
    hypergraph_rs::algorithms::connected::number_connected_components(&h.inner)
}

/// `xgi.node_connected_component(H, n)`. XGI raises `XGIError` for a missing node.
#[pyfunction]
fn node_connected_component<'py>(
    py: Python<'py>,
    h: PyRef<'_, PyHypergraph>,
    n: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PySet>> {
    let core =
        resolve_node(&h, n).map_err(|_| xgi_error("Specified node is not in the hypergraph!"))?;
    let comp = hypergraph_rs::algorithms::connected::node_connected_component(&h.inner, &core)
        .ok_or_else(|| xgi_error("Specified node is not in the hypergraph!"))?;
    node_set(py, &comp, &h.node_ids)
}

/// `xgi.equal(H1, H2, compare_edge_ids=True, compare_attrs=True)` (flags not independently
/// honored — see module doc).
#[pyfunction]
#[pyo3(signature = (h1, h2, compare_edge_ids=true, compare_attrs=true))]
fn equal(
    h1: PyRef<'_, PyHypergraph>,
    h2: PyRef<'_, PyHypergraph>,
    compare_edge_ids: bool,
    compare_attrs: bool,
) -> bool {
    let _ = (compare_edge_ids, compare_attrs);
    h1.inner == h2.inner
}

/// `xgi.num_edges_order(H, d=None)`.
#[pyfunction]
#[pyo3(signature = (h, d=None))]
fn num_edges_order(h: PyRef<'_, PyHypergraph>, d: Option<usize>) -> usize {
    hypergraph_rs::algorithms::properties::num_edges_order(&h.inner, d)
}

/// `xgi.max_edge_order(H)`.
#[pyfunction]
fn max_edge_order(h: PyRef<'_, PyHypergraph>) -> Option<usize> {
    hypergraph_rs::algorithms::properties::max_edge_order(&h.inner)
}

/// `xgi.is_uniform(H)` — `None`/`False` collapse (XGI returns `False`; we return `None`, both
/// falsy in Python, so `if not xgi.is_uniform(H)` reads the same).
#[pyfunction]
fn is_uniform(h: PyRef<'_, PyHypergraph>) -> Option<usize> {
    hypergraph_rs::algorithms::properties::is_uniform(&h.inner)
}

/// `xgi.is_possible_order(H, d)`.
#[pyfunction]
fn is_possible_order(h: PyRef<'_, PyHypergraph>, d: usize) -> bool {
    hypergraph_rs::algorithms::properties::is_possible_order(&h.inner, d)
}

/// `xgi.edge_neighborhood(H, n, include_self=False)`.
#[pyfunction]
#[pyo3(signature = (h, n, include_self=false))]
fn edge_neighborhood<'py>(
    py: Python<'py>,
    h: PyRef<'_, PyHypergraph>,
    n: &Bound<'_, PyAny>,
    include_self: bool,
) -> PyResult<Bound<'py, PyList>> {
    let core = resolve_node(&h, n)?;
    let sets =
        hypergraph_rs::algorithms::properties::edge_neighborhood(&h.inner, &core, include_self);
    let out = PyList::empty(py);
    for s in sets {
        out.append(node_set(py, &s, &h.node_ids)?)?;
    }
    Ok(out)
}

/// `xgi.degree_counts(H, order=None)`.
#[pyfunction]
#[pyo3(signature = (h, order=None))]
fn degree_counts(h: PyRef<'_, PyHypergraph>, order: Option<usize>) -> Vec<usize> {
    hypergraph_rs::algorithms::properties::degree_counts(&h.inner, order)
}

/// `xgi.degree_histogram(H)` → `(degrees, heights)`.
#[pyfunction]
fn degree_histogram(h: PyRef<'_, PyHypergraph>) -> (Vec<usize>, Vec<usize>) {
    hypergraph_rs::algorithms::properties::degree_histogram(&h.inner)
}

/// `xgi.unique_edge_sizes(H)`.
#[pyfunction]
fn unique_edge_sizes(h: PyRef<'_, PyHypergraph>) -> Vec<usize> {
    hypergraph_rs::algorithms::properties::unique_edge_sizes(&h.inner)
}

/// `xgi.density(H, order=None, max_order=None, ignore_singletons=False)`.
#[pyfunction]
#[pyo3(signature = (h, order=None, max_order=None, ignore_singletons=false))]
fn density(
    h: PyRef<'_, PyHypergraph>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> PyResult<f64> {
    let n = h.inner.num_nodes();
    if n < 1 {
        return Err(xgi_error("Density not defined for empty hypergraph"));
    }
    if let Some(mo) = max_order {
        if order.is_none() && mo >= n {
            return Err(PyValueError::new_err(
                "max_order must be smaller than the number of nodes",
            ));
        }
    }
    if let Some(o) = order {
        if o >= n {
            return Err(PyValueError::new_err(
                "order must be smaller than the number of nodes",
            ));
        }
    }
    Ok(hypergraph_rs::algorithms::properties::density(
        &h.inner,
        order,
        max_order,
        ignore_singletons,
    ))
}

/// `xgi.incidence_density(H, order=None, max_order=None, ignore_singletons=False)`.
#[pyfunction]
#[pyo3(signature = (h, order=None, max_order=None, ignore_singletons=false))]
fn incidence_density(
    h: PyRef<'_, PyHypergraph>,
    order: Option<usize>,
    max_order: Option<usize>,
    ignore_singletons: bool,
) -> PyResult<f64> {
    let n = h.inner.num_nodes();
    if n < 1 {
        return Err(xgi_error("Density not defined for empty hypergraph"));
    }
    if let Some(mo) = max_order {
        if order.is_none() && mo >= n {
            return Err(PyValueError::new_err(
                "max_order must be smaller than the number of nodes",
            ));
        }
    }
    if let Some(o) = order {
        if o >= n {
            return Err(PyValueError::new_err(
                "order must be smaller than the number of nodes",
            ));
        }
    }
    Ok(hypergraph_rs::algorithms::properties::incidence_density(
        &h.inner,
        order,
        max_order,
        ignore_singletons,
    ))
}

/// `xgi.single_source_shortest_path_length(H, source)`. XGI raises for a missing source.
#[pyfunction]
fn single_source_shortest_path_length<'py>(
    py: Python<'py>,
    h: PyRef<'_, PyHypergraph>,
    source: &Bound<'_, PyAny>,
) -> PyResult<Py<PyDict>> {
    let core = resolve_node(&h, source)?;
    let dist = hypergraph_rs::algorithms::shortest_path::single_source_shortest_path_length(
        &h.inner, &core,
    )
    .ok_or_else(|| id_not_found(&core))?;
    node_f64_dict(py, dist, &h.node_ids)
}

/// `xgi.shortest_path_length(H)` → `{source: {target: dist}}`.
#[pyfunction]
fn shortest_path_length(py: Python<'_>, h: PyRef<'_, PyHypergraph>) -> PyResult<Py<PyDict>> {
    let all = hypergraph_rs::algorithms::shortest_path::shortest_path_length(&h.inner);
    let d = PyDict::new(py);
    for (src, dist) in all {
        if let Some(o) = h.node_ids.get_obj(py, &src) {
            d.set_item(o, node_f64_dict(py, dist, &h.node_ids)?)?;
        }
    }
    Ok(d.unbind())
}

/// `xgi.edit_simpliciality(H, min_size=2, exclude_min_size=True)`.
#[pyfunction]
#[pyo3(signature = (h, min_size=2, exclude_min_size=true))]
fn edit_simpliciality(h: PyRef<'_, PyHypergraph>, min_size: usize, exclude_min_size: bool) -> f64 {
    hypergraph_rs::algorithms::simpliciality::edit_simpliciality(
        &h.inner,
        min_size,
        exclude_min_size,
    )
}

/// `xgi.simplicial_edit_distance(H, min_size=2, exclude_min_size=True, normalize=True)`.
#[pyfunction]
#[pyo3(signature = (h, min_size=2, exclude_min_size=true, normalize=true))]
fn simplicial_edit_distance(
    h: PyRef<'_, PyHypergraph>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    hypergraph_rs::algorithms::simpliciality::simplicial_edit_distance(
        &h.inner,
        min_size,
        exclude_min_size,
        normalize,
    )
}

/// `xgi.face_edit_simpliciality(H, min_size=2, exclude_min_size=True)`.
#[pyfunction]
#[pyo3(signature = (h, min_size=2, exclude_min_size=true))]
fn face_edit_simpliciality(
    h: PyRef<'_, PyHypergraph>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    hypergraph_rs::algorithms::simpliciality::face_edit_simpliciality(
        &h.inner,
        min_size,
        exclude_min_size,
    )
}

/// `xgi.mean_face_edit_distance(H, min_size=2, exclude_min_size=True, normalize=True)`.
#[pyfunction]
#[pyo3(signature = (h, min_size=2, exclude_min_size=true, normalize=true))]
fn mean_face_edit_distance(
    h: PyRef<'_, PyHypergraph>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    hypergraph_rs::algorithms::simpliciality::mean_face_edit_distance(
        &h.inner,
        min_size,
        exclude_min_size,
        normalize,
    )
}

/// `xgi.simplicial_fraction(H, min_size=2, exclude_min_size=True)`.
#[pyfunction]
#[pyo3(signature = (h, min_size=2, exclude_min_size=true))]
fn simplicial_fraction(h: PyRef<'_, PyHypergraph>, min_size: usize, exclude_min_size: bool) -> f64 {
    hypergraph_rs::algorithms::simpliciality::simplicial_fraction(
        &h.inner,
        min_size,
        exclude_min_size,
    )
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(dynamical_assortativity, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(degree_assortativity, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(clique_eigenvector_centrality, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(katz_centrality, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(clustering_coefficient, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(local_clustering_coefficient, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(two_node_clustering_coefficient, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(is_connected, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(connected_components, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(largest_connected_component, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(number_connected_components, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(node_connected_component, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(equal, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(num_edges_order, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(max_edge_order, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(is_uniform, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(is_possible_order, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(edge_neighborhood, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(degree_counts, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(degree_histogram, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(unique_edge_sizes, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(density, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(incidence_density, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        single_source_shortest_path_length,
        m
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(shortest_path_length, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(edit_simpliciality, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(simplicial_edit_distance, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(face_edit_simpliciality, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(mean_face_edit_distance, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(simplicial_fraction, m)?)?;
    Ok(())
}
