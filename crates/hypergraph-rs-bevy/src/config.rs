//! The clock and the shape of the motion.

use std::time::Duration;

use bevy::ecs::resource::Resource;

use crate::FieldError;

/// How much the picture moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    /// Everything eases and moves.
    #[default]
    Full,
    /// Changes jump at the end of the ease beat; marks do not travel.
    Reduced,
}

/// The beats of one period and the numbers that shape the motion.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct FieldConfig {
    /// The length of each beat.
    pub beats: Vec<Duration>,
    /// The beat in which the picture eases to the new frame.
    pub ease_beat: usize,
    /// The share of a beat that one mark spends travelling, in `(0, 1]`.
    pub mark_travel: f32,
    /// How far a mark's path bows sideways, as a share of its length.
    pub bow: f32,
    /// Full or reduced motion.
    pub motion: Motion,
    /// Segments in a drawn curve or ring (at least 8).
    pub curve_segments: u32,
    /// How far an outline sits above its surface, in scene units.
    pub outline_lift: f32,
}

fn bad(name: &'static str, value: f64) -> FieldError {
    FieldError::BadParameter { name, value }
}

impl FieldConfig {
    /// Three beats of one length, the ease in the last, with the usual
    /// travel (0.4), bow (0.08), 48 curve segments and a lift of 0.004.
    pub fn three_beats(beat: Duration) -> Self {
        Self {
            beats: vec![beat; 3],
            ease_beat: 2,
            mark_travel: 0.4,
            bow: 0.08,
            motion: Motion::Full,
            curve_segments: 48,
            outline_lift: 0.004,
        }
    }

    /// Checks every field; the error names the one at fault.
    pub fn validate(&self) -> Result<(), FieldError> {
        if self.beats.is_empty() {
            return Err(bad("beats", 0.0));
        }
        if let Some(b) = self.beats.iter().find(|b| b.is_zero()) {
            return Err(bad("beats", b.as_secs_f64()));
        }
        if self.ease_beat >= self.beats.len() {
            return Err(bad("ease_beat", self.ease_beat as f64));
        }
        if !(self.mark_travel > 0.0 && self.mark_travel <= 1.0) {
            return Err(bad("mark_travel", f64::from(self.mark_travel)));
        }
        if !(self.bow.is_finite() && self.bow >= 0.0) {
            return Err(bad("bow", f64::from(self.bow)));
        }
        if self.curve_segments < 8 {
            return Err(bad("curve_segments", f64::from(self.curve_segments)));
        }
        if !self.outline_lift.is_finite() {
            return Err(bad("outline_lift", f64::from(self.outline_lift)));
        }
        Ok(())
    }
}
