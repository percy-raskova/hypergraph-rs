//! Connected components of a hypergraph (`xgi/algorithms/connected.py`).
//!
//! Components are extracted by a deterministic node-neighbour BFS (III.7:
//! ordered by `node_ids()`), while `number_connected_components` delegates the
//! count to `rustworkx_core::connectivity` on the 1-skeleton — a genuine
//! rustworkx-core plugin reuse. A test asserts the two agree.

use std::collections::{HashSet, VecDeque};

use rustworkx_core::connectivity::number_connected_components as rx_ncc;

use crate::Hypergraph;

/// BFS the component containing `source`, returning its nodes in `node_ids()`
/// order. Assumes `source` exists.
fn bfs_component<N, E, M>(h: &Hypergraph<N, E, M>, source: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    seen.insert(source.to_string());
    queue.push_back(source.to_string());
    // Bounded: each node enters the queue at most once (guarded by `seen`),
    // so the loop runs at most `num_nodes` times.
    while let Some(cur) = queue.pop_front() {
        for nb in h.node_neighbors(&cur).unwrap_or_default() {
            if seen.insert(nb.clone()) {
                queue.push_back(nb);
            }
        }
    }
    seen
}

/// All connected components, each a node list in `node_ids()` order, the
/// components themselves ordered by their first-seen node.
/// XGI parity: `xgi.connected_components(H)` (`connected.py:48-82`).
pub fn connected_components<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<Vec<String>> {
    let node_ids = h.node_ids();
    let mut seen: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for v in &node_ids {
        if !seen.contains(v) {
            let comp = bfs_component(h, v);
            seen.extend(comp.iter().cloned());
            // Emit in node_ids() order for determinism.
            let ordered: Vec<String> =
                node_ids.iter().filter(|n| comp.contains(*n)).cloned().collect();
            components.push(ordered);
        }
    }
    components
}

/// Whether the hypergraph is connected (one component; empty ⇒ trivially so is
/// undefined — matches XGI, which indexes `list(H.nodes)[0]`, so guard empty).
/// XGI parity: `xgi.is_connected(H)` (`connected.py:16-45`).
pub fn is_connected<N, E, M>(h: &Hypergraph<N, E, M>) -> bool {
    if h.num_nodes() == 0 {
        return false;
    }
    number_connected_components(h) == 1
}

/// Number of connected components, delegated to rustworkx-core on the skeleton.
/// XGI parity: `xgi.number_connected_components(H)` (`connected.py:85-121`).
pub fn number_connected_components<N, E, M>(h: &Hypergraph<N, E, M>) -> usize {
    let g = h.skeleton();
    rx_ncc(&g)
}

/// The connected component containing node `n`, in `node_ids()` order, or
/// `None` if the node is absent. XGI parity: `node_connected_component`
/// (`connected.py:154-188`) — XGI raises; we return `None`.
pub fn node_connected_component<N, E, M>(h: &Hypergraph<N, E, M>, n: &str) -> Option<Vec<String>> {
    if !h.has_node(n) {
        return None;
    }
    let comp = bfs_component(h, n);
    Some(h.node_ids().into_iter().filter(|x| comp.contains(x)).collect())
}

/// The largest connected component (ties broken by first-seen order).
/// XGI parity: `largest_connected_component` (`connected.py:124-151`).
pub fn largest_connected_component<N, E, M>(h: &Hypergraph<N, E, M>) -> Vec<String> {
    connected_components(h)
        .into_iter()
        .max_by_key(|c| c.len())
        .unwrap_or_default()
}
