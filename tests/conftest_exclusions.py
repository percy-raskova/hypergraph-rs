"""Reader over compat/exclusions.toml — the single authored registry (spec §4 C2).

Re-exports FILE_EXCLUSIONS / TEST_EXCLUSIONS with their original types so
existing importers keep working, and adds ENTRY_CLASSES for the skip-vs-xfail
decision in conftest.py.
"""

import pathlib
import tomllib

_TOML = pathlib.Path(__file__).parent.parent / "compat" / "exclusions.toml"

with _TOML.open("rb") as fh:
    _DATA = tomllib.load(fh)

FILE_EXCLUSIONS: dict[str, str] = {
    e["path"]: e["reason"] for e in _DATA.get("file_exclusion", [])
}

FILE_CLASSES: dict[str, str] = {
    e["path"]: e["class"] for e in _DATA.get("file_exclusion", [])
}

TEST_EXCLUSIONS: dict[str, list[tuple[str, str]]] = {}
ENTRY_CLASSES: dict[tuple[str, str], str] = {}

# Bounded: one pass over the parsed TOML array.
for _e in _DATA.get("test_exclusion", []):
    TEST_EXCLUSIONS.setdefault(_e["path"], []).append((_e["test"], _e["reason"]))
    ENTRY_CLASSES[(_e["path"], _e["test"])] = _e["class"]


def entry_matches_name(name: str, entry: str) -> bool:
    """Does collected-test `name` match registry `entry`?

    Both are either `"test_func"` or `"ClassName::test_method"`. Matching is
    EXACT, with one tolerance: pytest appends a parametrization suffix
    (`test_foo[1-2]`), and `[` cannot occur in a Python identifier, so
    `name.startswith(entry + "[")` cannot reach a sibling test the way a bare
    `startswith` does. A bare entry never matches a class-scoped test and vice
    versa, so an entry can never claim a test it does not name (spec F1).
    """
    if ("::" in entry) != ("::" in name):
        return False
    if "::" in entry:
        entry_cls, entry_fn = entry.split("::", 1)
        name_cls, name_fn = name.split("::", 1)
        if entry_cls != name_cls:
            return False
    else:
        entry_fn, name_fn = entry, name
    return name_fn == entry_fn or name_fn.startswith(entry_fn + "[")
