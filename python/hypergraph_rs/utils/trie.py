"""Deep-import target: `from hypergraph_rs.utils.trie import Trie` (mirrors
xgi/utils/trie.py). `Trie` is not ported (Phase 6); its direct tests are a
TEST_EXCLUSIONS entry (Task 15, D5). This stub exists only so the module
importable at collection time (Fact 2, D4 mandatory)."""
from .._deferred import make_deferred

Trie = make_deferred("Trie")  # utils.trie.Trie not ported (Phase 6)
