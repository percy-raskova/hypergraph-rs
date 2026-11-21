//! Build a small hypergraph, read it back, and compute one statistic.
//!
//! Run it with `cargo run -p hypergraph-rs --example quickstart`.

use hypergraph_rs::algorithms::connected::number_connected_components;
use hypergraph_rs::Hypergraph;

fn main() {
    // Nodes, edges and memberships carry no value here, so all three types are `()`.
    let mut h: Hypergraph<(), (), ()> = Hypergraph::new();

    // An edge joins any number of nodes. Nodes that do not exist yet are created.
    h.add_edge(
        vec!["ana".into(), "bo".into(), "cy".into()],
        Some("club".into()),
        (),
    )
    .unwrap();
    h.add_edge(
        vec!["bo".into(), "cy".into(), "di".into(), "ed".into()],
        Some("choir".into()),
        (),
    )
    .unwrap();
    h.add_edge(vec!["fay".into(), "gus".into()], Some("pair".into()), ())
        .unwrap();

    // Listings keep the order in which things were added.
    println!("nodes: {:?}", h.node_ids());
    println!("edges: {:?}", h.edge_ids());
    println!("members of choir: {:?}", h.members("choir").unwrap());
    println!("bo belongs to: {:?}", h.memberships("bo").unwrap());
    assert_eq!(h.node_ids(), ["ana", "bo", "cy", "di", "ed", "fay", "gus"]);
    assert_eq!(h.members("choir").unwrap(), ["bo", "cy", "di", "ed"]);
    assert_eq!(h.memberships("bo").unwrap(), ["club", "choir"]);

    // One statistic: the number of connected components. "pair" shares no member with the
    // other two edges, so there are two.
    let parts = number_connected_components(&h);
    println!("connected components: {parts}");
    assert_eq!(parts, 2);
}
