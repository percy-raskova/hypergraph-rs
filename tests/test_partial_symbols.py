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
FIELDS = {"symbol", "class", "exception", "site", "trigger", "reason"}


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


def test_promote_partials_rejects_a_deferred_symbol() -> None:
    """Direct on the guard, not on its output: `manifest["symbols"]` has
    already been overwritten by the time `test_no_symbol_is_both_partial_and_
    deferred` reads it, so that test cannot fail if this guard is deleted.
    """
    from gen_manifest import ManifestError, _promote_partials

    entry = {"symbol": "x", "site": "s", "trigger": "t", "reason": "r"}
    with pytest.raises(ManifestError, match="deferred stub"):
        _promote_partials({"x": "deferred"}, [entry])


def test_promote_partials_rejects_a_name_not_in_all() -> None:
    from gen_manifest import ManifestError, _promote_partials

    entry = {"symbol": "ghost", "site": "s", "trigger": "t", "reason": "r"}
    with pytest.raises(ManifestError, match="not in"):
        _promote_partials({"x": "bound"}, [entry])


def test_manifest_exposes_the_detail() -> None:
    assert _manifest()["partial_symbols"] == _authored()


def test_degree_assortativity_exact_false_still_raises() -> None:
    """A direct pin: when this stops raising, the entry must be deleted."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([["a", "b"], ["b", "c"], ["a", "c"]])
    with pytest.raises(NotImplementedError):
        hrs.degree_assortativity(H, exact=False)


def test_di_stat_call_still_raises() -> None:
    """A direct pin for entries B and C: when this stops raising, both
    entries must be deleted."""
    import hypergraph_rs as hrs

    D = hrs.DiHypergraph()
    D.add_edges_from([({0, 1}, {2}), ({1, 2}, {4}), ({2, 3, 4}, {4, 5})])
    for stat in (D.nodes.in_degree, D.nodes.out_degree):
        with pytest.raises(NotImplementedError):
            stat()


def test_edge_attrs_stat_recall_still_raises() -> None:
    """A direct pin for entry D: when this stops raising, the entry must be
    deleted."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([({1, 2}, {"color": "blue"}), ({3, 4, 5}, {"color": "purple"})])
    with pytest.raises(NotImplementedError):
        H.edges.attrs("color")()


def test_every_entry_carries_a_taxonomy_class() -> None:
    """The nine-class set is closed and shared with exclusions.toml; a register
    outside it cannot be joined against the others."""
    from taxonomy import CLASSES

    for entry in _authored():
        assert entry["class"] in CLASSES, (
            f"{entry['symbol']}: class {entry['class']!r} is not one of "
            f"{sorted(CLASSES)}"
        )


def test_every_entry_names_the_exception_it_raises() -> None:
    """`bound-partial` is not NotImplementedError-specific. The drawing.*
    family raises ImportError, and a register that cannot say so would send a
    consumer without the mpl extra to a symbol reported usable."""
    for entry in _authored():
        assert entry["exception"], f"{entry['symbol']}: empty exception"
        assert entry["exception"] in {e.__name__ for e in (
            NotImplementedError, ImportError, TypeError, ValueError, KeyError,
            AttributeError, RuntimeError,
        )}, f"{entry['symbol']}: {entry['exception']!r} is not a known builtin"


def test_entries_are_unique() -> None:
    """A double-pasted entry is otherwise accepted silently: the symbol map is
    idempotent and the manifest list just grows."""
    seen = [(e["symbol"], e["site"]) for e in _authored()]
    assert len(seen) == len(set(seen)), f"duplicate (symbol, site): {seen}"
