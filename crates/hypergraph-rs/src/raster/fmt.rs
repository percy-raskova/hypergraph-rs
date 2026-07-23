//! Deterministic float formatter — copied verbatim from `viz/svg.rs:109-117`
//! (private there) so golden fixtures across `raster/` share the exact same
//! byte-exact float→text contract. Do NOT alter the algorithm.

/// Deterministic fixed-precision formatting (6 decimals, trailing zeros
/// trimmed, `-0` normalised to `0`).
pub fn fmt6(v: f64) -> String {
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s
    }
}
