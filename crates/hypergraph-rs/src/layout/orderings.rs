//! Ordering providers for babylon's topology render lane (design canon S9):
//! surfaces consume orderings, never force layouts. `paoh_ordering` mirrors
//! `src/babylon/projection/topology/paoh.py`; `levi_ego_tree` mirrors
//! `levi.py`, modeled on this crate's own bipartite (agent/hyperedge)
//! representation. Both are re-expressions of existing structure — no new
//! mathematics.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::hypergraph::Hypergraph;

/// The kind of node represented by a [`LeviNode`] in the Levi (bipartite)
/// ego-tree walk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LeviKind {
    Agent,
    Hyperedge,
}

/// A node in the Levi ego-tree: an agent or a hyperedge, with children in
/// membership insertion order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeviNode {
    pub id: String,
    pub kind: LeviKind,
    pub children: Vec<LeviNode>,
}

/// Build a breadth-limited bipartite ego-tree rooted at `root` (an agent id).
/// `depth` counts hyperedge expansions: at `depth == 0` the root is returned
/// with no children. Each agent/hyperedge is visited at most once (a
/// `HashSet<String>` guard per kind — Power-of-10 rule 2: the walk is bounded
/// by the hypergraph's finite node/edge count, so it always terminates).
/// Returns `None` if `root` does not exist.
pub fn levi_ego_tree<N, E, M>(
    h: &Hypergraph<N, E, M>,
    root: &str,
    depth: usize,
) -> Option<LeviNode> {
    if !h.node_ids().iter().any(|n| n == root) {
        return None;
    }
    let mut visited_agents: HashSet<String> = HashSet::new();
    let mut visited_edges: HashSet<String> = HashSet::new();
    visited_agents.insert(root.to_string());
    Some(build_agent_node(
        h,
        root,
        depth,
        &mut visited_agents,
        &mut visited_edges,
    ))
}

fn build_agent_node<N, E, M>(
    h: &Hypergraph<N, E, M>,
    agent_id: &str,
    remaining_depth: usize,
    visited_agents: &mut HashSet<String>,
    visited_edges: &mut HashSet<String>,
) -> LeviNode {
    let mut children = Vec::new();
    if remaining_depth > 0 {
        let memberships = h.memberships(agent_id).unwrap_or_default();
        for edge_id in memberships {
            if visited_edges.contains(&edge_id) {
                continue;
            }
            visited_edges.insert(edge_id.clone());
            children.push(build_edge_node(
                h,
                &edge_id,
                remaining_depth,
                visited_agents,
                visited_edges,
            ));
        }
    }
    LeviNode {
        id: agent_id.to_string(),
        kind: LeviKind::Agent,
        children,
    }
}

fn build_edge_node<N, E, M>(
    h: &Hypergraph<N, E, M>,
    edge_id: &str,
    remaining_depth: usize,
    visited_agents: &mut HashSet<String>,
    visited_edges: &mut HashSet<String>,
) -> LeviNode {
    let mut children = Vec::new();
    let next_depth = remaining_depth - 1;
    let members = h.members(edge_id).unwrap_or_default();
    for member_id in members {
        if visited_agents.contains(&member_id) {
            continue;
        }
        visited_agents.insert(member_id.clone());
        children.push(build_agent_node(
            h,
            &member_id,
            next_depth,
            visited_agents,
            visited_edges,
        ));
    }
    LeviNode {
        id: edge_id.to_string(),
        kind: LeviKind::Hyperedge,
        children,
    }
}

/// PAOH ordering (parallel aggregated ordered hypergraph): nodes in insertion
/// order; edges in insertion order unless `sort_attr` names a numeric edge
/// attribute, in which case edges are stable-sorted ascending by that attr's
/// `as_f64()` value, with missing/non-numeric attrs sorted LAST (ties by
/// insertion order) — matches babylon's `paoh.py` None-last rule.
pub fn paoh_ordering(
    h: &Hypergraph<serde_json::Value, serde_json::Value, serde_json::Value>,
    sort_attr: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let nodes = h.node_ids();
    let mut edges = h.edge_ids();
    if let Some(attr) = sort_attr {
        let key = |edge_id: &String| -> Option<f64> {
            h.edge_attrs(edge_id)
                .and_then(|v| v.get(attr))
                .and_then(|v| v.as_f64())
        };
        edges.sort_by(|a, b| match (key(a), key(b)) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }
    (nodes, edges)
}
