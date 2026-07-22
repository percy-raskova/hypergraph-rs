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
