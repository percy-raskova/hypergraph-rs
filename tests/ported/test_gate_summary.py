def test_no_unexcused_exclusions():
    """Every exclusion must carry a class from the closed taxonomy.

    Supersedes the previous hand-maintained keyword whitelist, which validated
    reason PROSE by substring and needed editing whenever a reason was worded
    differently. The taxonomy is exact and is the same field that drives the
    skip-vs-xfail decision in conftest.py, so this check and the runtime
    behaviour cannot disagree.
    """
    from conftest_exclusions import ENTRY_CLASSES, FILE_CLASSES

    valid = {
        "deferred-symbol", "core-gap", "binding-gap", "divergence",
        "float-parity", "upstream-private", "env", "upstream-bug",
        "out-of-scope",
    }
    bad = []
    # Bounded: one pass over each registry mapping.
    for path, cls in FILE_CLASSES.items():
        if cls not in valid:
            bad.append(f"{path}: class `{cls}` not in taxonomy")
    for (path, name), cls in ENTRY_CLASSES.items():
        if cls not in valid:
            bad.append(f"{path}::{name}: class `{cls}` not in taxonomy")
    assert bad == [], "exclusions with an invalid class:\n  " + "\n  ".join(bad)
