# tests/test_method_audit.py
"""The methods of the classes and views, audited against XGI.

`tools/audit_symbols.py` compares `__all__` only. A method XGI has and the library lacks
(`add_weighted_edges_from`, `DiEdgeView.sources`) went unseen for months. This audit lists
every public name of an XGI class, view or stat that the library's counterpart lacks, and
each must be registered in `compat/method_gaps.toml`, both ways.
"""

import pathlib
import tomllib

from tools.audit_methods import missing_methods, subjects
from tools.gen_manifest import require_xgi_target
from tools.taxonomy import CLASSES, OWNER_PATTERN

ROOT = pathlib.Path(__file__).parent.parent
FIELDS = {"subject", "name", "class", "reason", "owner"}


def load_gaps() -> list[dict]:
    with (ROOT / "compat" / "method_gaps.toml").open("rb") as fh:
        return tomllib.load(fh).get("gap", [])


def test_the_oracle_is_the_target() -> None:
    require_xgi_target()


def test_every_subject_is_built_on_both_sides() -> None:
    labels = [label for label, _, _ in subjects()]
    assert labels == [
        "Hypergraph",
        "Hypergraph.nodes",
        "Hypergraph.edges",
        "Hypergraph.nodes.degree",
        "Hypergraph.edges.order",
        "DiHypergraph",
        "DiHypergraph.nodes",
        "DiHypergraph.edges",
        "SimplicialComplex",
        "SimplicialComplex.nodes",
        "SimplicialComplex.edges",
    ]
    for label, ours, theirs in subjects():
        assert type(ours).__module__.split(".")[0] == "hypergraph_rs", label
        assert type(theirs).__module__.split(".")[0] == "xgi", label


def test_every_missing_method_is_registered() -> None:
    registered = {(g["subject"], g["name"]) for g in load_gaps()}
    found = {(s, n) for s, names in missing_methods().items() for n in names}
    assert found - registered == set(), sorted(found - registered)


def test_no_registered_gap_is_stale() -> None:
    found = {(s, n) for s, names in missing_methods().items() for n in names}
    stale = {(g["subject"], g["name"]) for g in load_gaps()} - found
    assert stale == set(), sorted(stale)


def test_every_gap_is_in_the_taxonomy() -> None:
    assert {g["class"] for g in load_gaps()} <= CLASSES


def test_every_gap_is_well_formed() -> None:
    for gap in load_gaps():
        assert set(gap) - {"owner"} == FIELDS - {"owner"}, gap
        assert set(gap) <= FIELDS, gap
        assert gap["reason"].strip(), gap
        if "owner" in gap:
            assert OWNER_PATTERN.match(gap["owner"]), gap
    keys = [(g["subject"], g["name"]) for g in load_gaps()]
    assert len(keys) == len(set(keys)), "a gap is registered twice"
