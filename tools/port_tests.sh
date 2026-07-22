#!/usr/bin/env bash
# Port XGI's oracle tests into tests/ported/ with the import swap (spec §9.1).
set -euo pipefail
ORACLE="/tmp/claude-1000/-home-user-projects-game-babylon--claude-worktrees-docs-hypergraph-rs-design/a9bdcfa3-d5c2-4825-afe9-6cfbcdc5df12/scratchpad/xgi-0.10.2/tests"
DEST="tests/ported"
mkdir -p "$DEST"
# D10: copy only test_*.py — NOT the oracle conftest.py. The single hand-authored conftest lives at
# the workspace-root tests/ (Step 2) with the exclusion glue; a copied ported/conftest.py would
# duplicate every fixture at a deeper level AND lack the exclusion glue. Fixtures propagate downward.
find "$ORACLE" -name 'test_*.py' -print0 | while IFS= read -r -d '' f; do   # bounded: fixed file set
  rel="${f#"$ORACLE"/}"
  mkdir -p "$DEST/$(dirname "$rel")"
  sed -E 's/^import xgi$/import hypergraph_rs as xgi/; s/from xgi(\.|[[:space:]])/from hypergraph_rs\1/' \
      "$f" > "$DEST/$rel"
done
# OQ-2 (orchestrator ruling: FETCH). The corpus clone lacks tests/readwrite/data/; fetch it from the
# xgi v0.10.2 git tag so the disk-fixture readwrite tests can run. Sparse checkout (no full clone):
if [ ! -d "$ORACLE/readwrite/data" ]; then
  TMP="$(mktemp -d)"
  git -C "$TMP" init -q
  git -C "$TMP" remote add origin https://github.com/xgi-org/xgi.git
  git -C "$TMP" config core.sparseCheckout true
  echo "tests/readwrite/data/*" > "$TMP/.git/info/sparse-checkout"
  # Non-fatal: if there is no network at execution, fall through to per-file exclusion (below).
  if git -C "$TMP" fetch -q --depth 1 origin refs/tags/v0.10.2 && git -C "$TMP" checkout -q FETCH_HEAD; then
    if [ -d "$TMP/tests/readwrite/data" ]; then
      mkdir -p "$ORACLE/readwrite"
      cp -r "$TMP/tests/readwrite/data" "$ORACLE/readwrite/data"
    else
      echo "WARN: v0.10.2 tag itself has no tests/readwrite/data/ (verified 2026-07-22) — dataset_folder is a dead var in test_incidence_matrix.py (tempfile-only tests); disk-fixture readwrite tests fall to exclusion"
    fi
  else
    echo "WARN: could not fetch readwrite/data from v0.10.2 (OQ-2) — disk-fixture readwrite tests fall to exclusion"
  fi
  rm -rf "$TMP"
fi
[ -d "$ORACLE/readwrite/data" ] && mkdir -p "$DEST/readwrite" && cp -r "$ORACLE/readwrite/data" "$DEST/readwrite/data" || \
  echo "WARN: no readwrite/data available — inline-string/tempfile readwrite tests still pass; disk-fixture ones excluded"
