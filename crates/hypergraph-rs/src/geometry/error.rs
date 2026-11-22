//! The errors of the geometry module.

/// What can go wrong in a geometry function.
///
/// In functions that take slices, `item` is the index written as `#<index>`; in functions that read a hypergraph it
/// is the node or edge id.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// A weight is negative or not finite.
    #[error("weight {value} of {item} is negative or not finite")]
    BadWeight {
        /// The item the weight belongs to.
        item: String,
        /// The weight.
        value: f64,
    },
    /// A position has a coordinate that is not finite.
    #[error("position of {item} is not finite")]
    NonFinitePosition {
        /// The item the position belongs to.
        item: String,
    },
    /// A parameter is out of its range.
    #[error("parameter {name} = {value} is out of range")]
    BadParameter {
        /// The parameter's name.
        name: &'static str,
        /// The value given.
        value: f64,
    },
    /// A row has the wrong count of numbers.
    #[error("{item} has {found} numbers, expected {expected}")]
    CountMismatch {
        /// The item whose row is wrong.
        item: String,
        /// The count expected.
        expected: usize,
        /// The count found.
        found: usize,
    },
    /// A hyperedge is not in the hypergraph.
    #[error("edge {0} is not in the hypergraph")]
    UnknownEdge(String),
    /// A member has a positive weight but no position.
    #[error("member {node} of edge {edge} has a weight but no position")]
    NoPosition {
        /// The edge.
        edge: String,
        /// The member without a position.
        node: String,
    },
    /// A grid would hold more nodes than the limit allows.
    #[error("a grid of {nodes} nodes exceeds the limit of {limit}")]
    GridTooLarge {
        /// The count of grid nodes asked for.
        nodes: usize,
        /// The limit.
        limit: usize,
    },
}
