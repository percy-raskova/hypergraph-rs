# tools/gen_manifest.py
"""Emit python/hypergraph_rs/compat/manifest.json — the single source of truth
(spec §3).

Derived by introspection: which symbols are bound vs deferred, which of real
xgi's dotted paths resolve. Authored: the exclusion registry and the divergence
register. The manifest carries NO test-run outcome — a static capability claim
belongs in the wheel, a run outcome belongs in CI (spec §5.2).
"""

import importlib
import json
import pathlib
import pkgutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from count_tests import file_exclusion_test_counts  # noqa: E402
from symbol_status import SymbolStatusError, symbol_status  # noqa: E402

DEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
XGI_TARGET = "0.10.2"


class ManifestError(RuntimeError):
    """The tree is in a state where an honest manifest cannot be produced."""


def _load_toml(name: str) -> dict:
    path = ROOT / "compat" / name
    with path.open("rb") as fh:
        return tomllib.load(fh)


def require_xgi_target() -> None:
    """Refuse to proceed against an xgi that is not the pinned target.

    Without this the whole gate keeps producing a score, and that score
    silently stops meaning "conformance to 0.10.2" — the failure mode is a
    number that still looks authoritative.
    """
    try:
        import xgi
    except ModuleNotFoundError as exc:
        raise ManifestError(
            f"real xgi must be installed (pip install xgi=={XGI_TARGET}): {exc}"
        ) from exc
    actual = getattr(xgi, "__version__", "unknown")
    if actual != XGI_TARGET:
        raise ManifestError(
            f"installed xgi is {actual}, but this port targets {XGI_TARGET}; "
            "a conformance score measured against a different xgi is not a "
            "conformance score. Install the target or update XGI_TARGET."
        )


def _dotted_paths(pkg_name: str) -> set[str]:
    """Every dotted submodule path under `pkg_name`, relative to it."""
    mod = importlib.import_module(pkg_name)
    out: set[str] = set()
    # Bounded: pkgutil walks a finite package tree.
    for _, name, _ in pkgutil.walk_packages(mod.__path__, pkg_name + "."):
        out.add(name.replace(pkg_name + ".", "", 1))
    return out


def _symbol_status() -> dict[str, str]:
    """`bound` vs `deferred` for every name in the package's `__all__`.

    See tools/symbol_status.py for the axis definition and why this fails loudly.
    """
    import hypergraph_rs

    try:
        return symbol_status(hypergraph_rs, hypergraph_rs.__all__)
    except SymbolStatusError as exc:
        raise ManifestError(str(exc)) from exc


def _partial_symbols() -> list[dict]:
    return _load_toml("partial_symbols.toml").get("partial_symbol", [])


def partial_symbol_names() -> set[str]:
    """The symbols recorded in compat/partial_symbols.toml.

    One derivation, two consumers: build() promotes them to `bound-partial`,
    and audit_symbols.py separates them out of the plain-bound ledger. When
    these drifted apart, two generated artifacts disagreed.
    """
    return {entry["symbol"] for entry in _partial_symbols()}


def _promote_partials(status: dict[str, str], partials: list[dict]) -> dict[str, str]:
    """Mark bound-but-raising symbols `bound-partial`.

    A symbol recorded partial that the package reports `deferred` is a
    contradiction — one of the two records is stale — so it raises rather than
    silently picking a winner.
    """
    out = dict(status)
    # Bounded: one pass over the authored register.
    for entry in partials:
        symbol = entry["symbol"]
        if symbol not in out:
            raise ManifestError(
                f"partial_symbols.toml names {symbol!r}, which is not in "
                "hypergraph_rs.__all__"
            )
        if out[symbol] == "deferred":
            raise ManifestError(
                f"{symbol!r} is recorded partial but is a deferred stub; "
                "delete the partial entry or the stub"
            )
        out[symbol] = "bound-partial"
    return out


def build() -> dict:
    require_xgi_target()
    import hypergraph_rs

    exclusions = _load_toml("exclusions.toml")
    divergences = _load_toml("divergences.toml")
    partials = _partial_symbols()

    ours = _dotted_paths("hypergraph_rs")
    theirs = _dotted_paths("xgi")

    return {
        "schema_version": 2,
        "hypergraph_rs_version": getattr(hypergraph_rs, "__version__", "unknown"),
        "xgi_target": XGI_TARGET,
        "symbols": _promote_partials(_symbol_status(), partials),
        "partial_symbols": partials,
        "namespaces": {
            "expected": sorted(theirs),
            "resolved": sorted(theirs & ours),
            "missing": sorted(theirs - ours),
        },
        "divergences": divergences.get("divergence", []),
        "divergences_status": (
            "Populated from compat/divergences.toml, measured against real xgi "
            f"{XGI_TARGET} rather than transcribed from the D1-D17 register, "
            "which covers under half of the live Python-visible surface. Each "
            "entry is a difference that is DELIBERATE and OBSERVABLE; deferred "
            "symbols and missing namespaces are gaps, recorded separately. "
            "A symbol absent from this list is NOT thereby verified equivalent."
        ),
        "symbols_status": (
            "`bound` means 'not routed through the _deferred stub factory', NOT "
            "'fully implemented'. Some bound symbols raise for particular "
            "arguments (from Rust) or environments (from Python-side import "
            "gates); only those MEASURED so far are marked `bound-partial` and "
            "recorded in `partial_symbols`. Read `symbols` together with "
            "`exclusions`, `partial_symbols` and `divergences`."
        ),
        "partial_symbols_status": (
            "A measured, INCOMPLETE record. Absence from this list is NOT "
            "evidence a symbol is free of raising paths — it may simply be "
            "unmeasured. Every entry here was observed raising, in the "
            "environment its trigger names."
        ),
        "exclusions": {
            "files": exclusions.get("file_exclusion", []),
            "tests": exclusions.get("test_exclusion", []),
            "invisible_test_counts": file_exclusion_test_counts(),
        },
    }


def main() -> int:
    try:
        data = build()
    except ManifestError as exc:
        raise SystemExit(str(exc)) from exc
    DEST.parent.mkdir(parents=True, exist_ok=True)
    DEST.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    print(f"wrote {DEST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
