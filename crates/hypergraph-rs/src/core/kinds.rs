//! The two node kinds and membership edge type for the bipartite representation.

use serde::{Deserialize, Serialize};

/// The two roles in the bipartite structure.
///
/// `Agent` nodes are the hypergraph's nodes (entities). `Hyperedge` nodes
/// are the hypergraph's edges — their members are exactly their `Agent`
/// neighbors in the bipartite graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeKind<N, E> {
    /// An entity node (XGI "node"). Carries its own attributes.
    Agent(N),
    /// A hyperedge node (XGI "edge"). The members of the hyperedge are
    /// exactly the Agent neighbors.
    Hyperedge(E),
}

/// A membership edge in the bipartite graph: Agent <-> Hyperedge.
///
/// Carries per-membership attributes (XGI's edge-member attrs).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MembershipEdge<M> {
    /// The per-membership data (e.g., role, strength, visibility).
    pub member_data: M,
}

/// The role of a node within a directed hyperedge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiRole {
    /// A sender: the node is in the edge's tail.
    Tail,
    /// A receiver: the node is in the edge's head.
    Head,
}

/// A directed membership edge in the bipartite graph: always oriented
/// Agent -> Hyperedge, with `role` distinguishing tail (sender) from head
/// (receiver).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiMembershipEdge<M> {
    /// The per-membership data.
    pub member_data: M,
    /// Whether the agent is a sender (`Tail`) or receiver (`Head`) of the edge.
    pub role: DiRole,
}
