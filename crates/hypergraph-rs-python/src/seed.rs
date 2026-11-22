//! One seed rule for every seeded binding.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;

const SEED_TYPE_MESSAGE: &str = "seed must be None, an int or a numpy.random.Generator";

/// XGI's seed rule, made deterministic: `None` -> `None`; an object with `__index__` (int, bool, numpy
/// integer) -> that value, or `ValueError` when it is negative or not below 2**64; a
/// `numpy.random.Generator` -> one draw, `int(seed.integers(0, 2**32 - 1))` (exactly how XGI 1.1's layouts
/// consume a Generator); anything else -> `TypeError`.
pub(crate) fn extract_seed(seed: Option<&Bound<'_, PyAny>>) -> PyResult<Option<u64>> {
    read_seed(seed, true)
}

/// The same checks as [`extract_seed`], without drawing from a `Generator`: for a path that never uses
/// the seed. XGI's exact `degree_assortativity` hands the seed to `default_rng`, which refuses a bad
/// type or value but leaves a `Generator` as it was.
pub(crate) fn check_seed(seed: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
    read_seed(seed, false).map(|_| ())
}

fn read_seed(seed: Option<&Bound<'_, PyAny>>, draw: bool) -> PyResult<Option<u64>> {
    let Some(obj) = seed else {
        return Ok(None);
    };
    if obj.is_none() {
        return Ok(None);
    }
    let py = obj.py();
    if obj.hasattr("__index__")? {
        let value: i128 = obj.call_method0("__index__")?.extract()?;
        return u64::try_from(value)
            .map(Some)
            .map_err(|_| PyValueError::new_err("seed must be non-negative and below 2**64"));
    }
    let generator = py.import("numpy.random")?.getattr("Generator")?;
    if obj.is_instance(&generator)? {
        if !draw {
            return Ok(None);
        }
        let draw: u64 = obj
            .call_method1("integers", (0_u64, u64::from(u32::MAX)))?
            .call_method0("__index__")?
            .extract()?;
        return Ok(Some(draw));
    }
    Err(PyTypeError::new_err(SEED_TYPE_MESSAGE))
}
