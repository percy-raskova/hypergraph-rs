//! Easing, the beat clock, and where a mark is within its beat.

use std::time::Duration;

use bevy::prelude::*;

use crate::config::Motion;

/// `x²(3 − 2x)` with `x` clamped to `[0, 1]`.
pub fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// When mark `k` of `n` of flow `flow` starts inside a beat of length `beat`.
///
/// Marks of one flow are evenly spaced through the beat, flows do not start in
/// step, and every mark arrives before the beat ends: a mark travels for
/// `travel × beat`.
pub fn mark_start(flow: usize, k: u32, n: u32, beat: Duration, travel: f32) -> Duration {
    let b = beat.as_secs_f32();
    let tau = travel * b;
    let phi = (fnv1a(&(flow as u64).to_le_bytes()) % 1024) as f32 / 1024.0;
    let n = n.max(1) as f32;
    let secs = ((k as f32 + phi) / n) * (b - tau);
    if secs.is_finite() && secs > 0.0 {
        Duration::from_secs_f32(secs)
    } else {
        Duration::ZERO
    }
}

/// The position of a mark a fraction `u` of the way from `from` to `to`.
///
/// It eases along the straight line and bows sideways by a parabola in `u`
/// (zero at both ends), by `bow × length × side`. `side` is `+1` or `−1`.
pub fn mark_position(from: Vec3, to: Vec3, u: f32, bow: f32, side: f32) -> Vec3 {
    let s = smoothstep(u);
    let delta = to - from;
    let length = delta.x.hypot(delta.z);
    let perp = if length > 0.0 {
        Vec3::new(-delta.z, 0.0, delta.x) / length
    } else {
        Vec3::ZERO
    };
    from + delta * s + perp * (bow * length * side * 4.0 * u * (1.0 - u))
}

/// Where the beats are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// No beat is playing.
    Resting,
    /// Beat `index` is playing, `fraction` of the way through it.
    Beat {
        /// The beat's index.
        index: usize,
        /// How far into it, `0..1`.
        fraction: f32,
    },
}

/// The clock of one period's beats. It rests until started.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct BeatClock {
    started_at: Option<Duration>,
    period: Option<u64>,
}

impl BeatClock {
    /// Start the beats of `period` at `now`.
    pub fn start(&mut self, now: Duration, period: u64) {
        self.started_at = Some(now);
        self.period = Some(period);
    }

    /// The phase at `now`: `Resting` when never started or past every beat.
    pub fn phase(&self, now: Duration, beats: &[Duration]) -> Phase {
        let Some(start) = self.started_at else {
            return Phase::Resting;
        };
        let elapsed = now.saturating_sub(start);
        let mut begin = Duration::ZERO;
        for (index, beat) in beats.iter().enumerate() {
            let end = begin + *beat;
            if elapsed < end {
                let fraction = (elapsed - begin).as_secs_f32() / beat.as_secs_f32();
                return Phase::Beat { index, fraction };
            }
            begin = end;
        }
        Phase::Resting
    }

    /// The period last started, if any.
    pub fn period(&self) -> Option<u64> {
        self.period
    }
}

/// A value moving from `from` to `to`, as the beats say.
///
/// Resting and beats after `ease_beat` give `to`; beats before it give `from`;
/// during it `Full` eases by `smoothstep` and `Reduced` holds `from` (it jumps
/// when the beat ends).
pub fn eased(from: f32, to: f32, phase: Phase, ease_beat: usize, motion: Motion) -> f32 {
    match phase {
        Phase::Resting => to,
        Phase::Beat { index, .. } if index < ease_beat => from,
        Phase::Beat { index, .. } if index > ease_beat => to,
        Phase::Beat { fraction, .. } => match motion {
            Motion::Full => from + (to - from) * smoothstep(fraction),
            Motion::Reduced => from,
        },
    }
}
