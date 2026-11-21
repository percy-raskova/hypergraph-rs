//! Give each membership a value: how many people of each group belong to each of two
//! overlapping bodies.
//!
//! Run it with `cargo run -p hypergraph-rs --example membership_data`.

use hypergraph_rs::Hypergraph;

fn main() {
    // The third type is the value a membership carries. Here it is a head count.
    let mut h: Hypergraph<(), (), u64> = Hypergraph::new();

    // Nodes are groups of people. Edges are the bodies they sit in. The two bodies overlap
    // in the "teachers" group.
    h.add_node_to_edge_with_data("council", "teachers", 3)
        .unwrap();
    h.add_node_to_edge_with_data("council", "nurses", 2)
        .unwrap();
    h.add_node_to_edge_with_data("union", "teachers", 40)
        .unwrap();
    h.add_node_to_edge_with_data("union", "drivers", 25)
        .unwrap();

    // A value can be changed later. A new membership starts at the default, 0.
    h.set_member_data("council", "nurses", 4).unwrap();
    h.add_node_to_edge("council", "drivers").unwrap();
    assert_eq!(h.member_data("council", "drivers"), Some(&0));

    for body in h.edge_ids() {
        let seats = h.members_with_data(&body).unwrap();
        let total: u64 = seats.iter().map(|(_, count)| **count).sum();
        println!("{body}: {seats:?}, {total} people in all");
    }

    let council: u64 = h
        .members_with_data("council")
        .unwrap()
        .iter()
        .map(|(_, count)| **count)
        .sum();
    let union: u64 = h
        .members_with_data("union")
        .unwrap()
        .iter()
        .map(|(_, count)| **count)
        .sum();
    assert_eq!(council, 3 + 4);
    assert_eq!(union, 40 + 25);

    // The same group holds a different value in each body.
    assert_eq!(h.member_data("council", "teachers"), Some(&3));
    assert_eq!(h.member_data("union", "teachers"), Some(&40));
}
