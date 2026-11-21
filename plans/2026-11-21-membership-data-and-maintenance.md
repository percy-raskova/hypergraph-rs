# Membership data, fast lookup and maintenance: implementation plan

> **For agentic workers:** implement this plan task by task. A fresh implementer builds one task, a
> fresh reviewer checks it, then the next begins.

**Goal:** Give every membership a value that can be read and written (issue #2), make `members()` and
`memberships()` constant time for each neighbour (issue #3), refresh the dependencies, and make the
crate usable on its own.

**Architecture:** The two id maps of `Hypergraph` and `DiHypergraph` become one small type, `IdMap`,
that keeps the insertion-ordered forward map and a reverse map in step, so no call site can update
one and forget the other. Membership data already has a slot (`MembershipEdge::member_data`); this
adds the public read and write around it. Nothing in the XGI-compatible surface changes.

**Tech stack:** Rust 1.91.1 (pinned), MSRV 1.85, `rustworkx-core` 0.18 (petgraph is reached through
it, never directly), `indexmap` 2, std `HashMap`.

**Spec:** issues #2 and #3 of this repository, and the owner's direction of 21 November 2026: fix
them here, pass the dependency and security scans, and make this a well-maintained library that can
be used on its own.

## Global Constraints

- Work only in this repository, on the branch `feat/membership-data-and-maintenance`. Never push.
- **Author identity.** This checkout has no identity of its own. Every commit must carry the
  identity this repository's history uses. Take it from the sibling checkout and pass it on every
  commit:

  ```sh
  N=$(git -C /media/user/dev-ssd/worktrees/households-homes-read/babylon config --get user.name)
  E=$(git -C /media/user/dev-ssd/worktrees/households-homes-read/babylon config --get user.email)
  GIT_AUTHOR_NAME="$N" GIT_AUTHOR_EMAIL="$E" GIT_COMMITTER_NAME="$N" GIT_COMMITTER_EMAIL="$E" \
    git commit -q -F <message file>
  ```

  After every commit check that
  `git log origin/main..HEAD --format='%an|%ae%n%cn|%ce' | sort -u` prints exactly one line and that
  it equals `git log -1 --format='%an|%ae' dc1c06a`. Never run `git config`.
- The public API only grows. No existing public signature changes, and no existing test's expected
  value changes. `members()` and `memberships()` return ids in the same order as today.
- No `unsafe`. No new dependency. No result may depend on the iteration order of a `HashMap`: it is
  used for point lookups only.
- MSRV 1.85: no library feature newer than that.
- The XGI-compatible formats (`readwrite`), the Python package and the WASM bindings are not
  changed.
- Tests live as files directly under `crates/hypergraph-rs/tests/`.
- One cargo process at a time. Never pass `-j` or `--jobs`.
- The scoped gate, one leg at a time, before the last commit of a task:
  1. `cargo fmt --all -- --check`
  2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
  3. `cargo test --workspace --locked`
  4. `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`
  5. `cargo deny check`

## Review Focus

1. **An index that is used again.** petgraph hands a removed node's index to the next node added.
   After `remove_node("b")` and `add_node("d")`, a lookup by index must name `d`, never `b`.
2. **Every mutator keeps both directions of `IdMap` in step:** `add_node`, `add_edge`,
   `add_node_to_edge`, `remove_node`, `remove_edge`, `remove_node_from_edge` (which can remove an
   emptied edge), `clear`, `clear_edges`, `copy`, and every other place that inserts into or removes
   from the two maps today.
3. **Both stored directions.** `Hypergraph` stores a membership as two directed edges, each with its
   own copy of the data. A write must change both, or a read can return the old value.
4. **A node in the tail and the head of one directed edge** has two memberships with separate data.
5. **A frozen graph** refuses a write of membership data the way it refuses every other mutation.
6. **A member that leaves and returns** starts again from the default value.

## File Structure

| File | Responsibility | Task |
| --- | --- | --- |
| `crates/hypergraph-rs/src/core/idmap.rs` (new) | `IdMap`: id to index and index to id, kept in step | 1 |
| `crates/hypergraph-rs/src/core/hypergraph.rs` | uses `IdMap`; membership data read and write | 1, 2 |
| `crates/hypergraph-rs/src/core/dihypergraph.rs` | uses `IdMap`; membership data read and write by role | 1, 2 |
| `crates/hypergraph-rs/tests/test_id_lookup.rs` (new) | order, index reuse, agreement after many mutations, a large edge | 1 |
| `crates/hypergraph-rs/tests/test_member_data.rs` (new) | defaults, writes, errors, copy, frozen, roles | 2 |
| `Cargo.lock` | refreshed | 3 |
| `Cargo.toml`, crate manifests, `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `crates/hypergraph-rs/src/lib.rs`, `crates/hypergraph-rs/examples/` | usable on its own | 4 |

---

### Task 1: `IdMap`, and lookups that do not scan (#3)

**Files:**
- Create: `crates/hypergraph-rs/src/core/idmap.rs`
- Modify: `crates/hypergraph-rs/src/core/mod.rs`, `core/hypergraph.rs`, `core/dihypergraph.rs`
- Test: `crates/hypergraph-rs/tests/test_id_lookup.rs` (Create)

**Interfaces:**
- Produces, in `core/idmap.rs`, crate-private:

```rust
pub(crate) struct IdMap {
    forward: IndexMap<String, NodeIndex>,  // insertion order: the order every listing uses
    reverse: HashMap<NodeIndex, String>,   // point lookups only, never iterated
}
impl IdMap {
    pub(crate) fn new() -> Self;
    pub(crate) fn id_of(&self, index: NodeIndex) -> Option<&str>;   // constant time
    pub(crate) fn insert(&mut self, id: String, index: NodeIndex);  // both directions
    pub(crate) fn shift_remove(&mut self, id: &str) -> Option<NodeIndex>;  // both directions
    pub(crate) fn clear(&mut self);                                  // both directions
}
```

  plus exactly the read operations of `IndexMap` that the two files use today (`len`, `is_empty`,
  `contains_key`, `get`, indexing by `&str`, `keys`, `values`, `iter`, `IntoIterator for &IdMap`,
  `Clone`), each with the meaning it has today. `forward` and `reverse` are private to the file: no
  other code can change one without the other.
- `Hypergraph::agent_ids`, `Hypergraph::hyperedge_ids`, `DiHypergraph::agent_ids` and
  `DiHypergraph::hyperedge_ids` become `IdMap`. Their call sites keep their meaning.
- `Hypergraph::members`, `Hypergraph::memberships`, and every place in `dihypergraph.rs` that finds
  an id by scanning a map (`dimembers`, `members`, `memberships` and their helpers), use `id_of`.

**Decisions:**
- *A type, not two loose maps:* a second map kept beside the first at eight call sites would be
  forgotten at one of them some day. `IdMap` makes that impossible.
- *`insert` of an id that is present* replaces its index and removes the old index from `reverse`.
- *`insert` of an index that is present under another id* cannot happen while the graph is
  consistent; `debug_assert!` it.
- *No change of order:* `shift_remove` keeps the relative order of the rest, as today.

- [ ] **Step 1: Write the failing tests** in `tests/test_id_lookup.rs`, for `Hypergraph` and, where
  marked, for `DiHypergraph`. Use `Hypergraph<(), (), ()>` and `DiHypergraph<(), (), ()>`.
  - `members_and_memberships_keep_insertion_order_through_removals`: edge `e` with `a, b, c, d`;
    `remove_node("b")`; `members("e") == ["a", "c", "d"]`. Edges `e1, e2, e3` all holding `x`;
    `remove_edge("e2")`; `memberships("x") == ["e1", "e3"]`.
  - `a_reused_index_names_the_new_node`: edge `e` with `a, b, c`; `remove_node("b")`;
    `add_node("d", ())`; `add_node_to_edge("e", "d")`; `members("e") == ["a", "c", "d"]` and
    `memberships("d") == ["e"]`; `memberships("b")` is `None`.
  - `a_reused_index_names_the_new_edge`: node `x` in `e1, e2`; `remove_edge("e1")`; add edge `e9`
    holding `x`; `memberships("x") == ["e2", "e9"]`.
  - `lookups_agree_with_the_listings_after_many_mutations`: 3,000 mutations drawn from a fixed
    linear congruential generator written in the test (seed 20261121): add an edge of one to five
    members out of 60 node names, `remove_node`, `remove_edge`, `add_node_to_edge`,
    `remove_node_from_edge`, and once in 500 `clear_edges`. Errors from removing what is absent are
    ignored. After every 100 mutations: for every edge in `edges()`, each id in `members(e)` is in
    `nodes()` and `memberships(id)` contains `e`; for every node, each id in `memberships(n)` is in
    `edges()` and `members(id)` contains `n`; and `num_nodes() == nodes().len()`,
    `num_edges() == edges().len()`.
  - `a_large_edge_is_read_without_scanning`: one edge of 100,000 members `n0 … n99999`;
    `members` returns 100,000 ids, the first `n0` and the last `n99999`; then `memberships` of the
    first 1,000 nodes. Measure with `std::time::Instant` and assert the whole read took less than
    five seconds. Say in a comment that the bound is about a hundred times the expected time and that
    the scan it replaces needs billions of comparisons.
  - The same five for `DiHypergraph`, with the members split between tail and head, checking
    `dimembers`, `members` and `memberships`.
- [ ] **Step 2: Run them.** `cargo test -p hypergraph-rs --locked --test test_id_lookup`. Expected:
  the order and agreement tests pass already (they pin today's behaviour); the two large-edge tests
  fail on the time bound. If a large-edge test passes before the change, raise its size until the
  old code fails, and say so in the commit.
- [ ] **Step 3: Implement `IdMap`** and use it in both files.
- [ ] **Step 4: Run Step 2's command, then the whole crate:** `cargo test -p hypergraph-rs --locked`.
  Expected: all pass, and every test that existed before passes unchanged.
- [ ] **Step 5: The scoped gate, then commit:**
  `perf(core): find a neighbour's id without scanning every id`, with a body that says what was
  slow, what `IdMap` guarantees and what the tests pin, ending with the line `Closes #3`.

### Task 2: read and write a membership's value (#2)

**Files:**
- Modify: `crates/hypergraph-rs/src/core/hypergraph.rs`, `core/dihypergraph.rs`
- Test: `crates/hypergraph-rs/tests/test_member_data.rs` (Create)

**Interfaces:**
- Produces, on `Hypergraph<N, E, M>`:

```rust
/// The value the membership of `node_id` in `edge_id` carries. `None` when the edge or the node
/// does not exist, or the node is not a member of the edge.
pub fn member_data(&self, edge_id: &str, node_id: &str) -> Option<&M>;

/// Replace that value. Refuses an unknown edge, an unknown node, and a node that is not a member.
pub fn set_member_data(&mut self, edge_id: &str, node_id: &str, data: M)
    -> Result<(), MembershipError> where M: Clone;

/// The members of an edge with their values, in the order of `members()`.
pub fn members_with_data(&self, edge_id: &str) -> Option<Vec<(String, &M)>>;

/// `add_node_to_edge`, with the value the new membership carries. If the node is already a
/// member, its value is replaced.
pub fn add_node_to_edge_with_data(&mut self, edge_id: &str, node_id: &str, data: M)
    -> Result<(), EdgeError> where N: Default, E: Default, M: Clone;
```

- Produces, on `DiHypergraph<N, E, M>`, the same four with the role named:

```rust
pub fn member_data(&self, edge_id: &str, node_id: &str, role: DiRole) -> Option<&M>;
pub fn set_member_data(&mut self, edge_id: &str, node_id: &str, role: DiRole, data: M)
    -> Result<(), MembershipError>;
/// (tail with values, head with values), each in the order of `dimembers()`.
pub fn dimembers_with_data(&self, edge_id: &str) -> Option<(Vec<(String, &M)>, Vec<(String, &M)>)>;
pub fn add_node_to_edge_with_data(&mut self, edge_id: &str, node_id: &str, role: DiRole, data: M)
    -> Result<(), EdgeError> where N: Default, E: Default;
```

- Consumes: `MembershipError::{EdgeNotFound, NodeNotFound, NotAMember}` and `EdgeError`, as they are.

**Decisions:**
- *The read returns `Option`,* as every other read of the crate does; *the write returns the
  existing `MembershipError`,* whose three variants are exactly its three refusals.
- *Frozen:* the two writes call the same guard as every other mutator.
- *Both directions:* `Hypergraph::set_member_data` and `add_node_to_edge_with_data` write the value
  to both stored edges of the membership.
- *The existing constructors keep `M::default()`.* Their signatures and trait bounds do not change.
- *Not in the XGI formats:* XGI has no value on a membership, so `readwrite`, the Python package and
  the WASM bindings stay as they are. The crate docs say so.

- [ ] **Step 1: Write the failing tests** in `tests/test_member_data.rs`, with `M = u64`:
  - `a_new_membership_carries_the_default`: edge `e` of `a, b`; `member_data("e", "a") == Some(&0)`.
  - `a_value_is_written_and_read_back`: `set_member_data("e", "a", 7)` is `Ok(())`; `a` reads 7 and
    `b` still 0; `members_with_data("e") == Some(vec![("a".into(), &7), ("b".into(), &0)])`.
  - `the_read_is_none_for_what_is_absent`: unknown edge, unknown node, and a node that exists but
    is not a member each give `None`; `members_with_data("nope")` is `None`.
  - `the_write_refuses_by_name`: the same three cases give `Err(MembershipError::EdgeNotFound { .. })`,
    `NodeNotFound { .. }` and `NotAMember { .. }`, each carrying the ids given.
  - `adding_with_data_creates_and_replaces`: on an empty graph
    `add_node_to_edge_with_data("e", "a", 5)` creates both and `a` reads 5; called again with 9, `a`
    reads 9 and `members("e") == ["a"]`.
  - `a_member_that_leaves_and_returns_starts_again`: set 7; `remove_node_from_edge("e", "a")` (with
    whatever arguments keep the edge); `add_node_to_edge("e", "a")`; `a` reads 0.
  - `a_copy_carries_the_values`: set 7; `copy()`; the copy reads 7; setting 8 on the copy leaves
    the original at 7.
  - `a_frozen_graph_refuses_the_write`: `freeze()`, then `set_member_data` panics with the message
    the crate's other mutators give (`#[should_panic(expected = …)]`, the text read from the guard).
  - `both_stored_directions_hold_the_value`: only if the crate exposes its inner graph to callers;
    then assert that the two directed edges of the membership both carry 7. If it does not, say so
    in the commit and rely on the reviewer for Review Focus 3.
  - For `DiHypergraph`: `tail_and_head_keep_separate_values` (node `x` in the tail and in the head
    of `e`: set 3 for `Tail` and 4 for `Head`; each reads its own; `dimembers_with_data` shows both),
    and the read, refusal and frozen tests with a role.
- [ ] **Step 2: Run them.** `cargo test -p hypergraph-rs --locked --test test_member_data`.
  Expected: it does not compile, `no method named member_data`.
- [ ] **Step 3: Implement** the eight methods. Each gets a doc comment with one runnable example.
- [ ] **Step 4: Run Step 2's command, then** `cargo test -p hypergraph-rs --locked` (the doc
  examples run here too). Expected: all pass.
- [ ] **Step 5: The scoped gate, then commit:**
  `feat(core): read and write the value a membership carries`, with a body that names the methods
  and the two decisions above, ending with the line `Closes #2`.

### Task 3: refresh the dependencies

**Files:** Modify `Cargo.lock` only.

- [ ] **Step 1:** `cargo update 2>&1 | tee <log>` and keep the list of what moved and of what stays
  behind its latest version.
- [ ] **Step 2: Every gate.** The scoped gate; then `mise run rust:msrv`; then, if `.venv` exists,
  `.venv/bin/maturin develop` and `mise run py:gate`.
- [ ] **Step 3:** If a moved crate breaks a gate or the MSRV, put that one crate back with
  `cargo update -p <crate> --precise <version>` and name it and the reason in the commit. Change no
  source file and no `Cargo.toml` in this task.
- [ ] **Step 4: Commit:** `chore(deps): update to the latest compatible versions`, with a body that
  lists the crates that moved, any held back and why, and the crates whose newer major version is
  left for a pull request of its own.

### Task 4: usable on its own

**Files:**
- Modify: `Cargo.toml`, the four crate manifests, `README.md`, `crates/hypergraph-rs/src/lib.rs`
- Create: `CHANGELOG.md`, `CONTRIBUTING.md`, `crates/hypergraph-rs/examples/quickstart.rs`,
  `crates/hypergraph-rs/examples/membership_data.rs`

**Decisions (the lead's, for the owner to check):**
- `repository` becomes `https://github.com/percy-raskova/hypergraph-rs`, where the code is. The
  address there today does not exist. `homepage` is removed unless the address it names answers.
- Version `0.2.0` for the workspace: the API grew.
- The core crate must pack: `cargo package -p hypergraph-rs --locked` passes. The Python and WASM
  binding crates are marked `publish = false`. The path dependencies on the core crate also name its
  version. Nothing is published.
- Keywords `hypergraph`, `graph`, `network`, `higher-order`, `simplicial`; categories
  `data-structures`, `science`, `mathematics`; `readme` and `documentation` set;
  `[package.metadata.docs.rs] all-features = true`.

- [ ] **Step 1: `examples/quickstart.rs` and `examples/membership_data.rs`**, each a `main` that
  builds a small hypergraph, prints something true about it and asserts it. `quickstart` adds nodes
  and edges, lists members and memberships, and computes one statistic the crate offers.
  `membership_data` uses `Hypergraph<(), (), u64>` to record how many people of each group belong to
  each of two overlapping bodies, reads the values back and sums them for each body. Run each with
  `cargo run -p hypergraph-rs --example <name>`.
- [ ] **Step 2: The crate docs** in `lib.rs`: what the crate is, a quick start that is a runnable doc
  example, the feature flags and what each turns on (read them from the manifest), where membership
  data stands with respect to XGI, and the MSRV.
- [ ] **Step 3: `README.md`:** what it is and what it is not; how to depend on it today (a git
  dependency, since it is not on crates.io); the quick start (the same code as the doc example); a
  table of the feature flags; membership data in six lines; the Python bindings and the conformance
  gate in a paragraph; the MSRV and the pinned toolchain; how to run the gates; security (a pointer
  to `SECURITY.md`); license. Every statement checked against the code: nothing promised that is not
  there.
- [ ] **Step 4: `CHANGELOG.md`** in the Keep a Changelog form, with `0.2.0` (unreleased) listing what
  this branch added, changed and fixed, and `0.1.0` as the port itself. **`CONTRIBUTING.md`:** the
  gates, where tests live, the commit style of the history, and how the XGI conformance gate works.
- [ ] **Step 5: The manifests,** as decided above.
- [ ] **Step 6: Every gate:** the scoped gate, `cargo package -p hypergraph-rs --locked`,
  `mise run rust:msrv`. Expected: all pass.
- [ ] **Step 7: Commit:** `docs: make the crate usable on its own (0.2.0)`.
