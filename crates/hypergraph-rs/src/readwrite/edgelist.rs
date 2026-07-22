//! Edgelist read/write (XGI `xgi/readwrite/edgelist.py` port).

use std::path::Path;

use serde_json::Value;

use super::ReadWriteError;
use crate::Hypergraph;

/// Split a line on `delimiter` (whitespace when `None`), dropping empties.
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

/// Strip the comment suffix (everything at/after the first `comments` token).
fn strip_comment(line: &str, comments: &str) -> String {
    match line.find(comments) {
        Some(p) => line[..p].to_string(),
        None => line.to_string(),
    }
}

/// Parse an iterable of lines into a hypergraph (one edge per line). XGI `parse_edgelist`.
pub fn parse_edgelist(
    lines: impl IntoIterator<Item = String>,
    delimiter: Option<char>,
    comments: &str,
) -> Result<Hypergraph, ReadWriteError> {
    let mut h = Hypergraph::new();
    for raw in lines {
        let line = strip_comment(&raw, comments);
        let members = split_line(&line, delimiter);
        if members.is_empty() {
            continue;
        }
        h.add_edge(members, None, Value::Null)
            .map_err(|e| ReadWriteError::Format(format!("bad edge line: {e}")))?;
    }
    Ok(h)
}

/// Read a hypergraph from an edgelist file. XGI `read_edgelist`.
pub fn read_edgelist(
    path: &Path,
    delimiter: Option<char>,
    comments: &str,
) -> Result<Hypergraph, ReadWriteError> {
    let s = std::fs::read_to_string(path)?;
    parse_edgelist(s.lines().map(|l| l.to_string()), delimiter, comments)
}

/// Write a hypergraph as an edgelist file (members joined by `delimiter`). XGI `write_edgelist`.
pub fn write_edgelist(h: &Hypergraph, path: &Path, delimiter: char) -> Result<(), ReadWriteError> {
    let mut out = String::new();
    for eid in h.edge_ids() {
        let members = h.members(&eid).unwrap_or_default();
        out.push_str(&members.join(&delimiter.to_string()));
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}
