//! The directed hypergraph data structure.
//!
//! Like [`Hypergraph`](super::hypergraph::Hypergraph), a `DiHypergraph` IS a
//! bipartite `StableDiGraph`, but every directed membership edge is oriented
//! Agent -> Hyperedge and carries a [`DiRole`] flag distinguishing the tail
//! (senders) from the head (receivers). XGI parity: `xgi.DiHypergraph`
//! (dihypergraph.py). Note XGI's `DiHypergraph` is NOT a subclass of
//! `Hypergraph`, and neither is this.

use indexmap::IndexMap;
use rustworkx_core::petgraph::stable_graph::{NodeIndex, StableDiGraph};

use super::error::EdgeError;
use super::kinds::{DiMembershipEdge, DiRole, NodeKind};

/// Deduplicate a Vec of ids, preserving first-seen order.
fn dedup_preserve(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

/// A directed hypergraph. XGI parity: `xgi.DiHypergraph`.
///
/// # Type Parameters
/// - `N` — node attribute type (default `serde_json::Value`)
/// - `E` — hyperedge attribute type (default `serde_json::Value`)
/// - `M` — per-membership attribute type (default `serde_json::Value`)
pub struct DiHypergraph<N = serde_json::Value, E = serde_json::Value, M = serde_json::Value> {
    /// Bipartite substrate. All membership edges are Agent -> Hyperedge with a
    /// DiRole flag. StableDiGraph preserves insertion order under removal.
    inner: StableDiGraph<NodeKind<N, E>, DiMembershipEdge<M>>,
    agent_ids: IndexMap<String, NodeIndex>,
    hyperedge_ids: IndexMap<String, NodeIndex>,
    edge_uid_counter: u64,
    graph_attrs: serde_json::Map<String, serde_json::Value>,
    frozen: bool,
}

impl<N, E, M> Default for DiHypergraph<N, E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> DiHypergraph<N, E, M> {
    /// Create an empty directed hypergraph.
    pub fn new() -> Self {
        Self {
            inner: StableDiGraph::new(),
            agent_ids: IndexMap::new(),
            hyperedge_ids: IndexMap::new(),
            edge_uid_counter: 0,
            graph_attrs: serde_json::Map::new(),
            frozen: false,
        }
    }

    /// Number of nodes. XGI parity: `DH.num_nodes`.
    pub fn num_nodes(&self) -> usize {
        self.agent_ids.len()
    }

    /// Number of edges. XGI parity: `DH.num_edges`.
    pub fn num_edges(&self) -> usize {
        self.hyperedge_ids.len()
    }

    /// Whether a node exists. XGI parity: `n in DH`.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.agent_ids.contains_key(node_id)
    }

    /// Whether an edge exists. XGI parity: `e in DH.edges`.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.hyperedge_ids.contains_key(edge_id)
    }

    fn assert_not_frozen(&self) {
        if self.frozen {
            panic!("Frozen higher-order network can't be modified");
        }
    }

    /// Ensure an agent node exists, creating it (with `N::default()`) if absent.
    fn ensure_agent(&mut self, node_id: &str)
    where
        N: Default,
    {
        if !self.agent_ids.contains_key(node_id) {
            let idx = self.inner.add_node(NodeKind::Agent(N::default()));
            self.agent_ids.insert(node_id.to_string(), idx);
        }
    }

    /// Add a node with attributes. Returns `true` if newly created.
    /// XGI parity: `DH.add_node(node, **attr)` (dihypergraph.py:327-349).
    pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool {
        self.assert_not_frozen();
        if self.agent_ids.contains_key(node_id) {
            return false;
        }
        let idx = self.inner.add_node(NodeKind::Agent(attrs));
        self.agent_ids.insert(node_id.to_string(), idx);
        true
    }

    /// Add a directed hyperedge with the given tail (senders) and head
    /// (receivers). Auto-creates member nodes. XGI parity:
    /// `DH.add_edge((tail, head), idx=)` (dihypergraph.py:525-585).
    ///
    /// Deviations: returns `Err(EdgeError::EmptyMembers)` only when BOTH tail
    /// and head are empty (deviation #1); `Err(EdgeError::AlreadyExists)` on a
    /// duplicate `idx` (deviation #2).
    pub fn add_edge(
        &mut self,
        tail: Vec<String>,
        head: Vec<String>,
        idx: Option<String>,
        attrs: E,
    ) -> Result<String, EdgeError>
    where
        N: Default,
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if tail.is_empty() && head.is_empty() {
            return Err(EdgeError::EmptyMembers);
        }

        let edge_id = match &idx {
            Some(id) => {
                if self.hyperedge_ids.contains_key(id) {
                    return Err(EdgeError::AlreadyExists { edge_id: id.clone() });
                }
                id.clone()
            }
            None => self.edge_uid_counter.to_string(),
        };

        // Advance the uid counter (fixing XGI's falsy-id bug, as in Phase 1).
        if let Some(id) = &idx {
            if let Ok(n) = id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        } else {
            self.edge_uid_counter += 1;
        }

        let he = self.inner.add_node(NodeKind::Hyperedge(attrs));
        self.hyperedge_ids.insert(edge_id.clone(), he);

        for node in &dedup_preserve(tail) {
            self.ensure_agent(node);
            let a = self.agent_ids[node];
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge { member_data: M::default(), role: DiRole::Tail },
            );
        }
        for node in &dedup_preserve(head) {
            self.ensure_agent(node);
            let a = self.agent_ids[node];
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge { member_data: M::default(), role: DiRole::Head },
            );
        }

        Ok(edge_id)
    }
}
