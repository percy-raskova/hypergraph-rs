# tests/test_xgi_version_pin.py
"""A conformance score against the wrong xgi is not a conformance score."""

import pathlib
import sys

import pytest
import xgi

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

import gen_manifest


def test_matching_version_passes() -> None:
    assert xgi.__version__ == gen_manifest.XGI_TARGET
    gen_manifest.require_xgi_target()


def test_mismatched_version_raises_naming_both(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(xgi, "__version__", "9.9.9", raising=False)
    with pytest.raises(gen_manifest.ManifestError) as excinfo:
        gen_manifest.require_xgi_target()
    message = str(excinfo.value)
    assert "9.9.9" in message
    assert gen_manifest.XGI_TARGET in message


def test_build_refuses_on_mismatch(monkeypatch: pytest.MonkeyPatch) -> None:
    """The check must gate build(), not merely exist."""
    monkeypatch.setattr(xgi, "__version__", "9.9.9", raising=False)
    with pytest.raises(gen_manifest.ManifestError):
        gen_manifest.build()
