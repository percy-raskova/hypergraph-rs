"""The importer of XGI's tests: its rewrite, its refusals and its pruning."""

import pathlib
import subprocess

import pytest

from tools import import_tests as it
from tools.import_tests import (
    PORTED,
    UpstreamImportError,
    commit_for,
    import_tests,
    leftovers,
    read_lock,
    rewrite,
    sentinel_line,
)


def test_rewrite_swaps_the_root_import():
    assert rewrite("import xgi\n") == "import hypergraph_rs as xgi\n"


def test_rewrite_swaps_from_imports_at_any_indent():
    src = "from xgi.exception import XGIError\n    from xgi.stats import _plot_hist\nfrom xgi import a\n"
    assert rewrite(src) == (
        "from hypergraph_rs.exception import XGIError\n"
        "    from hypergraph_rs.stats import _plot_hist\nfrom hypergraph_rs import a\n"
    )


def test_rewrite_swaps_quoted_module_targets():
    src = 'patch("xgi.readwrite.ahorn_data.request_from_url")\npatch(\'xgi.utils.request_from_url\')\n'
    assert rewrite(src) == (
        'patch("hypergraph_rs.readwrite.ahorn_data.request_from_url")\n'
        "patch('hypergraph_rs.utils.request_from_url')\n"
    )


def test_rewrite_leaves_other_text_alone():
    src = 'H = xgi.Hypergraph()\nmsg = "import xgi took 3s"\nf = load_xgi_data\n# from xgi import x\n'
    assert rewrite(src) == src


def test_leftovers_finds_each_unrewritten_reference():
    src = 'import xgi\nfrom xgi import a\nimport xgi.core\np = "xgi.utils.f"\nq = f"xgi.{p}"\n'
    assert [x.split(":")[0] for x in leftovers(src)] == [
        "line 1",
        "line 2",
        "line 3",
        "line 4",
        "line 5",
    ]


def test_leftovers_is_empty_for_every_ported_upstream_file():
    for f in read_lock().files:
        assert leftovers((PORTED / f.path).read_text()) == [], f.path


def test_unknown_tag_is_refused():
    with pytest.raises(UpstreamImportError, match="v9.9"):
        commit_for("v9.9")


def test_sentinel_line_names_the_version():
    assert (
        sentinel_line("1.1")
        == "# === END VERBATIM XGI 1.1 conftest — repo-authored code below this line ==="
    )


def _git(repo: pathlib.Path, *args: str) -> str:
    result = subprocess.run(
        [
            "git",
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-C",
            str(repo),
            *args,
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def _commit(repo: pathlib.Path, tag: str) -> str:
    _git(repo, "add", "-A")
    _git(repo, "commit", "-q", "-m", tag)
    _git(repo, "tag", tag)
    return _git(repo, "rev-parse", "HEAD")


def _make_repo(tmp_path: pathlib.Path) -> tuple[pathlib.Path, dict[str, str]]:
    repo = tmp_path / "xgi"
    (repo / "tests" / "a").mkdir(parents=True)
    _git(repo, "init", "-q")
    (repo / "tests" / "conftest.py").write_text("import xgi\n")
    (repo / "tests" / "a" / "test_x.py").write_text("import xgi\n")
    first = _commit(repo, "vT")
    (repo / "tests" / "a" / "test_x.py").unlink()
    (repo / "tests" / "b").mkdir()
    (repo / "tests" / "b" / "test_y.py").write_text('patch("xgi.f")\n')
    second = _commit(repo, "vU")
    return repo, {"vT": first, "vU": second}


def test_import_prunes_only_upstream_files_and_keeps_repo_code(tmp_path, monkeypatch):
    repo, shas = _make_repo(tmp_path)
    monkeypatch.setattr(it, "KNOWN_TAGS", shas)
    ported = tmp_path / "ported"
    ported.mkdir()
    (ported / "test_mine.py").write_text("# repo-authored\n")
    conftest = tmp_path / "conftest.py"
    conftest.write_text("OLD\n" + sentinel_line("T") + "\nREPO = 1\n")
    lock = tmp_path / "lock.toml"

    import_tests("vT", repo=repo, ported=ported, conftest=conftest, lock=lock)
    assert (ported / "a" / "test_x.py").exists()
    import_tests("vU", repo=repo, ported=ported, conftest=conftest, lock=lock)

    assert not (ported / "a" / "test_x.py").exists()
    assert (ported / "b" / "test_y.py").read_text() == 'patch("hypergraph_rs.f")\n'
    assert (ported / "test_mine.py").exists()
    assert conftest.read_text() == (
        "import hypergraph_rs as xgi\n" + sentinel_line("U") + "\nREPO = 1\n"
    )
    assert [f.path for f in read_lock(lock).files] == ["b/test_y.py"]


def test_a_data_file_under_tests_is_refused(tmp_path, monkeypatch):
    repo = tmp_path / "xgi"
    (repo / "tests" / "data").mkdir(parents=True)
    _git(repo, "init", "-q")
    (repo / "tests" / "conftest.py").write_text("import xgi\n")
    (repo / "tests" / "data" / "x.json").write_text("{}\n")
    sha = _commit(repo, "vD")
    monkeypatch.setattr(it, "KNOWN_TAGS", {"vD": sha})
    ported = tmp_path / "ported"
    ported.mkdir()
    conftest = tmp_path / "conftest.py"
    conftest.write_text(sentinel_line("D") + "\n")
    with pytest.raises(UpstreamImportError, match="data/x.json"):
        import_tests("vD", repo=repo, ported=ported, conftest=conftest, lock=tmp_path / "l.toml")
    assert list(ported.iterdir()) == []
