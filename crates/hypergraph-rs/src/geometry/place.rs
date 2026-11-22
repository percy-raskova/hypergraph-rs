//! Placement at a weighted mix of given positions.

use super::{GeometryError, Placement, Point, Positions};

fn check_anchors(anchors: &[Point]) -> Result<(), GeometryError> {
    for (k, a) in anchors.iter().enumerate() {
        if !(a.0.is_finite() && a.1.is_finite()) {
            return Err(GeometryError::NonFinitePosition {
                item: format!("#{k}"),
            });
        }
    }
    Ok(())
}

/// Weighted mean of `anchors`, with `weights` already counted and anchors already checked.
/// `name` gives the item named in a weight error.
fn mix(
    anchors: &[Point],
    weights: &[f64],
    name: &dyn Fn(usize) -> String,
) -> Result<Option<Point>, GeometryError> {
    let mut total = 0.0_f64;
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;
    for (k, (&w, a)) in weights.iter().zip(anchors).enumerate() {
        if !(w.is_finite() && w >= 0.0) {
            return Err(GeometryError::BadWeight {
                item: name(k),
                value: w,
            });
        }
        total += w;
        sx += w * a.0;
        sy += w * a.1;
    }
    if total > 0.0 {
        Ok(Some((sx / total, sy / total)))
    } else {
        Ok(None)
    }
}

/// The weighted mean of `anchors`: `(Σ wᵢ·xᵢ / Σ wᵢ, Σ wᵢ·yᵢ / Σ wᵢ)`, summed in index order.
///
/// Returns `Ok(None)` when every weight is zero or the slices are empty.
///
/// # Errors
///
/// `CountMismatch` (item `"weights"`) when the slices differ in length, `BadWeight` for a weight that is negative or
/// not finite, `NonFinitePosition` for an anchor that is not finite. Items are named `#<index>`.
pub fn barycentre(anchors: &[Point], weights: &[f64]) -> Result<Option<Point>, GeometryError> {
    if anchors.len() != weights.len() {
        return Err(GeometryError::CountMismatch {
            item: "weights".to_string(),
            expected: anchors.len(),
            found: weights.len(),
        });
    }
    check_anchors(anchors)?;
    mix(anchors, weights, &|k| format!("#{k}"))
}

/// Places each item at the mix of `corners` given by its row of shares.
///
/// Each row holds one item's shares over the corners; a row need not sum to one. A row of zeros leaves the item in
/// `unplaced`. Placed items and unplaced items both keep the input's order.
///
/// # Errors
///
/// `NonFinitePosition` (item `#<k>`) for a corner that is not finite, `CountMismatch` (item: the row's id) for a row
/// whose length is not the count of corners, `BadWeight` (item: the row's id) for a share that is negative or not
/// finite.
pub fn corner_mix(
    corners: &[Point],
    shares: &[(String, Vec<f64>)],
) -> Result<Placement, GeometryError> {
    check_anchors(corners)?;
    let mut placement = Placement {
        positions: Positions::new(),
        unplaced: Vec::new(),
    };
    for (id, row) in shares {
        if row.len() != corners.len() {
            return Err(GeometryError::CountMismatch {
                item: id.clone(),
                expected: corners.len(),
                found: row.len(),
            });
        }
        match mix(corners, row, &|_| id.clone())? {
            Some(at) => {
                placement.positions.insert(id.clone(), at);
            }
            None => placement.unplaced.push(id.clone()),
        }
    }
    Ok(placement)
}
