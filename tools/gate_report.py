# tools/gate_report.py
"""Emit compat-report.json: the honest score with its class breakdown.

Re-derives counts from the manifest and a pytest run's outcome counts. Kept
deliberately simple: the score is a CI artifact, never shipped. Invoked
automatically as the last step of `mise run py:gate`, which writes exact
counts to `.gate-stats.json` via `tests/conftest.py`'s `pytest_sessionfinish`
hook and passes that file with `--from-stats`.

Usage:  python -m tools.gate_report --from-stats <path>
   or:  python -m tools.gate_report <passed> <skipped> <xfailed>  (manual fallback)
"""

import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).parent.parent

from tools.gen_manifest import DEST  # noqa: E402


def main(passed: int, skipped: int, xfailed: int) -> int:
    manifest = json.loads(DEST.read_text())
    invisible = sum(manifest["exclusions"]["invisible_test_counts"].values())

    by_class: collections.Counter = collections.Counter()
    # Bounded: one pass over the registry's test entries.
    for entry in manifest["exclusions"]["tests"]:
        by_class[entry["class"]] += 1

    if passed < 0 or skipped < 0 or xfailed < 0:
        raise SystemExit("counts must be non-negative")
    if xfailed != sum(by_class.values()):
        raise SystemExit(
            f"inconsistent inputs: {xfailed} xfailed reported, but the manifest "
            f"lists {sum(by_class.values())} test-level gaps. Either the counts "
            "are from a different run, or the registry and the suite disagree."
        )

    denominator = passed + skipped + xfailed + invisible
    report = {
        "passed": passed,
        "skipped": skipped,
        "xfailed": xfailed,
        "invisible": invisible,
        "denominator": denominator,
        "pass_rate": round(passed / denominator, 4) if denominator else 0.0,
        "gaps_by_class": dict(by_class),
    }
    out = ROOT / "compat-report.json"
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


def _from_stats(path: pathlib.Path) -> tuple[int, int, int]:
    """Exact counts from the ported run, written by pytest_sessionfinish."""
    if not path.exists():
        raise SystemExit(
            f"{path} not found — run the ported suite first; the report is "
            "derived from a real run, never from assumed counts"
        )
    stats = json.loads(path.read_text())
    return (
        int(stats.get("passed", 0)),
        int(stats.get("skipped", 0)),
        int(stats.get("xfailed", 0)),
    )


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--from-stats":
        raise SystemExit(main(*_from_stats(pathlib.Path(sys.argv[2]))))
    if len(sys.argv) == 4:
        raise SystemExit(
            main(int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]))
        )
    raise SystemExit(
        "usage: python -m tools.gate_report <passed> <skipped> <xfailed>\n"
        "   or: python -m tools.gate_report --from-stats <path>"
    )
