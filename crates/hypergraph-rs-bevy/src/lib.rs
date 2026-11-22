//! Bevy integration for hypergraph-rs.
//!
//! This crate draws any hypergraph in Bevy: a ground of regions that bends or
//! rises and sinks in blocks, hatched outlines of hyperedges, particles, and
//! marks that move along paths in beats. It reads the core crate's `geometry`
//! module for outlines and positions. Everything it draws is described in
//! terms of hypergraphs, regions, particles, sets, streams and amounts; the
//! caller supplies every colour and every style.
//!
//! This module holds the palette, the styles, the config and the error type.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod config;
mod error;
mod palette;
mod style;

pub use config::{FieldConfig, Motion};
pub use error::FieldError;
pub use palette::Palette;
pub use style::{
    FieldStyles, Glyph, Hatch, HatchDirection, LineStyle, ParticleStyle, SetStyle, StreamStyle,
};
