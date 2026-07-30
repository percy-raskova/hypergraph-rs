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

DEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
XGI_TARGET = "0.10.2"


def _load_toml(name: str) -> dict:
    path = ROOT / "compat" / name
    with path.open("rb") as fh:
        return tomllib.load(fh)


def _dotted_paths(pkg_name: str) -> set[str]:
    """Every dotted submodule path under `pkg_name`, relative to it."""
    mod = importlib.import_module(pkg_name)
    out: set[str] = set()
    # Bounded: pkgutil walks a finite package tree.
    for _, name, _ in pkgutil.walk_packages(mod.__path__, pkg_name + "."):
        out.add(name.replace(pkg_name + ".", "", 1))
    return out


def _symbol_status() -> dict[str, str]:
    """`bound` vs `deferred` for every name in the package's __all__."""
    import hypergraph_rs

    out: dict[str, str] = {}
    # Bounded: one pass over __all__.
    for name in hypergraph_rs.__all__:
        obj = getattr(hypergraph_rs, name, None)
        mod = getattr(obj, "__module__", "") or ""
        out[name] = "deferred" if mod.endswith("_deferred") else "bound"
    return out


def build() -> dict:
    import hypergraph_rs

    exclusions = _load_toml("exclusions.toml")
    divergences = _load_toml("divergences.toml")

    ours = _dotted_paths("hypergraph_rs")
    try:
        theirs = _dotted_paths("xgi")
    except ModuleNotFoundError as exc:  # explicit: xgi is a hard requirement here
        raise SystemExit(
            "real xgi must be installed to generate the manifest "
            f"(pip install xgi=={XGI_TARGET}): {exc}"
        ) from exc

    return {
        "schema_version": 1,
        "hypergraph_rs_version": getattr(hypergraph_rs, "__version__", "unknown"),
        "xgi_target": XGI_TARGET,
        "symbols": _symbol_status(),
        "namespaces": {
            "expected": sorted(theirs),
            "resolved": sorted(theirs & ours),
            "missing": sorted(theirs - ours),
        },
        "divergences": divergences.get("divergence", []),
        "exclusions": {
            "files": exclusions.get("file_exclusion", []),
            "tests": exclusions.get("test_exclusion", []),
            "invisible_test_counts": file_exclusion_test_counts(),
        },
    }


def main() -> int:
    DEST.parent.mkdir(parents=True, exist_ok=True)
    DEST.write_text(json.dumps(build(), indent=2, sort_keys=True) + "\n")
    print(f"wrote {DEST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
