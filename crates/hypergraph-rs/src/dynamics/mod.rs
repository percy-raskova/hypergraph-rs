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
