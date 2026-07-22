//! Read/write hypergraphs in various formats (XGI `xgi/readwrite/*.py` port).

pub mod bigg;
pub mod bipartite;
pub mod edgelist;
pub mod hif;
pub mod incidence;
pub mod json;

use thiserror::Error;

/// Error raised by a read/write operation.
#[derive(Debug, Error)]
pub enum ReadWriteError {
    /// The underlying file could not be read or written.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The bytes were not valid JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The data was structurally invalid for the target format.
    #[error("format error: {0}")]
    Format(String),
}

pub use json::{from_hypergraph_dict, read_json, to_hypergraph_dict, write_json};
pub use edgelist::{parse_edgelist, read_edgelist, write_edgelist};
pub use bipartite::{parse_bipartite_edgelist, read_bipartite_edgelist, write_bipartite_edgelist};
pub use incidence::{read_incidence_matrix, write_incidence_matrix};
pub use hif::{from_hif_dict, read_hif, to_hif_dict, write_hif};
pub use bigg::bigg_dict_to_dihypergraph;
