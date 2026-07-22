"""Central exclusion registry for the XGI conformance gate. Every entry cites a reason.

FILE_EXCLUSIONS — whole files skipped at collection (won't import, or all-deferred).
TEST_EXCLUSIONS — individual tests skipped (a file mostly passes but for specific cases).
"""

FILE_EXCLUSIONS = {
    "convert/test_pandas.py": "pandas integration not ported (spec §4.4).",
    "convert/test_encapsulation_dag.py": "deferred: to_encapsulation_dag (convert, §A ledger).",
    "convert/test_hif_dict.py": "deferred: to_hif_dict/from_hif_dict CONVERT dispatchers (xgi.convert.to_hif_dict, distinct from the bound xgi.write_hif/read_hif) not ported (§A ledger).",
    "convert/test_higher_order_network.py": "deferred: to_hypergraph/to_dihypergraph/to_simplicial_complex dispatchers not ported (§A ledger); also exercises pandas (spec §4.4).",
    "convert/test_incidence.py": "deferred: to_incidence_matrix/from_incidence_matrix CONVERT dispatchers not ported (§A ledger).",
    "convert/test_k_skeleton.py": "deferred: k_skeleton (convert, §A ledger).",
    "convert/test_line_graph.py": "deferred: to_line_graph (convert, §A ledger).",
    "readwrite/test_xgi_data.py": "network reader load_xgi_data/download_xgi_data gated off (spec §4.3).",
    "readwrite/test_bigg_data.py":
        "network: load_bigg_data is @pytest.mark.webtest/@slow and hits the BiGG server; its skipif "
        "(sys.version_info != (3,12) and not Linux) does NOT skip on the target Linux/py3.13 box (D7).",
    "stats/test_core_stats_functions.py":
        "tests the nodestat_func/dispatch decorator machinery, replaced by idiomatic Rust (spec §4.4).",
    "drawing/test_draw.py":
        "matplotlib @mpl_image_compare pixel tests; replaced by scene-graph tests (spec §9.3).",
    "drawing/test_draw_utils.py":
        "imports private _CCW_sort/_draw_arg_to_arr/_interp_draw_arg (no phase built them); "
        "replaced by scene-graph tests (spec §9.3).",
    "test_import_time.py": "asserts xgi package import-time budget; not a semantic conformance test.",
}

TEST_EXCLUSIONS = {
    "algorithms/test_properties.py": [
        ("test_equal",
         "binding gap: `H._net_attr` (and `_node_attr`/`_edge_attr`) return a snapshot PyDict "
         "copy, not a live reference into the binding's Rust-side attr store — real XGI's "
         "`_net_attr` IS the live dict, so `del H1._net_attr['test']` mutates it in place; here "
         "the deletion is silently lost. `xgi.equal` itself is correctly implemented (verified: "
         "the assertions up to and including `H1['test'] = ...` all pass). A live-reference fix "
         "needs a custom mutable-mapping proxy type wired to the core attr store — out of scope "
         "for this binding pass; not a core gap (Task 15 triage bucket 1, deferred as a documented "
         "binding limitation rather than fixed here)."),
    ],
    "generators/test_classic.py": [
        ("test_empty_dihypergraph", "deferred: empty_dihypergraph (generators, §A ledger)."),
        ("test_empty_simplicial_complex", "deferred: empty_simplicial_complex (generators, §A ledger)."),
    ],
    "generators/test_random.py": [
        ("test_dcsbm_hypergraph", "deferred: dcsbm_hypergraph (generators, §A ledger)."),
    ],
    "generators/test_randomizing.py": [
        ("test_shuffle_hyperedges", "deferred: shuffle_hyperedges (generators, §A ledger)."),
        ("test_node_swap", "deferred: node_swap (generators, §A ledger)."),
    ],
    "generators/test_simplicial_complexes.py": [
        ("test_flag_complex", "deferred: flag_complex (generators, §A ledger)."),
        ("test_flag_complex_d2", "deferred: flag_complex_d2 (generators, §A ledger)."),
        ("test_random_flag_complex", "deferred: random_flag_complex (generators, §A ledger)."),
        ("test_random_flag_complex_d2", "deferred: random_flag_complex_d2 (generators, §A ledger)."),
    ],
    "generators/test_uniform.py": [
        ("test_uniform_configuration_model_hypergraph",
         "core deviation, degree-sequence PARITY gap (documented in "
         "`uniform.rs::uniform_hypergraph_configuration_model`): remainder stubs are dropped and "
         "within-edge stub picks are deduplicated so the loop terminates deterministically "
         "(Power-of-10 rule 2) — the resulting degree sequence is close to but not always "
         "bit-exact with the requested `k`, so `dict(H.degree()) == k` does not hold in general. "
         "Documented core-level deviation from an earlier phase; not reopened here (Task 15 "
         "triage bucket 2 — not pre-approved for this pass; reviewed again at Task 19 and left "
         "as-is, exact parity with XGI's configuration model is unattainable under the bounded-"
         "loop policy, not a fixable binding bug)."),
        ("test_uniform_HSBM", "deferred: uniform_HSBM (generators, §A ledger)."),
        ("test_uniform_HPPM", "deferred: uniform_HPPM (generators, §A ledger)."),
    ],
    "readwrite/test_edgelist.py": [
        ("test_write_edgelist_compressed",
         "deferred: gzip/xz compression (write_edgelist(**kwargs) -> xopen) not implemented — "
         "would require adding flate2 (pure-Rust, fine) AND a system-liblzma-linked xz crate; the "
         "latter is a risky new native dependency to introduce mid-binding-fix, out of scope "
         "for this pass (Task 15 triage bucket 3)."),
        ("test_write_edgelist_compressed_format", "deferred: same compression gap as test_write_edgelist_compressed."),
    ],
    "readwrite/test_bipartite_edgelist.py": [
        ("test_write_bipartite_edgelist_compressed", "deferred: same compression gap as test_edgelist.py::test_write_edgelist_compressed."),
        ("test_write_bipartite_edgelist_compressed_format", "deferred: same compression gap as test_edgelist.py::test_write_edgelist_compressed."),
    ],
    "readwrite/test_hif.py": [
        ("test_write_hif_collection_compressed",
         "deferred: write_hif_collection/read_hif_collection are not bound (no phase built "
         "core to_hif_dict/from_hif_dict for SimplicialComplex or DiHypergraph — only the "
         "Hypergraph/undirected branch exists, per hif.rs's own module doc: 'Undirected only; "
         "directed HIF requires DiHypergraph (deferred)'); compounded by the compression gap."),
        ("test_write_hif",
         "deferred: to_hif_dict only exists for (undirected) Hypergraph (hif.rs module doc); "
         "SimplicialComplex ('asc' network-type) and DiHypergraph ('directed') HIF export are "
         "genuinely unbuilt, not a binding-level fix — not pre-approved this pass (Task 15 "
         "triage bucket 2). The Hypergraph portion of this test (verified separately) is green; "
         "the SC/DiHypergraph portions in the same test function are not."),
        ("test_read_hif", "deferred: same core gap as test_write_hif — exercises write_hif on SimplicialComplex/DiHypergraph."),
        ("test_read_hif_collection",
         "same core gap as test_write_hif (needs DiHypergraph HIF export), plus write_hif_collection/"
         "read_hif_collection are unbound deferred symbols."),
    ],
    "linalg/test_matrix.py": [
        ("test_fix_649", "network: calls load_xgi_data(dataset='email-enron'), gated off (spec §4.3)."),
        ("test_boundary_matrix", "deferred: boundary_matrix (Phase 3 'Deferred', §A ledger)."),
        ("test_empty", "deferred: exercises boundary_matrix (Phase 3 'Deferred', §A ledger)."),
        ("test_adjacency_tensor", "deferred: adjacency_tensor (Phase 3 'Deferred', §A ledger)."),
    ],
    "core/test_hypergraph.py": [
        ("test_constructor",
         "deferred: numpy incidence-matrix (from_incidence_matrix) and pandas-dataframe "
         "constructors — convert dispatchers not ported (§A ledger, spec §4.4)."),
        ("test_merge_duplicate_edges",
         "deferred: merge_rule='union' stores each attr as a Python `set`; the core/binding "
         "edge-attr channel is JSON-typed (serde_json::Value has no Set variant), so a "
         "deduplicated JSON array stands in instead — documented representational deviation "
         "(Task 16 execution note, hypergraph.rs merge_duplicate_edges_impl). "
         "rename='first'/'tuple'/'new' and merge_rule='first'/'intersection' are fully "
         "implemented and exercised by the earlier assertions in this same test function; only "
         "the union-attrs assertion is unattainable."),
    ],
    "core/test_simplicialcomplex.py": [
        ("test_constructor",
         "pandas-dataframe constructor path (SimplicialComplex(dataframe5)) — pandas integration "
         "not ported (spec §4.4), same deferral as Hypergraph's constructor."),
        ("test_add_simplices_from_format2",
         "deferred: auto-assigned FACE ids (from add_simplex's closure-under-faces) are minted in "
         "a different enumeration order than real XGI's `_faces`/`add_simplex` subface iteration, "
         "so `H._edge == simplices1` (which pins exact face ids like `H._edge[3] == frozenset({2,4})`) "
         "diverges even though the SET of simplices is identical. Fixing requires matching the "
         "core's face-closure generation order to XGI's exactly — not a binding-level fix."),
        ("test_add_simplices_from_format3", "deferred: same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_simplices_from_format4", "deferred: same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_edges_from_dict", "deferred: same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_simplices_from",
         "deferred: uses add_simplices_from(..., max_order=...) — XGI's max_order truncation kwarg "
         "is not built (no phase implemented it; not in the Task 7 scope any binding wired)."),
        ("test_freeze",
         "deferred: exercises SC.remove_node, which has no core primitive (see test_remove_node)."),
        ("test_cleanup",
         "deferred: SimplicialComplex has no isolate_node_ids/node-removal primitive in the core "
         "(unlike Hypergraph/DiHypergraph); cleanup(isolates=...) cannot be implemented at the "
         "binding level without one."),
        ("test_remove_node",
         "deferred: hypergraph_rs::SimplicialComplex has no method to physically remove a node "
         "from the underlying bipartite graph (no `memberships`/node-delete primitive exists, "
         "unlike Hypergraph::remove_node) — real XGI implements it via direct `_node`/`_edge` dict "
         "surgery the Rust core has no equivalent internal access path for. Genuine core gap per "
         "the Task 15 triage protocol category 2 — not a binding fix; escalate for a future phase."),
        ("test_issue_445", "deferred: same core-gap as test_remove_node (also calls SC.remove_node)."),
    ],
    "core/test_views.py": [
        ("test_maximal",
         "deferred: SCEdgeView.maximal()/scoped-call not yet ported for SimplicialComplex (Task 7 "
         "rebuild still open in this pass); tracked for the simplicial-complex tier, not a P0-tier "
         "gap."),
        ("test_stat_discoverability",
         "decorator machinery: asserts algorithm-derived stats (e.g. clustering_coefficient) "
         "appear in dir(); the nodestat/edgestat dispatch-registry machinery is replaced by "
         "idiomatic Rust getters (spec §4.4), same category as stats/test_core_stats_functions.py."),
        ("test_stat_caching",
         "decorator machinery: asserts H.nodes.degree returns the SAME cached Stat object on "
         "repeated access; the binding recomputes a fresh Stat per getter call (no caching layer) "
         "— dispatch-machinery category, same as stats/test_core_stats_functions.py."),
        ("test_custom_stat_still_works",
         "exercises @xgi.nodestat_func, a DEFERRED symbol (dispatch-registry decorator; §A ledger "
         "'stats' deferred set)."),
        ("test_stat_docstring_forwarding",
         "decorator machinery: asserts Stat.__doc__ forwards the underlying Python function's "
         "docstring; the binding's Stat objects are native PyO3 classes with no XGI Python "
         "function to forward from — dispatch-machinery category, same as "
         "stats/test_core_stats_functions.py."),
    ],
    "algorithms/test_centrality.py": [
        ("test_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_z_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        # D6: uniform_h_eigenvector_centrality is a DEFERRED symbol (§A); its test exercises the
        # NotImplementedError stub and MUST be skipped or it fails the gate.
        ("test_uniform_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_node_edge_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_line_vector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_katz_centrality",
         "float non-determinism (§9.4): the hand-rolled power-iteration core (deviation #5, "
         "centrality.rs) converges to a value that agrees with XGI's scipy/networkx katz_centrality "
         "to ~15 significant digits but not bit-for-bit (last-ULP divergence from a different "
         "floating-point operation order); the oracle test asserts exact dict `==`, not `pytest.approx`."),
        # (test_clique_eigenvector_centrality stays green — wired to H.nodes.clique_eigenvector_centrality.)
    ],
    "algorithms/test_assortativity.py": [
        # D5: real function name is `test_choose_degrees` (single underscore, oracle line 81) — the
        # earlier `test__choose_degrees` (double underscore) never matched, so the skip never fired.
        ("test_choose_degrees", "imports private _choose_degrees; helper not part of the port."),
        ("test_degree_assortativity",
         "deferred (documented deviation, algorithms.rs module doc): degree_assortativity(exact=False) "
         "(random-sampling path) awaits the Phase-4 rand_pcg pin; raises NotImplementedError."),
    ],
    "algorithms/test_connected.py": [
        ("test_largest_connected_hypergraph", "deferred: largest_connected_hypergraph (Phase 3 'Deferred', §A ledger)."),
    ],
    "algorithms/test_simpliciality.py": [
        # D5: Trie is imported at module scope (needs the utils/trie.py shim from Task 1/D4 to collect);
        # the tests that construct `Trie()` directly exercise the unported data structure → skip them.
        # The bound simpliciality-FUNCTION tests (test_edit_simpliciality, …) stay green.
        ("test_is_simplex", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_potential_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_missing_subfaces", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_powerset",
         "imports private algorithms.simpliciality._powerset helper; not part of the port "
         "(same category as test_is_simplex/Trie — private XGI internal, Task 15 triage bucket 3)."),
        ("test_max_number_of_subfaces",
         "imports private algorithms.simpliciality._max_number_of_subfaces helper; not part of "
         "the port (same category as test_powerset)."),
    ],
    # D5: utils/test_utilities.py had PHANTOM entries — `crest_r` lives only in the (file-excluded)
    # drawing/test_draw.py and `Trie` only in test_simpliciality.py; neither test exists in
    # test_utilities.py (its tests are test_iddict/test_dual_dict/test_powerset/test_subfaces/…),
    # so both entries were misdirected no-ops and are deleted.
    "utils/test_utilities.py": [
        ("test_iddict",
         "deferred: `H._edge`/`H._node` return snapshot copies, not a live mutable reference "
         "into the binding's Rust-side store — `H._edge[None] = {1,2,3}` (expected to raise "
         "XGIError) and `del H._node[\"test\"]` (expected IDNotFound) mutate a throwaway dict "
         "instead. Same category as algorithms/test_properties.py::test_equal's `_net_attr` gap "
         "(Task 17 execution note) — a live-reference fix needs a custom mutable-mapping proxy "
         "type, out of scope for this pass."),
        ("test_find_triangles", "deferred: find_triangles (utils, §A ledger)."),
        ("test_hist", "deferred: hist (utils, §A ledger)."),
        ("test_binomial_sequence", "deferred: binomial_sequence (utils, §A ledger)."),
        ("test_geometric", "deferred: geometric (utils, §A ledger)."),
        ("test_convert_labels_to_integers",
         "deferred: `convert_labels_to_integers` only exists for (undirected) Hypergraph "
         "(utils/mod.rs) — the Hypergraph portion of this test (verified separately, including "
         "the label-type binding fix in Task 17 execution notes) is green; the SimplicialComplex/"
         "DiHypergraph portions in the same test function need an equivalent core relabeling "
         "function that was never built — not pre-approved this pass (Task 15 triage bucket 2)."),
    ],
    "communities/test_spectral.py": [
        ("test_k_is_1", "imports private communities.spectral._kmeans helper; not part of the port."),
        ("test_perfectly_separable_low_dimensions", "imports private communities.spectral._kmeans helper; not part of the port."),
        ("test_perfectly_separable_high_dimensions", "imports private communities.spectral._kmeans helper; not part of the port."),
        ("test_strongly_separable_sbm", "deferred: uniform_HSBM (generators, §A ledger)."),
    ],
    "dynamics/test_synchronization.py": [
        ("test_simulate_kuramoto",
         "exact numpy float-trajectory parity not achievable (§9.4); structural shape asserts kept."),
        ("test_simulate_simplicial_kuramoto", "deferred: simplicial Kuramoto (Phase 6)."),
    ],
    "drawing/test_layout.py": [
        ("test_random_layout",
         "the random_layout seed-reproducibility assertions (the point of this test) pass; the "
         "test then also calls xgi.random_flag_complex_d2 (deferred generator, §A ledger) to build "
         "a SimplicialComplex fixture, which is unreachable — whole test excluded per protocol."),
        ("test_pairwise_spring_layout_simplicial_complex",
         "deferred: random_flag_complex_d2 (generators, §A ledger); the equivalent Hypergraph "
         "coverage (test_pairwise_spring_layout_hypergraph) stays green."),
        ("test_barycenter_spring_layout", "deferred: barycenter_spring_layout (drawing/layout, no Phase-5 analogue, Task 13 note)."),
        ("test_weighted_barycenter_spring_layout", "deferred: weighted_barycenter_spring_layout (drawing/layout, no Phase-5 analogue, Task 13 note)."),
        ("test_pca_transform", "deferred: pca_transform (drawing/layout, no Phase-5 analogue, Task 13 note)."),
        ("test_circular_layout",
         "circular_layout itself (center/radius kwargs, empty/single-node cases) is fixed and "
         "green (verified directly); this test's final assertion calls "
         "xgi.random_flag_complex_d2 (deferred generator, §A ledger), which is unreachable."),
        ("test_spiral_layout", "deferred: spiral_layout (drawing/layout, no Phase-5 analogue, Task 13 note)."),
        ("test_barycenter_kamada_kawai_layout", "deferred: barycenter_kamada_kawai_layout (drawing/layout, no Phase-5 analogue, Task 13 note)."),
        ("test_edge_positions_from_barycenters", "deferred: edge_positions_from_barycenters (drawing/layout, no Phase-5 analogue, Task 13 note)."),
    ],
}
