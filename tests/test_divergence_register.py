"""The shipped divergence register must be complete, unique, and in-taxonomy."""

import pathlib
import sys
import tomllib

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from taxonomy import CLASSES

ROOT = pathlib.Path(__file__).parent.parent
FIELDS = {"id", "api", "ours", "xgi", "class", "reason"}


def _register() -> list[dict]:
    with (ROOT / "compat" / "divergences.toml").open("rb") as fh:
        return tomllib.load(fh).get("divergence", [])


def test_register_is_populated() -> None:
    """34 measured in the evidence document, plus D38 (F7, subclassability),
    which was found during the method-layer sweep after that document froze."""
    assert len(_register()) == 35, (
        "the measured live, Python-visible register is 35 entries; a short "
        "register reads as complete when it is not"
    )


def test_every_class_is_in_the_closed_taxonomy() -> None:
    offenders = sorted(
        {e["class"] for e in _register() if e["class"] not in CLASSES}
    )
    assert offenders == [], f"off-taxonomy classes: {offenders}"


def test_ids_are_unique() -> None:
    ids = [e["id"] for e in _register()]
    assert len(ids) == len(set(ids)), "duplicate divergence ids"


def test_no_entry_has_unknown_or_missing_fields() -> None:
    for entry in _register():
        extra = set(entry) - FIELDS
        assert extra == set(), f"{entry.get('id')}: unknown fields {sorted(extra)}"
        absent = FIELDS - set(entry)
        assert absent == set(), f"{entry.get('id')}: missing fields {sorted(absent)}"


def test_d34_names_the_default_path_not_in_place_false() -> None:
    """in_place=False is the one cleanup path that AGREES; the first draft
    inverted this and it was caught only by re-measurement."""
    entry = next(e for e in _register() if e["id"] == "D34")
    assert "in_place=False" not in entry["api"]


def test_dead_register_entries_are_absent() -> None:
    """D1, D6, D8, D10, D15, D16 are resolved, Rust-only, or never register
    entries. Shipping them would claim differences that do not exist."""
    ids = {e["id"] for e in _register()}
    assert ids.isdisjoint({"D1", "D6", "D8", "D10", "D15", "D16"})
