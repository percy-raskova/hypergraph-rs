//! Unit-hop shortest paths on a hypergraph (`xgi/algorithms/shortest_path.py`).
//!
//! XGI's Dijkstra uses a fixed increment of 1 per hop, i.e. BFS levels over the
//! node-neighbour relation. We hand-roll that BFS (unit-weight exact parity;
//! keeps the algorithm independent of the rustworkx-core `dijkstra` return type).

use std::collections::VecDeque;

use indexmap::IndexMap;

use crate::Hypergraph;

/// Distances from `source` to every node (unit hops). Unreachable nodes get
/// `f64::INFINITY`; `source` gets `0`. Keys in `node_ids()` order. `None` if
/// `source` is absent. XGI parity: `single_source_shortest_path_length`.
pub fn single_source_shortest_path_length<N, E, M>(
    h: &Hypergraph<N, E, M>,
    source: &str,
) -> Option<IndexMap<String, f64>> {
    if !h.has_node(source) {
        return None;
    }
    let mut dist: IndexMap<String, f64> = IndexMap::new();
    for n in h.node_ids() {
        dist.insert(n, f64::INFINITY);
    }
    dist.insert(source.to_string(), 0.0);
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(source.to_string());
    // Bounded: each node is enqueued at most once (distance set from INF once).
    while let Some(cur) = queue.pop_front() {
        let cur_d = dist[&cur];
        for nb in h.node_neighbors(&cur).unwrap_or_default() {
            if dist[&nb].is_infinite() {
                dist.insert(nb.clone(), cur_d + 1.0);
                queue.push_back(nb);
            }
        }
    }
    Some(dist)
}

/// Distances from every source. XGI parity: `shortest_path_length`.
pub fn shortest_path_length<N, E, M>(
    h: &Hypergraph<N, E, M>,
) -> Vec<(String, IndexMap<String, f64>)> {
    h.node_ids()
        .into_iter()
        .filter_map(|src| single_source_shortest_path_length(h, &src).map(|d| (src, d)))
        .collect()
}
