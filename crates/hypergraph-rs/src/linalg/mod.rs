//! Matrices derived from the bipartite hypergraph substrate.
//!
//! Row order follows [`Hypergraph::node_ids`] (node insertion order); column
//! order follows [`Hypergraph::edge_ids`] (edge insertion order). This is the
//! III.7 determinism contract and matches XGI's dict-ordering
//! (`xgi/linalg/hypergraph_matrix.py`).

mod hypergraph_matrix;

pub use hypergraph_matrix::{incidence_matrix, incidence_matrix_sparse};

use crate::Hypergraph;
use thiserror::Error;

/// Errors raised by linear-algebra operators that have preconditions.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum LinalgError {
    /// The normalized Laplacian is undefined when isolated nodes exist
    /// (division by a zero degree). XGI parity: `laplacian_matrix.py:222-225`.
    #[error("every node must be a member of an edge to avoid divide-by-zero")]
    IsolatedNodes,
}

/// Edge IDs kept after an optional order filter, in edge insertion order.
///
/// `order = Some(d)` keeps edges whose membership count equals `d + 1`
/// (edge order = size − 1). `order = None` keeps all edges.
pub(crate) fn filtered_edge_ids<N, E, M>(
    h: &Hypergraph<N, E, M>,
    order: Option<usize>,
) -> Vec<String> {
    match order {
        None => h.edge_ids(),
        Some(d) => h
            .edge_ids()
            .into_iter()
            .filter(|e| h.members(e).map(|m| m.len()) == Some(d + 1))
            .collect(),
    }
}
