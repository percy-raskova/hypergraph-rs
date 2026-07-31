"""The shipped divergence register must be complete, unique, and in-taxonomy."""

import pathlib
import sys
import tomllib

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from taxonomy import CLASSES

ROOT = pathlib.Path(__file__).parent.parent
FIELDS = {"id", "api", "ours", "xgi", "class", "reason"}

EVIDENCE_DOC = (
    ROOT
    / "docs"
    / "superpowers"
    / "specs"
    / "evidence"
    / "2026-07-30-divergence-register-measurement.md"
)

# D38 (F7, subclassability) was found during the method-layer sweep AFTER the
# evidence document froze, so it has no counterpart there by design — it is
# not a paraphrase gap, it is a genuinely later addition.
NO_EVIDENCE_COUNTERPART = {"D38"}


def _register() -> list[dict]:
    with (ROOT / "compat" / "divergences.toml").open("rb") as fh:
        return tomllib.load(fh).get("divergence", [])


def _evidence_entries() -> list[dict]:
    """Independently re-parse the 'Ready-to-paste TOML' block Step 4 copied
    into compat/divergences.toml, straight out of the evidence document.

    Re-deriving this from the source (rather than trusting the copy) is what
    lets a future silent paraphrase of a shipped entry be caught.
    """
    text = EVIDENCE_DOC.read_text()
    heading = "### Ready-to-paste TOML"
    start = text.index(heading)
    fence_start = text.index("```toml", start) + len("```toml")
    fence_end = text.index("```", fence_start)
    block = text[fence_start:fence_end]
    return tomllib.loads(block).get("divergence", [])


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


def test_evidence_document_yields_34_entries() -> None:
    """Guards against a truncated or unparseable extraction passing
    vacuously: an empty or short block would make the text-fidelity test
    below trivially pass by having nothing to compare."""
    assert len(_evidence_entries()) == 34


def test_register_text_matches_the_evidence_document_verbatim() -> None:
    """The register's whole value is that it is measured, not recalled.
    Every other test here pins SHAPE (count, taxonomy, uniqueness, fields);
    this one pins TEXT, so a future silent paraphrase of a shipped entry's
    api/ours/xgi/reason prose is caught. Matched by id, not position, so a
    reordering of either file does not false-fail."""
    evidence_by_id = {e["id"]: e for e in _evidence_entries()}
    shipped_by_id = {
        e["id"]: e for e in _register() if e["id"] not in NO_EVIDENCE_COUNTERPART
    }
    mismatches = []
    # Bounded: one pass over the evidence document's fixed 34 entries.
    for eid, evidence_entry in evidence_by_id.items():
        shipped_entry = shipped_by_id.get(eid)
        if shipped_entry is None:
            mismatches.append(f"{eid}: missing from the shipped register")
        elif shipped_entry != evidence_entry:
            mismatches.append(f"{eid}: fields differ from the evidence document")
    assert mismatches == [], "\n".join(mismatches)
