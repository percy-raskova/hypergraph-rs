# tools/gate_report.py
"""Emit compat-report.json: the honest score with its class breakdown.

Reads a pytest JSON-free summary by re-deriving counts from the manifest and a
pytest run's exit summary passed on argv. Kept deliberately simple: the score
is a CI artifact, never shipped. Nothing invokes this automatically — run it
by hand with the passed/skipped/xfailed counts from a `mise run py:gate` (or
CI) run.

Usage:  python tools/gate_report.py <passed> <skipped> <xfailed>
"""

import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from gen_manifest import DEST  # noqa: E402


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
        "usage: gate_report.py <passed> <skipped> <xfailed>\n"
        "   or: gate_report.py --from-stats <path>"
    )
