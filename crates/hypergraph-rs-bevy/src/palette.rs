//! The caller's colours.

use bevy::color::{Color, LinearRgba};
use bevy::ecs::resource::Resource;

/// Every colour the crate draws with. The caller supplies all of them; there
/// is no default.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Palette {
    /// The ramp of a tone: the colour at `-1`, at `0` and at `1`.
    pub ramp: [Color; 3],
    /// The colour of a region with no reading.
    pub no_reading: Color,
    /// The colours that styles name by index.
    pub swatches: Vec<Color>,
}

fn weighted(a: LinearRgba, wa: f32, b: LinearRgba, wb: f32) -> LinearRgba {
    LinearRgba {
        red: a.red * wa + b.red * wb,
        green: a.green * wa + b.green * wb,
        blue: a.blue * wa + b.blue * wb,
        alpha: a.alpha * wa + b.alpha * wb,
    }
}

impl Palette {
    /// The colour of a tone, in linear RGBA. `None` is [`Palette::no_reading`].
    ///
    /// A tone is clamped to `-1..=1`. Below zero the colour runs from
    /// `ramp[1]` to `ramp[0]`, from zero up it runs from `ramp[1]` to
    /// `ramp[2]`; each ramp colour comes out exactly at its own end.
    pub fn tone(&self, tone: Option<f32>) -> LinearRgba {
        let Some(t) = tone else {
            return self.no_reading.to_linear();
        };
        let t = t.clamp(-1.0, 1.0);
        let [low, mid, high] = self.ramp.map(|c| c.to_linear());
        if t < 0.0 {
            weighted(low, -t, mid, 1.0 + t)
        } else {
            weighted(mid, 1.0 - t, high, t)
        }
    }

    /// The swatch at `index`, if there is one.
    pub fn swatch(&self, index: u16) -> Option<Color> {
        self.swatches.get(usize::from(index)).copied()
    }
}
