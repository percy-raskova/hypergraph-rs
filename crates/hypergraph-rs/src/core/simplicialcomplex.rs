//! The simplicial complex data structure.
//!
//! A `SimplicialComplex` composes a [`Hypergraph`] and enforces closure under
//! faces: adding a simplex also adds every missing subface of size >= 2. A
//! lazily-computed [`FaceLattice`] caches the immediate-face relation; it is
//! invalidated on every mutation. XGI parity: `xgi.SimplicialComplex`
//! (simplicialcomplex.py).

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

use super::error::SimplexError;
use super::hypergraph::Hypergraph;

/// The cached immediate-face relation: each simplex id maps to the ids of the
/// codimension-1 simplices present in the complex that are its faces.
#[derive(Debug, Clone, Default)]
pub struct FaceLattice {
    /// simplex id -> ids of its codim-1 faces present in the complex.
    pub immediate_faces: BTreeMap<String, Vec<String>>,
}

/// A simplicial complex (undirected, closed under faces).
pub struct SimplicialComplex<N = serde_json::Value, E = serde_json::Value, M = serde_json::Value> {
    inner: Hypergraph<N, E, M>,
    face_lattice: OnceCell<FaceLattice>,
}

/// Deduplicate a Vec of ids into a sorted set-vector (deterministic order).
fn as_sorted_set(items: &[String]) -> Vec<String> {
    let set: BTreeSet<String> = items.iter().cloned().collect();
    set.into_iter().collect()
}

/// All subsets of `set` of sizes from `set.len()-1` down to 2 (never
/// singletons). XGI parity: `_subfaces` (simplicialcomplex.py:334-361).
fn subfaces(set: &[String]) -> Vec<Vec<String>> {
    let k = set.len();
    let mut out: Vec<Vec<String>> = Vec::new();
    // sizes k-1, k-2, ..., 2
    let mut size = k.saturating_sub(1);
    while size >= 2 {
        combinations(set, size, &mut out);
        size -= 1;
    }
    out
}

/// Push every `choose`-sized combination of `set` (sorted input -> sorted
/// combos) into `out`. Bounded by `total = n_choose_k(set.len(), choose)`,
/// computed BEFORE the loop (Power-of-10 rule 2 — 2026-07-19 preflight
/// correction, defect P2-D2: the prior `loop { ... }` form terminated via an
/// internal `return` after C(n,k) iterations, which is provably-but-not
/// *statically* bounded; this form gives a checkable `for` bound instead).
fn combinations(set: &[String], choose: usize, out: &mut Vec<Vec<String>>) {
    let n = set.len();
    if choose == 0 || choose > n {
        return;
    }
    let total = n_choose_k(n, choose);
    // index-based lexicographic combinations; bounded by n choose k.
    let mut idx: Vec<usize> = (0..choose).collect();
    for _ in 0..total {
        out.push(idx.iter().map(|&i| set[i].clone()).collect());
        // Advance to the next combination: find the rightmost index that can
        // still be incremented (bounded by `choose`, known before this
        // sub-loop starts).
        let mut pivot: Option<usize> = None;
        for i in (0..choose).rev() {
            if idx[i] != i + n - choose {
                pivot = Some(i);
                break;
            }
        }
        let i = match pivot {
            Some(i) => i,
            None => return, // exhausted (the final combination was just pushed)
        };
        idx[i] += 1;
        for j in (i + 1)..choose {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// `n` choose `k`, via the multiplicative formula (no factorials — this
/// crate's `n` is always small, but the formula avoids any needless overflow
/// risk regardless).
fn n_choose_k(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: usize = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

impl<N, E, M> Default for SimplicialComplex<N, E, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, M> SimplicialComplex<N, E, M> {
    /// Create an empty simplicial complex.
    pub fn new() -> Self {
        Self {
            inner: Hypergraph::new(),
            face_lattice: OnceCell::new(),
        }
    }

    /// The id the next simplex added without an explicit id will get.
    /// XGI parity: `S._edge_uid`.
    pub fn next_edge_id(&self) -> u64 {
        self.inner.next_edge_id()
    }

    /// Raise the counter past `idx` (see [`Hypergraph::update_uid_counter`]).
    pub fn update_uid_counter(&mut self, idx: u64) {
        self.inner.update_uid_counter(idx);
    }

    /// Rebuild a complex from its nodes and its simplices in id order, as
    /// `node_ids()` and `edge_ids()` list them, adding no face: every subface
    /// of size >= 2 of each simplex must be in the list. The first simplex
    /// with a face missing gives `SimplexError::NotClosed { idx }`. A repeated
    /// simplex id keeps its first entry, as `add_simplex` does.
    pub fn from_closed_simplices(
        nodes: Vec<(String, N)>,
        simplices: Vec<(String, Vec<String>, E)>,
    ) -> Result<Self, SimplexError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        let present: BTreeSet<Vec<String>> = simplices
            .iter()
            .map(|(_, members, _)| as_sorted_set(members))
            .collect();
        for (id, members, _) in &simplices {
            let set = as_sorted_set(members);
            if subfaces(&set)
                .iter()
                .any(|face| !face.is_empty() && !present.contains(face))
            {
                return Err(SimplexError::NotClosed { idx: id.clone() });
            }
        }
        let mut complex = Self::new();
        complex.add_nodes_from(nodes);
        for (id, members, attrs) in simplices {
            let _ = complex
                .inner
                .add_edge(as_sorted_set(&members), Some(id), attrs);
        }
        Ok(complex)
    }

    /// Number of nodes. XGI parity: `S.num_nodes`.
    pub fn num_nodes(&self) -> usize {
        self.inner.num_nodes()
    }

    /// Number of simplices. XGI parity: `S.num_edges`.
    pub fn num_edges(&self) -> usize {
        self.inner.num_edges()
    }

    /// Whether a node exists.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.inner.has_node(node_id)
    }

    /// Whether a simplex id exists.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.inner.has_edge(edge_id)
    }

    /// The members of a simplex (by id).
    pub fn members(&self, edge_id: &str) -> Option<Vec<String>> {
        self.inner.members(edge_id)
    }

    /// A node's attributes.
    pub fn node_attrs(&self, node_id: &str) -> Option<&N> {
        self.inner.node_attrs(node_id)
    }

    /// A simplex's attributes.
    pub fn edge_attrs(&self, edge_id: &str) -> Option<&E> {
        self.inner.edge_attrs(edge_id)
    }

    /// All node ids (insertion order).
    pub fn node_ids(&self) -> Vec<String> {
        self.inner.node_ids()
    }

    /// Add a node (no simplices). XGI parity: `S.add_node(id, **attr)`.
    /// (Phase 4 Task 9 addition: the generator `random_simplicial_complex`
    /// must add all `N` nodes up front regardless of which simplices are
    /// sampled — XGI's own generator does `S.add_nodes_from(nodes)` before
    /// `S.add_simplices_from(simplices)`. This method was not part of the
    /// Phase-2 surface the Task-9 plan text assumed; it mirrors
    /// `Hypergraph::add_node` exactly and required no other change.)
    pub fn add_node(&mut self, node_id: &str, attrs: N) -> bool {
        self.invalidate();
        self.inner.add_node(node_id, attrs)
    }

    /// Add many nodes (no simplices). XGI parity: `S.add_nodes_from(nbunch)`.
    pub fn add_nodes_from(&mut self, nodes: impl IntoIterator<Item = (String, N)>) {
        for (node_id, attrs) in nodes {
            self.add_node(&node_id, attrs);
        }
    }

    /// All simplex ids (insertion order).
    pub fn edge_ids(&self) -> Vec<String> {
        self.inner.edge_ids()
    }

    /// Whether a simplex with exactly these members is present (by member-set,
    /// not id). XGI parity: `S.has_simplex(simplex)` (simplicialcomplex.py:783-806).
    pub fn has_simplex(&self, members: &[String]) -> bool {
        let want: BTreeSet<String> = members.iter().cloned().collect();
        self.inner.edge_ids().iter().any(|e| {
            let got: BTreeSet<String> = self
                .inner
                .members(e)
                .unwrap_or_default()
                .into_iter()
                .collect();
            got == want
        })
    }

    /// Add a simplex and all its missing subfaces (size >= 2). XGI parity:
    /// `S.add_simplex(members, idx, **attr)` (simplicialcomplex.py:252-332).
    /// Duplicate members or a duplicate `idx` are silent no-ops (deviation #5).
    pub fn add_simplex(
        &mut self,
        members: Vec<String>,
        idx: Option<String>,
        attrs: E,
    ) -> Result<(), SimplexError>
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        let set = as_sorted_set(&members);
        // D1: XGI's docstring claims empty simplices cannot be added; the
        // runtime CREATES one. A repeat empty add is a no-op via the
        // has_simplex check below (an empty set is a member of itself).
        if self.has_simplex(&set) {
            return Ok(());
        }
        if let Some(id) = &idx {
            if self.inner.has_edge(id) {
                return Ok(()); // duplicate idx -> no-op (XGI warns+returns)
            }
        }
        // Top simplex. `add_edge` has no member-count floor to pass — it
        // accepts an empty member list exactly as it accepts any other (see
        // the documentation-defect fix on `Hypergraph::add_empty_edge`,
        // same D1 register entry this function's own comment above cites);
        // an empty `set` here (the empty-simplex case) succeeds the same way.
        let _ = self.inner.add_edge(set.clone(), idx, attrs);
        // Missing subfaces (size >= 2), attribute-free.
        for face in subfaces(&set) {
            if face.is_empty() || self.has_simplex(&face) {
                continue;
            }
            let _ = self.inner.add_edge(face, None, E::default());
        }
        self.invalidate();
        Ok(())
    }

    /// `add_edge` is not available on a simplicial complex.
    /// XGI parity: raises XGIError (simplicialcomplex.py:135-139).
    pub fn add_edge(
        &mut self,
        _members: Vec<String>,
        _idx: Option<String>,
        _attrs: E,
    ) -> Result<(), SimplexError> {
        Err(SimplexError::NotAvailable {
            method: "add_edge".to_string(),
            alternative: "add_simplex".to_string(),
        })
    }

    /// `remove_edge` is not available on a simplicial complex.
    /// XGI parity: raises XGIError (simplicialcomplex.py:153-157).
    pub fn remove_edge(&mut self, _edge_id: &str) -> Result<(), SimplexError> {
        Err(SimplexError::NotAvailable {
            method: "remove_edge".to_string(),
            alternative: "remove_simplex_id".to_string(),
        })
    }

    /// The lazily-computed face lattice (immediate-face relation), rebuilt on
    /// first access after a mutation.
    pub fn face_lattice(&self) -> &FaceLattice {
        self.face_lattice
            .get_or_init(|| self.compute_face_lattice())
    }

    fn compute_face_lattice(&self) -> FaceLattice {
        // Precompute each simplex's member-set.
        let sets: Vec<(String, BTreeSet<String>)> = self
            .inner
            .edge_ids()
            .into_iter()
            .map(|e| {
                let s: BTreeSet<String> = self
                    .inner
                    .members(&e)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                (e, s)
            })
            .collect();
        let mut immediate_faces: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (eid, me) in &sets {
            let faces: Vec<String> = sets
                .iter()
                .filter(|(fid, mf)| fid != eid && mf.len() + 1 == me.len() && mf.is_subset(me))
                .map(|(fid, _)| fid.clone())
                .collect();
            immediate_faces.insert(eid.clone(), faces);
        }
        FaceLattice { immediate_faces }
    }

    /// Invalidate the cached face lattice (called after any mutation).
    fn invalidate(&mut self) {
        self.face_lattice = OnceCell::new();
    }

    /// Add many simplices (each via `add_simplex`, so each closes under faces).
    /// XGI parity: `S.add_simplices_from(ebunch)` — 3-tuple form
    /// (simplicialcomplex.py:373-636).
    pub fn add_simplices_from(&mut self, simplices: Vec<(Vec<String>, Option<String>, E)>)
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        for (members, idx, attrs) in simplices {
            let _ = self.add_simplex(members, idx, attrs);
        }
    }

    /// Add all missing subfaces of every existing simplex.
    /// XGI parity: `S.close()` (simplicialcomplex.py:638-662).
    pub fn close(&mut self)
    where
        N: Default,
        E: Default,
        M: Default + Clone,
    {
        let existing: Vec<Vec<String>> = self
            .inner
            .edge_ids()
            .iter()
            .filter_map(|e| self.inner.members(e))
            .collect();
        for members in existing {
            for face in subfaces(&as_sorted_set(&members)) {
                if !face.is_empty() && !self.has_simplex(&face) {
                    let _ = self.inner.add_edge(face, None, E::default());
                }
            }
        }
        self.invalidate();
    }

    /// Remove a simplex AND every simplex that strictly contains it (its
    /// supfaces), preserving closure. XGI parity: `S.remove_simplex_id(idx)`
    /// (simplicialcomplex.py:726-756).
    pub fn remove_simplex_id(&mut self, idx: &str) -> Result<(), SimplexError> {
        if !self.inner.has_edge(idx) {
            return Err(SimplexError::NotFound {
                idx: idx.to_string(),
            });
        }
        let target: BTreeSet<String> = self
            .inner
            .members(idx)
            .unwrap_or_default()
            .into_iter()
            .collect();
        // Supfaces: simplices whose member-set is a strict superset of target.
        let supfaces: Vec<String> = self
            .inner
            .edge_ids()
            .into_iter()
            .filter(|e| {
                let m: BTreeSet<String> = self
                    .inner
                    .members(e)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                m.len() > target.len() && target.is_subset(&m)
            })
            .collect();
        for s in supfaces {
            let _ = self.inner.remove_edge(&s);
        }
        let _ = self.inner.remove_edge(idx);
        self.invalidate();
        Ok(())
    }

    /// Remove several simplices by id (each with its supfaces).
    pub fn remove_simplex_ids_from(&mut self, ids: &[String]) {
        for idx in ids {
            // A supface removal earlier may already have removed this id; skip.
            if self.inner.has_edge(idx) {
                let _ = self.remove_simplex_id(idx);
            }
        }
    }

    /// An independent deep copy (face lattice recomputed lazily).
    /// XGI parity: `S.copy()`.
    pub fn copy(&self) -> Self
    where
        N: Clone,
        E: Clone,
        M: Clone,
    {
        Self {
            inner: self.inner.copy(),
            face_lattice: OnceCell::new(),
        }
    }

    /// Freeze against modification (via the inner hypergraph's frozen guard).
    /// XGI parity: `S.freeze()`.
    pub fn freeze(&mut self) {
        self.inner.freeze();
    }

    /// Whether the complex is frozen. XGI parity: `S.is_frozen`.
    pub fn is_frozen(&self) -> bool {
        self.inner.is_frozen()
    }
}
