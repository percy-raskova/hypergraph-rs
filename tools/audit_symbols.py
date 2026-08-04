#!/usr/bin/env python3
"""Compare hypergraph_rs's public surface against XGI 0.10.2's 164 symbols; emit the ledger.

Fails if __all__ != XGI's 164. Classifies each symbol as bound or deferred (a stub whose
__module__ is exactly the `_deferred` stub factory module). Writes tests/DEFERRED_SYMBOLS.md.
"""
import importlib


def main():
    xgi = importlib.import_module(
        "xgi", package=None)  # the installed oracle; used only to read the canonical 164 names

    from tools.gen_manifest import require_xgi_target

    require_xgi_target()

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
    with open("tests/DEFERRED_SYMBOLS.md", "w") as f:
        f.write(f"# Deferred symbols ({len(deferred)}/164 not built by any phase)\n\n")
        for n in deferred:
            f.write(f"- `{n}` — no Phase 0-6 plan built it; calling raises NotImplementedError.\n")
        f.write(f"\n# Bound-partial symbols ({len(partials)}/164)\n\n")
        f.write("These are bound, but some argument shapes or environments "
                "raise. See `compat/partial_symbols.toml` for the exact "
                "triggers.\n\n")
        for n in partials:
            f.write(f"- `{n}`\n")
        f.write(f"\n# Bound symbols ({len(bound)}/164)\n\n")
        f.write("Bound is a BINDING fact, not a behaviour one: it means the "
                "symbol is not routed through the `_deferred` stub factory. "
                "Absence from the partial list above is not evidence a symbol "
                "never raises — it may simply be unmeasured.\n\n")
        for n in bound:
            f.write(f"- `{n}`\n")
    print(f"bound={len(bound)} partial={len(partials)} deferred={len(deferred)} "
          f"total={len(bound)+len(partials)+len(deferred)}")


if __name__ == "__main__":
    main()
