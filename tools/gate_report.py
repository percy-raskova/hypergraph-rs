# tools/gate_report.py
"""Emit compat-report.json: the honest score with its class breakdown.

Reads a pytest JSON-free summary by re-deriving counts from the manifest and a
pytest run's exit summary passed on argv. Kept deliberately simple: the score
is a CI artifact, never shipped.

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


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("usage: gate_report.py <passed> <skipped> <xfailed>")
    raise SystemExit(main(int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])))
