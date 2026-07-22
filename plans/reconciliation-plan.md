# Lineage Reconciliation Plan — conformance-lineage → main

## 1. Header

**Goal.** Land the executable XGI-0.10.2 conformance harness (fixtures generator + xgi_ground_truth.json + tests/conformance/main.rs) on main so `cargo test --workspace` replays it green, folding register-mandated XGI runtime behaviors into main's core while preserving main's reviewed Phase 3+4 breadth (views, globalviews, linalg, algorithms, generators, FaceLattice).

**Ground rules.**
- The D-register pins XGI runtime behavior and wins, unless a main-side review explicitly ruled otherwise (only D10 qualifies).
- Core-owned divergences encoding a real XGI-runtime difference where main currently diverges are folded into main's core. Binding-owned (D2 error channel, D6 dict-merge, D7 dynamic typing, D14 runtime shape-validation) leave the core its typed/stricter shape; the core adopts only the substrate Phase-7 needs.
- Pure naming/shape choices with no XGI-runtime content where main has a reviewed impl (DiRole vs Direction, SimplexError, two-arg add_edge, Result-wrapped infallible mutators, ordering impl) → keep-main; adapt the grafted test, each adaptation cited to a ruling. The harness pins outcomes, which are API-shape-independent.
- Where main matches XGI better than the lineage core (D10) → rule for the XGI-matching side and flip the expectation.
- Preserve `// D<N>:` register comments verbatim.

**Gate (every task), single-flight cargo:** `mise run rust:check` (fmt --check && clippy --workspace --all-targets --locked -D warnings && test --workspace --locked && doc -D warnings). Removing EdgeError::EmptyMembers (D1 deletes its only construction site) is mandatory: -D warnings fails a never-constructed variant.

**Commit (every task).** Plain `git commit` (no hooks), conventional message, verify HEAD moved, trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

**Verified facts (read-only spot-checks):**
- main clear() (hypergraph.rs:354-360) does NOT reset the uid counter → matches XGI; fixture clear_all.auto_ids_after_clear == ["1"]. D10 dissolves toward main.
- main add_edge rejects empty (:142-143); EdgeError has EmptyMembers+TooFewEdges (error.rs:28,31).
- main add_simplex already Option<String> (D17 satisfied) but short-circuits empty (:191-193, D1 violation); returns Result<(),SimplexError>.
- main remove_node is 2-arg (node_id, strong) (:252) — no remove_empty (D9 gap); main's Di remove_node is already 3-arg.
- main copy() hardcodes frozen:false for H (:400) and Di (dihypergraph.rs:509) → D13.
- Bulk ops return (), discard errors; internal callers hypergraph.rs:1010/1014, globalviews.rs:53.
- "D15" is a lineage src module-docstring label (simplicialcomplex.rs:14), absent from the D-register — no register authority.
- Harness resolves fixture via env!("CARGO_MANIFEST_DIR")+"/conformance/xgi_ground_truth.json"; Cargo auto-discovers tests/conformance/main.rs as target conformance. .venv/bin/python has xgi 0.10.2 (oracle live).

## 2. Rulings ledger

| ID | Ruling | Rationale | Evidence |
|---|---|---|---|
| D1 | adopt-lineage-core | XGI runtime CREATES empty edges/simplices; core-owned; register's first case. Remove empty checks; delete EdgeError::EmptyMembers. | hypergraph.rs:142-143; simplicialcomplex.rs:191-193 |
| D1-ripple | adapt-main | globalviews.rs:53 !members.is_empty() wrongly drops empty edges; laplacian.rs divides by edge-size, no zero-guard on edge axis. | RIPPLE globalviews/laplacian |
| D2 (msg/type) | adopt-lineage-core | Distinct per-branch messages XGI raises are runtime content; adopt MembershipError{EdgeNotFound,NodeNotFound,NotAMember} (H+Di). Channel stays Phase-7. | register D2; CORE DIFF |
| D2 (bulk) | adopt-lineage-core | remove_edges_from stop-on-first (partial effects) and remove_nodes_from continue-per-item are XGI state differences (main over-applies). Return Vec<Result>; fix 3 callers. | register D2; hypergraph.rs:514-522,1010-1014; globalviews.rs:53 |
| D3, D4 | keep-main (conformant) | bump iff parse::<u64> already byte-identical. | CORE DIFF |
| D5 | keep-main; defer strict-order tests | XGI hash-unordered; register pins only sorted/multiset, which main satisfies. Lineage exact-insertion tests pin a stronger-than-XGI guarantee main's Di (alphabetical) lacks. | register D5; TEST MAP |
| D6 | adopt-lineage-core | Binding merges then calls core; replace composes, main's drop discards merged dict. H+Di add_node. | register D6 |
| D7 | keep-main (binding-owned) | Core already stringly-typed; shape-detection Phase-7. | register D7 |
| D8 | keep-main (nicety) | XGI returns None; Ok(id) is extra info, not required. Keep Result<()>/SimplexError; adapt D8 tests to assert state. | register D8 |
| D9 | adopt-lineage-core | XGI remove_node has remove_empty; main's 2-arg can't express weak+false. Add 3-arg (matches main's Di). | register D9; hypergraph.rs:252 |
| D10 | keep-main (DISSOLVED) | main deliberately preserves counter, matching XGI; fixture pins "1". Flip diverge_d10→conform_d10, final assert "0"→"1". | hypergraph.rs:354-360; main.rs:206-247 |
| D11 | keep-main (conformant) | Counter-bump logic identical (H+Di). | CORE DIFF |
| D12 | adopt-lineage-core (audit) | Uniform freeze-guard core-owned. main guards clear_edges already; audit Di add_node_to_edge/remove_node_from_edge. | hypergraph.rs:366-367; register D12 |
| D13 | adopt-lineage-core | Register's deliberate safety divergence; no explicit main ruling (unlike D10). copy() carries frozen (H+Di). | :400,dihypergraph.rs:509 |
| D14 | keep-main core (DiRole); binding-owned | Type-level prevention satisfied by both enums; naming has no XGI-runtime content. Adapt Di tests to DiRole. Serde spelling → open Q. | register D14; kinds.rs:30 |
| D15 | keep-main (not a register entry) | Lineage src label only; SC stub-Err vs type-absent is architecture. Keep SimplexError::NotAvailable. | simplicialcomplex.rs:14,217-232 |
| D16 | keep-main (no Di clear_edges) | Lineage extension; XGI has no Di clear_edges. Skip/adapt those vectors. | register D16 |
| D17 | keep-main (already exact) | main add_simplex already Option<String> honoring Some("0"). | simplicialcomplex.rs:179-183 |
| Di shape | keep-main; adapt harness | tuple add_edge, infallible add_node_to_edge: pure shape, no outcome difference. | CORE DIFF; TEST MAP |
| Test layout | keep-main flat tests/*.rs; graft tests/conformance/ dir; port tests/core/ bodies as appended #[test] fns | Preserves Phase 3/4 test files untouched. | TEST MAP |

## 3. Ordered TDD task list

### Task 1 — Graft non-conflicting lineage unit-test hardening (safety net, no core change)
- Red→green: port lineage tests/core/*.rs bodies pinning behaviors main already satisfies (D3/D4 parse, D11 bump, D17 falsy-idx, D7 stringly-typed, matching debug-format) as #[test] fns appended into test_hypergraph.rs/test_dihypergraph.rs/test_simplicialcomplex.rs. Do NOT port D5 strict-insertion tests or any not-yet-landed assertion. If one fails, stop and re-adjudicate.
- Impl: none. Gate: mise run rust:check.
- Commit: `test(conformance): graft non-conflicting lineage unit-test hardening (D3/D4/D7/D11/D17)`

### Task 2 — D1: empty edges/simplices become first-class
- Red: flip test_add_edge_empty_members_returns_error→_creates_edge (Ok, num_edges()==1, id "0", num_nodes()==0); Di both_empty→allowed; SC empty_simplex_is_created (2nd add no-op).
- Impl: delete empty-guards in H/Di add_edge and SC add_simplex short-circuit (:191-193); remove EdgeError::EmptyMembers + imports/matches; keep TooFewEdges.
- Gate: mise run rust:check.
- Commit: `feat(core)!: create empty edges/simplices to match XGI runtime (D1); drop EdgeError::EmptyMembers`

### Task 3 — D1 ripple: subhypergraph retention, Laplacian guard, order-stats
- Red: (a) subhypergraph retains a selected empty edge (currently dropped by globalviews.rs:53); (b) normalized Laplacian on a size-0 edge yields no Inf/NaN; (c) assert order-stat treatment of size-0 edges in properties.rs.
- Impl: drop empty exclusion from subhypergraph; add edge-axis zero guard in laplacian.rs; make order-stat decision explicit (recommend excluding size-0 from order counts — see open Q).
- Gate: mise run rust:check.
- Commit: `fix(core): propagate D1 empty-edge semantics into subhypergraph, laplacian, order-stats`

### Task 4 — D9: undirected remove_node gains remove_empty
- Red: port test_remove_node_weak_remove_empty_false_keeps_emptied_edge + _strong_ignores_remove_empty_false; update 2-arg call sites.
- Impl: remove_node(node_id, strong, remove_empty) -> Result<(),NodeError> (mirror Di); internal remove_nodes_from passes remove_empty=true.
- Gate: mise run rust:check.
- Commit: `feat(core)!: undirected remove_node gains remove_empty three-mode semantics (D9)`

### Task 5 — D2: MembershipError family for remove_node_from_edge
- Red: port test_remove_node_from_edge_missing_edge/node/not_a_member pinning variants + exact Display strings (H+Di).
- Impl: add MembershipError (error.rs); H/Di remove_node_from_edge → Result<(),MembershipError>, keep role: DiRole; export from lib.rs.
- Gate: mise run rust:check.
- Commit: `feat(core)!: MembershipError family for remove_node_from_edge, XGI-exact messages (D2)`

### Task 6 — D2: bulk-removal Vec<Result> + continuation policies
- Red: port test_remove_nodes_from_missing_per_item_err_continues + test_remove_edges_from_missing_stops_after_first_err.
- Impl: remove_nodes_from(ids,strong,remove_empty)->Vec<Result<(),NodeError>> (continue); remove_edges_from(ids)->Vec<Result<(),EdgeError>> (stop-on-first). Fix callers :1010/:1014, globalviews.rs:53 (let _ =; ids from existing → policy never triggers, verify).
- Gate: mise run rust:check.
- Commit: `feat(core)!: bulk remove ops return per-item Results with XGI continuation semantics (D2)`

### Task 7 — D6: add_node replaces attrs on existing id
- Red: port test_add_node_existing_replaces_attrs (exact JSON, H+Di).
- Impl: on existing id replace stored attrs (still return false); doc note binding merges first.
- Gate: mise run rust:check.
- Commit: `feat(core): add_node replaces attrs on existing id to compose with binding merge (D6)`

### Task 8 — D13 copy-carries-frozen + D12 uniform freeze-guard audit
- Red: port test_copy_of_frozen_is_frozen (H+Di); Di freeze-guard tests for add_node_to_edge/remove_node_from_edge.
- Impl: copy() carries frozen:self.frozen (:400,dihypergraph.rs:509); add missing Di assert_not_frozen guards.
- Gate: mise run rust:check.
- Commit: `feat(core): copy carries frozen flag (D13) + uniform freeze-guard coverage (D12)`

### Task 9 — Graft conformance harness dir + adapt undirected vectors; flip D10
- Graft: copy conformance/generate_fixtures.py + xgi_ground_truth.json to crates/hypergraph-rs/conformance/, and tests/conformance/main.rs to crates/hypergraph-rs/tests/conformance/ (auto-discovered; fixture path correct). Preserve // D<N>: comments.
- Adapt (cited): flip D10 — rename to conform_d10_clear_preserves_uid_counter, final assert "0"→"1" (ruling D10). Verify undirected vectors (D1/D3/D4/D5-sorted/D6/D9/D11/D12/D13/D17) pass.
- Gate: mise run rust:check (conformance target green).
- Commit: `test(conformance): graft XGI replay harness; flip D10 to XGI-conformant`

### Task 10 — Adapt harness Di + SC vectors to main's API surface
- Adapt (cited): Di vectors → DiRole::Tail/Head (D14), two-arg add_edge (Di shape), .unwrap() on Result add_node_to_edge (D8), MembershipError (Task 5). SC D8 vectors → assert state not returned id, keep SimplexError (D8/D15). Skip/comment Di clear_edges (D16) + Di bulk vectors with // keep-main ruling notes.
- Impl: test edits only. Gate: mise run rust:check (full suite green).
- Commit: `test(conformance): adapt Di/SC vectors to main's DiRole/SimplexError surface (D8/D14/D15/D16)`

### Task 11 — Wire rust:fixtures into .mise.toml
- Impl: add [tasks."rust:fixtures"] run = ".venv/bin/python crates/hypergraph-rs/conformance/generate_fixtures.py" (read-only .venv oracle, xgi 0.10.2 verified; NEVER babylon's venv; NOT added to rust:check — committed JSON is the contract). Update the "deliberately ABSENT" comment.
- Gate: mise run rust:fixtures yields byte-identical JSON (git clean); then mise run rust:check.
- Commit: `build(mise): wire rust:fixtures generator (read-only .venv XGI oracle)`

### Task 12 — Final full-workspace gate + close-out
- Verify: mise run rust:check green; conformance target in the run; tree clean; HEAD advanced; Phase 3/4 test files still pass untouched.
- Impl: update plans/EXECUTION-STATE.md — reconciliation DONE, rulings ledger location, Phase-7 Task 0 subsumed by D1, Phase 4 resumes at task 6.
- Gate: mise run rust:check.
- Commit: `docs(plans): close lineage reconciliation; harness green on main, Phase 4 resumes at task 6`

## 4. Open questions for the owner
See the structured open_questions list.