//! The one error type of this crate.

use hypergraph_rs::geometry::GeometryError;

/// Why a style, a config, a frame or a shape was refused.
///
/// Every variant names the item at fault, so a caller can show the message
/// as it is.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FieldError {
    /// An index points past the end of a list the caller defined.
    #[error("{what} {index} is not defined (there are {len})")]
    UnknownStyle {
        /// The list: `"swatch"`, `"particle style"`, `"set style"`, `"stream"` or `"beat"`.
        what: &'static str,
        /// The index that was used.
        index: u16,
        /// The length of the list.
        len: usize,
    },
    /// An id appears twice where each must be unique.
    #[error("{0} appears twice")]
    DuplicateId(String),
    /// A region is named that no ground holds.
    #[error("region {0} is in no ground")]
    UnknownRegion(String),
    /// A number is `NaN` or infinite.
    #[error("{item} is not finite")]
    NonFinite {
        /// The item that holds the number.
        item: String,
    },
    /// A tone is outside `-1..=1`.
    #[error("tone {value} of {region} is outside -1..=1")]
    ToneOutOfRange {
        /// The region the tone belongs to.
        region: String,
        /// The tone.
        value: f32,
    },
    /// A parameter is out of its range.
    #[error("parameter {name} = {value} is out of range")]
    BadParameter {
        /// The parameter's name.
        name: &'static str,
        /// The value given.
        value: f64,
    },
    /// A region's shape cannot be drawn.
    #[error("region {region}: {problem}")]
    BadShape {
        /// The region.
        region: String,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// An outline of a kind this crate does not draw.
    #[error("an outline of a kind this crate does not draw")]
    UnsupportedOutline,
    /// The core's geometry refused its input.
    #[error(transparent)]
    Geometry(#[from] GeometryError),
}
