#!/usr/bin/env python3
"""Compare hypergraph_rs's public surface against XGI 0.10.2's 164 symbols; emit the ledger.

Fails if __all__ != XGI's 164. Classifies each symbol as bound or deferred (a stub whose
__module__ endswith '_deferred'). Writes tests/DEFERRED_SYMBOLS.md.
"""
import importlib
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))


def main():
    xgi = importlib.import_module(
        "xgi", package=None)  # the installed oracle; used only to read the canonical 164 names
    hr = importlib.import_module("hypergraph_rs")
    want, have = set(xgi.__all__), set(hr.__all__)
    assert have == want, f"surface mismatch: missing={want-have} extra={have-want}"

    from symbol_status import symbol_status

    status = symbol_status(hr, sorted(want))
    deferred = [n for n, s in status.items() if s == "deferred"]
    bound = [n for n, s in status.items() if s == "bound"]
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
