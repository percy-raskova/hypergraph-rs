//! `h3o` u64 boundary (PHASE-R-MAP §4 / divergence D7): all bitwise spatial
//! algebra for the raster crate lives here, never leaking into graph
//! semantics. `CellIndex` (u64-backed `NonZeroU64`) is the spatial index
//! domain type end-to-end; it is stringified only when it crosses the
//! graph's stringly-typed ID boundary. Feature-gated `cells3d`. Task 9.

use h3o::error::LocalIjError;
use h3o::{CellIndex, CoordIJ, LatLng, LocalIJ, Resolution};

/// A fixed, deterministic origin cell for the deck's invariant industry hex
/// grid, at `resolution`. The deck's own grid (`topology-deck.html:169-172`)
/// is a synthetic axial-coordinate mock with no geographic meaning; `h3o`
/// requires a real anchor, so `(0,0)` lat/lng is used — arbitrary but fixed,
/// making every `industry_cell` call a pure function of `(hq,hr)`.
pub fn origin_cell(resolution: Resolution) -> CellIndex {
    LatLng::new(0.0, 0.0)
        .expect("(0.0, 0.0) is always a valid lat/lng")
        .to_cell(resolution)
}

/// Snap an industry's axial `(hq, hr)` offset to a real `h3o::CellIndex`,
/// anchored at `origin` — the **inverse** of `to_local_ij` (Finding 10):
/// `CellIndex::try_from(LocalIJ::new(origin, CoordIJ::new(hq, hr)))`.
/// Returns `Err(LocalIjError::Pentagon)` explicitly when the offset crosses
/// pentagon distortion (12 of 122 base cells are pentagons) — callers MUST
/// handle it, never `.unwrap()` (Power-of-10 rule 4).
pub fn industry_cell(origin: CellIndex, hq: i32, hr: i32) -> Result<CellIndex, LocalIjError> {
    CellIndex::try_from(LocalIJ::new(origin, CoordIJ::new(hq, hr)))
}

/// Extensive (summable) quantities carried per grid cell (`aggregateCells`,
/// `topology-deck.html:371-380`): `c`, `v`, `s`, `t` (transformation
/// transfer), and `n` (member count) all sum under aggregation; `hx`/`hz`
/// (ground position) sum too, then are divided by `n` at the call site to
/// recover the mean position (mirrors the deck's `hx/=n; hz/=n`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extensives {
    pub c: f64,
    pub v: f64,
    pub s: f64,
    pub t: f64,
    pub n: u32,
    pub hx: f64,
    pub hz: f64,
}

impl Extensives {
    /// Componentwise sum — the zoom functor's `GROUP BY parent` reducer.
    fn merge(self, other: Self) -> Self {
        Self {
            c: self.c + other.c,
            v: self.v + other.v,
            s: self.s + other.s,
            t: self.t + other.t,
            n: self.n + other.n,
            hx: self.hx + other.hx,
            hz: self.hz + other.hz,
        }
    }
}

/// `GROUP BY parent(h3_index)` (Deviation 7): coarse-grains `cells` — each
/// `(CellIndex, Extensives)` pair rolls up to `parent(coarse_res)`, extensives
/// summed. Output order is the order each parent is FIRST encountered while
/// scanning `cells` (deterministic, independent of any hash order — no
/// `HashMap` iteration order leaks into the returned `Vec`). Bounded:
/// `cells.len()` iterations, `cells.len()` worst-case output rows.
pub fn aggregate_by_parent(
    cells: &[(CellIndex, Extensives)],
    coarse_res: Resolution,
) -> Vec<(CellIndex, Extensives)> {
    let mut order: Vec<CellIndex> = Vec::with_capacity(cells.len());
    let mut by_parent: std::collections::HashMap<CellIndex, Extensives> =
        std::collections::HashMap::with_capacity(cells.len());

    for &(cell, ext) in cells {
        let parent = cell
            .parent(coarse_res)
            .expect("coarse_res must be <= the fine cell's resolution");
        match by_parent.get_mut(&parent) {
            Some(acc) => *acc = acc.merge(ext),
            None => {
                by_parent.insert(parent, ext);
                order.push(parent);
            }
        }
    }

    order
        .into_iter()
        .map(|parent| (parent, by_parent[&parent]))
        .collect()
}
