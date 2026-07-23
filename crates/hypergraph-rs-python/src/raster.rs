//! PyO3 exposure of topology ingest + rasterize + pixel plate (Phase I,
//! Task 8) — `_hypergraph_rs_core.raster`. Cells3d-gated, matching the core
//! crate's `instruments` module; `render_pixels`/`render_png` are
//! additionally `raster-png`-gated because they touch `raster::png`, which
//! the core crate only compiles under that feature.
//!
//! Every pyfunction here is a thin FFI wrapper: the actual conversions
//! (str -> `Instrument`, dict -> `AttrContract`, dict -> `Camera`,
//! `PixelBuffer` -> bytes) are pure Rust helpers below, unit-tested without
//! a Python interpreter. No `unwrap`/`expect` at the FFI boundary — every
//! failure becomes a `PyErr` via [`crate::errors::xgi_error`], and every
//! `IngestError` message (offending node/edge id + key) crosses verbatim.

use pyo3::prelude::*;
#[cfg(feature = "raster-png")]
use pyo3::types::PyBytes;
use pyo3::types::PyDict;

use hypergraph_rs::raster::ingest::{self, AttrContract, IngestError, IngestReport, TensionSource};
use hypergraph_rs::raster::instruments::{CylinderHeight, Instrument, InstrumentOpts};
#[cfg(feature = "raster-png")]
use hypergraph_rs::raster::png::{
    render_pixels as core_render_pixels, render_png as core_render_png, PixelBuffer,
};
#[cfg(feature = "raster-png")]
use hypergraph_rs::raster::Rgb;
use hypergraph_rs::raster::{rasterize as core_rasterize, Camera};

use crate::errors::xgi_error;
use crate::hypergraph::PyHypergraph;
use crate::pyid::json_to_py;

/// The camera used when the caller passes no `camera` dict — `ry=rx=0`
/// (straight-on), `dist=4.0`, `fov=70.0` degrees; the same front-camera
/// convention the core crate's own tests use (`tests/test_raster.rs`,
/// `tests/test_ingest.rs::front_camera`).
const DEFAULT_CAMERA: Camera = Camera {
    ry: 0.0,
    rx: 0.0,
    dist: 4.0,
    fov: 70.0,
};

// ---------------------------------------------------------------------
// Pure conversion helpers — no `Python`/GIL involved, unit-tested below.
// ---------------------------------------------------------------------

/// Parse an instrument name. `Err` carries the raw (unrecognised) string so
/// the caller can build a precise error message.
fn parse_instrument(name: &str) -> Result<Instrument, String> {
    match name {
        "cylinder" => Ok(Instrument::Cylinder),
        "terrain" => Ok(Instrument::Terrain),
        "sankey" => Ok(Instrument::Sankey),
        other => Err(other.to_string()),
    }
}

/// Parse a `cylinder_height` option name.
fn parse_cylinder_height(name: &str) -> Result<CylinderHeight, String> {
    match name {
        "org_degree" => Ok(CylinderHeight::OrgDegree),
        "hull_fill" => Ok(CylinderHeight::HullFill),
        other => Err(other.to_string()),
    }
}

/// Apply `(field, key)` overrides onto a default `AttrContract`. Unknown
/// field names are rejected (`Err` carries the bad field name) — a typo in
/// a contract override must not be silently ignored, per the crate's
/// "never invent data" discipline.
fn apply_contract_overrides(
    base: AttrContract,
    overrides: &[(String, String)],
) -> Result<AttrContract, String> {
    let mut c = base;
    for (field, value) in overrides {
        match field.as_str() {
            "wealth" => c.wealth = value.clone(),
            "subsistence" => c.subsistence = value.clone(),
            "bloc" => c.bloc = value.clone(),
            "tension" => c.tension = value.clone(),
            "h3_cell" => c.h3_cell = value.clone(),
            "kind" => c.kind = value.clone(),
            "name" => c.name = value.clone(),
            "links" => c.links = value.clone(),
            "head" => c.head = value.clone(),
            "mass" => c.mass = value.clone(),
            "lane" => c.lane = value.clone(),
            "c" => c.c = value.clone(),
            "v" => c.v = value.clone(),
            "e" => c.e = value.clone(),
            "repression" => c.repression = value.clone(),
            "zoom" => c.zoom = value.clone(),
            other => return Err(other.to_string()),
        }
    }
    Ok(c)
}

/// Apply `(ry, rx, dist, fov)` overrides onto [`DEFAULT_CAMERA`], each
/// `None` leaving the default in place.
fn camera_from_overrides(
    ry: Option<f64>,
    rx: Option<f64>,
    dist: Option<f64>,
    fov: Option<f64>,
) -> Camera {
    let mut cam = DEFAULT_CAMERA;
    if let Some(v) = ry {
        cam.ry = v;
    }
    if let Some(v) = rx {
        cam.rx = v;
    }
    if let Some(v) = dist {
        cam.dist = v;
    }
    if let Some(v) = fov {
        cam.fov = v;
    }
    cam
}

/// Flatten a `PixelBuffer` into row-major RGB8 bytes (`len == w*h*3`) — the
/// same flattening `render_png` (`png.rs:61`) does before handing data to
/// the PNG encoder, reused here for the raw-bytes surface.
#[cfg(feature = "raster-png")]
fn pixel_bytes(pb: &PixelBuffer) -> Vec<u8> {
    let mut out = Vec::with_capacity(pb.px.len() * 3);
    for Rgb(r, g, b) in &pb.px {
        out.push(*r);
        out.push(*g);
        out.push(*b);
    }
    out
}

// ---------------------------------------------------------------------
// PyO3 glue — dict/str extraction, delegating to the pure helpers above.
// ---------------------------------------------------------------------

fn ingest_err_to_py(e: IngestError) -> PyErr {
    xgi_error(e.to_string())
}

fn parse_instrument_py(name: &str) -> PyResult<Instrument> {
    parse_instrument(name).map_err(|bad| {
        xgi_error(format!(
            "unknown instrument `{bad}`; expected one of \"cylinder\", \"terrain\", \"sankey\""
        ))
    })
}

fn build_opts(cylinder_height: Option<&str>) -> PyResult<InstrumentOpts> {
    let cylinder_height = match cylinder_height {
        None => CylinderHeight::default(),
        Some(s) => parse_cylinder_height(s).map_err(|bad| {
            xgi_error(format!(
                "unknown cylinder_height `{bad}`; expected \"org_degree\" or \"hull_fill\""
            ))
        })?,
    };
    Ok(InstrumentOpts { cylinder_height })
}

fn contract_overrides_from_dict(d: &Bound<'_, PyDict>) -> PyResult<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(d.len());
    for (k, v) in d.iter() {
        let field: String = k
            .extract()
            .map_err(|_| xgi_error("attribute-contract override keys must be strings"))?;
        let key: String = v.extract().map_err(|_| {
            xgi_error(format!(
                "attribute-contract override `{field}` must map to a string key name"
            ))
        })?;
        out.push((field, key));
    }
    Ok(out)
}

fn build_contract(contract: Option<&Bound<'_, PyDict>>) -> PyResult<AttrContract> {
    match contract {
        None => Ok(AttrContract::default()),
        Some(d) => {
            let overrides = contract_overrides_from_dict(d)?;
            apply_contract_overrides(AttrContract::default(), &overrides)
                .map_err(|field| xgi_error(format!("unknown attribute-contract field `{field}`")))
        }
    }
}

fn extract_opt_f64(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<f64>> {
    match d.get_item(key)? {
        None => Ok(None),
        Some(v) => Ok(Some(v.extract::<f64>().map_err(|_| {
            xgi_error(format!("camera key `{key}` must be a number"))
        })?)),
    }
}

fn build_camera(camera: Option<&Bound<'_, PyDict>>) -> PyResult<Camera> {
    match camera {
        None => Ok(DEFAULT_CAMERA),
        Some(d) => {
            let ry = extract_opt_f64(d, "ry")?;
            let rx = extract_opt_f64(d, "rx")?;
            let dist = extract_opt_f64(d, "dist")?;
            let fov = extract_opt_f64(d, "fov")?;
            Ok(camera_from_overrides(ry, rx, dist, fov))
        }
    }
}

/// `IngestReport` -> a plain dict. `IngestReport` has no `Serialize` (Task
/// 1 only derives `Debug + Clone + PartialEq` on it), so this is built
/// field-by-field rather than through `serde_json::to_value` +
/// `json_to_py` (the idiom used for `scene`, which IS `Serialize`).
fn report_to_dict(py: Python<'_>, report: &IngestReport) -> PyResult<Py<PyDict>> {
    let d = PyDict::new(py);
    d.set_item("contract_version", report.contract_version)?;
    d.set_item("node_ids", report.node_ids.clone())?;
    d.set_item("agent_count", report.agent_count)?;
    d.set_item("community_count", report.community_count)?;
    d.set_item("extraction_count", report.extraction_count)?;
    d.set_item("industry_count", report.industry_count)?;
    d.set_item("skipped_edges", report.skipped_edges.clone())?;
    d.set_item("bloc_defaulted", report.bloc_defaulted)?;
    d.set_item("lane_defaulted", report.lane_defaulted)?;
    d.set_item("repression_defaulted", report.repression_defaulted)?;
    d.set_item(
        "tension_source",
        match report.tension_source {
            TensionSource::Derived => "derived",
            TensionSource::Attr => "attr",
        },
    )?;
    d.set_item("h3_cells", report.h3_cells.clone())?;
    Ok(d.unbind())
}

// ---------------------------------------------------------------------
// Exposed functions.
// ---------------------------------------------------------------------

/// `raster.build_scene(h, instrument, contract=None, cylinder_height=None)
/// -> (scene: dict, report: dict)`.
#[pyfunction]
#[pyo3(signature = (h, instrument, contract=None, cylinder_height=None))]
fn build_scene(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    instrument: &str,
    contract: Option<&Bound<'_, PyDict>>,
    cylinder_height: Option<&str>,
) -> PyResult<(Py<PyDict>, Py<PyDict>)> {
    let inst = parse_instrument_py(instrument)?;
    let c = build_contract(contract)?;
    let opts = build_opts(cylinder_height)?;

    let (scene, report) =
        ingest::build_scene(&h.inner, &c, inst, &opts).map_err(ingest_err_to_py)?;

    let value = serde_json::to_value(&scene)
        .map_err(|e| xgi_error(format!("scene serialization failed: {e}")))?;
    let scene_dict = json_to_py(py, &value)?.cast_into::<PyDict>()?.unbind();
    let report_dict = report_to_dict(py, &report)?;

    Ok((scene_dict, report_dict))
}

/// `raster.rasterize_text(h, instrument, cols, rows, camera=None,
/// contract=None) -> str` — the Tier-0 glyph frame.
#[pyfunction]
#[pyo3(signature = (h, instrument, cols, rows, camera=None, contract=None))]
fn rasterize_text(
    h: PyRef<'_, PyHypergraph>,
    instrument: &str,
    cols: u16,
    rows: u16,
    camera: Option<&Bound<'_, PyDict>>,
    contract: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    let inst = parse_instrument_py(instrument)?;
    let c = build_contract(contract)?;
    let cam = build_camera(camera)?;
    let opts = InstrumentOpts::default();

    let (scene, _report) =
        ingest::build_scene(&h.inner, &c, inst, &opts).map_err(ingest_err_to_py)?;
    let grid = core_rasterize(&scene, &cam, cols, rows);
    Ok(grid.to_text())
}

/// `raster.render_pixels(h, instrument, width, height, camera=None,
/// contract=None) -> (w, h, bytes)` — row-major RGB8.
#[cfg(feature = "raster-png")]
#[pyfunction]
#[pyo3(signature = (h, instrument, width, height, camera=None, contract=None))]
#[allow(clippy::too_many_arguments)]
fn render_pixels(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    instrument: &str,
    width: u32,
    height: u32,
    camera: Option<&Bound<'_, PyDict>>,
    contract: Option<&Bound<'_, PyDict>>,
) -> PyResult<(u32, u32, Py<PyBytes>)> {
    let inst = parse_instrument_py(instrument)?;
    let c = build_contract(contract)?;
    let cam = build_camera(camera)?;
    let opts = InstrumentOpts::default();

    let (scene, _report) =
        ingest::build_scene(&h.inner, &c, inst, &opts).map_err(ingest_err_to_py)?;
    let pb = core_render_pixels(&scene, &cam, width, height);
    let bytes = pixel_bytes(&pb);
    Ok((pb.w, pb.h, PyBytes::new(py, &bytes).unbind()))
}

/// `raster.render_png(h, instrument, width, height, camera=None,
/// contract=None) -> bytes`.
#[cfg(feature = "raster-png")]
#[pyfunction]
#[pyo3(signature = (h, instrument, width, height, camera=None, contract=None))]
#[allow(clippy::too_many_arguments)]
fn render_png(
    py: Python<'_>,
    h: PyRef<'_, PyHypergraph>,
    instrument: &str,
    width: u32,
    height: u32,
    camera: Option<&Bound<'_, PyDict>>,
    contract: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyBytes>> {
    let inst = parse_instrument_py(instrument)?;
    let c = build_contract(contract)?;
    let cam = build_camera(camera)?;
    let opts = InstrumentOpts::default();

    let (scene, _report) =
        ingest::build_scene(&h.inner, &c, inst, &opts).map_err(ingest_err_to_py)?;
    let bytes = core_render_png(&scene, &cam, width, height);
    Ok(PyBytes::new(py, &bytes).unbind())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "raster")?;
    m.add_function(pyo3::wrap_pyfunction!(build_scene, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(rasterize_text, &m)?)?;
    #[cfg(feature = "raster-png")]
    {
        m.add_function(pyo3::wrap_pyfunction!(render_pixels, &m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(render_png, &m)?)?;
    }
    parent.add_submodule(&m)?;
    // `add_submodule` alone doesn't register a `sys.modules` entry, so
    // `import _hypergraph_rs_core.raster` / `from _hypergraph_rs_core.raster
    // import build_scene` wouldn't resolve without this (pyo3 #759/#1517).
    py.import("sys")?
        .getattr("modules")?
        .set_item("_hypergraph_rs_core.raster", &m)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_instrument_known_names() {
        assert_eq!(parse_instrument("cylinder"), Ok(Instrument::Cylinder));
        assert_eq!(parse_instrument("terrain"), Ok(Instrument::Terrain));
        assert_eq!(parse_instrument("sankey"), Ok(Instrument::Sankey));
    }

    #[test]
    fn parse_instrument_unknown_is_error_not_default() {
        // An unrecognised instrument name must produce an error, never
        // silently fall back to some default instrument.
        assert_eq!(parse_instrument("bogus"), Err("bogus".to_string()));
    }

    #[test]
    fn parse_cylinder_height_known_names() {
        assert_eq!(
            parse_cylinder_height("org_degree"),
            Ok(CylinderHeight::OrgDegree)
        );
        assert_eq!(
            parse_cylinder_height("hull_fill"),
            Ok(CylinderHeight::HullFill)
        );
        assert_eq!(parse_cylinder_height("bogus"), Err("bogus".to_string()));
    }

    #[test]
    fn contract_overrides_apply_named_fields_only() {
        let c = apply_contract_overrides(
            AttrContract::default(),
            &[("wealth".to_string(), "econ_wealth".to_string())],
        )
        .unwrap();
        assert_eq!(c.wealth, "econ_wealth");
        // Untouched fields keep their default spelling.
        assert_eq!(c.subsistence, AttrContract::default().subsistence);
    }

    #[test]
    fn contract_overrides_reject_unknown_field() {
        let err = apply_contract_overrides(
            AttrContract::default(),
            &[("not_a_field".to_string(), "x".to_string())],
        );
        assert_eq!(err, Err("not_a_field".to_string()));
    }

    #[test]
    fn camera_overrides_apply_over_default_leaving_rest_untouched() {
        let cam = camera_from_overrides(Some(1.0), None, None, None);
        assert_eq!(cam.ry, 1.0);
        assert_eq!(cam.rx, DEFAULT_CAMERA.rx);
        assert_eq!(cam.dist, DEFAULT_CAMERA.dist);
        assert_eq!(cam.fov, DEFAULT_CAMERA.fov);
    }

    #[cfg(feature = "raster-png")]
    #[test]
    fn pixel_bytes_length_is_w_times_h_times_3() {
        let pb = PixelBuffer {
            w: 3,
            h: 2,
            px: vec![Rgb(1, 2, 3); 6],
        };
        assert_eq!(pixel_bytes(&pb).len(), (pb.w * pb.h * 3) as usize);
        assert_eq!(pixel_bytes(&pb)[0..3], [1, 2, 3]);
    }
}
