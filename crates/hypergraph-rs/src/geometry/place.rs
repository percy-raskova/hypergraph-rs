//! Placement at a weighted mix of given positions.

use indexmap::IndexMap;

use super::{GeometryError, Placement, Point, Positions};
use crate::core::hypergraph::Hypergraph;

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

/// The running sums of a weighted mean: total weight, weighted x, weighted y.
#[derive(Clone, Copy, Default)]
struct Sums {
    total: f64,
    x: f64,
    y: f64,
}

impl Sums {
    /// Checks the weight `w` of the item `name` and adds it, with its position `at`, to the sums.
    fn add(
        &mut self,
        w: f64,
        at: Point,
        name: impl FnOnce() -> String,
    ) -> Result<(), GeometryError> {
        if !(w.is_finite() && w >= 0.0) {
            return Err(GeometryError::BadWeight {
                item: name(),
                value: w,
            });
        }
        self.total += w;
        self.x += w * at.0;
        self.y += w * at.1;
        Ok(())
    }

    /// The mean, or `None` when the weights sum to zero. A sum or a result that is not finite (finite inputs can
    /// overflow) is an error naming `name`, never an infinite or NaN point.
    fn finish(self, name: impl FnOnce() -> String) -> Result<Option<Point>, GeometryError> {
        if !(self.total.is_finite() && self.x.is_finite() && self.y.is_finite()) {
            return Err(GeometryError::NonFinitePosition { item: name() });
        }
        if self.total > 0.0 {
            let at = (self.x / self.total, self.y / self.total);
            if !(at.0.is_finite() && at.1.is_finite()) {
                return Err(GeometryError::NonFinitePosition { item: name() });
            }
            Ok(Some(at))
        } else {
            Ok(None)
        }
    }
}

/// Weighted mean of `anchors`, with `weights` already counted and anchors already checked.
/// `name` gives the item named in a weight error; `whole` the item named when the result is not finite.
fn mix(
    anchors: &[Point],
    weights: &[f64],
    name: &dyn Fn(usize) -> String,
    whole: &dyn Fn() -> String,
) -> Result<Option<Point>, GeometryError> {
    let mut sums = Sums::default();
    for (k, (&w, a)) in weights.iter().zip(anchors).enumerate() {
        sums.add(w, *a, || name(k))?;
    }
    sums.finish(whole)
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
    mix(anchors, weights, &|k| format!("#{k}"), &|| {
        "barycentre".to_string()
    })
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
/// finite. A row whose sums overflow gives `NonFinitePosition` (item: the row's id).
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
        match mix(corners, row, &|_| id.clone(), &|| id.clone())? {
            Some(at) => {
                placement.positions.insert(id.clone(), at);
            }
            None => placement.unplaced.push(id.clone()),
        }
    }
    Ok(placement)
}

/// Places each node of `h` at the weighted mean of the positions of the anchored hyperedges it belongs to.
///
/// An anchor is a hyperedge with a fixed position in `anchors`. A node's weight on an anchor is
/// `weight(&membership value)`. Memberships of hyperedges that are not anchors are ignored. A node whose weights on
/// anchors sum to zero (or that belongs to no anchor) is listed in `unplaced`, never put at the origin.
///
/// Sums run over `anchors` in their order, then over each anchor's members in `members_with_data` order.
/// `positions` lists placed nodes in `node_ids()` order; `unplaced` lists the rest in `node_ids()` order.
///
/// # Errors
///
/// `UnknownEdge` for an anchor id not in `h`, `NonFinitePosition` (item: the edge id) for an anchor position that is
/// not finite, `BadWeight` (item: the node id) for a weight that is negative or not finite, `NonFinitePosition`
/// (item: the node id) for a node whose sums overflow.
pub fn anchored_placement<N, E, M>(
    h: &Hypergraph<N, E, M>,
    anchors: &Positions,
    weight: impl Fn(&M) -> f64,
) -> Result<Placement, GeometryError> {
    let ids = h.node_ids();
    let index: IndexMap<&str, usize> = ids
        .iter()
        .enumerate()
        .map(|(k, id)| (id.as_str(), k))
        .collect();
    // total weight, weighted x, weighted y, per node in node order
    let mut sums = vec![Sums::default(); ids.len()];
    for (edge, at) in anchors.iter() {
        let Some(members) = h.members_with_data(edge) else {
            return Err(GeometryError::UnknownEdge(edge.to_string()));
        };
        if !(at.0.is_finite() && at.1.is_finite()) {
            return Err(GeometryError::NonFinitePosition {
                item: edge.to_string(),
            });
        }
        for (node, data) in members {
            let w = weight(data);
            let slot = index.get(node.as_str()).copied();
            match slot {
                Some(k) => sums[k].add(w, at, || node)?,
                // A member that is not a node still has its weight checked.
                None => Sums::default().add(w, at, || node)?,
            }
        }
    }
    let mut placement = Placement::default();
    for (id, s) in ids.into_iter().zip(sums) {
        match s.finish(|| id.clone())? {
            Some(at) => {
                placement.positions.insert(id, at);
            }
            None => placement.unplaced.push(id),
        }
    }
    Ok(placement)
}
