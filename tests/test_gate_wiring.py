"""A reporting tool nothing invokes cannot catch anything."""

import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).parent.parent


def _gate_run() -> str:
    with (ROOT / ".mise.toml").open("rb") as fh:
        config = tomllib.load(fh)
    return config["tasks"]["py:gate"]["run"]


def test_gate_invokes_gate_report() -> None:
    assert "tools/gate_report.py" in _gate_run(), (
        "gate_report.py computes the honest score and cross-checks xfail counts "
        "against the manifest; orphaned, it can never disagree with anything"
    )


def test_gate_report_reads_exact_stats_not_a_parsed_summary() -> None:
    run = _gate_run()
    assert "--from-stats" in run, (
        "counts must come from the reporter's own stats, not from scraping "
        "pytest's human-readable summary line"
    )


def test_gate_still_runs_the_ported_suite_last() -> None:
    run = _gate_run()
    assert "tests/ported" in run
    assert run.index("tests/ported") < run.index("tools/gate_report.py"), (
        "the report must be produced from the ported run, so it comes after it"
    )


def test_gate_discovers_meta_tests_rather_than_listing_them() -> None:
    """Task 1's review found six guards that would have run in no gate at all,
    because py:gate enumerated four test files by hand. A list is a proxy for
    'the tests run' — forgetting an entry is silent."""
    run = _gate_run()
    assert "tests/ --ignore=tests/ported" in run, (
        "py:gate must DISCOVER tests/, not enumerate files"
    )
    listed = re.findall(r"tests/test_\w+\.py", run)
    assert listed == [], (
        f"py:gate names individual test files ({listed}); any guard not on that "
        "list runs nowhere, which is how test_symbol_status.py was orphaned at birth"
    )
