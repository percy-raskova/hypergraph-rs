//! Kuramoto synchronization dynamics ported from `xgi/dynamics/synchronization.py`.
//!
//! NOTE: XGI's `dynamics/` slice is Kuramoto synchronization, NOT epidemic (SIR/SIS)
//! models — the spec's "epidemic" summary is wrong (see the Phase-6 plan header). This
//! module ports the pieces whose dependencies exist by Phase 6: `simulate_kuramoto`
//! (pairwise + triangle coupling) and `compute_kuramoto_order_parameter`. The simplicial
//! Kuramoto pair needs the Phase-3 Hodge boundary matrix + `SimplicialComplex` and is
//! deferred.
//!
//! Complex arithmetic is done manually with `(re, im)` f64 pairs (no `num-complex`
//! dependency). Randomness (default `omega`/`theta`) uses seeded `rand_pcg::Pcg64Mcg`;
//! sequence-level parity with numpy is NOT claimed (spec §9.4).

use indexmap::IndexMap;
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;

use crate::Hypergraph;

/// The Kuramoto order parameter timeseries.
///
/// XGI parity: `xgi.compute_kuramoto_order_parameter`
/// (`dynamics/synchronization.py:121-141`). For each timestep `t`,
/// `r_t = |(1/N) Σ_n exp(i·theta[t][n])|` — a synchrony measure in `[0, 1]`.
pub fn compute_kuramoto_order_parameter(theta_time: &[Vec<f64>]) -> Vec<f64> {
    theta_time
        .iter()
        .map(|row| {
            let n = row.len();
            if n == 0 {
                return 0.0;
            }
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for &theta in row {
                re += theta.cos();
                im += theta.sin();
            }
            re /= n as f64;
            im /= n as f64;
            (re * re + im * im).sqrt()
        })
        .collect()
}

/// Accumulate `exp(i·phi)` into a `(re, im)` complex accumulator.
#[inline]
fn add_exp(acc: &mut (f64, f64), phi: f64) {
    acc.0 += phi.cos();
    acc.1 += phi.sin();
}

/// Imaginary part of `r · exp(-i·theta)` for `r = (re, im)`:
/// `Im((re + i·im)(cos θ − i·sin θ)) = im·cos θ − re·sin θ`.
#[inline]
fn imag_times_conj(r: (f64, f64), theta: f64) -> f64 {
    r.1 * theta.cos() - r.0 * theta.sin()
}

/// Simulate the Kuramoto model on a hypergraph (pairwise + triangle coupling).
///
/// XGI parity: `xgi.simulate_kuramoto` (`dynamics/synchronization.py:18-118`). Integrates
/// the phase ODE via the explicit Euler method over `timesteps` steps of size `dt`.
/// Returns `(theta_time, times)` where `theta_time[t][n]` is node `n`'s phase at step `t`.
///
/// Node indices follow `h.node_ids()` insertion order. `links` are size-2 edges and
/// `triangles` are size-3 edges (other sizes do not contribute — matching XGI's
/// `filterby("size", 2 | 3)`). Defaulted `omega` ~ N(0,1) (Box–Muller) and
/// `theta` ~ U[0,2π) are drawn from a seeded `Pcg64Mcg`; supply both for exact
/// determinism independent of the RNG.
#[allow(clippy::too_many_arguments)]
pub fn simulate_kuramoto(
    h: &Hypergraph,
    k2: f64,
    k3: f64,
    omega: Option<Vec<f64>>,
    theta: Option<Vec<f64>>,
    timesteps: usize,
    dt: f64,
    seed: Option<u64>,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = h.num_nodes();
    let node_index: IndexMap<String, usize> = h
        .node_ids()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    // Extract links (size 2) and triangles (size 3) as index tuples.
    let mut links: Vec<(usize, usize)> = Vec::new();
    let mut triangles: Vec<(usize, usize, usize)> = Vec::new();
    for e in h.edge_ids() {
        let members = h.members(&e).unwrap_or_default();
        match members.len() {
            2 => links.push((node_index[&members[0]], node_index[&members[1]])),
            3 => triangles.push((
                node_index[&members[0]],
                node_index[&members[1]],
                node_index[&members[2]],
            )),
            _ => {}
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed.unwrap_or(0));

    let omega: Vec<f64> = omega.unwrap_or_else(|| {
        // N(0, 1) via Box–Muller from the seeded PCG.
        (0..n)
            .map(|_| {
                let u1: f64 = 1.0 - rng.random_range(0.0..1.0); // in (0, 1]
                let u2: f64 = rng.random_range(0.0..1.0);
                (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
            })
            .collect()
    });
    let mut theta: Vec<f64> = theta.unwrap_or_else(|| {
        (0..n)
            .map(|_| rng.random_range(0.0..1.0) * 2.0 * std::f64::consts::PI)
            .collect()
    });

    let mut theta_time: Vec<Vec<f64>> = vec![vec![0.0; n]; timesteps];
    let times: Vec<f64> = (0..timesteps).map(|t| t as f64 * dt).collect();

    // `t` indexes `theta_time` only, but the loop body below also needs the plain
    // integer count (dt-scaled elsewhere); the index form reads clearer than an
    // iterator adapter here.
    #[allow(clippy::needless_range_loop)]
    for t in 0..timesteps {
        theta_time[t] = theta.clone();

        let mut r1 = vec![(0.0_f64, 0.0_f64); n];
        let mut r2 = vec![(0.0_f64, 0.0_f64); n];

        for &(i, j) in &links {
            add_exp(&mut r1[i], theta[j]);
            add_exp(&mut r1[j], theta[i]);
        }
        for &(i, j, k) in &triangles {
            add_exp(&mut r2[i], 2.0 * theta[j] - theta[k]);
            add_exp(&mut r2[i], 2.0 * theta[k] - theta[j]);
            add_exp(&mut r2[j], 2.0 * theta[i] - theta[k]);
            add_exp(&mut r2[j], 2.0 * theta[k] - theta[i]);
            add_exp(&mut r2[k], 2.0 * theta[i] - theta[j]);
            add_exp(&mut r2[k], 2.0 * theta[j] - theta[i]);
        }

        let mut next = vec![0.0_f64; n];
        for idx in 0..n {
            let d_theta = omega[idx]
                + k2 * imag_times_conj(r1[idx], theta[idx])
                + k3 * imag_times_conj(r2[idx], theta[idx]);
            next[idx] = theta[idx] + d_theta * dt;
        }
        theta = next;
    }

    (theta_time, times)
}
