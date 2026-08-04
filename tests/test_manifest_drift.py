# tests/test_manifest_drift.py
"""The committed manifest must match what the current tree generates (spec §3).

This is what makes drift impossible: an exclusion whose gap was fixed, a symbol
that became bound, or a namespace that started resolving all change the
generated manifest, and the build fails until the committed copy is refreshed.
"""

import json
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).parent.parent

from tools.gen_manifest import DEST, build  # noqa: E402


def test_committed_manifest_matches_generated():
    committed = json.loads(DEST.read_text())
    generated = build()
    assert committed == generated, (
        "manifest.json is stale — regenerate with `mise run py:manifest` and "
        "commit the result. If the diff is a symbol that became bound or an "
        "exclusion that no longer applies, update compat/exclusions.toml too."
    )


def test_divergences_status_no_longer_claims_unpopulated() -> None:
    manifest = json.loads(DEST.read_text())
    assert manifest["divergences"], "the register is populated; the manifest must say so"
    assert "unpopulated-T0" not in manifest["divergences_status"], (
        "the placeholder status outlived the placeholder — a shipped artifact "
        "describing itself as empty when it is not is worse than an empty one"
    )
    assert "NOT thereby verified equivalent" in manifest["divergences_status"], (
        "the load-bearing sentence: absence from the register is not evidence "
        "of equivalence. Plan C's COVERED rule depends on this distinction"
    )


def test_manifest_divergences_match_the_authored_register() -> None:
    """The manifest must carry the register verbatim, not a stale copy."""
    manifest = json.loads(DEST.read_text())
    with (ROOT / "compat" / "divergences.toml").open("rb") as fh:
        authored = tomllib.load(fh)["divergence"]
    assert manifest["divergences"] == authored
