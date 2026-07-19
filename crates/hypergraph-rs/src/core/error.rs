//! Error types for hypergraph operations.

use thiserror::Error;

/// Error raised when a node operation fails.
#[derive(Debug, Clone, Error)]
pub enum NodeError {
    /// The node ID was not found in the hypergraph.
    #[error("node {node_id} does not exist")]
    NotFound { node_id: String },
}

/// Error raised when an edge operation fails.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum EdgeError {
    /// The edge ID was not found in the hypergraph.
    #[error("edge {edge_id} does not exist")]
    NotFound { edge_id: String },
    /// The edge ID already exists (duplicate on add_edge).
    #[error("edge {edge_id} already exists")]
    AlreadyExists { edge_id: String },
    /// The members collection was empty.
    #[error("cannot add an empty edge")]
    EmptyMembers,
    /// `random_edge_shuffle` needs at least two edges to swap between.
    #[error("random_edge_shuffle requires at least 2 edges")]
    TooFewEdges,
}

/// Error raised by `double_edge_swap`.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SwapError {
    /// A specified node or edge was not found, or the node is not a member of
    /// the named edge.
    #[error("id {id} not found (or node not a member of the named edge)")]
    IdNotFound { id: String },
    /// The swap would not preserve edge sizes (the incoming node is already a
    /// member of the target edge).
    #[error("swap does not preserve edge sizes")]
    SizeNotPreserved,
}
