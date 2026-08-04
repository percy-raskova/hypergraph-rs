# tests/test_cross_register.py
"""Artifacts that describe the same symbol must not contradict each other."""

import json
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).parent.parent
MANIFEST = ROOT / "python" / "hypergraph_rs" / "compat" / "manifest.json"
LEDGER = ROOT / "tests" / "DEFERRED_SYMBOLS.md"


def _manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def _test_exclusions() -> list[dict]:
    """`test_exclusion` ONLY, deliberately.

    `file_exclusion` entries are keyed on a path, not a test name, so the
    embeds-the-symbol-under-test convention this guard relies on does not
    hold for them — a path could contain a bound-partial symbol's name as a
    pure substring accident, with no contradiction behind it. (`drawing/
    test_draw.py` itself was NOT such a case: before 2026-08-04 it was classed
    `deferred-symbol` while the manifest called `draw`/`draw_nodes`/
    `draw_hyperedges` bound-partial — a genuine contradiction, not a
    collision, fixed by reclassing the file exclusion to `divergence` in
    compat/exclusions.toml rather than by widening this guard.) Test names are
    written to embed the symbol under test, so the heuristic is sound only
    here.
    """
    with (ROOT / "compat" / "exclusions.toml").open("rb") as fh:
        return tomllib.load(fh).get("test_exclusion", [])


def _partial_symbols() -> set[str]:
    return {n for n, s in _manifest()["symbols"].items() if s == "bound-partial"}


def _symbol_is_named_in(symbol: str, test_name: str) -> bool:
    """Snake_case, token-boundary-aware, deliberately stricter than a bare
    substring check: true only if `symbol`'s underscore-separated tokens
    appear as a contiguous run of tokens in `test_name`.

    What this catches: `degree_assortativity` inside
    `test_degree_assortativity_extra` (a real reference to the symbol). What
    a bare substring check would wrongly also flag: `draw` inside
    `test_drawing_something` — `drawing` is one token, not `draw` followed by
    a boundary, so this rejects it.

    What this CANNOT catch, by construction: a CamelCase symbol
    (`Hypergraph`, `DiHypergraph`) against a conventionally-lowercase
    snake_case pytest test name. The comparison is case-sensitive and pytest
    test names do not carry CamelCase tokens, so those two symbols are
    structurally unreachable by this guard regardless of matching strategy —
    a real blind spot, not one this function closes.
    """
    test_tokens = test_name.split("_")
    symbol_tokens = symbol.split("_")
    n = len(symbol_tokens)
    return any(
        test_tokens[i : i + n] == symbol_tokens
        for i in range(len(test_tokens) - n + 1)
    )


def test_the_ledger_does_not_call_a_partial_symbol_plainly_bound() -> None:
    """DEFERRED_SYMBOLS.md is generated. Before this task it listed every
    partial symbol under 'Bound symbols' with no marker, so two generated
    artifacts disagreed."""
    ledger = LEDGER.read_text()
    assert "# Bound symbols" in ledger, "the ledger's section headings changed"
    bound_section = ledger.split("# Bound symbols")[1]
    for symbol in sorted(_partial_symbols()):
        assert f"- `{symbol}`\n" not in bound_section, (
            f"{symbol} is bound-partial in the manifest but the ledger lists "
            f"it as plainly bound"
        )


def test_the_ledger_names_every_partial_symbol() -> None:
    ledger = LEDGER.read_text()
    for symbol in sorted(_partial_symbols()):
        assert f"- `{symbol}`\n" in ledger, f"{symbol} vanished from the ledger"


def test_no_exclusion_calls_a_bound_partial_symbol_deferred() -> None:
    """Name-based, deliberately: a test exclusion whose test name embeds a
    symbol the manifest calls bound-partial must not be classed
    `deferred-symbol`. The symbol is bound — only some argument shapes or
    environments raise. See `_symbol_is_named_in` for what "embeds" means
    here, and what it structurally cannot detect."""
    partial = _partial_symbols()
    for excl in _test_exclusions():
        if excl["class"] != "deferred-symbol":
            continue
        for symbol in sorted(partial):
            assert not _symbol_is_named_in(symbol, excl["test"]), (
                f"{excl['test']} is classed deferred-symbol, but the manifest "
                f"calls {symbol} bound-partial — the binding exists"
            )
