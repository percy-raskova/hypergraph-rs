//! The core `Hypergraph` data structure.

use rustworkx_core::petgraph::stable_graph::{NodeIndex, StableDiGraph};
use indexmap::IndexMap;

use super::error::{EdgeError, NodeError};
use super::kinds::{MembershipEdge, NodeKind};

/// A hypergraph, represented as a bipartite graph.
///
/// The bipartite graph has two node kinds: `Agent` (the hypergraph's nodes)
/// and `Hyperedge` (the hypergraph's edges). Membership edges connect agents
/// to their hyperedges. This representation makes the hypergraph a genuine
/// rustworkx-core plugin — all petgraph/rustworkx-core algorithms work on
/// the bipartite graph directly.
///
/// # Type Parameters
///
/// - `N` — agent node attribute type (defaults to `serde_json::Value`)
/// - `E` — hyperedge attribute type (defaults to `serde_json::Value`)
/// - `M` — per-membership attribute type (defaults to `serde_json::Value`)
pub struct Hypergraph<N = serde_json::Value, E = serde_json::Value, M = serde_json::Value> {
    /// The bipartite graph: Agent nodes + Hyperedge nodes + membership edges.
    /// StableDiGraph preserves insertion order under removal (leaves holes,
    /// doesn't compact) — required for III.7 determinism parity.
    inner: StableDiGraph<NodeKind<N, E>, MembershipEdge<M>>,

    /// Insertion-ordered bimap: agent_id -> NodeIndex in `inner`.
    agent_ids: IndexMap<String, NodeIndex>,

    /// Insertion-ordered bimap: edge_id -> NodeIndex in `inner`.
    hyperedge_ids: IndexMap<String, NodeIndex>,

    /// Auto-incrementing counter for edges added without an explicit `idx`.
    edge_uid_counter: u64,

    /// Graph-level attributes (XGI's `H.graph` dict).
    graph_attrs: serde_json::Map<String, serde_json::Value>,
}

impl<N, E, M> Default for Hypergraph<N, E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> Hypergraph<N, E, M> {
    /// Create an empty hypergraph.
    pub fn new() -> Self {
        Self {
            inner: StableDiGraph::new(),
            agent_ids: IndexMap::new(),
            hyperedge_ids: IndexMap::new(),
            edge_uid_counter: 0,
            graph_attrs: serde_json::Map::new(),
        }
    }

    /// The number of agent nodes in the hypergraph.
    pub fn num_nodes(&self) -> usize {
        self.agent_ids.len()
    }

    /// The number of hyperedges in the hypergraph.
    pub fn num_edges(&self) -> usize {
        self.hyperedge_ids.len()
    }

    /// Add a node with attributes. Returns `true` if a new node was created,
    /// `false` if it already existed.
    ///
    /// XGI parity: `H.add_node(node, **attr)`.
    pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool {
        if self.agent_ids.contains_key(node_id) {
            return false;
        }
        let idx = self.inner.add_node(NodeKind::Agent(attrs));
        self.agent_ids.insert(node_id.to_string(), idx);
        true
    }

    /// Check if a node exists in the hypergraph.
    ///
    /// XGI parity: `n in H` / `H.has_node(n)`.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.agent_ids.contains_key(node_id)
    }

    /// Add a hyperedge connecting the given members.
    ///
    /// XGI parity: `H.add_edge(members, idx=id, **attr)`.
    pub fn add_edge(
        &mut self,
        members: Vec<String>,
        idx: Option<String>,
        attrs: E,
    ) -> Result<String, EdgeError>
    where
        N: Default,
        M: Default + Clone,
    {
        if members.is_empty() {
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

        if let Some(id) = &idx {
            if let Ok(n) = id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        } else {
            self.edge_uid_counter += 1;
        }

        let mut seen = std::collections::HashSet::new();
        let unique_members: Vec<String> = members
            .into_iter()
            .filter(|m| seen.insert(m.clone()))
            .collect();

        for member in &unique_members {
            if !self.agent_ids.contains_key(member) {
                let nidx = self.inner.add_node(NodeKind::Agent(N::default()));
                self.agent_ids.insert(member.clone(), nidx);
            }
        }

        let he_idx = self.inner.add_node(NodeKind::Hyperedge(attrs));
        self.hyperedge_ids.insert(edge_id.clone(), he_idx);

        for member in &unique_members {
            let agent_idx = self.agent_ids[member];
            let membership = MembershipEdge { member_data: M::default() };
            self.inner.add_edge(agent_idx, he_idx, membership.clone());
            self.inner.add_edge(he_idx, agent_idx, membership);
        }

        Ok(edge_id)
    }

    /// Check if a hyperedge exists.
    /// XGI parity: `id in H.edges`.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.hyperedge_ids.contains_key(edge_id)
    }

    /// Get the edge IDs of which a node is a member.
    /// XGI parity: `H.nodes.memberships(n)`.
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        let agent_idx = *self.agent_ids.get(node_id)?;
        let mut result: Vec<String> = Vec::new();
        for neighbor_idx in self.inner.neighbors(agent_idx) {
            if let Some(NodeKind::Hyperedge(_)) = self.inner.node_weight(neighbor_idx) {
                for (eid, &idx) in &self.hyperedge_ids {
                    if idx == neighbor_idx {
                        result.push(eid.clone());
                        break;
                    }
                }
            }
        }
        Some(result)
    }

    /// Get the node IDs that are members of an edge.
    /// XGI parity: `H.edges.members(e)`.
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        let he_idx = *self.hyperedge_ids.get(edge_id)?;
        let mut result: Vec<String> = Vec::new();
        for neighbor_idx in self.inner.neighbors(he_idx) {
            if let Some(NodeKind::Agent(_)) = self.inner.node_weight(neighbor_idx) {
                for (nid, &idx) in &self.agent_ids {
                    if idx == neighbor_idx {
                        result.push(nid.clone());
                        break;
                    }
                }
            }
        }
        Some(result)
    }

    /// All node IDs in insertion order (III.7 determinism parity).
    /// XGI parity: `list(H.nodes)`.
    pub fn node_ids(&self) -> Vec<String> {
        self.agent_ids.keys().cloned().collect()
    }

    /// All edge IDs in insertion order (III.7 determinism parity).
    /// XGI parity: `list(H.edges)`.
    pub fn edge_ids(&self) -> Vec<String> {
        self.hyperedge_ids.keys().cloned().collect()
    }

    /// Remove a node from the hypergraph.
    /// XGI parity: `H.remove_node(n, strong=False, remove_empty=True)`.
    pub fn remove_node(&mut self, node_id: &str, strong: bool) -> Result<(), NodeError> {
        let agent_idx = *self.agent_ids.get(node_id).ok_or(NodeError::NotFound {
            node_id: node_id.to_string(),
        })?;

        let edge_ids: Vec<String> = self.memberships(node_id).unwrap_or_default();

        if strong {
            for eid in &edge_ids {
                let he_idx = self.hyperedge_ids[eid];
                self.inner.remove_node(he_idx);
                self.hyperedge_ids.shift_remove(eid);
            }
        } else {
            for eid in &edge_ids {
                let he_idx = self.hyperedge_ids[eid];
                if let Some(e) = self.inner.find_edge(agent_idx, he_idx) {
                    self.inner.remove_edge(e);
                }
                if let Some(e) = self.inner.find_edge(he_idx, agent_idx) {
                    self.inner.remove_edge(e);
                }
                let has_members = self.inner.neighbors(he_idx).any(|n| {
                    matches!(self.inner.node_weight(n), Some(NodeKind::Agent(_)))
                });
                if !has_members {
                    self.inner.remove_node(he_idx);
                    self.hyperedge_ids.shift_remove(eid);
                }
            }
        }

        self.inner.remove_node(agent_idx);
        self.agent_ids.shift_remove(node_id);
        Ok(())
    }

    /// Remove a hyperedge from the hypergraph.
    /// XGI parity: `H.remove_edge(e)`.
    pub fn remove_edge(&mut self, edge_id: &str) -> Result<(), EdgeError> {
        let he_idx = *self.hyperedge_ids.get(edge_id).ok_or(EdgeError::NotFound {
            edge_id: edge_id.to_string(),
        })?;
        self.inner.remove_node(he_idx);
        self.hyperedge_ids.shift_remove(edge_id);
        Ok(())
    }

    /// Read a node's attributes. XGI parity: `H.nodes[n]`.
    pub fn node_attrs(&self, node_id: &str) -> Option<&N> {
        let idx = self.agent_ids.get(node_id)?;
        match self.inner.node_weight(*idx)? {
            NodeKind::Agent(attrs) => Some(attrs),
            _ => None,
        }
    }

    /// Mutably access a node's attributes.
    pub fn node_attrs_mut(&mut self, node_id: &str) -> Option<&mut N> {
        let idx = *self.agent_ids.get(node_id)?;
        match self.inner.node_weight_mut(idx)? {
            NodeKind::Agent(attrs) => Some(attrs),
            _ => None,
        }
    }

    /// Read a hyperedge's attributes. XGI parity: `H.edges[e]`.
    pub fn edge_attrs(&self, edge_id: &str) -> Option<&E> {
        let idx = self.hyperedge_ids.get(edge_id)?;
        match self.inner.node_weight(*idx)? {
            NodeKind::Hyperedge(attrs) => Some(attrs),
            _ => None,
        }
    }

    /// Mutably access a hyperedge's attributes.
    pub fn edge_attrs_mut(&mut self, edge_id: &str) -> Option<&mut E> {
        let idx = *self.hyperedge_ids.get(edge_id)?;
        match self.inner.node_weight_mut(idx)? {
            NodeKind::Hyperedge(attrs) => Some(attrs),
            _ => None,
        }
    }

    /// Read a graph-level attribute. XGI parity: `H["name"]`.
    pub fn graph_attr(&self, key: &str) -> Option<&serde_json::Value> {
        self.graph_attrs.get(key)
    }

    /// Set a graph-level attribute. XGI parity: `H["name"] = value`.
    pub fn set_graph_attr(&mut self, key: &str, value: serde_json::Value) {
        self.graph_attrs.insert(key.to_string(), value);
    }

    /// Remove all nodes and edges. XGI parity: `H.clear()`.
    ///
    /// The edge-uid counter is deliberately NOT reset — XGI's `clear()`
    /// never touches `_edge_uid` (verified 0.10.2: auto-ids continue the
    /// pre-clear sequence).
    pub fn clear(&mut self) {
        self.inner = StableDiGraph::new();
        self.agent_ids.clear();
        self.hyperedge_ids.clear();
        self.graph_attrs.clear();
    }

    /// Return an independent deep copy. XGI parity: `H.copy()`.
    pub fn copy(&self) -> Self
    where
        N: Clone,
        E: Clone,
        M: Clone,
    {
        Self {
            inner: self.inner.clone(),
            agent_ids: self.agent_ids.clone(),
            hyperedge_ids: self.hyperedge_ids.clone(),
            edge_uid_counter: self.edge_uid_counter,
            graph_attrs: self.graph_attrs.clone(),
        }
    }

    /// Add multiple nodes. XGI parity: `H.add_nodes_from(nodes_for_adding)`.
    pub fn add_nodes_from(&mut self, nodes: impl IntoIterator<Item = (String, N)>) {
        for (node_id, attrs) in nodes {
            self.add_node(&node_id, attrs);
        }
    }

    /// Add multiple edges. Returns a result per edge.
    pub fn add_edges_from(
        &mut self,
        edges: impl IntoIterator<Item = (Vec<String>, Option<String>, E)>,
    ) -> Vec<Result<String, EdgeError>>
    where
        N: Default,
        M: Default + Clone,
    {
        edges.into_iter().map(|(m, i, a)| self.add_edge(m, i, a)).collect()
    }
}
