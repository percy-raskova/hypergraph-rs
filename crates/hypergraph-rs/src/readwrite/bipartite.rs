//! Bipartite edgelist read/write (XGI `xgi/readwrite/bipartite.py` port).

use std::path::Path;

use super::ReadWriteError;
use crate::Hypergraph;

fn split_line(line: &str, delimiter: Option<char>) -> Vec<String> {
    match delimiter {
        Some(d) => line
            .split(d)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => line.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

fn strip_comment(line: &str, comments: &str) -> String {
    match line.find(comments) {
        Some(p) => line[..p].to_string(),
        None => line.to_string(),
    }
}

/// Parse a bipartite edgelist. Each line has `>= 2` tokens; the node column is 0
/// and the edge column is 1 (swapped when `dual`). XGI `parse_bipartite_edgelist`.
pub fn parse_bipartite_edgelist(
    lines: impl IntoIterator<Item = String>,
    delimiter: Option<char>,
    comments: &str,
    dual: bool,
) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();
    let (node_col, edge_col) = if dual { (1, 0) } else { (0, 1) };
    for raw in lines {
        let line = strip_comment(&raw, comments);
        let toks = split_line(&line, delimiter);
        if toks.is_empty() {
            continue;
        }
        if toks.len() < 2 {
            return Err(ReadWriteError::Format(
                "each line must contain at least two entries".into(),
            ));
        }
        let node = &toks[node_col];
        let edge = &toks[edge_col];
        h.add_node_to_edge(edge, node)
            .map_err(|e| ReadWriteError::Format(format!("bad incidence: {e}")))?;
    }
    Ok(h)
}

/// Read a bipartite edgelist file. XGI `read_bipartite_edgelist`.
pub fn read_bipartite_edgelist(
    path: &Path,
    delimiter: Option<char>,
    comments: &str,
    dual: bool,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    parse_bipartite_edgelist(s.lines().map(|l| l.to_string()), delimiter, comments, dual)
}

/// Write a bipartite edgelist (`node<delim>edge` per membership). XGI `write_bipartite_edgelist`.
pub fn write_bipartite_edgelist(
    h: &Hypergraph,
    path: &Path,
    delimiter: char,
) -> Result<(), ReadWriteError> {
    let mut out = String::new();
    for eid in h.edge_ids() {
        for node in h.members(&eid).unwrap_or_default() {
            out.push_str(&node);
            out.push(delimiter);
            out.push_str(&eid);
            out.push('\n');
        }
    }
    std::fs::write(path, out)?;
    Ok(())
}
