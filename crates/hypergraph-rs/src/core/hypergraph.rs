//! The core `Hypergraph` data structure.

use indexmap::IndexMap;
use rand::RngExt;
use rand_core::SeedableRng;
use rand_pcg::Pcg64Mcg;
use rustworkx_core::petgraph::graph::{NodeIndex as GraphNodeIndex, UnGraph};
use rustworkx_core::petgraph::stable_graph::{NodeIndex, StableDiGraph};
use rustworkx_core::petgraph::visit::NodeIndexable;
use serde_json::Value;

use super::error::{EdgeError, MembershipError, NodeError, SwapError};
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

    /// Whether this hypergraph is frozen (mutation methods panic when true).
    frozen: bool,
}

/// How to name a merged edge (subset of XGI's `rename`; `"tuple"` deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRename {
    /// Reuse the smallest (sorted) duplicate id.
    First,
    /// Assign a fresh auto-generated id.
    New,
}

/// How to combine attrs of merged edges (XGI's `merge_rule`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRule {
    /// Take the smallest-id duplicate's attrs.
    First,
    /// For each key, a JSON array of the distinct values (missing -> null).
    Union,
    /// For each key, the shared value or null if the duplicates disagree.
    Intersection,
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
            frozen: false,
        }
    }

    /// Panic if the hypergraph is frozen. Called at the top of every
    /// mutation method. XGI parity: frozen-method guard (hypergraph.py:1550).
    fn assert_not_frozen(&self) {
        if self.frozen {
            panic!("Frozen hypergraph can't be modified");
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
    /// XGI parity: `H.add_node(node, **attr)`. If `node_id` already exists,
    /// its stored attrs are REPLACED (XGI's binding merges the incoming
    /// dict into the existing one before calling; a generic `N` can't
    /// merge, so the core replaces and the PyO3 layer composes the merge
    /// on top — divergence D6).
    pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool {
        self.assert_not_frozen();
        if let Some(idx) = self.agent_ids.get(node_id) {
            if let Some(NodeKind::Agent(existing)) = self.inner.node_weight_mut(*idx) {
                *existing = attrs;
            }
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
        self.assert_not_frozen();

        let edge_id = match &idx {
            Some(id) => {
                if self.hyperedge_ids.contains_key(id) {
                    return Err(EdgeError::AlreadyExists {
                        edge_id: id.clone(),
                    });
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
            let membership = MembershipEdge {
                member_data: M::default(),
            };
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

    /// Get the edge IDs of which a node is a member, in insertion (membership-add)
    /// order. petgraph's `neighbors` walks a node's adjacency list in REVERSE
    /// insertion order, so we collect + reverse before mapping back to ids
    /// (same correction as DiHypergraph's D5 owner upgrade).
    /// XGI parity: `H.nodes.memberships(n)`.
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        let agent_idx = *self.agent_ids.get(node_id)?;
        let mut neighbor_idxs: Vec<_> = self.inner.neighbors(agent_idx).collect();
        neighbor_idxs.reverse();
        let mut result: Vec<String> = Vec::new();
        for neighbor_idx in neighbor_idxs {
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

    /// Get the node IDs that are members of an edge, in insertion order (see
    /// [`Self::memberships`] for the reverse-adjacency correction).
    /// XGI parity: `H.edges.members(e)`.
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        let he_idx = *self.hyperedge_ids.get(edge_id)?;
        let mut neighbor_idxs: Vec<_> = self.inner.neighbors(he_idx).collect();
        neighbor_idxs.reverse();
        let mut result: Vec<String> = Vec::new();
        for neighbor_idx in neighbor_idxs {
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

    /// Remove a node from the hypergraph. Weak removal (`strong=false`)
    /// detaches the node from its edges and, when `remove_empty` is true,
    /// drops edges that become empty as a result; strong removal drops all
    /// containing edges regardless of `remove_empty` (D9).
    /// XGI parity: `H.remove_node(n, strong=False, remove_empty=True)`.
    pub fn remove_node(
        &mut self,
        node_id: &str,
        strong: bool,
        remove_empty: bool,
    ) -> Result<(), NodeError> {
        self.assert_not_frozen();
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
                if remove_empty {
                    let has_members = self
                        .inner
                        .neighbors(he_idx)
                        .any(|n| matches!(self.inner.node_weight(n), Some(NodeKind::Agent(_))));
                    if !has_members {
                        self.inner.remove_node(he_idx);
                        self.hyperedge_ids.shift_remove(eid);
                    }
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
        self.assert_not_frozen();
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
        self.assert_not_frozen();
        self.inner = StableDiGraph::new();
        self.agent_ids.clear();
        self.hyperedge_ids.clear();
        self.graph_attrs.clear();
    }

    /// Remove all edges, keeping nodes.
    ///
    /// XGI parity: `H.clear_edges()` — the edge-uid counter is NOT reset
    /// (XGI's `clear_edges` never touches `_edge_uid`, verified 0.10.2).
    pub fn clear_edges(&mut self) {
        self.assert_not_frozen();
        let hyperedge_indices: Vec<NodeIndex> = self.hyperedge_ids.values().copied().collect();
        for he_idx in hyperedge_indices {
            self.inner.remove_node(he_idx);
        }
        self.hyperedge_ids.clear();
    }

    /// Freeze the hypergraph, preventing modification.
    /// XGI parity: `H.freeze()`.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Check if the hypergraph is frozen.
    /// XGI parity: `H.is_frozen`.
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Return an independent deep copy. XGI parity: `H.copy()`.
    /// D13: the copy carries `frozen`, unlike XGI's `copy()` — XGI's
    /// freeze is per-instance method-swizzling that a fresh instance
    /// never inherits, but the core's `frozen` is data, so the safety
    /// property must propagate.
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
            frozen: self.frozen,
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
        edges
            .into_iter()
            .map(|(m, i, a)| self.add_edge(m, i, a))
            .collect()
    }

    /// Add a node to an existing edge. Auto-creates both if missing.
    /// XGI parity: `H.add_node_to_edge(edge, node)`.
    pub fn add_node_to_edge(&mut self, edge_id: &str, node_id: &str) -> Result<(), EdgeError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        self.assert_not_frozen();
        // Auto-create edge if missing
        if !self.hyperedge_ids.contains_key(edge_id) {
            let he_idx = self.inner.add_node(NodeKind::Hyperedge(E::default()));
            self.hyperedge_ids.insert(edge_id.to_string(), he_idx);
            // Update uid counter if edge_id is numeric
            if let Ok(n) = edge_id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        }
        // Auto-create node if missing
        if !self.agent_ids.contains_key(node_id) {
            let nidx = self.inner.add_node(NodeKind::Agent(N::default()));
            self.agent_ids.insert(node_id.to_string(), nidx);
        }
        // Add the membership edges (both directions for undirected)
        let agent_idx = self.agent_ids[node_id];
        let he_idx = self.hyperedge_ids[edge_id];
        // Check if membership already exists
        if self.inner.find_edge(agent_idx, he_idx).is_none() {
            let membership = MembershipEdge {
                member_data: M::default(),
            };
            self.inner.add_edge(agent_idx, he_idx, membership.clone());
            self.inner.add_edge(he_idx, agent_idx, membership);
        }
        Ok(())
    }

    /// Remove a node from an existing edge.
    ///
    /// XGI parity: `H.remove_node_from_edge(edge, node, remove_empty=True)`.
    /// XGI raises `XGIError` for a missing edge, a missing node, or a node
    /// that exists but is not a member of the named edge — the core returns
    /// the matching [`MembershipError`] variant so callers discriminate
    /// without string-matching; the PyO3 binding translates `Err` -> raise,
    /// reproducing XGI's exact per-branch messages (D2 error channel).
    pub fn remove_node_from_edge(
        &mut self,
        edge_id: &str,
        node_id: &str,
        remove_empty: bool,
    ) -> Result<(), MembershipError> {
        self.assert_not_frozen();
        let he_idx = *self
            .hyperedge_ids
            .get(edge_id)
            .ok_or(MembershipError::EdgeNotFound {
                edge_id: edge_id.to_string(),
            })?;
        let agent_idx = *self
            .agent_ids
            .get(node_id)
            .ok_or(MembershipError::NodeNotFound {
                node_id: node_id.to_string(),
            })?;

        // Check if the node is actually in the edge
        let edge_to_he = self.inner.find_edge(agent_idx, he_idx);
        let edge_from_he = self.inner.find_edge(he_idx, agent_idx);
        if edge_to_he.is_none() {
            return Err(MembershipError::NotAMember {
                node_id: node_id.to_string(),
                edge_id: edge_id.to_string(),
            });
        }

        // Remove the bipartite membership edges
        if let Some(e) = edge_to_he {
            self.inner.remove_edge(e);
        }
        if let Some(e) = edge_from_he {
            self.inner.remove_edge(e);
        }

        // Check if edge is now empty
        if remove_empty {
            let has_members = self
                .inner
                .neighbors(he_idx)
                .any(|n| matches!(self.inner.node_weight(n), Some(NodeKind::Agent(_))));
            if !has_members {
                self.inner.remove_node(he_idx);
                self.hyperedge_ids.shift_remove(edge_id);
            }
        }
        Ok(())
    }

    /// Remove multiple edges, returning a result per ATTEMPTED edge.
    ///
    /// XGI parity: `H.remove_edges_from(ebunch)` — XGI iterates in order
    /// and RAISES on the first missing id, so ids after it are never
    /// attempted (partial effects: ids before the failure are already
    /// removed). The core mirrors this: stop on the first `Err`.
    pub fn remove_edges_from(&mut self, edge_ids: &[String]) -> Vec<Result<(), EdgeError>> {
        let mut results = Vec::new();
        for eid in edge_ids {
            let result = self.remove_edge(eid);
            let stop = result.is_err();
            results.push(result);
            if stop {
                break;
            }
        }
        results
    }

    /// Remove multiple nodes, returning a result per item.
    ///
    /// XGI parity: `H.remove_nodes_from(nodes)` — XGI warns and skips a
    /// missing id, then CONTINUES; the core records a per-item `Err` and
    /// continues (D2-class channel translation — the binding warns per
    /// `Err` item).
    pub fn remove_nodes_from(
        &mut self,
        node_ids: &[String],
        strong: bool,
        remove_empty: bool,
    ) -> Vec<Result<(), NodeError>> {
        node_ids
            .iter()
            .map(|nid| self.remove_node(nid, strong, remove_empty))
            .collect()
    }

    /// Add nodes then edges. XGI parity: `H.update(edges=, nodes=)`
    /// (hypergraph.py:1210-1229).
    pub fn update(&mut self, nodes: Vec<(String, N)>, edges: Vec<(Vec<String>, Option<String>, E)>)
    where
        N: Default,
        M: Default + Clone,
    {
        self.add_nodes_from(nodes);
        let _ = self.add_edges_from(edges);
    }

    /// Node ids with zero memberships, in insertion order.
    /// XGI parity: `H.nodes.isolates()` (views.py:632-667, default).
    pub fn isolate_node_ids(&self) -> Vec<String> {
        self.node_ids()
            .into_iter()
            .filter(|n| self.memberships(n).is_some_and(|m| m.is_empty()))
            .collect()
    }

    /// Edge ids with exactly one member, in insertion order.
    /// XGI parity: `H.edges.singletons()` (views.py:749-761).
    pub fn singleton_edge_ids(&self) -> Vec<String> {
        self.edge_ids()
            .into_iter()
            .filter(|e| self.members(e).is_some_and(|m| m.len() == 1))
            .collect()
    }

    /// Read-only view of the graph-level attribute dict (XGI's `H.graph`).
    pub fn graph_attrs(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.graph_attrs
    }

    /// Attach `node_id` to `edge_id` (both bipartite directions). No-op if the
    /// membership already exists. Both ids must already exist.
    fn attach_member(&mut self, edge_id: &str, node_id: &str)
    where
        M: Default + Clone,
    {
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        if self.inner.find_edge(a, he).is_none() {
            let m = MembershipEdge {
                member_data: M::default(),
            };
            self.inner.add_edge(a, he, m.clone());
            self.inner.add_edge(he, a, m);
        }
    }

    /// Detach `node_id` from `edge_id` (both bipartite directions). No-op if
    /// the membership is absent.
    fn detach_member(&mut self, edge_id: &str, node_id: &str) {
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        if let Some(e) = self.inner.find_edge(a, he) {
            self.inner.remove_edge(e);
        }
        if let Some(e) = self.inner.find_edge(he, a) {
            self.inner.remove_edge(e);
        }
    }

    /// Swap the edge memberships of two nodes between two edges.
    /// XGI parity: `H.double_edge_swap(n1, n2, e1, e2)` (hypergraph.py:918-992).
    pub fn double_edge_swap(
        &mut self,
        n_id1: &str,
        n_id2: &str,
        e_id1: &str,
        e_id2: &str,
    ) -> Result<(), SwapError>
    where
        M: Default + Clone,
    {
        self.assert_not_frozen();

        let members1 = self.members(e_id1).ok_or(SwapError::IdNotFound {
            id: e_id1.to_string(),
        })?;
        let members2 = self.members(e_id2).ok_or(SwapError::IdNotFound {
            id: e_id2.to_string(),
        })?;
        if !members1.iter().any(|m| m == n_id1) {
            return Err(SwapError::IdNotFound {
                id: n_id1.to_string(),
            });
        }
        if !members2.iter().any(|m| m == n_id2) {
            return Err(SwapError::IdNotFound {
                id: n_id2.to_string(),
            });
        }

        // Compute the post-swap member sets and verify size preservation BEFORE
        // mutating (XGI hypergraph.py:980-986).
        use std::collections::BTreeSet;
        let new1: BTreeSet<String> = members1
            .iter()
            .filter(|m| *m != n_id1)
            .cloned()
            .chain(std::iter::once(n_id2.to_string()))
            .collect();
        let new2: BTreeSet<String> = members2
            .iter()
            .filter(|m| *m != n_id2)
            .cloned()
            .chain(std::iter::once(n_id1.to_string()))
            .collect();
        if new1.len() != members1.len() || new2.len() != members2.len() {
            return Err(SwapError::SizeNotPreserved);
        }

        // Commit: e1 loses n1 gains n2; e2 loses n2 gains n1.
        self.detach_member(e_id1, n_id1);
        self.attach_member(e_id1, n_id2);
        self.detach_member(e_id2, n_id2);
        self.attach_member(e_id2, n_id1);
        Ok(())
    }

    /// Randomly redistribute nodes between two hyperedges, preserving both
    /// sizes. XGI parity: `H.random_edge_shuffle(e1, e2, seed)`
    /// (hypergraph.py:994-1078). Requires an explicit seed (deviation #3);
    /// the rand_pcg sequence does not match numpy, so only structural
    /// invariants are guaranteed across implementations.
    pub fn random_edge_shuffle(
        &mut self,
        e_id1: Option<&str>,
        e_id2: Option<&str>,
        seed: u64,
    ) -> Result<(), EdgeError>
    where
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if self.hyperedge_ids.len() < 2 {
            return Err(EdgeError::TooFewEdges);
        }
        let mut rng = Pcg64Mcg::seed_from_u64(seed);

        // Resolve the two edge ids (given, or two distinct random).
        let (id1, id2) = match (e_id1, e_id2) {
            (Some(a), Some(b)) => (a.to_string(), b.to_string()),
            _ => {
                let ids = self.edge_ids();
                let i = rng.random_range(0..ids.len());
                let mut j = rng.random_range(0..ids.len() - 1);
                if j >= i {
                    j += 1;
                }
                (ids[i].clone(), ids[j].clone())
            }
        };

        use std::collections::BTreeSet;
        let s1: BTreeSet<String> = self
            .members(&id1)
            .ok_or(EdgeError::NotFound {
                edge_id: id1.clone(),
            })?
            .into_iter()
            .collect();
        let s2: BTreeSet<String> = self
            .members(&id2)
            .ok_or(EdgeError::NotFound {
                edge_id: id2.clone(),
            })?
            .into_iter()
            .collect();

        let both: BTreeSet<String> = s1.intersection(&s2).cloned().collect();
        let only1: BTreeSet<String> = s1.difference(&both).cloned().collect();
        let only2: BTreeSet<String> = s2.difference(&both).cloned().collect();

        // Bucket in a deterministic base order (BTreeSet -> sorted).
        let mut bucket: Vec<String> = only1.union(&only2).cloned().collect();
        let k = only1.len();
        let n = bucket.len();
        // Partial Fisher-Yates: bounded by k (computed above), never exceeds n.
        for i in 0..k {
            let j = i + rng.random_range(0..(n - i));
            bucket.swap(i, j);
        }
        let new_only1: BTreeSet<String> = bucket[..k].iter().cloned().collect();
        let new_e1: BTreeSet<String> = new_only1.union(&both).cloned().collect();
        let new_only2: BTreeSet<String> = bucket[k..].iter().cloned().collect();
        let new_e2: BTreeSet<String> = new_only2.union(&both).cloned().collect();

        // Rewire: detach all old members, attach the new ones (ids/attrs kept).
        for node in &s1 {
            self.detach_member(&id1, node);
        }
        for node in &s2 {
            self.detach_member(&id2, node);
        }
        for node in &new_e1 {
            self.attach_member(&id1, node);
        }
        for node in &new_e2 {
            self.attach_member(&id2, node);
        }
        Ok(())
    }

    /// The undirected 1-skeleton (pairwise projection): a node for every agent
    /// (weight = its id, added in `node_ids()` order so `NodeIndex::new(i)`
    /// corresponds to `node_ids()[i]`), and an edge (weight `1.0`) between two
    /// agents that co-occur in at least one hyperedge.
    ///
    /// Undirected (deviation from spec §4.1's `Graph`): the connectivity,
    /// shortest-path, and clustering algorithms operate on this graph.
    pub fn skeleton(&self) -> UnGraph<String, f64> {
        let mut g = UnGraph::<String, f64>::new_undirected();
        let node_ids = self.node_ids();
        let mut idx_of: IndexMap<String, GraphNodeIndex> = IndexMap::new();
        for n in &node_ids {
            let ix = g.add_node(n.clone());
            idx_of.insert(n.clone(), ix);
        }
        let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for e in self.edge_ids() {
            let members = self.members(&e).unwrap_or_default();
            // Bounded: |members| choose 2.
            for a in 0..members.len() {
                for b in (a + 1)..members.len() {
                    let ia = idx_of[&members[a]];
                    let ib = idx_of[&members[b]];
                    let key = if g.to_index(ia) < g.to_index(ib) {
                        (g.to_index(ia), g.to_index(ib))
                    } else {
                        (g.to_index(ib), g.to_index(ia))
                    };
                    if seen.insert(key) {
                        g.add_edge(ia, ib, 1.0);
                    }
                }
            }
        }
        g
    }

    /// The neighbours of a node: all nodes that share at least one hyperedge
    /// with it, excluding itself, in insertion order (deduped). `None` if the
    /// node is absent. XGI parity: `H.nodes.neighbors(n)`.
    pub fn node_neighbors(&self, node_id: &str) -> Option<Vec<String>> {
        let edges = self.memberships(node_id)?;
        let mut result: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in edges {
            for m in self.members(&e).unwrap_or_default() {
                if m != node_id && seen.insert(m.clone()) {
                    result.push(m);
                }
            }
        }
        Some(result)
    }

    /// The degree of a node = number of edges it belongs to. With
    /// `order = Some(d)`, counts only edges of that order (size `d+1`).
    /// `None` if the node is absent. XGI parity: `H.degree(n, order)`.
    pub fn node_degree(&self, node_id: &str, order: Option<usize>) -> Option<usize> {
        let edges = self.memberships(node_id)?;
        Some(match order {
            None => edges.len(),
            Some(d) => edges
                .iter()
                .filter(|e| self.edge_size(e) == Some(d + 1))
                .count(),
        })
    }

    /// The size (number of members) of an edge. `None` if the edge is absent.
    pub fn edge_size(&self, edge_id: &str) -> Option<usize> {
        self.members(edge_id).map(|m| m.len())
    }
}

impl<M> Hypergraph<serde_json::Value, serde_json::Value, M> {
    /// Set node attributes from (id, attr_map) pairs.
    /// XGI parity: `H.set_node_attributes(values, name=None)`.
    /// Silently skips missing node IDs (XGI warns).
    pub fn set_node_attributes(
        &mut self,
        values: impl IntoIterator<Item = (String, serde_json::Map<String, serde_json::Value>)>,
    ) {
        for (node_id, attrs) in values {
            if let Some(node_attrs) = self.node_attrs_mut(&node_id) {
                if !node_attrs.is_object() {
                    *node_attrs = serde_json::Value::Object(serde_json::Map::new());
                }
                if let Some(obj) = node_attrs.as_object_mut() {
                    for (k, v) in attrs {
                        obj.insert(k, v);
                    }
                }
            }
            // Silently skip missing nodes (XGI warns)
        }
    }

    /// Set edge attributes from (id, attr_map) pairs.
    /// XGI parity: `H.set_edge_attributes(values, name=None)`.
    /// Silently skips missing edge IDs (XGI warns).
    pub fn set_edge_attributes(
        &mut self,
        values: impl IntoIterator<Item = (String, serde_json::Map<String, serde_json::Value>)>,
    ) {
        for (edge_id, attrs) in values {
            if let Some(edge_attrs) = self.edge_attrs_mut(&edge_id) {
                if !edge_attrs.is_object() {
                    *edge_attrs = serde_json::Value::Object(serde_json::Map::new());
                }
                if let Some(obj) = edge_attrs.as_object_mut() {
                    for (k, v) in attrs {
                        obj.insert(k, v);
                    }
                }
            }
        }
    }
}

impl Hypergraph<Value, Value, Value> {
    /// The dual: nodes become edges and edges become nodes.
    /// XGI parity: `H.dual()` (hypergraph.py:1446-1466). Isolate nodes are
    /// dropped (they would map to empty dual-edges, which our strict add_edge
    /// cannot represent — deviation #8).
    pub fn dual(&self) -> Self {
        let mut d = Self::new();

        // Original edges -> dual nodes (id = edge id, attrs = edge attrs).
        // Done FIRST so their attrs survive the auto-node-creation in add_edge.
        for eid in self.edge_ids() {
            let attrs = self.edge_attrs(&eid).cloned().unwrap_or(Value::Null);
            d.add_node(&eid, attrs);
        }

        // Original nodes -> dual edges (id = node id, members = memberships).
        for nid in self.node_ids() {
            let memberships = self.memberships(&nid).unwrap_or_default();
            if memberships.is_empty() {
                continue; // isolate node -> empty dual-edge -> skipped
            }
            let attrs = self.node_attrs(&nid).cloned().unwrap_or(Value::Null);
            let _ = d.add_edge(memberships, Some(nid.clone()), attrs);
        }

        // Graph attrs.
        for (k, val) in self.graph_attrs().clone() {
            d.set_graph_attr(&k, val);
        }
        d
    }
}

impl Hypergraph<Value, Value, Value> {
    /// Merge edges that have identical member-sets.
    /// XGI parity: `H.merge_duplicate_edges(rename, merge_rule, multiplicity)`
    /// (hypergraph.py:1257-1419). `rename="tuple"` is deferred (deviation #6);
    /// union produces a JSON array, not a Python set.
    pub fn merge_duplicate_edges(
        &mut self,
        rename: MergeRename,
        merge_rule: MergeRule,
        multiplicity: Option<&str>,
    ) {
        self.assert_not_frozen();
        use std::collections::BTreeMap;

        // Group edge ids by their sorted member-set (deterministic).
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for eid in self.edge_ids() {
            let mut mem = self.members(&eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid);
        }

        let mut to_remove: Vec<String> = Vec::new();
        let mut to_add: Vec<(Vec<String>, String, Value)> = Vec::new();
        for (members, dup_ids) in &groups {
            if dup_ids.len() <= 1 {
                continue;
            }
            to_remove.extend(dup_ids.iter().cloned());
            let new_id = match rename {
                MergeRename::First => {
                    let mut sorted = dup_ids.clone();
                    sorted.sort();
                    sorted[0].clone()
                }
                MergeRename::New => {
                    let id = self.edge_uid_counter.to_string();
                    self.edge_uid_counter += 1;
                    id
                }
            };
            let mut new_attrs = self.merged_edge_attrs(dup_ids, merge_rule);
            if let Some(mkey) = multiplicity {
                // merged_edge_attrs(MergeRule::First) can yield a non-object
                // (e.g. Value::Null, our default empty attrs) — coerce before
                // inserting the multiplicity key (2026-07-19 preflight defect
                // P2-D1: Null.as_object_mut() silently dropped the insert).
                if !new_attrs.is_object() {
                    new_attrs = serde_json::json!({});
                }
                new_attrs
                    .as_object_mut()
                    .unwrap()
                    .insert(mkey.to_string(), serde_json::json!(dup_ids.len()));
            }
            to_add.push((members.clone(), new_id, new_attrs));
        }

        for eid in &to_remove {
            let _ = self.remove_edge(eid);
        }
        for (members, id, attrs) in to_add {
            let _ = self.add_edge(members, Some(id), attrs);
        }
    }

    /// Combine the attrs of a group of duplicate edges per `merge_rule`.
    fn merged_edge_attrs(&self, dup_ids: &[String], rule: MergeRule) -> Value {
        match rule {
            MergeRule::First => {
                let min_id = dup_ids.iter().min().cloned().unwrap_or_default();
                self.edge_attrs(&min_id)
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}))
            }
            MergeRule::Union | MergeRule::Intersection => {
                use std::collections::BTreeSet;
                let mut keys: BTreeSet<String> = BTreeSet::new();
                for id in dup_ids {
                    if let Some(obj) = self.edge_attrs(id).and_then(|a| a.as_object()) {
                        keys.extend(obj.keys().cloned());
                    }
                }
                let mut out = serde_json::Map::new();
                for k in keys {
                    let mut distinct: Vec<Value> = Vec::new();
                    for id in dup_ids {
                        let val = self
                            .edge_attrs(id)
                            .and_then(|a| a.get(&k))
                            .cloned()
                            .unwrap_or(Value::Null);
                        if !distinct.contains(&val) {
                            distinct.push(val);
                        }
                    }
                    let merged = match rule {
                        MergeRule::Union => Value::Array(distinct),
                        _ => {
                            if distinct.len() == 1 {
                                distinct.into_iter().next().unwrap()
                            } else {
                                Value::Null
                            }
                        }
                    };
                    out.insert(k, merged);
                }
                Value::Object(out)
            }
        }
    }

    /// Remove undesirable artifacts in place. XGI parity: `H.cleanup(...)`
    /// (hypergraph.py:1468-1518). `connected` and `relabel` are deferred
    /// (need Phase 3 algorithms + a utils module — deviation #6). Boolean
    /// params mean *allow*: `isolates=false` removes isolate nodes.
    pub fn cleanup(&mut self, isolates: bool, singletons: bool, multiedges: bool) {
        self.assert_not_frozen();
        if !multiedges {
            self.merge_duplicate_edges(MergeRename::First, MergeRule::First, None);
        }
        if !singletons {
            let s = self.singleton_edge_ids();
            let _ = self.remove_edges_from(&s);
        }
        if !isolates {
            let i = self.isolate_node_ids();
            let _ = self.remove_nodes_from(&i, false, true);
        }
    }

    /// Return a cleaned copy, leaving `self` untouched.
    pub fn cleaned(&self, isolates: bool, singletons: bool, multiedges: bool) -> Self {
        let mut c = self.copy();
        c.cleanup(isolates, singletons, multiedges);
        c
    }
}

impl<N: PartialEq, E: PartialEq, M: PartialEq> PartialEq for Hypergraph<N, E, M> {
    /// Two hypergraphs are equal if they have the same nodes, edges,
    /// memberships, and attributes. XGI parity: `H1 == H2`.
    fn eq(&self, other: &Self) -> bool {
        if self.agent_ids.len() != other.agent_ids.len() {
            return false;
        }
        for (nid, _) in &self.agent_ids {
            match (self.node_attrs(nid), other.node_attrs(nid)) {
                (Some(a), Some(b)) if a == b => {}
                _ => return false,
            }
        }
        if self.hyperedge_ids.len() != other.hyperedge_ids.len() {
            return false;
        }
        for (eid, _) in &self.hyperedge_ids {
            match (self.edge_attrs(eid), other.edge_attrs(eid)) {
                (Some(a), Some(b)) if a == b => {}
                _ => return false,
            }
            let mut m1 = self.members(eid).unwrap_or_default();
            let mut m2 = other.members(eid).unwrap_or_default();
            m1.sort();
            m2.sort();
            if m1 != m2 {
                return false;
            }
        }
        self.graph_attrs == other.graph_attrs
    }
}

impl<N, E, M> std::fmt::Debug for Hypergraph<N, E, M>
where
    N: std::fmt::Debug,
    E: std::fmt::Debug,
    M: std::fmt::Debug,
{
    /// XGI parity: `__repr__` returns `Hypergraph([{members}, ...])`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hypergraph(")?;
        let edges: Vec<String> = self
            .hyperedge_ids
            .keys()
            .map(|eid| {
                let members = self.members(eid).unwrap_or_default();
                format!("{{{}}}", members.join(", "))
            })
            .collect();
        write!(f, "[{}])", edges.join(", "))
    }
}
