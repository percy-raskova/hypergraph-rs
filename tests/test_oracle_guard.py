"""The guard that keeps upstream ported tests away from the real XGI oracle."""

import contextlib
import importlib
import sys
import types
import unittest.mock

import pytest
from oracle_guard import (
    VIOLATIONS,
    OracleImportError,
    oracle_hidden,
    upstream_paths,
)

from tools.import_tests import read_lock


def test_oracle_import_inside_the_guard_raises():
    import xgi  # the real oracle, loaded outside the guard

    before = len(VIOLATIONS)
    try:
        with oracle_hidden("t"):
            with pytest.raises(OracleImportError, match="xgi"):
                importlib.import_module("xgi")
        assert sys.modules["xgi"] is xgi
    finally:
        del VIOLATIONS[before:]


def test_patch_of_the_oracle_inside_the_guard_raises():
    before = len(VIOLATIONS)
    with oracle_hidden("t"), pytest.raises(OracleImportError):
        unittest.mock.patch("xgi.Hypergraph").start()
    del VIOLATIONS[before:]


def test_the_guard_error_is_not_an_import_error():
    assert not issubclass(OracleImportError, ImportError)


def test_violations_are_recorded():
    before = len(VIOLATIONS)
    with oracle_hidden("here"), contextlib.suppress(OracleImportError):
        importlib.import_module("xgi.core")
    assert len(VIOLATIONS) == before + 1 and VIOLATIONS[-1].startswith("here")
    del VIOLATIONS[before:]


def test_upstream_paths_come_from_the_lock():
    up = upstream_paths(read_lock())
    assert "core/test_hypergraph.py" in up and "test_property_differential.py" not in up


def test_patch_of_the_oracle_fails_the_session():
    import conftest

    assert hasattr(conftest, "pytest_runtest_protocol")
    assert hasattr(conftest, "pytest_make_collect_report")
    # sessionfinish turns a recorded violation into a failed exit status, before its early
    # return for a run with no terminal reporter:
    config = types.SimpleNamespace(
        pluginmanager=types.SimpleNamespace(get_plugin=lambda name: None)
    )
    session = types.SimpleNamespace(exitstatus=0, config=config)
    VIOLATIONS.append("x: imported xgi")
    try:
        conftest.pytest_sessionfinish(session, 0)
        assert session.exitstatus == pytest.ExitCode.TESTS_FAILED
    finally:
        VIOLATIONS.pop()
