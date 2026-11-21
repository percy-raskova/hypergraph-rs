//! `IdMap`: string id to node index and node index back to string id, kept in step.
//!
//! The forward map is insertion ordered (every listing uses that order). The
//! reverse map is for point lookups only and is never iterated, so no result
//! depends on the iteration order of a `HashMap`. Both fields are private:
//! nothing outside this file can change one without the other.

use std::collections::HashMap;

use indexmap::map::{Iter, Keys, Values};
use indexmap::IndexMap;
use rustworkx_core::petgraph::stable_graph::NodeIndex;

#[derive(Clone, Default)]
pub(crate) struct IdMap {
    /// Insertion order: the order every listing uses.
    forward: IndexMap<String, NodeIndex>,
    /// Point lookups only, never iterated.
    reverse: HashMap<NodeIndex, String>,
}

impl IdMap {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The id that currently holds `index`, in constant time.
    pub(crate) fn id_of(&self, index: NodeIndex) -> Option<&str> {
        self.reverse.get(&index).map(String::as_str)
    }

    /// Insert or replace an id, updating both directions. An id that is
    /// present keeps its position and takes the new index; its old index
    /// leaves the reverse map.
    pub(crate) fn insert(&mut self, id: String, index: NodeIndex) {
        debug_assert!(
            self.reverse.get(&index).is_none_or(|owner| *owner == id),
            "index {index:?} is already held by another id"
        );
        if let Some(old) = self.forward.insert(id.clone(), index) {
            if old != index {
                self.reverse.remove(&old);
            }
        }
        self.reverse.insert(index, id);
    }

    /// Remove an id, keeping the order of the rest, in both directions.
    pub(crate) fn shift_remove(&mut self, id: &str) -> Option<NodeIndex> {
        let index = self.forward.shift_remove(id)?;
        self.reverse.remove(&index);
        Some(index)
    }

    pub(crate) fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.forward.len()
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }

    pub(crate) fn contains_key(&self, id: &str) -> bool {
        self.forward.contains_key(id)
    }

    pub(crate) fn get(&self, id: &str) -> Option<&NodeIndex> {
        self.forward.get(id)
    }

    pub(crate) fn keys(&self) -> Keys<'_, String, NodeIndex> {
        self.forward.keys()
    }

    pub(crate) fn values(&self) -> Values<'_, String, NodeIndex> {
        self.forward.values()
    }

    #[allow(dead_code)]
    pub(crate) fn iter(&self) -> Iter<'_, String, NodeIndex> {
        self.forward.iter()
    }
}

impl std::ops::Index<&str> for IdMap {
    type Output = NodeIndex;

    fn index(&self, id: &str) -> &NodeIndex {
        &self.forward[id]
    }
}

impl<'a> IntoIterator for &'a IdMap {
    type Item = (&'a String, &'a NodeIndex);
    type IntoIter = Iter<'a, String, NodeIndex>;

    fn into_iter(self) -> Self::IntoIter {
        self.forward.iter()
    }
}
