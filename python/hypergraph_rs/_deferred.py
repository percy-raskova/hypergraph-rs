"""Factory for symbols no Rust phase implemented (see tests/DEFERRED_SYMBOLS.md).
They import cleanly (so test modules collect) but raise on call."""


def make_deferred(name):
    def _deferred(*args, **kwargs):
        raise NotImplementedError(
            f"deferred: no hypergraph-rs phase implemented `{name}`; see tests/DEFERRED_SYMBOLS.md"
        )

    _deferred.__name__ = name
    _deferred.__qualname__ = name
    return _deferred
