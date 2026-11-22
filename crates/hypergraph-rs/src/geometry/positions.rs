//! Positions by id, and the result of a placement.

use indexmap::IndexMap;

use super::Point;

/// Positions keyed by id, in insertion order.
///
/// The derived `PartialEq` compares the inner map, which ignores order. Compare `iter()` listings to compare order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Positions {
    map: IndexMap<String, Point>,
}

impl Positions {
    /// An empty set of positions.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the position of `id`. A repeated id keeps its first place in the order; the old point is returned.
    pub fn insert(&mut self, id: impl Into<String>, at: Point) -> Option<Point> {
        self.map.insert(id.into(), at)
    }

    /// The position of `id`, if it has one.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<Point> {
        self.map.get(id).copied()
    }

    /// How many ids have a position.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether no id has a position.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Every `(id, position)`, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Point)> + '_ {
        self.map.iter().map(|(id, at)| (id.as_str(), *at))
    }

    /// The point at `index` in insertion order. Panics when out of range.
    #[allow(dead_code)]
    pub(crate) fn point_mut(&mut self, index: usize) -> &mut Point {
        self.map
            .get_index_mut(index)
            .expect("index in range of positions")
            .1
    }

    /// The id at `index` in insertion order. Panics when out of range.
    #[allow(dead_code)]
    pub(crate) fn id_at(&self, index: usize) -> &str {
        self.map
            .get_index(index)
            .expect("index in range of positions")
            .0
            .as_str()
    }
}

impl FromIterator<(String, Point)> for Positions {
    fn from_iter<I: IntoIterator<Item = (String, Point)>>(iter: I) -> Self {
        Self {
            map: iter.into_iter().collect(),
        }
    }
}

/// The result of placing items.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placement {
    /// Placed items, in input order.
    pub positions: Positions,
    /// Items with no positive weight, in input order. They are never put at the origin.
    pub unplaced: Vec<String>,
}
