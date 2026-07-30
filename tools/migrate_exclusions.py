"""One-shot: convert tests/conftest_exclusions.py to compat/exclusions.toml.

Classification is deterministic from the existing reason text — the reasons
were written with consistent vocabulary. Every entry the rules cannot classify
is emitted with class="UNCLASSIFIED" and must be resolved by hand before the
TOML is committed; the tool prints them at the end and exits non-zero.
"""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "tests"))

from conftest_exclusions import FILE_EXCLUSIONS, TEST_EXCLUSIONS  # noqa: E402

# Ordered most-specific first; the first matching rule wins.
RULES: list[tuple[str, str]] = [
    # Task 2's four temporary entries open with "env:". Without this rule they
    # migrate as UNCLASSIFIED, and a hand-resolution slip would flip them from
    # `skip` to strict `xfail` under Task 5 — where they would XPASS, since
    # those four tests actually pass. (Task 2 review, minor.)
    ("env:", "env"),
    ("network:", "env"),
    ("network reader", "env"),
    ("pandas", "env"),
    ("matplotlib", "env"),
    ("float non-determinism", "float-parity"),
    ("float-trajectory", "float-parity"),
    ("not achievable", "float-parity"),
    ("core gap", "core-gap"),
    ("core-gap", "core-gap"),
    ("no core primitive", "core-gap"),
    ("core deviation", "divergence"),
    ("documented deviation", "divergence"),
    ("representational deviation", "divergence"),
    ("binding gap", "binding-gap"),
    ("binding-gap", "binding-gap"),
    ("private", "upstream-private"),
    ("not part of the port", "upstream-private"),
    ("decorator machinery", "binding-gap"),
    ("deferred:", "deferred-symbol"),
    ("Inconsistent seeding", "upstream-bug"),
]


def classify(reason: str) -> str:
    # Bounded: one pass over the fixed RULES table.
    for needle, cls in RULES:
        if needle.lower() in reason.lower():
            return cls
    return "UNCLASSIFIED"


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")


def main() -> int:
    out: list[str] = [
        "# Conformance exclusion registry (spec §4 C2).",
        "# Keys are EXACT test names: `test_name` or `ClassName::test_name`.",
        "# Every entry carries one `class` from the closed taxonomy:",
        "#   deferred-symbol | core-gap | binding-gap | divergence",
        "#   float-parity | upstream-private | env | upstream-bug",
        "# `env` and `upstream-bug` produce skip; everything else xfail(strict).",
        "",
    ]
    unclassified: list[str] = []

    for rel, reason in FILE_EXCLUSIONS.items():
        cls = classify(reason)
        if cls == "UNCLASSIFIED":
            unclassified.append(f"FILE {rel}")
        out += [
            "[[file_exclusion]]",
            f'path = "{esc(rel)}"',
            f'class = "{cls}"',
            f'reason = "{esc(reason)}"',
            "",
        ]

    for rel, entries in TEST_EXCLUSIONS.items():
        for entry, reason in entries:
            cls = classify(reason)
            if cls == "UNCLASSIFIED":
                unclassified.append(f"TEST {rel}::{entry}")
            out += [
                "[[test_exclusion]]",
                f'path = "{esc(rel)}"',
                f'test = "{esc(entry)}"',
                f'class = "{cls}"',
                f'reason = "{esc(reason)}"',
                "",
            ]

    dest = pathlib.Path("compat/exclusions.toml")
    dest.parent.mkdir(exist_ok=True)
    dest.write_text("\n".join(out))
    print(f"wrote {dest}")

    if unclassified:
        print("\nUNCLASSIFIED — resolve by hand before committing:")
        # Bounded: one pass over the collected list.
        for u in unclassified:
            print("  ", u)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
