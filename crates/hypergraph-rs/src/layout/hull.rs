//! Convex hull via Andrew's monotone chain (replaces XGI's scipy ConvexHull).

/// Compute the convex hull of a set of 2D points. Returns the hull vertices in
/// counter-clockwise order, with collinear points removed. For 0, 1, or 2
/// unique points the "hull" is those points themselves (a point or a segment);
/// callers render degenerate hulls specially.
///
/// Deterministic: the input is sorted by (x, then y) before construction.
pub fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = points.to_vec();
    // Deterministic total order on finite f64 (positions are always finite);
    // NaN would compare Equal, which is acceptable for layout coordinates.
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup(); // exact duplicates are now adjacent after the sort
    let n = pts.len();
    if n <= 2 {
        return pts;
    }

    // 2D cross product of (a-o) x (b-o).
    fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }

    let mut hull: Vec<(f64, f64)> = Vec::with_capacity(2 * n);
    // Lower hull. The inner `while` is bounded: each pop strictly shrinks
    // `hull`, and total pops across the whole build are <= total pushes (2n).
    for &p in pts.iter() {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    // Upper hull.
    let lower_len = hull.len() + 1;
    for &p in pts.iter().rev() {
        while hull.len() >= lower_len
            && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
        {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop(); // the last point duplicates the first point of the lower hull
    hull
}
