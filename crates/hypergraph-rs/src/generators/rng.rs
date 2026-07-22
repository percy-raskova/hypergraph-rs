//! Seeded RNG + sampling + combinatorics helpers for the generators.
//!
//! We drive `rand_pcg::Pcg64Mcg` through the version-stable `rand_core` traits
//! (`SeedableRng::seed_from_u64` + `RngCore::next_u64`) and derive all sampling
//! primitives ourselves. This sidesteps rand 0.10's `Rng`/`RngExt` trait split
//! (the probe showed `random_range` moved to `RngExt`) and guarantees a fixed,
//! documented f64 construction for cross-build reproducibility.

use rand_core::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Build a `Pcg64Mcg` from an optional seed. `None` uses a fixed default seed so
/// that "unseeded" callers are STILL deterministic within a process/build
/// (III.7: non-determinism is a bug — we never reach for entropy).
pub fn seeded_rng(seed: Option<u64>) -> Pcg64Mcg {
    Pcg64Mcg::seed_from_u64(seed.unwrap_or(0))
}

/// A uniform f64 in `[0, 1)` from 53 bits of the generator (IEEE-754 double
/// mantissa). Fixed construction: `(next_u64() >> 11) / 2^53`.
pub fn uniform_f64(rng: &mut Pcg64Mcg) -> f64 {
    let bits = rng.next_u64() >> 11;
    (bits as f64) * (1.0f64 / ((1u64 << 53) as f64))
}

/// A uniform integer in `[0, n)`. Panics via debug-assert if `n == 0`.
/// Uses modulo reduction (deterministic; slight low-bit bias is acceptable —
/// generator conformance is structural, not sequence-exact).
pub fn random_below(rng: &mut Pcg64Mcg, n: u64) -> u64 {
    debug_assert!(n > 0, "random_below requires n > 0");
    rng.next_u64() % n
}

/// Sample `k` distinct indices from `0..pool_len` (partial Fisher–Yates).
/// Returns them in draw order. Requires `k <= pool_len`.
pub fn choice_without_replacement(rng: &mut Pcg64Mcg, pool_len: usize, k: usize) -> Vec<usize> {
    debug_assert!(k <= pool_len, "cannot draw {k} distinct from {pool_len}");
    let mut idx: Vec<usize> = (0..pool_len).collect();
    let mut out = Vec::with_capacity(k);
    // Bounded loop: exactly k iterations.
    for i in 0..k {
        let remaining = pool_len - i;
        let j = i + (random_below(rng, remaining as u64) as usize);
        idx.swap(i, j);
        out.push(idx[i]);
    }
    out
}

/// The geometric sampler (number of trials to first success), clamped to `>= 1`.
///
/// XGI: `ceil(log(r) / log(1 - p))`, `p == 1 -> 1`, `p == 0 -> inf`
/// (`xgi/utils/utilities.py:564`). We clamp to `max(1)` so the `index += geometric`
/// loops in the random/uniform generators strictly advance (Power-of-10 rule 2).
pub fn geometric(p: f64, rng: &mut Pcg64Mcg) -> u64 {
    if p >= 1.0 {
        return 1;
    }
    if p <= 0.0 {
        return u64::MAX; // "infinity" sentinel; callers guard p > 0
    }
    let mut r = uniform_f64(rng);
    if r <= 0.0 {
        r = f64::MIN_POSITIVE; // avoid log(0)
    }
    let g = (r.ln() / (1.0 - p).ln()).ceil();
    let g = if g.is_finite() && g >= 1.0 {
        g as u64
    } else {
        1
    };
    g.max(1)
}

/// Exact binomial coefficient `C(n, k)` (0 if `k > n`). Multiplicative form,
/// integer-exact because the running product of `k` consecutive integers is
/// divisible by `k!` at each step.
///
/// FINDING 2: the running product `acc * (n - i)` (computed BEFORE dividing by
/// `(i + 1)`) can exceed `u64::MAX` even when the final `C(n, k)` itself fits
/// in a `u64` (e.g. `comb(100, 16)` — see `test_comb_large_but_legal_does_not_panic_and_matches_oracle`).
/// We accumulate in `u128` (headroom for any `n, k` we realistically see —
/// XGI itself uses `scipy.special.comb(..., exact=True)`, i.e. Python
/// bigints) and only narrow to `u64` once, at the end. If even the `u128`
/// product overflows (astronomically large `n`/`k`) or the final value
/// exceeds `u64::MAX`, we saturate to `u64::MAX` rather than panic/wrap: at
/// that magnitude the geometric-skip callers (`fast_random_hypergraph`,
/// `uniform_erdos_renyi_hypergraph`) would never exhaustively enumerate the
/// combination space anyway, so a saturated `max_index` is a documented,
/// harmless approximation, not a correctness hazard.
pub fn comb(n: u64, k: u64) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut acc: u128 = 1;
    // Bounded loop: exactly k iterations.
    for i in 0..k {
        let numerator = match acc.checked_mul((n - i) as u128) {
            Some(v) => v,
            None => return u64::MAX,
        };
        acc = numerator / ((i + 1) as u128);
    }
    acc.min(u64::MAX as u128) as u64
}

/// Map a linear index to the `m`-combination of `0..n` at that index (sorted,
/// strictly increasing). Port of `_index_to_edge_comb` (`uniform.py:457`).
pub fn index_to_edge_comb(index: u64, n: u64, m: u64) -> Vec<usize> {
    let mut c: Vec<usize> = Vec::with_capacity(m as usize);
    let mut r = index as i128 + 1; // 1-indexed internally
    let mut j: i128 = -1;
    // Bounded loop: exactly m iterations.
    for s in 1..=m {
        let mut cs = j + 1;
        // Bounded: cs climbs within 0..n, at most n steps.
        loop {
            let take = comb((n as i128 - 1 - cs) as u64, m - s) as i128;
            if r - take > 0 {
                r -= take;
                cs += 1;
            } else {
                break;
            }
        }
        c.push(cs as usize);
        j = cs;
    }
    c
}

/// Map a linear index to an `m`-tuple over `0..n` with repetition (product order,
/// last coordinate fastest). Port of `_index_to_edge_prod` (`uniform.py:405`).
pub fn index_to_edge_prod(index: u64, n: u64, m: u64) -> Vec<usize> {
    let mut out = Vec::with_capacity(m as usize);
    // Bounded loop: exactly m iterations.
    for r in (0..m).rev() {
        let place = n.pow(r as u32);
        out.push(((index / place) % n) as usize);
    }
    out
}
