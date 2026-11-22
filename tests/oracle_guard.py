"""Keep the real XGI out of reach of the ported upstream tests.

The oracle (XGI) is installed in the same environment as this library for the
differential tests. An upstream test that still reached it, through an import
the importer did not rewrite or a `mock.patch("xgi...")` target, would test XGI
against itself and pass. While an upstream test runs (collection, setup, call,
teardown), the module `xgi` is hidden from `sys.modules` and any attempt to
import it raises `OracleImportError` and is recorded in `VIOLATIONS`, which
turns the session's exit status into a failure.
"""

from __future__ import annotations

import contextlib
import importlib.abc
import pathlib
import sys
from collections.abc import Iterator

from tools.import_tests import Lock

ORACLE = "xgi"


class OracleImportError(RuntimeError):
    """An upstream ported test tried to import the real XGI.

    Deliberately not an ImportError: code that tolerates a missing optional
    import must not swallow it.
    """


VIOLATIONS: list[str] = []


def upstream_paths(lock: Lock) -> frozenset[str]:
    """Paths of the upstream files, relative to tests/ported."""
    return frozenset(f.path for f in lock.files)


def is_upstream(path: pathlib.Path, upstream: frozenset[str]) -> bool:
    text = str(path)
    if "/ported/" not in text:
        return False
    return text.split("/ported/", 1)[1] in upstream


def _is_oracle(name: str) -> bool:
    return name == ORACLE or name.startswith(ORACLE + ".")


class _Finder(importlib.abc.MetaPathFinder):
    def __init__(self, where: str) -> None:
        self.where = where

    def find_spec(self, fullname, path=None, target=None):  # type: ignore[no-untyped-def]
        if _is_oracle(fullname):
            VIOLATIONS.append(f"{self.where}: imported {fullname}")
            raise OracleImportError(
                f"{self.where}: an upstream ported test imported {fullname!r}, the real "
                "XGI oracle; the importer must rewrite it to hypergraph_rs"
            )
        return None


@contextlib.contextmanager
def oracle_hidden(where: str) -> Iterator[None]:
    saved = {name: mod for name, mod in sys.modules.items() if _is_oracle(name)}
    for name in saved:
        del sys.modules[name]
    finder = _Finder(where)
    sys.meta_path.insert(0, finder)
    try:
        yield
    finally:
        with contextlib.suppress(ValueError):
            sys.meta_path.remove(finder)
        for name in [n for n in sys.modules if _is_oracle(n)]:
            del sys.modules[name]
        sys.modules.update(saved)
