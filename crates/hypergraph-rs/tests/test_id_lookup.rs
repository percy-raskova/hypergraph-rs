//! Id lookups: order, index reuse, agreement after many mutations, large edges.

use hypergraph_rs::{DiHypergraph, DiRole, Hypergraph};
use std::time::Instant;

type H = Hypergraph<(), (), ()>;
type D = DiHypergraph<(), (), ()>;

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

/// A fixed linear congruential generator, so the sequence never changes.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

fn node_name(i: u64) -> String {
    format!("n{i}")
}

// ---------------------------------------------------------------- Hypergraph

#[test]
fn members_and_memberships_keep_insertion_order_through_removals() {
    let mut h = H::new();
    h.add_edge(s(&["a", "b", "c", "d"]), Some("e".into()), ())
        .unwrap();
    h.remove_node("b", false, false).unwrap();
    assert_eq!(h.members("e").unwrap(), s(&["a", "c", "d"]));

    let mut h = H::new();
    for e in ["e1", "e2", "e3"] {
        h.add_edge(s(&["x"]), Some(e.into()), ()).unwrap();
    }
    h.remove_edge("e2").unwrap();
    assert_eq!(h.memberships("x").unwrap(), s(&["e1", "e3"]));
}

#[test]
fn a_reused_index_names_the_new_node() {
    let mut h = H::new();
    h.add_edge(s(&["a", "b", "c"]), Some("e".into()), ())
        .unwrap();
    h.remove_node("b", false, false).unwrap();
    h.add_node("d", ());
    h.add_node_to_edge("e", "d").unwrap();
    assert_eq!(h.members("e").unwrap(), s(&["a", "c", "d"]));
    assert_eq!(h.memberships("d").unwrap(), s(&["e"]));
    assert!(h.memberships("b").is_none());
}

#[test]
fn a_reused_index_names_the_new_edge() {
    let mut h = H::new();
    h.add_edge(s(&["x"]), Some("e1".into()), ()).unwrap();
    h.add_edge(s(&["x"]), Some("e2".into()), ()).unwrap();
    h.remove_edge("e1").unwrap();
    h.add_edge(s(&["x"]), Some("e9".into()), ()).unwrap();
    assert_eq!(h.memberships("x").unwrap(), s(&["e2", "e9"]));
}

fn check_agreement_h(h: &H) {
    let nodes = h.node_ids();
    let edges = h.edge_ids();
    for e in &edges {
        for n in h.members(e).unwrap() {
            assert!(nodes.contains(&n), "member {n} of {e} is not a node");
            assert!(h.memberships(&n).unwrap().contains(e));
        }
    }
    for n in &nodes {
        for e in h.memberships(n).unwrap() {
            assert!(edges.contains(&e), "membership {e} of {n} is not an edge");
            assert!(h.members(&e).unwrap().contains(n));
        }
    }
    assert_eq!(h.num_nodes(), nodes.len());
    assert_eq!(h.num_edges(), edges.len());
}

#[test]
fn lookups_agree_with_the_listings_after_many_mutations() {
    let mut rng = Lcg(20261121);
    let mut h = H::new();
    let mut counter = 0u64;
    for step in 1..=3000u32 {
        match rng.next(5) {
            0 => {
                let k = 1 + rng.next(5);
                let mut members: Vec<String> = Vec::new();
                for _ in 0..k {
                    let m = node_name(rng.next(60));
                    if !members.contains(&m) {
                        members.push(m);
                    }
                }
                counter += 1;
                h.add_edge(members, Some(format!("e{counter}")), ())
                    .unwrap();
            }
            1 => {
                let _ = h.remove_node(&node_name(rng.next(60)), false, false);
            }
            2 => {
                let ids = h.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = h.remove_edge(&e);
                }
            }
            3 => {
                let ids = h.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = h.add_node_to_edge(&e, &node_name(rng.next(60)));
                }
            }
            _ => {
                let ids = h.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = h.remove_node_from_edge(&e, &node_name(rng.next(60)), true);
                }
            }
        }
        if step % 500 == 0 {
            h.clear_edges();
        }
        if step % 100 == 0 {
            check_agreement_h(&h);
        }
    }
}

#[test]
fn a_large_edge_is_read_without_scanning() {
    let members: Vec<String> = (0..100_000).map(|i| format!("n{i}")).collect();
    let mut h = H::new();
    h.add_edge(members, Some("big".into()), ()).unwrap();

    let start = Instant::now();
    let read = h.members("big").unwrap();
    assert_eq!(read.len(), 100_000);
    assert_eq!(read[0], "n0");
    assert_eq!(read[99_999], "n99999");
    for i in 0..1000 {
        assert_eq!(h.memberships(&format!("n{i}")).unwrap(), s(&["big"]));
    }
    // The bound is about a hundred times the expected time; the scan this
    // replaces needs billions of comparisons.
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
}

// -------------------------------------------------------------- DiHypergraph

#[test]
fn di_members_and_memberships_keep_insertion_order_through_removals() {
    let mut d = D::new();
    d.add_edge(s(&["a", "b"]), s(&["c", "d"]), Some("e".into()), ())
        .unwrap();
    d.remove_node("b", false, false).unwrap();
    assert_eq!(d.members("e").unwrap(), s(&["a", "c", "d"]));
    assert_eq!(d.dimembers("e").unwrap(), (s(&["a"]), s(&["c", "d"])));

    let mut d = D::new();
    for e in ["e1", "e2", "e3"] {
        d.add_edge(s(&["x"]), s(&["y"]), Some(e.into()), ())
            .unwrap();
    }
    d.remove_edge("e2").unwrap();
    assert_eq!(d.memberships("x").unwrap(), s(&["e1", "e3"]));
}

#[test]
fn di_a_reused_index_names_the_new_node() {
    let mut d = D::new();
    d.add_edge(s(&["a", "b"]), s(&["c"]), Some("e".into()), ())
        .unwrap();
    d.remove_node("b", false, false).unwrap();
    d.add_node("d", ());
    d.add_node_to_edge("e", "d", DiRole::Head).unwrap();
    assert_eq!(d.members("e").unwrap(), s(&["a", "c", "d"]));
    assert_eq!(d.dimembers("e").unwrap(), (s(&["a"]), s(&["c", "d"])));
    assert_eq!(d.memberships("d").unwrap(), s(&["e"]));
    assert!(d.memberships("b").is_none());
}

#[test]
fn di_a_reused_index_names_the_new_edge() {
    let mut d = D::new();
    d.add_edge(s(&["x"]), s(&["y"]), Some("e1".into()), ())
        .unwrap();
    d.add_edge(s(&["x"]), s(&["y"]), Some("e2".into()), ())
        .unwrap();
    d.remove_edge("e1").unwrap();
    d.add_edge(s(&["x"]), s(&["y"]), Some("e9".into()), ())
        .unwrap();
    assert_eq!(d.memberships("x").unwrap(), s(&["e2", "e9"]));
    let (inn, out) = d.dimemberships("x").unwrap();
    assert!(inn.is_empty());
    assert_eq!(out, s(&["e2", "e9"]));
}

fn check_agreement_d(d: &D) {
    let nodes = d.node_ids();
    let edges = d.edge_ids();
    for e in &edges {
        for n in d.members(e).unwrap() {
            assert!(nodes.contains(&n), "member {n} of {e} is not a node");
            assert!(d.memberships(&n).unwrap().contains(e));
        }
    }
    for n in &nodes {
        for e in d.memberships(n).unwrap() {
            assert!(edges.contains(&e), "membership {e} of {n} is not an edge");
            assert!(d.members(&e).unwrap().contains(n));
        }
    }
    assert_eq!(d.num_nodes(), nodes.len());
    assert_eq!(d.num_edges(), edges.len());
}

#[test]
fn di_lookups_agree_with_the_listings_after_many_mutations() {
    let mut rng = Lcg(20261121);
    let mut d = D::new();
    let mut counter = 0u64;
    let role = |r: u64| if r == 0 { DiRole::Tail } else { DiRole::Head };
    for step in 1..=3000u32 {
        match rng.next(5) {
            0 => {
                let mut tail: Vec<String> = Vec::new();
                let mut head: Vec<String> = Vec::new();
                for _ in 0..(1 + rng.next(5)) {
                    let m = node_name(rng.next(60));
                    let target = if rng.next(2) == 0 {
                        &mut tail
                    } else {
                        &mut head
                    };
                    if !target.contains(&m) {
                        target.push(m);
                    }
                }
                counter += 1;
                d.add_edge(tail, head, Some(format!("e{counter}")), ())
                    .unwrap();
            }
            1 => {
                let _ = d.remove_node(&node_name(rng.next(60)), false, false);
            }
            2 => {
                let ids = d.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = d.remove_edge(&e);
                }
            }
            3 => {
                let ids = d.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = d.add_node_to_edge(&e, &node_name(rng.next(60)), role(rng.next(2)));
                }
            }
            _ => {
                let ids = d.edge_ids();
                if !ids.is_empty() {
                    let e = ids[rng.next(ids.len() as u64) as usize].clone();
                    let _ = d.remove_node_from_edge(
                        &e,
                        &node_name(rng.next(60)),
                        role(rng.next(2)),
                        true,
                    );
                }
            }
        }
        if step % 500 == 0 {
            d.clear();
        }
        if step % 100 == 0 {
            check_agreement_d(&d);
        }
    }
}

#[test]
fn di_a_large_edge_is_read_without_scanning() {
    let tail: Vec<String> = (0..50_000).map(|i| format!("n{i}")).collect();
    let head: Vec<String> = (50_000..100_000).map(|i| format!("n{i}")).collect();
    let mut d = D::new();
    d.add_edge(tail, head, Some("big".into()), ()).unwrap();

    let start = Instant::now();
    let (t, h) = d.dimembers("big").unwrap();
    assert_eq!(t.len(), 50_000);
    assert_eq!(h.len(), 50_000);
    assert_eq!(t[0], "n0");
    assert_eq!(h[49_999], "n99999");
    let all = d.members("big").unwrap();
    assert_eq!(all.len(), 100_000);
    assert_eq!(all[0], "n0");
    assert_eq!(all[99_999], "n99999");
    for i in 0..1000 {
        assert_eq!(d.memberships(&format!("n{i}")).unwrap(), s(&["big"]));
    }
    // The bound is about a hundred times the expected time; the scan this
    // replaces needs billions of comparisons.
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
}
