//! Incidence-matrix read/write (XGI `xgi/readwrite/incidence.py` port).
//!
//! Deviation from XGI: we emit integer 0/1 cells (not numpy `%.18e`) and read any
//! non-zero cell as membership. Our writer + reader round-trip together; node/edge
//! ids are positional ("0".."k-1") on read, matching `from_incidence_matrix`.

use std::path::Path;

use serde_json::Value;

use super::ReadWriteError;
use crate::Hypergraph;

/// Write the dense incidence matrix: rows = nodes (node-id order), cols = edges
/// (edge-id order), cell = 1 if the node is a member. XGI `write_incidence_matrix`.
pub fn write_incidence_matrix(
    h: &Hypergraph,
    path: &Path,
    delimiter: char,
) -> Result<(), ReadWriteError> {
    let nodes = h.node_ids();
    let edges = h.edge_ids();
    // membership lookup: for each edge, the set of member node ids
    let edge_members: Vec<std::collections::HashSet<String>> = edges
        .iter()
        .map(|e| h.members(e).unwrap_or_default().into_iter().collect())
        .collect();

    let mut out = String::new();
    for node in &nodes {
        let row: Vec<&str> = edge_members
            .iter()
            .map(|set| if set.contains(node) { "1" } else { "0" })
            .collect();
        out.push_str(&row.join(&delimiter.to_string()));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

/// Read a dense incidence matrix into a hypergraph with positional ids.
/// XGI `read_incidence_matrix` (via `from_incidence_matrix`).
pub fn read_incidence_matrix(
    path: &Path,
    delimiter: Option<char>,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    let rows: Vec<Vec<f64>> = s
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let cells: Vec<&str> = match delimiter {
                Some(d) => line
                    .split(d)
                    .map(|c| c.trim())
                    .filter(|c| !c.is_empty())
                    .collect(),
                None => line.split_whitespace().collect(),
            };
            cells
                .iter()
                .map(|c| c.parse::<f64>().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let n_nodes = rows.len();
    let n_edges = rows.first().map(|r| r.len()).unwrap_or(0);

    let mut h = Hypergraph::new();
    h.add_nodes_from((0..n_nodes).map(|i| (i.to_string(), Value::Null)));

    // Column-major: for each edge (col), collect member nodes (rows with nonzero).
    for e in 0..n_edges {
        let members: Vec<String> = (0..n_nodes)
            .filter(|&r| rows[r].get(e).copied().unwrap_or(0.0) != 0.0)
            .map(|r| r.to_string())
            .collect();
        if members.is_empty() {
            continue; // our core rejects empty edges; skip all-zero columns
        }
        h.add_edge(members, Some(e.to_string()), Value::Null)
            .map_err(|err| ReadWriteError::Format(format!("column {e}: {err}")))?;
    }
    Ok(h)
}
