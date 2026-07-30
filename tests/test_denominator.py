"""The honest denominator includes tests no pytest run can see (spec F3)."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from count_tests import file_exclusion_test_counts  # noqa: E402


def test_whole_file_exclusions_hide_a_known_number_of_tests():
    """Pins F3's measurement so a new file exclusion cannot silently shrink
    the denominator. Update this number deliberately, with the reason, when a
    file exclusion is added or removed."""
    counts = file_exclusion_test_counts()
    assert sum(counts.values()) == 100, (
        "invisible-test count changed; update the expected value ONLY with a "
        f"recorded reason. Per-file: {counts}"
    )


def test_no_file_exclusion_is_silently_empty():
    """A file exclusion counting zero tests is either a wrong path or a file
    that no longer has tests — both are registry bugs."""
    zero = [rel for rel, n in file_exclusion_test_counts().items() if n == 0]
    assert zero == [], f"file exclusions that match no tests: {zero}"
