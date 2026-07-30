"""The honest denominator includes tests no pytest run can see (spec F3)."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tools"))

from count_tests import file_exclusion_test_counts  # noqa: E402

EXPECTED_INVISIBLE: dict[str, int] = {
    "stats/test_core_stats_functions.py": 50,
    "drawing/test_draw.py": 16,
    "convert/test_higher_order_network.py": 11,
    "convert/test_line_graph.py": 4,
    "readwrite/test_bigg_data.py": 3,
    "drawing/test_draw_utils.py": 3,
    "convert/test_pandas.py": 2,
    "convert/test_hif_dict.py": 2,
    "convert/test_incidence.py": 2,
    "readwrite/test_xgi_data.py": 2,
    "convert/test_encapsulation_dag.py": 1,
    "convert/test_k_skeleton.py": 1,
    "test_import_time.py": 1,
}


def test_whole_file_exclusions_hide_a_known_number_of_tests() -> None:
    """Pins the per-file invisible counts, not just their sum.

    A total-only pin lets two files drift by equal and opposite amounts and
    still pass — the silent-drift failure this tier exists to remove. Update a
    number here only deliberately, with the reason recorded.
    """
    assert file_exclusion_test_counts() == EXPECTED_INVISIBLE


def test_invisible_total_is_ninety_eight() -> None:
    """The denominator contribution. 394 collected + 98 invisible = 492."""
    assert sum(EXPECTED_INVISIBLE.values()) == 98


def test_no_file_exclusion_is_silently_empty():
    """A file exclusion counting zero tests is either a wrong path or a file
    that no longer has tests — both are registry bugs."""
    zero = [rel for rel, n in file_exclusion_test_counts().items() if n == 0]
    assert zero == [], f"file exclusions that match no tests: {zero}"
