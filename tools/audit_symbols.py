#!/usr/bin/env python3
"""Compare hypergraph_rs's public surface against XGI 0.10.2's 164 symbols; emit the ledger.

Fails if __all__ != XGI's 164. Classifies each symbol as bound or deferred (a stub whose
__module__ endswith '_deferred'). Writes tests/DEFERRED_SYMBOLS.md.
"""
import importlib


def main():
    xgi = importlib.import_module(
        "xgi", package=None)  # the installed oracle; used only to read the canonical 164 names
    hr = importlib.import_module("hypergraph_rs")
    want, have = set(xgi.__all__), set(hr.__all__)
    assert have == want, f"surface mismatch: missing={want-have} extra={have-want}"
    deferred, bound = [], []
    for name in sorted(want):
        obj = getattr(hr, name, None)
        is_stub = getattr(obj, "__module__", "").endswith("_deferred")
        (deferred if is_stub else bound).append(name)
    with open("tests/DEFERRED_SYMBOLS.md", "w") as f:
        f.write(f"# Deferred symbols ({len(deferred)}/164 not built by any phase)\n\n")
        for n in deferred:
            f.write(f"- `{n}` — no Phase 0-6 plan built it; calling raises NotImplementedError.\n")
        f.write(f"\n# Bound symbols ({len(bound)}/164)\n\n")
        for n in bound:
            f.write(f"- `{n}`\n")
    print(f"bound={len(bound)} deferred={len(deferred)} total={len(bound)+len(deferred)}")


if __name__ == "__main__":
    main()
