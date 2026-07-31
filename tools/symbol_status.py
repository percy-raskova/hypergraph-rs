"""The one predicate deciding whether a public symbol is a deferred stub.

Two copies of this logic existed and disagreed. `gen_manifest.py` compared the
module string exactly and raised on an unbound name; `audit_symbols.py` used
`__module__.endswith("_deferred")` against a `getattr(..., None)` lookup, so it
mis-classified any module whose name merely ends in `_deferred` and silently
reported an unbound `__all__` name as `bound`. Both directions are fail-open in
an artifact whose whole purpose is an honest capability claim, and the drift
check cannot catch either: regeneration reproduces the same wrong answer.

NOTE ON THE AXIS: `bound` means "not routed through the `_deferred` stub
factory" — NOT "fully implemented". Some bound symbols raise
`NotImplementedError` from Rust for particular arguments; those are recorded
separately in `compat/partial_symbols.toml`.
"""

from collections.abc import Sequence
from types import ModuleType

DEFERRED_MODULE = "hypergraph_rs._deferred"


class SymbolStatusError(RuntimeError):
    """A namespace declares a name it does not bind; no honest status exists."""


def symbol_status(namespace: ModuleType, names: Sequence[str]) -> dict[str, str]:
    """Classify each name as `bound` or `deferred`, or raise.

    Fails loudly rather than defaulting: an unbound name would otherwise be
    classed `bound`, over-claiming capability.
    """
    out: dict[str, str] = {}
    # Bounded: one pass over the caller's finite name sequence.
    for name in names:
        if not hasattr(namespace, name):
            raise SymbolStatusError(
                f"{namespace.__name__} declares {name!r} but does not bind it; "
                "a status here would over-claim capability"
            )
        obj = getattr(namespace, name)
        module = getattr(obj, "__module__", "") or ""
        out[name] = "deferred" if module == DEFERRED_MODULE else "bound"
    return out
