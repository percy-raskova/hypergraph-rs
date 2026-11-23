//! A sampler of frame intervals: fixed-size windows, each with its median and
//! its 95th percentile, after a time of settling.

use std::time::Duration;

/// The figures of one window of frame intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowStats {
    /// The window's number, from zero.
    pub index: usize,
    /// The median interval: the mean of the two middle ones for an even window.
    pub median: Duration,
    /// The 95th percentile by nearest rank: `sorted[ceil(0.95 n) - 1]`.
    pub p95: Duration,
}

/// Collects frame intervals into windows.
///
/// An interval is ignored while the sum of the intervals ignored before it is
/// below `settle` (so the interval that completes the settle time is ignored
/// too). After that every `window` intervals make one [`WindowStats`].
#[derive(Clone, Debug)]
pub struct FrameWindows {
    window: usize,
    settle: Duration,
    ignored: Duration,
    pending: Vec<Duration>,
    next_index: usize,
    slowest: Option<Duration>,
}

impl FrameWindows {
    /// A sampler of windows of `window` intervals after `settle` of settling.
    ///
    /// # Panics
    ///
    /// If `window` is zero.
    #[must_use]
    pub fn new(window: usize, settle: Duration) -> Self {
        assert!(window > 0, "a window holds at least one interval");
        Self {
            window,
            settle,
            ignored: Duration::ZERO,
            pending: Vec::with_capacity(window),
            next_index: 0,
            slowest: None,
        }
    }

    /// Adds one interval; returns the window's figures when it fills.
    pub fn push(&mut self, interval: Duration) -> Option<WindowStats> {
        if self.ignored < self.settle {
            self.ignored += interval;
            return None;
        }
        self.pending.push(interval);
        if self.pending.len() < self.window {
            return None;
        }
        self.pending.sort();
        let n = self.pending.len();
        let median = if n.is_multiple_of(2) {
            (self.pending[n / 2 - 1] + self.pending[n / 2]) / 2
        } else {
            self.pending[n / 2]
        };
        let p95 = self.pending[(95 * n).div_ceil(100) - 1];
        self.pending.clear();
        let stats = WindowStats {
            index: self.next_index,
            median,
            p95,
        };
        self.next_index += 1;
        self.slowest = Some(self.slowest.map_or(p95, |s| s.max(p95)));
        Some(stats)
    }

    /// The largest 95th percentile of the windows so far.
    #[must_use]
    pub fn slowest_p95(&self) -> Option<Duration> {
        self.slowest
    }
}
