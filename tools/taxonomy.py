"""The closed exclusion/divergence/partial-symbol taxonomy — one definition,
three registries.

The set was previously inline in tests/ported/test_gate_summary.py.
compat/exclusions.toml, compat/divergences.toml, and compat/partial_symbols.toml
are all validated against it, so the three files cannot drift into private
vocabularies.
"""

CLASSES: frozenset[str] = frozenset(
    {
        "deferred-symbol",
        "core-gap",
        "binding-gap",
        "divergence",
        "float-parity",
        "upstream-private",
        "env",
        "upstream-bug",
        "out-of-scope",
    }
)
