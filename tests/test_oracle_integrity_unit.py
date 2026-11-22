"""The oracle comparison must notice a MISSING file, not only a drifted one."""

import pathlib

from test_oracle_integrity import _allowlist_verdict, _compare_tree


def test_missing_ported_file_is_reported(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    (upstream / "sub").mkdir(parents=True)
    ported.mkdir()
    (upstream / "sub" / "test_x.py").write_text("import xgi\nassert True\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == ["sub/test_x.py"]
    assert drifted == []


def test_compare_tree_reports_a_ported_file_left_unrewritten(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    upstream.mkdir()
    ported.mkdir()
    (upstream / "test_y.py").write_text("import xgi\n")
    (ported / "test_y.py").write_text("import xgi\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == []
    assert drifted == ["test_y.py"]


def test_compare_tree_accepts_the_rewrite(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    upstream.mkdir()
    ported.mkdir()
    (upstream / "test_z.py").write_text("import xgi\nfrom xgi import Hypergraph\n")
    (ported / "test_z.py").write_text(
        "import hypergraph_rs as xgi\nfrom hypergraph_rs import Hypergraph\n"
    )

    missing, drifted = _compare_tree(upstream, ported)

    assert (missing, drifted) == ([], [])


def test_allowlisted_missing_file_is_suppressed() -> None:
    """A legitimate UNPORTED entry must actually suppress its would-be failure."""
    unexplained, stale = _allowlist_verdict(
        ["sub/test_x.py"], {"sub/test_x.py": "some recorded reason"}
    )

    assert unexplained == []
    assert stale == []


def test_unallowlisted_missing_file_is_not_suppressed() -> None:
    """Suppression must not be blanket: a missing file with no allowlist entry
    at all still surfaces in `unexplained`."""
    unexplained, stale = _allowlist_verdict(["sub/test_x.py"], {})

    assert unexplained == ["sub/test_x.py"]
    assert stale == []


def test_stale_allowlist_entry_is_reported() -> None:
    """An UNPORTED entry naming a file that IS present must trip staleness."""
    unexplained, stale = _allowlist_verdict([], {"sub/test_x.py": "no longer missing"})

    assert unexplained == []
    assert stale == ["sub/test_x.py"]


def test_empty_missing_and_empty_allowlist_is_clean() -> None:
    """Pins today's live case: nothing missing, nothing allowlisted."""
    unexplained, stale = _allowlist_verdict([], {})

    assert (unexplained, stale) == ([], [])


from test_oracle_integrity import SENTINEL, _upstream_prefix


def test_prefix_split_returns_everything_above_the_sentinel() -> None:
    body = b"import xgi\nFIXTURE = 1\n" + SENTINEL + b"\nREPO = 2\n"
    assert _upstream_prefix(body) == b"import xgi\nFIXTURE = 1\n"


def test_missing_sentinel_raises() -> None:
    import pytest

    with pytest.raises(AssertionError, match="sentinel"):
        _upstream_prefix(b"import xgi\nno sentinel here\n")


def test_duplicated_sentinel_raises() -> None:
    import pytest

    body = b"a\n" + SENTINEL + b"\nb\n" + SENTINEL + b"\nc\n"
    with pytest.raises(AssertionError, match="exactly once"):
        _upstream_prefix(body)
