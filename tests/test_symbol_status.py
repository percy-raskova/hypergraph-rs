# tests/test_symbol_status.py
"""The deferred-stub predicate must pin the module identity, not a name suffix."""

import types

import pytest

from tools.symbol_status import DEFERRED_MODULE, SymbolStatusError, symbol_status


def _ns(name: str, **attrs: object) -> types.ModuleType:
    mod = types.ModuleType(name)
    for key, value in attrs.items():
        setattr(mod, key, value)
    return mod


def _obj_from(module_name: str) -> type:
    cls = type("Probe", (), {})
    cls.__module__ = module_name
    return cls


def test_real_stub_is_deferred() -> None:
    ns = _ns("fake", T=_obj_from(DEFERRED_MODULE))
    assert symbol_status(ns, ["T"]) == {"T": "deferred"}


def test_module_merely_ending_in_deferred_is_bound() -> None:
    """The endswith() predicate answers 'deferred' here. That is the bug."""
    ns = _ns("fake", T=_obj_from("hypergraph_rs.legacy_not_deferred"))
    assert symbol_status(ns, ["T"]) == {"T": "bound"}


def test_ordinary_symbol_is_bound() -> None:
    ns = _ns("fake", T=_obj_from("hypergraph_rs._hypergraph_rs_core"))
    assert symbol_status(ns, ["T"]) == {"T": "bound"}


def test_declared_but_unbound_name_raises() -> None:
    """getattr(ns, name, None) would silently class this 'bound'."""
    ns = _ns("fake")
    with pytest.raises(SymbolStatusError, match="ghost"):
        symbol_status(ns, ["ghost"])


def test_object_without_module_attribute_is_bound() -> None:
    ns = _ns("fake", T=object())
    assert symbol_status(ns, ["T"]) == {"T": "bound"}
