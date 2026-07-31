"""The oracle comparison must notice a MISSING file, not only a drifted one."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

from test_oracle_integrity import _compare_tree


def test_missing_ported_file_is_reported(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    (upstream / "sub").mkdir(parents=True)
    ported.mkdir()
    (upstream / "sub" / "test_x.py").write_text("import xgi\nassert True\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == ["sub/test_x.py"]
    assert drifted == []


def test_drifted_file_is_reported(tmp_path: pathlib.Path) -> None:
    upstream = tmp_path / "up"
    ported = tmp_path / "ported"
    upstream.mkdir()
    ported.mkdir()
    (upstream / "test_y.py").write_text("import xgi\nassert 1 == 1\n")
    (ported / "test_y.py").write_text("import hypergraph_rs as xgi\nassert 1 == 2\n")

    missing, drifted = _compare_tree(upstream, ported)

    assert missing == []
    assert drifted == ["test_y.py"]


def test_import_swap_alone_is_not_drift(tmp_path: pathlib.Path) -> None:
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
