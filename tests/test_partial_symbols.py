# tests/test_partial_symbols.py
"""`bound` must not hide a symbol that raises for ordinary arguments."""

import json
import pathlib
import sys
import tomllib

import pytest

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
FIELDS = {"symbol", "site", "trigger", "reason"}


def _authored() -> list[dict]:
    with (ROOT / "compat" / "partial_symbols.toml").open("rb") as fh:
        return tomllib.load(fh).get("partial_symbol", [])


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def test_register_is_non_empty_and_well_formed() -> None:
    entries = _authored()
    assert entries, "three partial sites are confirmed; an empty file hides them"
    for entry in entries:
        assert set(entry) == FIELDS, f"{entry.get('symbol')}: fields {sorted(entry)}"


def test_manifest_promotes_them_to_bound_partial() -> None:
    manifest = _manifest()
    for entry in _authored():
        symbol = entry["symbol"]
        assert manifest["symbols"][symbol] == "bound-partial", (
            f"{symbol} is recorded partial but the manifest still calls it "
            f"{manifest['symbols'][symbol]!r}"
        )


def test_no_symbol_is_both_partial_and_deferred() -> None:
    manifest = _manifest()
    partial = {e["symbol"] for e in _authored()}
    deferred = {n for n, s in manifest["symbols"].items() if s == "deferred"}
    assert partial.isdisjoint(deferred)


def test_manifest_exposes_the_detail() -> None:
    assert _manifest()["partial_symbols"] == _authored()


def test_degree_assortativity_exact_false_still_raises() -> None:
    """A direct pin: when this stops raising, the entry must be deleted."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([["a", "b"], ["b", "c"], ["a", "c"]])
    with pytest.raises(NotImplementedError):
        hrs.degree_assortativity(H, exact=False)
