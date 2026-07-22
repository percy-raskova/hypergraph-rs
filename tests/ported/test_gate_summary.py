def test_no_unexcused_exclusions():
    """Every exclusion must carry one of the sanctioned reasons (triage protocol)."""
    from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS
    sanctioned = ("pandas", "network", "decorator", "image", "private", "deferred",
                  "trajectory", "parity", "colormap", "Trie", "pickle", "scene-graph",
                  "import-time", "fetch")   # "fetch" = OQ-2 disk-fixture reason (f)
    def ok(r): return any(s in r for s in sanctioned)
    for path, reason in FILE_EXCLUSIONS.items():
        assert ok(reason), f"{path}: unexcused exclusion `{reason}`"
    for path, entries in TEST_EXCLUSIONS.items():
        for name, reason in entries:
            assert ok(reason), f"{path}::{name}: unexcused exclusion `{reason}`"
