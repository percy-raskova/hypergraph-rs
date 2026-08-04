"""Pin the two-armed contract of conftest.py's pytest_ignore_collect.

pytest_ignore_collect is firstresult=True: pytest stops at the first
implementation that returns non-None. A bare-bool return for the
not-excluded arm (`False`, which is falsy but NOT None) silently disables
pytest's own --ignore / --ignore-glob handling for every path in the repo,
because `False` still short-circuits the hook chain. The excluded arm must
return True; the not-excluded arm must return None so pytest's core hook
gets a turn — asserting `is None` (not merely falsiness) is the whole point,
since `False` would pass a falsiness check and reintroduce the bug.
"""

import pathlib

from conftest import pytest_ignore_collect
from conftest_exclusions import FILE_EXCLUSIONS


def test_excluded_path_returns_true() -> None:
    excluded_rel = next(iter(FILE_EXCLUSIONS))
    collection_path = pathlib.Path(f"/repo/tests/ported/{excluded_rel}")
    assert pytest_ignore_collect(collection_path, config=None) is True


def test_non_excluded_path_returns_none_not_false() -> None:
    """A False here (falsy but not None) would silently disable --ignore repo-wide."""
    collection_path = pathlib.Path("/repo/tests/ported/not_an_excluded_module.py")
    assert collection_path.name not in FILE_EXCLUSIONS
    assert pytest_ignore_collect(collection_path, config=None) is None
