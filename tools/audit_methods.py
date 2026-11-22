#!/usr/bin/env python3
"""Audit the public methods of the library's classes, views and stats against XGI.

`tools/audit_symbols.py` compares `__all__` only. This compares `dir()` of the same object
built the same way on both sides, and lists every public name XGI has that the library
lacks. Names the library has and XGI lacks are the library's own and are not audited
(Task 15 of plans/2026-11-21-xgi-1-1-raise.md handles the view methods XGI 1.1 forbids).
Registered in `compat/method_gaps.toml`; see `tests/test_method_audit.py`.

Callers must call `tools.gen_manifest.require_xgi_target()` first: this only imports xgi
to build the objects, it does not guard the version.
"""

import importlib


def _subjects_of(mod: object) -> list[tuple[str, object]]:
    h = mod.Hypergraph([[1, 2], [2, 3]])  # type: ignore[attr-defined]
    d = mod.DiHypergraph([([1], [2])])  # type: ignore[attr-defined]
    s = mod.SimplicialComplex([[1, 2, 3]])  # type: ignore[attr-defined]
    return [
        ("Hypergraph", h),
        ("Hypergraph.nodes", h.nodes),
        ("Hypergraph.edges", h.edges),
        ("Hypergraph.nodes.degree", h.nodes.degree),
        ("Hypergraph.edges.order", h.edges.order),
        ("DiHypergraph", d),
        ("DiHypergraph.nodes", d.nodes),
        ("DiHypergraph.edges", d.edges),
        ("SimplicialComplex", s),
        ("SimplicialComplex.nodes", s.nodes),
        ("SimplicialComplex.edges", s.edges),
    ]


def subjects() -> list[tuple[str, object, object]]:
    """(label, ours, theirs), each built the same way on both sides."""
    ours = _subjects_of(importlib.import_module("hypergraph_rs"))
    theirs = _subjects_of(importlib.import_module("xgi"))
    return [(lo, o, t) for (lo, o), (_, t) in zip(ours, theirs, strict=True)]


def _public(obj: object) -> set[str]:
    return {n for n in dir(obj) if not n.startswith("_")}


def missing_methods() -> dict[str, set[str]]:
    """label -> public names in dir(theirs) and not in dir(ours); labels with none are left out."""
    out: dict[str, set[str]] = {}
    for label, ours, theirs in subjects():
        gone = _public(theirs) - _public(ours)
        if gone:
            out[label] = gone
    return out


if __name__ == "__main__":
    from tools.gen_manifest import require_xgi_target

    require_xgi_target()
    for label, names in missing_methods().items():
        for name in sorted(names):
            print(f"{label}\t{name}")
