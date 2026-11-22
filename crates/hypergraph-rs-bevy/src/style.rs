//! The styles that frames name by index.

use std::num::NonZeroU64;

use bevy::ecs::resource::Resource;

use crate::{FieldConfig, FieldError, Palette};

/// The shape of a particle or a mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Glyph {
    /// A filled disc.
    Disc,
    /// A disc with a hole, the same outer size.
    Ring,
    /// A filled square.
    Square,
    /// A square with a hole, the same outer size.
    SquareFrame,
    /// A short bar that trails behind its head.
    Streak,
}

/// The direction of hatch strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchDirection {
    /// Along +x.
    Deg0,
    /// Up and to the right.
    Deg45,
    /// Along +y.
    Deg90,
    /// Up and to the left.
    Deg135,
}

/// The stroke of an outline.
#[derive(Clone, Debug, PartialEq)]
pub struct LineStyle {
    /// Stroke width in scene units.
    pub width: f32,
    /// Dash and gap lengths in scene units; `None` is a solid line.
    pub dash: Option<(f32, f32)>,
}

/// Parallel strokes inside an outline.
#[derive(Clone, Debug, PartialEq)]
pub struct Hatch {
    /// The direction of the strokes.
    pub direction: HatchDirection,
    /// The distance between strokes.
    pub spacing: f32,
    /// The width of a stroke.
    pub width: f32,
}

/// How one kind of particle looks.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleStyle {
    /// The shape.
    pub glyph: Glyph,
    /// The size, in scene units.
    pub size: f32,
    /// The index of its colour in the palette.
    pub swatch: u16,
}

/// How one kind of set looks.
#[derive(Clone, Debug, PartialEq)]
pub struct SetStyle {
    /// The index of its colour in the palette.
    pub swatch: u16,
    /// The outline.
    pub line: LineStyle,
    /// Hatching inside the outline, if any.
    pub hatch: Option<Hatch>,
}

/// How one stream of marks looks and how much of it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamStyle {
    /// The beat in which its marks move.
    pub beat: usize,
    /// How much one mark stands for.
    pub unit: NonZeroU64,
    /// The glyph of a counted mark.
    pub full: Glyph,
    /// The glyph of a mark that is shown but not counted.
    pub hollow: Glyph,
    /// The size of a mark, in scene units.
    pub size: f32,
    /// The index of its colour in the palette.
    pub swatch: u16,
    /// The share of the amount that is drawn, in `(0, 1]`.
    pub share_drawn: f64,
    /// The most marks one flow may have (at least 1).
    pub max_marks: u32,
}

/// Every style a frame can name.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct FieldStyles {
    /// Particle styles, by index.
    pub particles: Vec<ParticleStyle>,
    /// Set styles, by index.
    pub sets: Vec<SetStyle>,
    /// Stream styles, by index.
    pub streams: Vec<StreamStyle>,
    /// The swatch of region outlines; `None` draws none.
    pub region_outline: Option<u16>,
}

fn bad(name: &'static str, value: f64) -> FieldError {
    FieldError::BadParameter { name, value }
}

fn positive(name: &'static str, value: f32) -> Result<(), FieldError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(bad(name, f64::from(value)))
    }
}

fn swatch(palette: &Palette, index: u16) -> Result<(), FieldError> {
    if usize::from(index) < palette.swatches.len() {
        Ok(())
    } else {
        Err(FieldError::UnknownStyle {
            what: "swatch",
            index,
            len: palette.swatches.len(),
        })
    }
}

impl FieldStyles {
    /// Checks every style against the palette and the config: swatches and
    /// beats exist, sizes, widths and spacings are finite and above zero.
    pub fn validate(&self, palette: &Palette, config: &FieldConfig) -> Result<(), FieldError> {
        for p in &self.particles {
            swatch(palette, p.swatch)?;
            positive("size", p.size)?;
        }
        for s in &self.sets {
            swatch(palette, s.swatch)?;
            positive("width", s.line.width)?;
            if let Some((on, off)) = s.line.dash {
                if !(on.is_finite() && on > 0.0 && off.is_finite() && off > 0.0) {
                    let value = if on.is_finite() && on > 0.0 { off } else { on };
                    return Err(bad("dash", f64::from(value)));
                }
            }
            if let Some(h) = &s.hatch {
                positive("spacing", h.spacing)?;
                positive("width", h.width)?;
            }
        }
        for s in &self.streams {
            swatch(palette, s.swatch)?;
            if s.beat >= config.beats.len() {
                return Err(FieldError::UnknownStyle {
                    what: "beat",
                    index: u16::try_from(s.beat).unwrap_or(u16::MAX),
                    len: config.beats.len(),
                });
            }
            positive("size", s.size)?;
            if !(s.share_drawn > 0.0 && s.share_drawn <= 1.0) {
                return Err(bad("share_drawn", s.share_drawn));
            }
            if s.max_marks < 1 {
                return Err(bad("max_marks", f64::from(s.max_marks)));
            }
        }
        if let Some(i) = self.region_outline {
            swatch(palette, i)?;
        }
        Ok(())
    }
}
