//! Simpliciality measures (`xgi/algorithms/simpliciality.py`).
//!
//! A face's subfaces are its powerset for sizes `min_size..len-1`. Edge
//! membership is tested against a set of canonicalized (sorted) member lists.

use std::collections::HashSet;

use crate::Hypergraph;

/// Sorted-member set of all edges, for O(1) subface presence queries.
struct FaceSet {
    faces: HashSet<Vec<String>>,
}

impl FaceSet {
    fn build<N, E, M>(h: &Hypergraph<N, E, M>) -> Self {
        let mut faces = HashSet::new();
        for e in h.edge_ids() {
            let mut m = h.members(&e).unwrap_or_default();
            m.sort();
            faces.insert(m);
        }
        FaceSet { faces }
    }
    fn contains(&self, face: &[String]) -> bool {
        let mut key = face.to_vec();
        key.sort();
        self.faces.contains(&key)
    }
}

/// Subfaces of `face` for sizes in `min_size..=max_size`.
fn powerset(face: &[String], min_size: usize, max_size: usize) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let n = face.len();
    // Bounded: 2^n subsets, filtered by size (n is a single edge's arity).
    // The bound is only statically provable for n < 64 (u64 shift overflow
    // above that) — see the Task 11 "Bound note".
    debug_assert!(n < 64, "powerset: edge arity must be < 64");
    for mask in 0u64..(1u64 << n) {
        let size = mask.count_ones() as usize;
        if size >= min_size && size <= max_size {
            let subset: Vec<String> = (0..n)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| face[i].clone())
                .collect();
            out.push(subset);
        }
    }
    out
}

/// Number of subfaces (sizes `min_size..len-1`) of `face` absent from the set.
fn count_missing_subfaces(fs: &FaceSet, face: &[String], min_size: usize) -> usize {
    if face.len() < min_size + 1 {
        return 0;
    }
    powerset(face, min_size, face.len() - 1)
        .into_iter()
        .filter(|sub| !fs.contains(sub))
        .count()
}

/// Maximum possible subfaces of a face of `max_size`, sizes ≥ `min_size`,
/// excluding the face itself and the empty set. XGI: `_max_number_of_subfaces`.
fn max_number_of_subfaces(min_size: usize, max_size: usize) -> usize {
    let mut d: i128 = 2i128.pow(max_size as u32) - 2;
    for i in 1..min_size {
        d -= comb(max_size, i) as i128;
    }
    d.max(0) as usize
}

fn comb(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num *= (n - i) as u128;
        den *= (i + 1) as u128;
    }
    (num / den) as usize
}

/// Edge IDs whose member sets are maximal (not a subset of another edge).
fn maximal_edges<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<String> {
    let all: Vec<(String, HashSet<String>)> = h
        .edge_ids()
        .into_iter()
        .map(|e| {
            let set: HashSet<String> = h.members(&e).unwrap_or_default().into_iter().collect();
            (e, set)
        })
        .collect();
    all.iter()
        .filter(|(_id, set)| {
            !all.iter()
                .any(|(_oid, other)| other.len() > set.len() && set.is_subset(other))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Whether an edge is a simplex: every subface (size ≥ min_size) is present.
fn is_simplex(fs: &FaceSet, edge: &[String], min_size: usize) -> bool {
    if edge.len() < min_size {
        return true;
    }
    powerset(edge, min_size, edge.len())
        .into_iter()
        .all(|sub| fs.contains(&sub))
}

/// Simplicial fraction: what fraction of potential-simplex edges are simplices.
/// XGI parity: `simplicial_fraction` (`simpliciality.py:325-383`).
pub fn simplicial_fraction<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let candidates: Vec<Vec<String>> = h
        .edge_ids()
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    let ps = candidates.len();
    if ps == 0 {
        return f64::NAN;
    }
    let ns = candidates
        .iter()
        .filter(|e| is_simplex(&fs, e, min_size))
        .count();
    ns as f64 / ps as f64
}

/// Simplicial edit distance (normalized): fraction of missing subfaces needed
/// to make `H` a simplicial complex. XGI parity: `simplicial_edit_distance`
/// (`simpliciality.py:79-182`), including the redundant-missing-face
/// correction (`:156-172`): a missing subface that is a subset of two or
/// more maximal faces is otherwise double-counted once per maximal face that
/// contains it. For each maximal face (in our own `maximal_edges()` order),
/// XGI subtracts the *union* (deduplicated) of missing subfaces of its
/// intersection with every EARLIER maximal face — by XGI's own linear scan
/// order — whenever that intersection has `>= min_size` members.
///
/// We reuse `maximal_edges()`'s order rather than reproducing XGI's exact
/// edge-insertion order. This is safe because the correction's *total*
/// across the whole hypergraph is order-independent: consider a missing
/// subface `s` (size `>= min_size`) that is a common subface of exactly `k`
/// maximal faces. Any two of those `k` faces automatically intersect in a
/// superset of `s` (so the `>= min_size` gate always fires for that pair,
/// regardless of ordering). Visiting the `k` faces in ANY linear order, the
/// first one contributes no subtraction for `s` (no earlier face to pair
/// with), and each of the remaining `k - 1` faces contributes exactly one
/// subtraction for `s` — the per-face union dedups repeated contributions
/// from multiple earlier faces sharing `s`, so it's never subtracted twice
/// from the same face. The total subtraction for `s` is therefore always
/// `k - 1`, independent of which of the `k` faces is visited first, second,
/// etc. — i.e. independent of the linear order chosen. `s`'s raw
/// (uncorrected) count is exactly `k` (once per containing maximal face), so
/// the corrected count is `k - (k - 1) = 1`, matching XGI's total regardless
/// of maximal-face ordering.
pub fn simplicial_edit_distance<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let max_faces: Vec<Vec<String>> = maximal_edges(h)
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    if max_faces.is_empty() {
        return f64::NAN;
    }
    let mut ms: i128 = 0;
    for (i, e) in max_faces.iter().enumerate() {
        let mf = count_missing_subfaces(&fs, e, min_size) as i128;
        let e_set: HashSet<&String> = e.iter().collect();
        let mut redundant: HashSet<Vec<String>> = HashSet::new();
        for e2 in &max_faces[..i] {
            let e2_set: HashSet<&String> = e2.iter().collect();
            let inter: Vec<String> = e_set.intersection(&e2_set).map(|s| (*s).clone()).collect();
            // XGI: `maxH.edges.neighbors(id1)` (share >= 1 node) gated by
            // `len(c) >= min_size` (simpliciality.py:158-161).
            if inter.is_empty() || inter.len() < min_size {
                continue;
            }
            // Missing subfaces of the intersection, sizes min_size..=len —
            // i.e. `_missing_subfaces` (strict subsets) plus the intersection
            // itself if absent (simpliciality.py:162,167-168), in one pass.
            for sub in powerset(&inter, min_size, inter.len()) {
                if !fs.contains(&sub) {
                    let mut key = sub;
                    key.sort();
                    redundant.insert(key);
                }
            }
        }
        ms += mf - redundant.len() as i128;
    }
    if !normalize {
        return ms as f64;
    }
    let s = h
        .edge_ids()
        .into_iter()
        .filter(|e| h.edge_size(e).map(|sz| sz >= min_size).unwrap_or(false))
        .count();
    let mf = max_faces.len();
    let denom = s as i128 - mf as i128 + ms;
    if denom > 0 {
        ms as f64 / denom as f64
    } else {
        f64::NAN
    }
}

/// Edit simpliciality = 1 − simplicial_edit_distance.
/// XGI parity: `edit_simpliciality` (`simpliciality.py:17-76`).
pub fn edit_simpliciality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    1.0 - simplicial_edit_distance(h, min_size, exclude_min_size, true)
}

/// Mean face edit distance over maximal faces.
/// XGI parity: `mean_face_edit_distance` (`simpliciality.py:248-322`).
pub fn mean_face_edit_distance<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
    normalize: bool,
) -> f64 {
    let fs = FaceSet::build(h);
    let floor = min_size + exclude_min_size as usize;
    let max_faces: Vec<Vec<String>> = maximal_edges(h)
        .into_iter()
        .filter_map(|e| h.members(&e))
        .filter(|m| m.len() >= floor)
        .collect();
    if max_faces.is_empty() {
        return 0.0;
    }
    let count = max_faces.len() as f64;
    let mut avg = 0.0;
    for e in &max_faces {
        if e.len() >= min_size {
            let mut d = count_missing_subfaces(&fs, e, min_size) as f64;
            let m = max_number_of_subfaces(min_size, e.len());
            if normalize && m != 0 {
                d /= m as f64;
            }
            avg += d / count;
        }
    }
    avg
}

/// Face edit simpliciality = 1 − mean_face_edit_distance.
/// XGI parity: `face_edit_simpliciality` (`simpliciality.py:185-245`).
pub fn face_edit_simpliciality<N, E, M>(
    h: &Hypergraph<N, E, M>,
    min_size: usize,
    exclude_min_size: bool,
) -> f64 {
    1.0 - mean_face_edit_distance(h, min_size, exclude_min_size, true)
}
