"""The closed exclusion/divergence/partial-symbol taxonomy — one definition,
three registries.

The set was previously inline in tests/ported/test_gate_summary.py.
compat/exclusions.toml, compat/divergences.toml, and compat/partial_symbols.toml
are all validated against it, so the three files cannot drift into private
vocabularies.
"""

import re

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

OWNER_PATTERN = re.compile(r"^(xgi-1-1-raise:T(1[0-9]|20)|later:(online-loaders|stat-functions|doc-examples))$")
"""Who clears an exclusion entry: a later task of the raise to XGI 1.1 (plans/2026-11-21-xgi-1-1-raise.md)
or a later plan of its own. An entry with no owner is a standing gap that the raise does not close."""
