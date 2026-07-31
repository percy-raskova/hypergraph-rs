# tests/test_manifest_drift.py
"""The committed manifest must match what the current tree generates (spec §3).

This is what makes drift impossible: an exclusion whose gap was fixed, a symbol
that became bound, or a namespace that started resolving all change the
generated manifest, and the build fails until the committed copy is refreshed.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from gen_manifest import DEST, build  # noqa: E402


def test_committed_manifest_matches_generated():
    committed = json.loads(DEST.read_text())
    generated = build()
    assert committed == generated, (
        "manifest.json is stale — regenerate with `mise run py:manifest` and "
        "commit the result. If the diff is a symbol that became bound or an "
        "exclusion that no longer applies, update compat/exclusions.toml too."
    )
