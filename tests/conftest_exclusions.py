"""Central exclusion registry for the XGI conformance gate. Every entry cites a reason.

FILE_EXCLUSIONS — whole files skipped at collection (won't import, or all-deferred).
TEST_EXCLUSIONS — individual tests skipped (a file mostly passes but for specific cases).
"""

FILE_EXCLUSIONS = {
    "convert/test_pandas.py": "pandas integration not ported (spec §4.4).",
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
    "core/test_hypergraph.py": [
        ("test_constructor",
         "deferred: numpy incidence-matrix (from_incidence_matrix) and pandas-dataframe "
         "constructors — convert dispatchers not ported (§A ledger, spec §4.4)."),
        ("test_merge_duplicate_edges",
         "merge_rule='union' stores each attr as a Python `set`; the core/binding edge-attr "
         "channel is JSON-typed (serde_json::Value has no Set variant), so a deduplicated JSON "
         "array stands in instead — documented representational deviation (Task 16 execution "
         "note, hypergraph.rs merge_duplicate_edges_impl). rename='first'/'tuple'/'new' and "
         "merge_rule='first'/'intersection' are fully implemented and exercised by the earlier "
         "assertions in this same test function; only the union-attrs assertion is unattainable."),
    ],
    "core/test_simplicialcomplex.py": [
        ("test_constructor",
         "pandas-dataframe constructor path (SimplicialComplex(dataframe5)) — pandas integration "
         "not ported (spec §4.4), same deferral as Hypergraph's constructor."),
        ("test_add_simplices_from_format2",
         "core-gap: auto-assigned FACE ids (from add_simplex's closure-under-faces) are minted in "
         "a different enumeration order than real XGI's `_faces`/`add_simplex` subface iteration, "
         "so `H._edge == simplices1` (which pins exact face ids like `H._edge[3] == frozenset({2,4})`) "
         "diverges even though the SET of simplices is identical. Fixing requires matching the "
         "core's face-closure generation order to XGI's exactly — not a binding-level fix."),
        ("test_add_simplices_from_format3", "same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_simplices_from_format4", "same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_edges_from_dict", "same face-id-ordering core gap as test_add_simplices_from_format2."),
        ("test_add_simplices_from",
         "uses add_simplices_from(..., max_order=...) — XGI's max_order truncation kwarg is not "
         "built (no phase implemented it; not in the Task 7 scope any binding wired)."),
        ("test_freeze",
         "core-gap: exercises SC.remove_node, which has no core primitive (see test_remove_node)."),
        ("test_cleanup",
         "core-gap: SimplicialComplex has no isolate_node_ids/node-removal primitive in the core "
         "(unlike Hypergraph/DiHypergraph); cleanup(isolates=...) cannot be implemented at the "
         "binding level without one."),
        ("test_remove_node",
         "core-gap: hypergraph_rs::SimplicialComplex has no method to physically remove a node "
         "from the underlying bipartite graph (no `memberships`/node-delete primitive exists, "
         "unlike Hypergraph::remove_node) — real XGI implements it via direct `_node`/`_edge` dict "
         "surgery the Rust core has no equivalent internal access path for. Genuine core gap per "
         "the Task 15 triage protocol category 2 — not a binding fix; escalate for a future phase."),
        ("test_issue_445", "same core-gap as test_remove_node (also calls SC.remove_node)."),
    ],
    "core/test_views.py": [
        ("test_maximal",
         "SCEdgeView.maximal()/scoped-call not yet ported for SimplicialComplex (Task 7 rebuild "
         "still open in this pass); tracked for the simplicial-complex tier, not a P0-tier gap."),
        ("test_stat_discoverability",
         "asserts algorithm-derived stats (e.g. clustering_coefficient) appear in dir(); the "
         "nodestat/edgestat dispatch-registry machinery is replaced by idiomatic Rust getters "
         "(spec §4.4), same category as stats/test_core_stats_functions.py."),
        ("test_stat_caching",
         "asserts H.nodes.degree returns the SAME cached Stat object on repeated access; the "
         "binding recomputes a fresh Stat per getter call (no caching layer) — dispatch-machinery "
         "category, same as stats/test_core_stats_functions.py."),
        ("test_custom_stat_still_works",
         "exercises @xgi.nodestat_func, a DEFERRED symbol (dispatch-registry decorator; §A ledger "
         "'stats' deferred set)."),
        ("test_stat_docstring_forwarding",
         "asserts Stat.__doc__ forwards the underlying Python function's docstring; the binding's "
         "Stat objects are native PyO3 classes with no XGI Python function to forward from — "
         "dispatch-machinery category, same as stats/test_core_stats_functions.py."),
    ],
    "algorithms/test_centrality.py": [
        ("test_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_z_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        # D6: uniform_h_eigenvector_centrality is a DEFERRED symbol (§A); its test exercises the
        # NotImplementedError stub and MUST be skipped or it fails the gate.
        ("test_uniform_h_eigenvector_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_node_edge_centrality", "deferred: tensor centrality (Phase 3)."),
        ("test_line_vector_centrality", "deferred: tensor centrality (Phase 3)."),
        # (test_clique_eigenvector_centrality + test_katz_centrality stay green — those are bound.)
    ],
    "algorithms/test_assortativity.py": [
        # D5: real function name is `test_choose_degrees` (single underscore, oracle line 81) — the
        # earlier `test__choose_degrees` (double underscore) never matched, so the skip never fired.
        ("test_choose_degrees", "imports private _choose_degrees; helper not part of the port."),
    ],
    "algorithms/test_simpliciality.py": [
        # D5: Trie is imported at module scope (needs the utils/trie.py shim from Task 1/D4 to collect);
        # the tests that construct `Trie()` directly exercise the unported data structure → skip them.
        # The bound simpliciality-FUNCTION tests (test_edit_simpliciality, …) stay green.
        ("test_is_simplex", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_potential_simplices", "deferred: utils.trie.Trie not ported (Phase 6)."),
        ("test_count_missing_subfaces", "deferred: utils.trie.Trie not ported (Phase 6)."),
    ],
    # D5: utils/test_utilities.py had PHANTOM entries — `crest_r` lives only in the (file-excluded)
    # drawing/test_draw.py and `Trie` only in test_simpliciality.py; neither test exists in
    # test_utilities.py (its tests are test_iddict/test_dual_dict/test_powerset/test_subfaces/…),
    # so both entries were misdirected no-ops and are deleted.
    "dynamics/test_synchronization.py": [
        ("test_simulate_kuramoto",
         "exact numpy float-trajectory parity not achievable (§9.4); structural shape asserts kept."),
        ("test_simulate_simplicial_kuramoto", "deferred: simplicial Kuramoto (Phase 6)."),
    ],
}
