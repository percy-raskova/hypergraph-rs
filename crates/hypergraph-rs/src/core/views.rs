//! Read-only proxy views over a [`Hypergraph`]: `NodeView` and `EdgeView`.
//!
//! Each view borrows the hypergraph and holds an insertion-ordered subset of
//! ids. Filtering methods (`isolates`, `singletons`, `filterby_*`) return a new
//! sub-view over the same hypergraph. XGI parity: `H.nodes` / `H.edges`.

use super::hypergraph::Hypergraph;

/// Comparison mode for `filterby_*`. XGI parity: the `mode` argument of
/// `IDView.filterby` (views.py:197-287). `between` is a dedicated method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Eq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl FilterMode {
    /// Whether `actual <mode> val` holds.
    pub fn matches(self, actual: usize, val: usize) -> bool {
        match self {
            FilterMode::Eq => actual == val,
            FilterMode::Neq => actual != val,
            FilterMode::Lt => actual < val,
            FilterMode::Gt => actual > val,
            FilterMode::Leq => actual <= val,
            FilterMode::Geq => actual >= val,
        }
    }
}

/// A read-only view over the nodes of a hypergraph.
pub struct NodeView<'a, N, E, M> {
    graph: &'a Hypergraph<N, E, M>,
    ids: Vec<String>,
}

impl<N, E, M> Hypergraph<N, E, M> {
    /// A view over all nodes (insertion order). XGI parity: `H.nodes`.
    pub fn nodes(&self) -> NodeView<'_, N, E, M> {
        NodeView { graph: self, ids: self.node_ids() }
    }
}

impl<'a, N, E, M> NodeView<'a, N, E, M> {
    /// The tracked node ids, in insertion order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The number of nodes tracked.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `node_id` is tracked by this view.
    pub fn contains(&self, node_id: &str) -> bool {
        self.ids.iter().any(|n| n == node_id)
    }

    /// The node's attributes. XGI parity: `H.nodes[n]`.
    pub fn attrs(&self, node_id: &str) -> Option<&'a N> {
        self.graph.node_attrs(node_id)
    }

    /// The number of edges the node belongs to. XGI parity: `H.nodes.degree`.
    pub fn degree(&self, node_id: &str) -> usize {
        self.graph.memberships(node_id).map_or(0, |m| m.len())
    }

    /// The edge ids the node is a member of. XGI parity:
    /// `H.nodes.memberships(n)` (views.py:604-630).
    pub fn memberships(&self, node_id: &str) -> Option<Vec<String>> {
        self.graph.memberships(node_id)
    }

    /// Sub-view of degree-0 nodes. XGI parity: `H.nodes.isolates()`.
    pub fn isolates(&self) -> NodeView<'a, N, E, M> {
        let ids = self.ids.iter().filter(|n| self.degree(n) == 0).cloned().collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes in no edge of size >= 2. XGI parity:
    /// `H.nodes.isolates(ignore_singletons=True)` (views.py:632-667).
    pub fn isolates_ignoring_singletons(&self) -> NodeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let mut in_big_edges: BTreeSet<String> = BTreeSet::new();
        for eid in self.graph.edge_ids() {
            let members = self.graph.members(&eid).unwrap_or_default();
            if members.len() >= 2 {
                in_big_edges.extend(members);
            }
        }
        let ids = self
            .ids
            .iter()
            .filter(|n| !in_big_edges.contains(*n))
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes whose degree satisfies `mode` vs `val`.
    /// XGI parity: `H.nodes.filterby('degree', val, mode)`.
    pub fn filterby_degree(&self, val: usize, mode: FilterMode) -> NodeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|n| mode.matches(self.degree(n), val))
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }

    /// Sub-view of nodes with `lo <= degree <= hi`.
    /// XGI parity: `filterby('degree', (lo, hi), 'between')`.
    pub fn filterby_degree_between(&self, lo: usize, hi: usize) -> NodeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|n| {
                let d = self.degree(n);
                lo <= d && d <= hi
            })
            .cloned()
            .collect();
        NodeView { graph: self.graph, ids }
    }
}
