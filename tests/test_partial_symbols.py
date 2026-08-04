# tests/test_partial_symbols.py
"""`bound` must not hide a symbol that raises for ordinary arguments."""

import json
import pathlib
import tomllib

import pytest

ROOT = pathlib.Path(__file__).parent.parent

MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
FIELDS = {"symbol", "class", "exception", "site", "trigger", "reason"}


def _authored() -> list[dict]:
    with (ROOT / "compat" / "partial_symbols.toml").open("rb") as fh:
        return tomllib.load(fh).get("partial_symbol", [])


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def test_register_is_non_empty_and_well_formed() -> None:
    entries = _authored()
    assert entries, (
        "compat/partial_symbols.toml is empty; every confirmed partial site "
        "is hidden — the manifest's bound-partial symbols name what should "
        "be here"
    )
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
    from tools.gen_manifest import ManifestError, _promote_partials

    entry = {
        "symbol": "x",
        "class": "core-gap",
        "exception": "NotImplementedError",
        "site": "s",
        "trigger": "t",
        "reason": "r",
    }
    with pytest.raises(ManifestError, match="deferred stub"):
        _promote_partials({"x": "deferred"}, [entry])


def test_promote_partials_rejects_a_name_not_in_all() -> None:
    from tools.gen_manifest import ManifestError, _promote_partials

    entry = {
        "symbol": "ghost",
        "class": "core-gap",
        "exception": "NotImplementedError",
        "site": "s",
        "trigger": "t",
        "reason": "r",
    }
    with pytest.raises(ManifestError, match="not in"):
        _promote_partials({"x": "bound"}, [entry])


def test_manifest_exposes_the_detail() -> None:
    assert _manifest()["partial_symbols"] == _authored()


def _expected(site_fragment: str) -> type[BaseException]:
    """The exception type the register claims for this site."""
    import builtins

    entries = [e for e in _authored() if site_fragment in e["site"]]
    assert len(entries) == 1, f"{site_fragment}: {len(entries)} entries, want 1"
    return getattr(builtins, entries[0]["exception"])


def test_degree_assortativity_exact_false_still_raises() -> None:
    """A direct pin: when this stops raising, the entry must be deleted."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([["a", "b"], ["b", "c"], ["a", "c"]])
    with pytest.raises(_expected("algorithms.rs"), match="call with exact=True"):
        hrs.degree_assortativity(H, exact=False)


def test_di_stat_in_degree_call_still_raises() -> None:
    """A direct pin for entry B: when this stops raising, the entry must be
    deleted. `match` narrows away unrelated NotImplementedErrors rather than
    distinguishing this site from out_degree's — the two sites share the
    identical message 'this Stat does not accept parameters'."""
    import hypergraph_rs as hrs

    D = hrs.DiHypergraph()
    D.add_edges_from([({0, 1}, {2}), ({1, 2}, {4}), ({2, 3, 4}, {4, 5})])
    with pytest.raises(_expected("in_degree"), match="does not accept parameters"):
        D.nodes.in_degree()


def test_di_stat_out_degree_call_still_raises() -> None:
    """A direct pin for entry C: when this stops raising, the entry must be
    deleted. `match` narrows away unrelated NotImplementedErrors rather than
    distinguishing this site from in_degree's — the two sites share the
    identical message 'this Stat does not accept parameters'."""
    import hypergraph_rs as hrs

    D = hrs.DiHypergraph()
    D.add_edges_from([({0, 1}, {2}), ({1, 2}, {4}), ({2, 3, 4}, {4, 5})])
    with pytest.raises(_expected("out_degree"), match="does not accept parameters"):
        D.nodes.out_degree()


def test_edge_attrs_stat_recall_still_raises() -> None:
    """A direct pin for entry D: when this stops raising, the entry must be
    deleted. `match` narrows away unrelated NotImplementedErrors rather than
    distinguishing this site from the DiStat sites' — all three share the
    identical message 'this Stat does not accept parameters'."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([({1, 2}, {"color": "blue"}), ({3, 4, 5}, {"color": "purple"})])
    with pytest.raises(
        _expected("PyEdgeStat::__call__"), match="does not accept parameters"
    ):
        H.edges.attrs("color")()


def test_every_entry_carries_a_taxonomy_class() -> None:
    """The nine-class set is closed and shared with exclusions.toml; a register
    outside it cannot be joined against the others."""
    from tools.taxonomy import CLASSES

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


def test_the_attrs_trigger_does_not_overstate_the_shape() -> None:
    """H.edges.attrs('color')('color') raises TypeError, not the entry's
    recorded exception: PyEdgeStat::__call__ is signature=(degree=None) over
    Option<usize>, so a str positional dies in extraction before the Attr arm.
    The entry must not claim that shape."""
    entries = [e for e in _authored() if "PyEdgeStat::__call__" in e["site"]]
    assert entries, "the Hypergraph.edges.attrs entry vanished"
    for entry in entries:
        assert "or with a new key" not in entry["trigger"], (
            "a new key raises TypeError from PyO3 extraction, not "
            f"{entry['exception']} — the recorded shape is mis-described"
        )


def test_a_new_key_really_does_raise_typeerror() -> None:
    """The behavioural half. Without this, the test above pins the absence of
    one English phrase — prose, not behaviour. If PyO3 ever gains a str
    overload, the phrase check would stay green while the register's claim
    became wrong in the other direction."""
    import hypergraph_rs as hrs

    H = hrs.Hypergraph()
    H.add_edges_from([({1, 2}, {"color": "blue"})])
    with pytest.raises(TypeError):
        H.edges.attrs("color")("color")
