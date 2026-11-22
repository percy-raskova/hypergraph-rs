//! Error types for hypergraph operations.

use thiserror::Error;

/// Error raised when a node operation fails.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum NodeError {
    /// The node ID was not found in the hypergraph.
    #[error("node {node_id} does not exist")]
    NotFound { node_id: String },
    /// The named edge (not node) was not found — returned by methods that
    /// take both an edge id and a node id (e.g. `remove_node_from_edge`).
    #[error("edge {edge_id} not found")]
    EdgeNotFound { edge_id: String },
}

/// Error raised when a membership operation fails
/// (`Hypergraph::remove_node_from_edge` / `DiHypergraph::remove_node_from_edge`).
///
/// XGI raises `XGIError` for all three branches, each with a distinct
/// message (probed v0.10.2); the core returns a dedicated variant per
/// branch so callers discriminate without string-matching. The PyO3
/// binding translates `Err` -> raise, reproducing XGI's exact messages
/// (D2 error-channel class).
#[derive(Debug, Clone, Error, PartialEq)]
pub enum MembershipError {
    /// The edge ID was not found in the hypergraph.
    #[error("edge {edge_id} does not exist")]
    EdgeNotFound { edge_id: String },
    /// The node ID was not found in the hypergraph.
    #[error("node {node_id} does not exist")]
    NodeNotFound { node_id: String },
    /// Both exist, but the node is not a member of the edge.
    #[error("node {node_id} is not a member of edge {edge_id}")]
    NotAMember { node_id: String, edge_id: String },
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

/// Error raised by `SimplicialComplex` operations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SimplexError {
    /// A `Hypergraph` method that is not available on a `SimplicialComplex`.
    #[error("{method} is not available for SimplicialComplex; use {alternative}")]
    NotAvailable { method: String, alternative: String },
    /// The simplex id was not found.
    #[error("simplex {idx} is not in the simplicial complex")]
    NotFound { idx: String },
    /// A complex rebuilt from a list of simplices lacks a face of this one.
    #[error("simplex {idx} has a face that is not in the list")]
    NotClosed { idx: String },
}
