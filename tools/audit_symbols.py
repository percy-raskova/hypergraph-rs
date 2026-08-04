#!/usr/bin/env python3
"""Compare hypergraph_rs's public surface against XGI 0.10.2's 164 symbols; emit the ledger.

Fails if __all__ != XGI's 164. Classifies each symbol into one of three sections: bound,
bound-partial (bound but recorded raising for some argument shape or environment — see
compat/partial_symbols.toml), or deferred (a stub whose __module__ is exactly the
`_deferred` stub factory module). Writes tests/DEFERRED_SYMBOLS.md — or, with --check,
verifies the committed copy still matches the generated ledger without writing, so a stale
ledger fails loudly instead of being silently regenerated (the guard `mise run py:gate`
uses; analogous to tests/test_manifest_drift.py for manifest.json). Regenerate deliberately
with `mise run py:symbols`.
"""
import argparse
import importlib
import pathlib

ROOT = pathlib.Path(__file__).parent.parent
DEST = ROOT / "tests" / "DEFERRED_SYMBOLS.md"


def _ledger_text() -> tuple[str, dict[str, int]]:
    """Pure: compute the ledger's full text and its three section counts.

    Callers must call require_xgi_target() first — this only imports xgi to
    read its __all__, it does not guard the version.
    """
    xgi = importlib.import_module(
        "xgi", package=None)  # the installed oracle; used only to read the canonical 164 names
    hr = importlib.import_module("hypergraph_rs")
    want, have = set(xgi.__all__), set(hr.__all__)
    assert have == want, f"surface mismatch: missing={want-have} extra={have-want}"

    from tools.gen_manifest import partial_symbol_names
    from tools.symbol_status import symbol_status

    status = symbol_status(hr, sorted(want))
    partial = partial_symbol_names()
    deferred = [n for n, s in status.items() if s == "deferred"]
    bound = [n for n, s in status.items() if s == "bound" and n not in partial]
    partials = [n for n, s in status.items() if s == "bound" and n in partial]

    lines = [f"# Deferred symbols ({len(deferred)}/164 not built by any phase)\n\n"]
    for n in deferred:
        lines.append(f"- `{n}` — no Phase 0-6 plan built it; calling raises NotImplementedError.\n")
    lines.append(f"\n# Bound-partial symbols ({len(partials)}/164)\n\n")
    lines.append("These are bound, but some argument shapes or environments "
                  "raise. See `compat/partial_symbols.toml` for the exact "
                  "triggers.\n\n")
    for n in partials:
        lines.append(f"- `{n}`\n")
    lines.append(f"\n# Bound symbols ({len(bound)}/164)\n\n")
    lines.append("Bound is a BINDING fact, not a behaviour one: it means the "
                  "symbol is not routed through the `_deferred` stub factory. "
                  "Absence from the partial list above is not evidence a symbol "
                  "never raises — it may simply be unmeasured.\n\n")
    for n in bound:
        lines.append(f"- `{n}`\n")

    counts = {"bound": len(bound), "partial": len(partials), "deferred": len(deferred)}
    return "".join(lines), counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true",
        help=f"verify {DEST} matches the generated ledger without writing it; "
             "exit non-zero on drift instead of silently regenerating",
    )
    args = parser.parse_args()

    from tools.gen_manifest import require_xgi_target

    require_xgi_target()

    text, counts = _ledger_text()
    total = counts["bound"] + counts["partial"] + counts["deferred"]
    summary = (f"bound={counts['bound']} partial={counts['partial']} "
               f"deferred={counts['deferred']} total={total}")

    if args.check:
        committed = DEST.read_text()
        if committed != text:
            raise SystemExit(
                f"{DEST} is stale — regenerate with `mise run py:symbols` and "
                "commit the result. If the diff is a symbol that became bound, "
                "bound-partial, or deferred, that's expected; the ledger just "
                "needs to catch up."
            )
        print(f"{DEST} matches the generated ledger: {summary}")
        return 0

    DEST.write_text(text)
    print(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
