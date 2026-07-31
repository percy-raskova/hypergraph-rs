"""The closed exclusion/divergence taxonomy — one definition, two registries.

The set was previously inline in tests/ported/test_gate_summary.py. Both
compat/exclusions.toml and compat/divergences.toml are validated against it, so
the two files cannot drift into private vocabularies.
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
