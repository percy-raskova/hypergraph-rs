//! Node/edge statistics (XGI `xgi/stats/*.py` port).
//!
//! XGI wraps stats in lazy proxy objects (`IDStat`) with `.asdict()`/`.aslist()`/
//! `.filterby()`. We compute eagerly into an insertion-ordered [`Stat`] value that
//! offers the same terminal operations as pure data.

pub mod diedgestats;
pub mod dinodestats;
pub mod edgestats;
pub mod nodestats;

pub use diedgestats::{di_edge_order, di_edge_size, di_head_size, di_tail_size};
pub use dinodestats::{di_degree, di_in_degree, di_out_degree};
pub use edgestats::{edge_attrs, edge_order, edge_size};
pub use nodestats::{node_attrs, node_degree, node_degree_weighted};

use indexmap::IndexMap;

/// An id → quantity mapping, insertion-ordered (mirrors XGI's `IDStat` result dict).
///
/// `T` is the stat's value type (`usize` for degree/order/size, `serde_json::Value`
/// for attribute stats). Terminal ops mirror XGI's `as*` / `filterby` surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Stat<T> {
    inner: IndexMap<String, T>,
}

impl<T> Stat<T> {
    /// Construct from an insertion-ordered map (internal to the stat functions).
    pub fn new(inner: IndexMap<String, T>) -> Self {
        Self { inner }
    }

    /// XGI `.asdict()` — borrow the id → value map.
    pub fn asdict(&self) -> &IndexMap<String, T> {
        &self.inner
    }

    /// XGI `.aslist()` — values in id insertion order.
    pub fn aslist(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.inner.values().cloned().collect()
    }

    /// Number of ids in the mapping (XGI `len(stat)`).
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the mapping is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Value for one id (XGI `stat[idx]`), if present.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.inner.get(id)
    }
}

impl<T: PartialEq> Stat<T> {
    /// XGI `.filterby('stat', value)` — ids whose value equals `value`, in order.
    pub fn filterby(&self, value: &T) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(_, v)| *v == value)
            .map(|(k, _)| k.clone())
            .collect()
    }
}

impl<T: PartialOrd> Stat<T> {
    /// Ids whose value is strictly greater than `value` (XGI `filterby(..., mode="gt")`).
    pub fn filterby_gt(&self, value: &T) -> Vec<String> {
        self.inner
            .iter()
            .filter(|(_, v)| *v > value)
            .map(|(k, _)| k.clone())
            .collect()
    }
}
