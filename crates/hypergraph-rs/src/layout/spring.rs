//! Fruchterman–Reingold spring layout on the 1-skeleton, plus the
//! hyperedge-aware variant. Hand-rolled (no rustworkx-core layout exists) and
//! seeded by `rand_pcg::Pcg64Mcg` for Constitution III.7 determinism.

use std::collections::{HashMap, HashSet};

use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg64Mcg;

use crate::core::hypergraph::Hypergraph;

use super::hull::convex_hull;
use super::{bounding_box_of, HyperedgeGeometry, LayoutResult, DEFAULT_LAYOUT_SEED};

/// Build the 1-skeleton (pairwise projection) as an edge list over node
/// indices. Every pair of members of each hyperedge becomes one undirected
/// edge (deduplicated). Node indices are positions in `node_ids` (insertion
/// order). Returns `(node_ids, edges)`.
///
/// The internal `HashMap`/`HashSet` are lookup/dedup only — the returned
/// `edges` order is derived from `edge_ids()` (true insertion order) and
/// `members()` (deterministic petgraph adjacency order, NOT insertion order —
/// see Phase 0+1 `members()`), so the result is deterministic though not
/// insertion-ordered end to end.
fn skeleton_edges<N, E, M>(h: &Hypergraph<N, E, M>) -> (Vec<String>, Vec<(usize, usize)>) {
    let node_ids = h.node_ids();
    let mut index_of: HashMap<&str, usize> = HashMap::new();
    for (i, id) in node_ids.iter().enumerate() {
        index_of.insert(id.as_str(), i);
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        for a in 0..members.len() {
            for b in (a + 1)..members.len() {
                let ia = index_of[members[a].as_str()];
                let ib = index_of[members[b].as_str()];
                let key = if ia < ib { (ia, ib) } else { (ib, ia) };
                if seen.insert(key) {
                    edges.push(key);
                }
            }
        }
    }
    (node_ids, edges)
}

/// Fruchterman–Reingold force-directed placement of `n` nodes given the
/// undirected `edges` (index pairs). `iterations` cooling steps, seeded RNG.
/// Every loop is bounded by `n`, `iterations`, or `edges.len()`.
fn fruchterman_reingold(
    n: usize,
    edges: &[(usize, usize)],
    iterations: usize,
    seed: u64,
) -> Vec<(f64, f64)> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|_| (rng.random_range(0.0..1.0), rng.random_range(0.0..1.0)))
        .collect();
    if n <= 1 {
        return pos;
    }
    let k = (1.0f64 / n as f64).sqrt();
    let mut t = 0.1f64;
    let dt = t / (iterations as f64 + 1.0);

    for _ in 0..iterations {
        let mut disp = vec![(0.0f64, 0.0f64); n];
        // Repulsive forces: every ordered pair.
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                let force = k * k / dist;
                disp[i].0 += dx / dist * force;
                disp[i].1 += dy / dist * force;
            }
        }
        // Attractive forces: each skeleton edge.
        for &(a, b) in edges {
            let dx = pos[a].0 - pos[b].0;
            let dy = pos[a].1 - pos[b].1;
            let dist = (dx * dx + dy * dy).sqrt().max(0.01);
            let force = dist * dist / k;
            let ox = dx / dist * force;
            let oy = dy / dist * force;
            disp[a].0 -= ox;
            disp[a].1 -= oy;
            disp[b].0 += ox;
            disp[b].1 += oy;
        }
        // Apply displacement, capped by the cooling temperature.
        for i in 0..n {
            let d = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1)
                .sqrt()
                .max(0.01);
            let capped = d.min(t);
            pos[i].0 += disp[i].0 / d * capped;
            pos[i].1 += disp[i].1 / d * capped;
        }
        t -= dt;
    }
    pos
}

/// Look up a node's position from an ordered position list (linear scan;
/// acceptable at Babylon's ~10^3-node scale).
fn pos_of(positions: &[(String, (f64, f64))], id: &str) -> (f64, f64) {
    positions
        .iter()
        .find(|(n, _)| n == id)
        .map(|(_, p)| *p)
        .unwrap_or((0.0, 0.0))
}

/// Spring layout: FR positions for agents; each hyperedge is a `Spline` through
/// its members' positions, in `members()` traversal order (deterministic
/// petgraph adjacency order — NOT insertion order; only `node_ids()`/
/// `edge_ids()` are true insertion order, see Phase 0+1 `members()`).
/// `seed = None` uses `DEFAULT_LAYOUT_SEED`.
pub fn spring_layout<N, E, M>(
    h: &Hypergraph<N, E, M>,
    iterations: usize,
    seed: Option<u64>,
) -> LayoutResult {
    let (node_ids, edges) = skeleton_edges(h);
    let seed = seed.unwrap_or(DEFAULT_LAYOUT_SEED);
    let raw = fruchterman_reingold(node_ids.len(), &edges, iterations, seed);
    let node_positions: Vec<(String, (f64, f64))> =
        node_ids.iter().cloned().zip(raw.iter().copied()).collect();

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let control_points: Vec<(f64, f64)> =
                members.iter().map(|m| pos_of(&node_positions, m)).collect();
            (eid.clone(), HyperedgeGeometry::Spline { control_points })
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}

/// Expand a convex hull outward from its centroid by `padding`. A degenerate
/// hull (single point) is returned unchanged; a 2-point hull is pushed apart
/// along its line.
fn pad_hull(hull: &[(f64, f64)], padding: f64) -> Vec<(f64, f64)> {
    if hull.is_empty() || padding == 0.0 {
        return hull.to_vec();
    }
    let count = hull.len() as f64;
    let cx = hull.iter().map(|p| p.0).sum::<f64>() / count;
    let cy = hull.iter().map(|p| p.1).sum::<f64>() / count;
    hull.iter()
        .map(|&(x, y)| {
            let dx = x - cx;
            let dy = y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 1e-9 {
                (x, y)
            } else {
                (x + dx / dist * padding, y + dy / dist * padding)
            }
        })
        .collect()
}

/// Hyperedge-aware layout: FR agent positions (seeded at `DEFAULT_LAYOUT_SEED`),
/// each hyperedge rendered as the `halo_padding`-expanded convex hull of its
/// members. `halo_padding` corresponds to XGI's halo "radius margin"
/// (`draw.py:199-200`, default 0.05).
pub fn hyperedge_aware_layout<N, E, M>(
    h: &Hypergraph<N, E, M>,
    iterations: usize,
    halo_padding: f64,
) -> LayoutResult {
    let (node_ids, edges) = skeleton_edges(h);
    let raw = fruchterman_reingold(node_ids.len(), &edges, iterations, DEFAULT_LAYOUT_SEED);
    let node_positions: Vec<(String, (f64, f64))> =
        node_ids.iter().cloned().zip(raw.iter().copied()).collect();

    let edge_geometries: Vec<(String, HyperedgeGeometry)> = h
        .edge_ids()
        .iter()
        .map(|eid| {
            let members = h.members(eid).unwrap_or_default();
            let member_pts: Vec<(f64, f64)> =
                members.iter().map(|m| pos_of(&node_positions, m)).collect();
            let hull = convex_hull(&member_pts);
            let padded = pad_hull(&hull, halo_padding);
            (
                eid.clone(),
                HyperedgeGeometry::ConvexHull { points: padded },
            )
        })
        .collect();

    let bounding_box = bounding_box_of(&node_positions, &edge_geometries);
    LayoutResult {
        node_positions,
        edge_geometries,
        bounding_box,
    }
}
