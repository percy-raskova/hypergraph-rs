//! Read-only proxy views over a [`Hypergraph`]: `NodeView` and `EdgeView`.
//!
//! Each view borrows the hypergraph and holds an insertion-ordered subset of
//! ids. Filtering methods (`isolates`, `singletons`, `filterby_*`) return a new
//! sub-view over the same hypergraph. XGI parity: `H.nodes` / `H.edges`.

use super::hypergraph::Hypergraph;
use std::collections::{BTreeSet, HashMap};

/// Group `(id, key)` pairs by key; keep groups of two or more, in order of first occurrence.
fn group_by_key(items: impl Iterator<Item = (String, BTreeSet<String>)>) -> Vec<Vec<String>> {
    let mut index: HashMap<BTreeSet<String>, usize> = HashMap::new();
    let mut groups: Vec<Vec<String>> = Vec::new();
    for (id, key) in items {
        match index.get(&key) {
            Some(&i) => groups[i].push(id),
            None => {
                index.insert(key, groups.len());
                groups.push(vec![id]);
            }
        }
    }
    groups.retain(|g| g.len() > 1);
    groups
}

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
        NodeView {
            graph: self,
            ids: self.node_ids(),
        }
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

    /// Node ids with the same edge-membership set, grouped. Only groups of two or more; groups in
    /// order of first occurrence in this view; ids in a group in insertion order. XGI parity:
    /// `H.nodes.duplicate_groups()`.
    pub fn duplicate_groups(&self) -> Vec<Vec<String>> {
        group_by_key(self.ids.iter().map(|n| {
            let key: BTreeSet<String> = self
                .graph
                .memberships(n)
                .unwrap_or_default()
                .into_iter()
                .collect();
            (n.clone(), key)
        }))
    }

    /// Sub-view of degree-0 nodes. XGI parity: `H.nodes.isolates()`.
    pub fn isolates(&self) -> NodeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|n| self.degree(n) == 0)
            .cloned()
            .collect();
        NodeView {
            graph: self.graph,
            ids,
        }
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
        NodeView {
            graph: self.graph,
            ids,
        }
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
        NodeView {
            graph: self.graph,
            ids,
        }
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
        NodeView {
            graph: self.graph,
            ids,
        }
    }
}

/// A read-only view over the edges of a hypergraph.
pub struct EdgeView<'a, N, E, M> {
    graph: &'a Hypergraph<N, E, M>,
    ids: Vec<String>,
}

impl<N, E, M> Hypergraph<N, E, M> {
    /// A view over all edges (insertion order). XGI parity: `H.edges`.
    pub fn edges(&self) -> EdgeView<'_, N, E, M> {
        EdgeView {
            graph: self,
            ids: self.edge_ids(),
        }
    }
}

impl<'a, N, E, M> EdgeView<'a, N, E, M> {
    /// The tracked edge ids, in insertion order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The number of edges tracked.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the view is empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `edge_id` is tracked by this view.
    pub fn contains(&self, edge_id: &str) -> bool {
        self.ids.iter().any(|e| e == edge_id)
    }

    /// The edge's attributes. XGI parity: `H.edges[e]`.
    pub fn attrs(&self, edge_id: &str) -> Option<&'a E> {
        self.graph.edge_attrs(edge_id)
    }

    /// The number of members. XGI parity: `H.edges.size` (edgestats.py:148).
    pub fn size(&self, edge_id: &str) -> usize {
        self.graph.members(edge_id).map_or(0, |m| m.len())
    }

    /// The order (size − 1). XGI parity: `H.edges.order` (edgestats.py:108).
    /// Saturates at 0 (our edges are never empty in normal use).
    pub fn order(&self, edge_id: &str) -> usize {
        self.size(edge_id).saturating_sub(1)
    }

    /// The member node ids. XGI parity: `H.edges.members(e)` (views.py:706-747).
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.graph.members(edge_id)
    }

    /// Sub-view of size-1 edges. XGI parity: `H.edges.singletons()`.
    pub fn singletons(&self) -> EdgeView<'a, N, E, M> {
        self.filterby_size(1, FilterMode::Eq)
    }

    /// Sub-view of size-0 edges. XGI parity: `H.edges.empty()`.
    pub fn empty(&self) -> EdgeView<'a, N, E, M> {
        self.filterby_size(0, FilterMode::Eq)
    }

    /// Sub-view of edges whose size satisfies `mode` vs `val`.
    pub fn filterby_size(&self, val: usize, mode: FilterMode) -> EdgeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|e| mode.matches(self.size(e), val))
            .cloned()
            .collect();
        EdgeView {
            graph: self.graph,
            ids,
        }
    }

    /// Sub-view of edges whose order satisfies `mode` vs `val`.
    pub fn filterby_order(&self, val: usize, mode: FilterMode) -> EdgeView<'a, N, E, M> {
        let ids = self
            .ids
            .iter()
            .filter(|e| mode.matches(self.order(e), val))
            .cloned()
            .collect();
        EdgeView {
            graph: self.graph,
            ids,
        }
    }

    /// Edge ids with the same member set, grouped. Only groups of two or more; groups in order of
    /// first occurrence in this view; ids in a group in insertion order. XGI parity:
    /// `H.edges.duplicate_groups()`.
    pub fn duplicate_groups(&self) -> Vec<Vec<String>> {
        group_by_key(self.ids.iter().map(|e| {
            let key: BTreeSet<String> = self
                .graph
                .members(e)
                .unwrap_or_default()
                .into_iter()
                .collect();
            (e.clone(), key)
        }))
    }

    /// Sub-view of edges that duplicate an earlier one (same member-set).
    /// The first (sorted) id of each group is kept; the rest are duplicates.
    /// XGI parity: `H.edges.duplicates()` (views.py:409-465).
    pub fn duplicates(&self) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for eid in &self.ids {
            let mut mem = self.graph.members(eid).unwrap_or_default();
            mem.sort();
            groups.entry(mem).or_default().push(eid.clone());
        }
        let mut dups: Vec<String> = Vec::new();
        for members_ids in groups.values() {
            if members_ids.len() > 1 {
                let mut sorted = members_ids.clone();
                sorted.sort();
                dups.extend(sorted.into_iter().skip(1));
            }
        }
        // Preserve insertion order among the flagged ids.
        let ids = self
            .ids
            .iter()
            .filter(|e| dups.contains(e))
            .cloned()
            .collect();
        EdgeView {
            graph: self.graph,
            ids,
        }
    }

    /// Sub-view of edges whose member-set equals `members`.
    /// XGI parity: `H.edges.lookup(members)` (views.py:467-509).
    pub fn lookup(&self, members: &[String]) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let want: BTreeSet<String> = members.iter().cloned().collect();
        let ids = self
            .ids
            .iter()
            .filter(|e| {
                let got: BTreeSet<String> = self
                    .graph
                    .members(e)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                got == want
            })
            .cloned()
            .collect();
        EdgeView {
            graph: self.graph,
            ids,
        }
    }

    /// Sub-view of maximal edges (not a subset of any other edge).
    /// `strict=false` keeps maximal multiedges; `strict=true` excludes any edge
    /// that is a subset of or equal to another. XGI parity: `H.edges.maximal()`
    /// (views.py:777-847).
    pub fn maximal(&self, strict: bool) -> EdgeView<'a, N, E, M> {
        use std::collections::BTreeSet;
        let sets: Vec<(String, BTreeSet<String>)> = self
            .ids
            .iter()
            .map(|e| {
                let s: BTreeSet<String> = self
                    .graph
                    .members(e)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                (e.clone(), s)
            })
            .collect();

        let ids = self
            .ids
            .iter()
            .filter(|e| {
                let me = &sets.iter().find(|(id, _)| id == *e).unwrap().1;
                // Keep e unless some OTHER edge f makes it non-maximal.
                !sets.iter().any(|(fid, mf)| {
                    if fid == *e {
                        return false;
                    }
                    if strict {
                        me.is_subset(mf) // subset OR equal disqualifies (strict)
                    } else {
                        me.is_subset(mf) && me.len() < mf.len() // strict subset only
                    }
                })
            })
            .cloned()
            .collect();
        EdgeView {
            graph: self.graph,
            ids,
        }
    }
}
