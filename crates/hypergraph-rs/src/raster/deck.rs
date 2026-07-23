//! `mulberry32` deck RNG port + `buildWorld` fixture + o(C) organization degree
//! (`topology-deck.html:120-121, 129-179, 196-208`). Task 6.
//!
//! `mulberry32` is a fixture-only RNG that never enters the general
//! layout/community API (`Pcg64Mcg` remains the crate's canonical
//! generator) — see the plan's CRITICAL REFERENCE CORRECTION on
//! mulberry32 vs Pcg64Mcg.

use std::collections::HashSet;

use serde_json::Value;

use crate::algorithms::connected::connected_components;
use crate::Hypergraph;

use super::algebra::{deck_industries, Industry};

/// Byte-exact Rust port of the deck's `mulberry32(a)` closure
/// (`topology-deck.html:120-121`). State is a wrapping `u32`; `next()`
/// reproduces the JS `Math.imul`/`>>>`/`|0` arithmetic bit-for-bit (pure
/// wrapping-integer arithmetic — no float-tolerance concern, user CLAUDE.md
/// §4).
#[derive(Debug, Clone)]
pub struct Mulberry32 {
    a: u32,
}

impl Mulberry32 {
    /// Seed the generator (the deck's `mulberry32(seed)` call).
    pub fn new(seed: u32) -> Self {
        Self { a: seed }
    }

    /// Advance the generator and return the next `[0,1)` draw. Named `next`
    /// (not `Iterator`) to match the Step-1 golden-vector test contract and
    /// the deck's own `rnd()` call shape; it is a fixture RNG, not a
    /// general-purpose iterator.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6D2B79F5);
        let mut t = self.a;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4294967296.0
    }
}

/// One of the 28 seeded agents (`topology-deck.html:129-134`).
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub id: usize,
    pub bloc: u8,
    pub wealth: f64,
    pub subsistence: f64,
}

/// An organizing community: a member set plus internal cohesion links
/// (`topology-deck.html:135-143`).
#[derive(Debug, Clone, PartialEq)]
pub struct Community {
    pub name: String,
    pub members: Vec<usize>,
    pub links: Vec<(usize, usize)>,
}

/// A multilateral extraction hyperedge: deduped periphery tails -> one
/// capital head (`topology-deck.html:145-149`).
#[derive(Debug, Clone, PartialEq)]
pub struct Extraction {
    pub tails: Vec<usize>,
    pub head: usize,
    pub mass: f64,
}

/// The full seeded deck fixture (`topology-deck.html:151-179`'s `buildWorld`
/// return value), bundling the 13-industry table alongside agents/
/// communities/extraction so `do_tick` (Task 10) can mutate agents and
/// industries together (Finding 1).
#[derive(Debug, Clone, PartialEq)]
pub struct DeckWorld {
    pub seed: u32,
    pub agents: Vec<Agent>,
    pub communities: Vec<Community>,
    pub extraction: Vec<Extraction>,
    pub industries: Vec<Industry>,
    pub repression: f64,
    pub tick: u32,
    pub zoom: u8,
}

/// Agent ids of the given bloc, in `agents` order.
fn agents_in_bloc(agents: &[Agent], bloc: u8) -> Vec<usize> {
    agents
        .iter()
        .filter(|a| a.bloc == bloc)
        .map(|a| a.id)
        .collect()
}

/// The deck's `mk(pool,size,name)`: seeded Fisher-Yates shuffle of the full
/// pool, then take the first `size` (`topology-deck.html:137-138`). Bounded:
/// `pool.len()` is at most 14 (the largest bloc).
fn shuffle_take(rnd: &mut Mulberry32, pool: Vec<usize>, size: usize, name: &str) -> Community {
    let mut m = pool;
    for i in (1..m.len()).rev() {
        let j = (rnd.next() * (i as f64 + 1.0)) as usize;
        m.swap(i, j);
    }
    m.truncate(size);
    Community {
        name: name.to_string(),
        members: m,
        links: Vec::new(),
    }
}

/// Port `buildWorld(seed)` (`topology-deck.html:129-179`): 28 agents in 3
/// blocs, 4 organizing communities (capital δ fully cohesive, the rest
/// atomized with 1 link), 4 extraction hyperedges, and the 13-industry
/// table.
pub fn build_world(seed: u32) -> DeckWorld {
    let mut rnd = Mulberry32::new(seed);
    let mut agents = Vec::new();
    let blocs: [(usize, u8, f64); 3] = [(5, 0, 8.5), (9, 1, 5.6), (14, 2, 2.1)];
    let mut id = 0usize;
    for (n, bloc, w) in blocs {
        for _ in 0..n {
            agents.push(Agent {
                id,
                bloc,
                wealth: w + rnd.next() * 1.4 - 0.7,
                subsistence: 3.0,
            });
            id += 1;
        }
    }

    let mut communities = vec![
        shuffle_take(&mut rnd, agents_in_bloc(&agents, 2), 6, "cell \u{3b1}"),
        shuffle_take(&mut rnd, agents_in_bloc(&agents, 2), 6, "cell \u{3b2}"),
        shuffle_take(&mut rnd, agents_in_bloc(&agents, 1), 6, "union \u{3b3}"),
        shuffle_take(&mut rnd, agents_in_bloc(&agents, 0), 4, "capital \u{3b4}"),
    ];

    for c in communities.iter_mut() {
        let full = c.name == "capital \u{3b4}";
        let want = if full { c.members.len() - 1 } else { 1 };
        for i in 0..want {
            c.links.push((c.members[i], c.members[i + 1]));
        }
    }

    let per = agents_in_bloc(&agents, 2);
    let cap = agents_in_bloc(&agents, 0);
    let mut extraction = Vec::new();
    for e in 0..4usize {
        let mut tails_raw = Vec::new();
        for _ in 0..4 {
            let idx = (rnd.next() * per.len() as f64) as usize;
            tails_raw.push(per[idx]);
        }
        // JS `new Set([...])` dedupes preserving first-seen order.
        let mut seen: HashSet<usize> = HashSet::new();
        let mut tails = Vec::new();
        for t in tails_raw {
            if seen.insert(t) {
                tails.push(t);
            }
        }
        let head = cap[e % cap.len()];
        let mass = 1.4 + rnd.next() * 1.2;
        extraction.push(Extraction { tails, head, mass });
    }

    DeckWorld {
        seed,
        agents,
        communities,
        extraction,
        industries: deck_industries(),
        repression: 2.6,
        tick: 0,
        zoom: 0,
    }
}

/// Port of `doTick()` (`topology-deck.html:243-252`, Task 10). Order
/// verbatim (Finding 1): (1) wealth extraction over `world.extraction` —
/// `take = mass·0.12`; each tail loses `take/|tails|` under a
/// `Math.max(0.2, …)` floor; the head gains `take·0.8`; every bloc-1
/// (core-labor) agent gains `take·0.2/9`; THEN (2) c-accumulation — each
/// industry's `c ×= 1.035` if `lane>=2` else `×= 1.015` (`v,e` untouched).
/// Does not itself call `transform` — callers compute readouts from the
/// mutated `world.industries` afterward (the crate's `transform` is a pure
/// fn with no `Industry`-side effects to "re-run", unlike the deck's
/// closure-captured `W.industries`). Bounded:
/// `world.extraction.len() * (max tails + agents.len()) + world.industries.len()`.
pub fn do_tick(world: &mut DeckWorld) {
    world.tick += 1;

    for e in &world.extraction {
        let take = e.mass * 0.12;
        let per_tail = take / e.tails.len() as f64;
        for &t in &e.tails {
            world.agents[t].wealth = (world.agents[t].wealth - per_tail).max(0.2);
        }
        world.agents[e.head].wealth += take * 0.8;
        for a in world.agents.iter_mut().filter(|a| a.bloc == 1) {
            a.wealth += take * 0.2 / 9.0;
        }
    }

    for ind in world.industries.iter_mut() {
        ind.c *= if ind.lane >= 2 { 1.035 } else { 1.015 };
    }
}

/// The shared `DeckWorld -> &Hypergraph` adapter (Finding 6): 28 agents
/// become nodes keyed by their id, each community link becomes a 2-node
/// edge. The single bridge from `DeckWorld` to the shipped `&Hypergraph`
/// APIs (`subhypergraph`, `connected_components`, Task 7's
/// `normalized_laplacian`), none of which take `DeckWorld` directly.
///
/// Task 7 deviation: at seed 1917 only 10 of the 28 agents ever appear as a
/// community-link endpoint (cell α/β and union γ each contribute one link =
/// 2 agents; capital δ's chain covers all 4 of its members; several agents
/// belong to no community at all) — feeding that graph straight into
/// `normalized_laplacian` (Task 7) trips `SpectralError::IsolatedNode` for
/// every uncovered agent, since the normalized Laplacian divides by node
/// degree. Rather than fork a second `DeckWorld -> Hypergraph` adapter
/// (violating DRY / the "shared adapter" contract this fn documents), any
/// agent left with degree 0 after the community-link pass gets a singleton
/// self-edge (`add_edge(vec![id])`), giving it degree 1 with zero coupling
/// to any other node (`L[i,i] = 0`, all other entries in its row/col are 0)
/// — an inert placeholder position for the eigensolver, not a real semantic
/// edge. This does not change the Laplacian block for already-connected
/// agents.
pub fn deck_hypergraph(w: &DeckWorld) -> Hypergraph<Value, Value, Value> {
    let mut h: Hypergraph<Value, Value, Value> = Hypergraph::new();
    for a in &w.agents {
        h.add_node(&a.id.to_string(), Value::Null);
    }
    let mut degree = vec![0u32; w.agents.len()];
    for c in &w.communities {
        for (a, b) in &c.links {
            let _ = h.add_edge(vec![a.to_string(), b.to_string()], None, Value::Null);
            degree[*a] += 1;
            degree[*b] += 1;
        }
    }
    for a in &w.agents {
        if degree[a.id] == 0 {
            let _ = h.add_edge(vec![a.id.to_string()], None, Value::Null);
        }
    }
    h
}

/// Organization degree `o(C) = 1` if `|C|<2` else `1 - (|Π₀(C)|-1)/(|C|-1)`
/// (`topology-deck.html:206-208`). Built as an isolated per-community graph
/// (own members as nodes, own links as edges) rather than a slice of the
/// shared `deck_hypergraph`: the deck's `orgOf` walks only `c.links`, and
/// the periphery communities draw from an overlapping pool (`cell α`/`cell
/// β` both shuffle the full 14-agent bloc), so filtering a shared multi-
/// community graph by node membership alone (`subhypergraph`) could admit
/// another community's edge whose endpoints happen to fall inside this
/// community's member set — this isolated construction is immune to that
/// and reproduces `orgOf` exactly. Reuses `connected_components`
/// (`algorithms::connected`) for `|Π₀(C)|`, per Task 6.
pub fn org_degree(c: &Community) -> f64 {
    let n = c.members.len();
    if n < 2 {
        return 1.0;
    }
    let mut h: Hypergraph<Value, Value, Value> = Hypergraph::new();
    for m in &c.members {
        h.add_node(&m.to_string(), Value::Null);
    }
    for (a, b) in &c.links {
        let _ = h.add_edge(vec![a.to_string(), b.to_string()], None, Value::Null);
    }
    let comps = connected_components(&h).len();
    1.0 - (comps as f64 - 1.0) / (n as f64 - 1.0)
}

/// Path-halving union-find over `[0, n)`, bounded by `n` unions (Power-of-10
/// rule 2: no loop here is unbounded — `find`'s `while` shrinks the parent
/// chain toward the root every step and terminates once `parent[x] == x`,
/// which every union eventually establishes over a graph of `n` nodes).
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Global Π₀: union all community links and all extraction tails→head over
/// the full 28-agent population (`topology-deck.html:196-204`'s
/// `globalPi0`), hand-rolled (not `algorithms::connected_components`,
/// which walks a `&Hypergraph` — `DeckWorld`'s extraction hyperedges are
/// directed tails→head, not the crate's undirected membership shape).
pub fn global_pi0(w: &DeckWorld) -> usize {
    let mut uf = UnionFind::new(w.agents.len());
    for c in &w.communities {
        for &(a, b) in &c.links {
            uf.union(a, b);
        }
    }
    for e in &w.extraction {
        for &t in &e.tails {
            uf.union(t, e.head);
        }
    }
    let mut roots: HashSet<usize> = HashSet::new();
    for a in &w.agents {
        roots.insert(uf.find(a.id));
    }
    roots.len()
}

/// `sigmoid(x) = 1/(1+e^-x)` (`topology-deck.html:122`). Routes through
/// `exp` — not byte-exact across libm implementations (Finding 3, user
/// CLAUDE.md §4); callers golden-test it tolerance-bounded (`< 1e-12`).
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Mean wealth of the bloc-2 (periphery) agents, in `agents` order
/// (`topology-deck.html:439-440`'s `pw`). Pure arithmetic — byte-exact.
pub fn periphery_mean_wealth(w: &DeckWorld) -> f64 {
    let per: Vec<&Agent> = w.agents.iter().filter(|a| a.bloc == 2).collect();
    per.iter().map(|a| a.wealth).sum::<f64>() / per.len() as f64
}

/// `psa = sigmoid(pw − per[0].subsistence)` (`topology-deck.html:443`) — the
/// periphery's assimilation survival probability. Tolerance-bounded golden
/// (routes through `sigmoid`'s `exp`, Finding 3), NOT byte-exact.
pub fn psa(w: &DeckWorld) -> f64 {
    let subsistence = w
        .agents
        .iter()
        .find(|a| a.bloc == 2)
        .map_or(3.0, |a| a.subsistence);
    sigmoid(periphery_mean_wealth(w) - subsistence)
}

/// `psr = min(1, (org(cell α)+org(cell β))/2 / (repression/2.6·0.9))`
/// (`topology-deck.html:444`) — the deck's *calibrated* rupture-survival
/// form; diverges from the textbook `P(S|R)=Org/Repression` (see OPEN
/// DECISION D-R7). Pure division — byte-exact. `communities[0]`/`[1]` are
/// `cell α`/`cell β` by `build_world`'s push order.
pub fn psr(w: &DeckWorld) -> f64 {
    let org0 = org_degree(&w.communities[0]);
    let org1 = org_degree(&w.communities[1]);
    let mean_org = (org0 + org1) / 2.0;
    (mean_org / (w.repression / 2.6 * 0.9)).min(1.0)
}
