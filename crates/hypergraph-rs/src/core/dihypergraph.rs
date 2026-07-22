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
use rustworkx_core::petgraph::visit::EdgeRef;
use rustworkx_core::petgraph::Direction;

use super::error::{EdgeError, MembershipError, NodeError};
use super::kinds::{DiMembershipEdge, DiRole, NodeKind};

/// Deduplicate a Vec of ids, preserving first-seen order.
fn dedup_preserve(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|x| seen.insert(x.clone()))
        .collect()
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
    /// If `node_id` already exists, its stored attrs are REPLACED (XGI's
    /// binding merges the incoming dict into the existing one before
    /// calling; a generic `N` can't merge, so the core replaces and the
    /// PyO3 layer composes the merge on top — divergence D6).
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

    /// Read a node's attributes. XGI parity: `DH.nodes[n]`.
    pub fn node_attrs(&self, node_id: &str) -> Option<&N> {
        let idx = self.agent_ids.get(node_id)?;
        match self.inner.node_weight(*idx)? {
            NodeKind::Agent(attrs) => Some(attrs),
            _ => None,
        }
    }

    /// Add a directed hyperedge with the given tail (senders) and head
    /// (receivers). Auto-creates member nodes. XGI parity:
    /// `DH.add_edge((tail, head), idx=)` (dihypergraph.py:525-585).
    ///
    /// Deviations: `Err(EdgeError::AlreadyExists)` on a duplicate `idx`
    /// (deviation #2). D1: an all-empty (tail, head) pair creates an empty
    /// edge, matching the XGI runtime.
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
                DiMembershipEdge {
                    member_data: M::default(),
                    role: DiRole::Tail,
                },
            );
        }
        for node in &dedup_preserve(head) {
            self.ensure_agent(node);
            let a = self.agent_ids[node];
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge {
                    member_data: M::default(),
                    role: DiRole::Head,
                },
            );
        }

        Ok(edge_id)
    }

    /// All node ids, insertion order. XGI parity: `list(DH.nodes)`.
    pub fn node_ids(&self) -> Vec<String> {
        self.agent_ids.keys().cloned().collect()
    }

    /// All edge ids, insertion order. XGI parity: `list(DH.edges)`.
    pub fn edge_ids(&self) -> Vec<String> {
        self.hyperedge_ids.keys().cloned().collect()
    }

    /// Map a hyperedge NodeIndex back to its id (linear scan, Phase 1 pattern).
    fn edge_id_of(&self, idx: NodeIndex) -> Option<String> {
        self.hyperedge_ids
            .iter()
            .find(|(_, &i)| i == idx)
            .map(|(k, _)| k.clone())
    }

    /// Map an agent NodeIndex back to its id (linear scan, Phase 1 pattern).
    fn node_id_of(&self, idx: NodeIndex) -> Option<String> {
        self.agent_ids
            .iter()
            .find(|(_, &i)| i == idx)
            .map(|(k, _)| k.clone())
    }

    /// (tail, head) node ids of an edge, in membership-add order (D5 owner
    /// upgrade: petgraph's `edges_directed` walks each node's adjacency list
    /// in REVERSE insertion order, so we collect + reverse before splitting
    /// by role). XGI parity: `DH.edges.dimembers(e)` = (edge["in"],
    /// edge["out"]) (views.py:1017-1071), except XGI's sets are
    /// hash-unordered; we are strictly more defined.
    pub fn dimembers(&self, edge_id: &str) -> Option<(Vec<String>, Vec<String>)> {
        let he = *self.hyperedge_ids.get(edge_id)?;
        let mut incoming: Vec<_> = self.inner.edges_directed(he, Direction::Incoming).collect();
        incoming.reverse();
        let mut tail = Vec::new();
        let mut head = Vec::new();
        for er in incoming {
            if let Some(nid) = self.node_id_of(er.source()) {
                match er.weight().role {
                    DiRole::Tail => tail.push(nid),
                    DiRole::Head => head.push(nid),
                }
            }
        }
        Some((tail, head))
    }

    /// Tail (sender) node ids. XGI parity: `DH.edges.tail(e)`.
    pub fn tail(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(t, _)| t)
    }

    /// Head (receiver) node ids. XGI parity: `DH.edges.head(e)`.
    pub fn head(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(_, h)| h)
    }

    /// Union of tail and head, tail-then-head first-seen order (D5 owner
    /// upgrade). XGI parity: `DH.edges.members(e)`.
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.dimembers(edge_id).map(|(mut t, h)| {
            t.extend(h);
            dedup_preserve(t)
        })
    }

    /// (in, out) edge ids for a node: `in` = edges where the node is a head
    /// (receiver), `out` = edges where it is a tail (sender), in
    /// membership-add order (D5 owner upgrade; see [`Self::dimembers`] for
    /// the reverse-adjacency correction). XGI parity: `DH.nodes.dimemberships(n)`
    /// = (node["in"], node["out"]) (views.py:889-918).
    pub fn dimemberships(&self, node_id: &str) -> Option<(Vec<String>, Vec<String>)> {
        let a = *self.agent_ids.get(node_id)?;
        let mut outgoing: Vec<_> = self.inner.edges_directed(a, Direction::Outgoing).collect();
        outgoing.reverse();
        let mut in_edges = Vec::new();
        let mut out_edges = Vec::new();
        for er in outgoing {
            if let Some(eid) = self.edge_id_of(er.target()) {
                match er.weight().role {
                    DiRole::Head => in_edges.push(eid),
                    DiRole::Tail => out_edges.push(eid),
                }
            }
        }
        Some((in_edges, out_edges))
    }

    /// All edge ids the node belongs to (head or tail), first-seen edge
    /// order (D5 owner upgrade): the raw chronological order in which the
    /// node was added to each edge, deduped (a node in both directions of
    /// the same edge counts once, at its first occurrence). XGI parity:
    /// `DH.nodes.memberships(n)`.
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        let a = *self.agent_ids.get(node_id)?;
        let mut outgoing: Vec<_> = self.inner.edges_directed(a, Direction::Outgoing).collect();
        outgoing.reverse();
        let ids: Vec<String> = outgoing
            .into_iter()
            .filter_map(|er| self.edge_id_of(er.target()))
            .collect();
        Some(dedup_preserve(ids))
    }

    /// Remove an edge. XGI parity: `DH.remove_edge(idx)` (dihypergraph.py:834-860).
    pub fn remove_edge(&mut self, edge_id: &str) -> Result<(), EdgeError> {
        self.assert_not_frozen();
        let he = *self.hyperedge_ids.get(edge_id).ok_or(EdgeError::NotFound {
            edge_id: edge_id.to_string(),
        })?;
        self.inner.remove_node(he);
        self.hyperedge_ids.shift_remove(edge_id);
        Ok(())
    }

    /// Remove a node. Weak removal detaches the node from its edges and drops
    /// edges that become empty (when `remove_empty`); strong removal drops all
    /// containing edges. XGI parity: `DH.remove_node(n, strong, remove_empty)`
    /// (dihypergraph.py:385-436).
    pub fn remove_node(
        &mut self,
        node_id: &str,
        strong: bool,
        remove_empty: bool,
    ) -> Result<(), NodeError> {
        self.assert_not_frozen();
        let a = *self.agent_ids.get(node_id).ok_or(NodeError::NotFound {
            node_id: node_id.to_string(),
        })?;
        let (in_e, out_e) = self.dimemberships(node_id).unwrap_or_default();
        let incident: std::collections::BTreeSet<String> = in_e.into_iter().chain(out_e).collect();

        if strong {
            for eid in &incident {
                let _ = self.remove_edge(eid);
            }
        } else {
            // Detach: remove all of the agent's outgoing membership edges.
            let eidxs: Vec<_> = self.inner.edges(a).map(|er| er.id()).collect();
            for e in eidxs {
                self.inner.remove_edge(e);
            }
            // Drop now-empty edges.
            for eid in &incident {
                if let Some(he) = self.hyperedge_ids.get(eid).copied() {
                    let empty = self
                        .inner
                        .edges_directed(he, Direction::Incoming)
                        .next()
                        .is_none();
                    if empty && remove_empty {
                        self.inner.remove_node(he);
                        self.hyperedge_ids.shift_remove(eid);
                    }
                }
            }
        }

        self.inner.remove_node(a);
        self.agent_ids.shift_remove(node_id);
        Ok(())
    }

    /// Add a node to an edge in the given role. Auto-creates both if missing.
    /// XGI parity: `DH.add_node_to_edge(edge, node, direction)`
    /// (dihypergraph.py:794-832).
    pub fn add_node_to_edge(
        &mut self,
        edge_id: &str,
        node_id: &str,
        role: DiRole,
    ) -> Result<(), EdgeError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        self.assert_not_frozen();
        if !self.hyperedge_ids.contains_key(edge_id) {
            let he = self.inner.add_node(NodeKind::Hyperedge(E::default()));
            self.hyperedge_ids.insert(edge_id.to_string(), he);
            if let Ok(n) = edge_id.parse::<u64>() {
                if n >= self.edge_uid_counter {
                    self.edge_uid_counter = n + 1;
                }
            }
        }
        self.ensure_agent(node_id);
        let a = self.agent_ids[node_id];
        let he = self.hyperedge_ids[edge_id];
        let exists = self
            .inner
            .edges_directed(he, Direction::Incoming)
            .any(|er| er.source() == a && er.weight().role == role);
        if !exists {
            self.inner.add_edge(
                a,
                he,
                DiMembershipEdge {
                    member_data: M::default(),
                    role,
                },
            );
        }
        Ok(())
    }

    /// Remove a node from an edge's given role. Errors if the edge/node is
    /// missing or the node is not in that role-set; drops the edge if empty and
    /// `remove_empty`. XGI parity: `DH.remove_node_from_edge(edge, node,
    /// direction, remove_empty)` (dihypergraph.py:891-943). XGI raises
    /// `XGIError` for a missing edge, a missing node, or a node not a
    /// member in that direction — the core returns the matching
    /// [`MembershipError`] variant and the PyO3 binding translates
    /// `Err` -> raise, reproducing XGI's exact per-direction messages
    /// (D2 error channel).
    pub fn remove_node_from_edge(
        &mut self,
        edge_id: &str,
        node_id: &str,
        role: DiRole,
        remove_empty: bool,
    ) -> Result<(), MembershipError> {
        self.assert_not_frozen();
        let he = *self
            .hyperedge_ids
            .get(edge_id)
            .ok_or(MembershipError::EdgeNotFound {
                edge_id: edge_id.to_string(),
            })?;
        let a = *self
            .agent_ids
            .get(node_id)
            .ok_or(MembershipError::NodeNotFound {
                node_id: node_id.to_string(),
            })?;
        let target = self
            .inner
            .edges_directed(he, Direction::Incoming)
            .find(|er| er.source() == a && er.weight().role == role)
            .map(|er| er.id());
        match target {
            Some(e) => {
                self.inner.remove_edge(e);
            }
            None => {
                return Err(MembershipError::NotAMember {
                    node_id: node_id.to_string(),
                    edge_id: edge_id.to_string(),
                });
            }
        }
        if remove_empty
            && self
                .inner
                .edges_directed(he, Direction::Incoming)
                .next()
                .is_none()
        {
            self.inner.remove_node(he);
            self.hyperedge_ids.shift_remove(edge_id);
        }
        Ok(())
    }

    /// Node degree: number of distinct edges it belongs to. XGI parity:
    /// `DH.nodes.degree` (dinodestats.py:138-139).
    pub fn degree(&self, node_id: &str) -> usize {
        self.memberships(node_id).map_or(0, |m| m.len())
    }

    /// Node in-degree: edges where the node is a head (receiver).
    pub fn in_degree(&self, node_id: &str) -> usize {
        self.dimemberships(node_id).map_or(0, |(i, _)| i.len())
    }

    /// Node out-degree: edges where the node is a tail (sender).
    pub fn out_degree(&self, node_id: &str) -> usize {
        self.dimemberships(node_id).map_or(0, |(_, o)| o.len())
    }

    /// Edge size: |tail ∪ head|. XGI parity: `DH.edges.size`.
    pub fn size(&self, edge_id: &str) -> usize {
        self.members(edge_id).map_or(0, |m| m.len())
    }

    /// Edge order: size − 1. XGI parity: `DH.edges.order`.
    pub fn order(&self, edge_id: &str) -> usize {
        self.size(edge_id).saturating_sub(1)
    }

    /// Number of tail (sender) members.
    pub fn tail_size(&self, edge_id: &str) -> usize {
        self.tail(edge_id).map_or(0, |t| t.len())
    }

    /// Number of head (receiver) members.
    pub fn head_size(&self, edge_id: &str) -> usize {
        self.head(edge_id).map_or(0, |h| h.len())
    }

    /// Tail size − 1.
    pub fn tail_order(&self, edge_id: &str) -> usize {
        self.tail_size(edge_id).saturating_sub(1)
    }

    /// Head size − 1.
    pub fn head_order(&self, edge_id: &str) -> usize {
        self.head_size(edge_id).saturating_sub(1)
    }

    /// Node ids with degree 0. XGI parity: `DH.nodes.isolates()`.
    pub fn isolate_node_ids(&self) -> Vec<String> {
        self.node_ids()
            .into_iter()
            .filter(|n| self.degree(n) == 0)
            .collect()
    }

    /// Remove all nodes and edges (keeps the uid counter, like Phase 1).
    /// XGI parity: `DH.clear()` (dihypergraph.py:1000-1017).
    pub fn clear(&mut self) {
        self.assert_not_frozen();
        self.inner = StableDiGraph::new();
        self.agent_ids.clear();
        self.hyperedge_ids.clear();
        self.graph_attrs.clear();
    }

    /// An independent deep copy. XGI parity: `DH.copy()`.
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

    /// Freeze against modification. XGI parity: `DH.freeze()`.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Whether the dihypergraph is frozen. XGI parity: `DH.is_frozen`.
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
}

impl<N: PartialEq, E: PartialEq, M> PartialEq for DiHypergraph<N, E, M> {
    /// Equal iff same nodes (with attrs), same edges (with (tail,head) sets and
    /// attrs), and same graph attrs.
    fn eq(&self, other: &Self) -> bool {
        if self.agent_ids.len() != other.agent_ids.len() {
            return false;
        }
        for (nid, &idx) in &self.agent_ids {
            let a = match self.inner.node_weight(idx) {
                Some(NodeKind::Agent(a)) => a,
                _ => return false,
            };
            let b_idx = match other.agent_ids.get(nid) {
                Some(i) => *i,
                None => return false,
            };
            match other.inner.node_weight(b_idx) {
                Some(NodeKind::Agent(b)) if a == b => {}
                _ => return false,
            }
        }
        if self.hyperedge_ids.len() != other.hyperedge_ids.len() {
            return false;
        }
        for (eid, &idx) in &self.hyperedge_ids {
            let a = match self.inner.node_weight(idx) {
                Some(NodeKind::Hyperedge(a)) => a,
                _ => return false,
            };
            let b_idx = match other.hyperedge_ids.get(eid) {
                Some(i) => *i,
                None => return false,
            };
            match other.inner.node_weight(b_idx) {
                Some(NodeKind::Hyperedge(b)) if a == b => {}
                _ => return false,
            }
            let mut a_dm = self.dimembers(eid).unwrap_or_default();
            let mut b_dm = other.dimembers(eid).unwrap_or_default();
            a_dm.0.sort();
            a_dm.1.sort();
            b_dm.0.sort();
            b_dm.1.sort();
            if a_dm != b_dm {
                return false;
            }
        }
        self.graph_attrs == other.graph_attrs
    }
}

impl<N, E, M> std::fmt::Debug for DiHypergraph<N, E, M> {
    /// XGI parity: `__repr__` = `DiHypergraph([(tail, head), ...])`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiHypergraph(")?;
        let edges: Vec<String> = self
            .edge_ids()
            .iter()
            .map(|eid| {
                let (t, h) = self.dimembers(eid).unwrap_or_default();
                format!("({{{}}}, {{{}}})", t.join(", "), h.join(", "))
            })
            .collect();
        write!(f, "[{}])", edges.join(", "))
    }
}
