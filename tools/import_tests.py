"""Import XGI's own test suite at a tag into tests/ported, behind a lock.

Usage: python -m tools.import_tests <tag> [--repo PATH]

The tests are read with git from a clone of XGI (never run, never imported),
rewritten so that they test this library instead of the real XGI, and written
to tests/ported together with tests/conftest.py's upstream section. The lock
(compat/upstream_tests.toml) records the tag, its commit and the sha256 of
every upstream file, so the integrity check can compare the ported copies with
the wheel and the oracle guard can tell upstream files from repo-authored ones.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import io
import pathlib
import re
import subprocess
import sys
import tokenize
import tomllib
from dataclasses import dataclass

ROOT = pathlib.Path(__file__).resolve().parent.parent

XGI_REPO_URL = "https://github.com/xgi-org/xgi.git"
KNOWN_TAGS: dict[str, str] = {
    "v0.10.2": "a61d6700096c3b5e864f34ef58c415f7b5c072ff",
    "v1.1": "c9891680372393a325657e45ace033050fb9a2f8",
}
CACHE: pathlib.Path = ROOT / ".cache" / "xgi"
LOCK: pathlib.Path = ROOT / "compat" / "upstream_tests.toml"
PORTED: pathlib.Path = ROOT / "tests" / "ported"
CONFTEST: pathlib.Path = ROOT / "tests" / "conftest.py"

_IMPORT_ROOT = re.compile(r"^(\s*)import xgi(\s*)$")
_FROM_ROOT = re.compile(r"^(\s*)from xgi(\.|\s)")
_MODULE_TARGET = re.compile(r"xgi(\.[A-Za-z_][A-Za-z0-9_]*)+")
_SENTINEL = re.compile(
    r"^# === END VERBATIM XGI \S+ conftest — repo-authored code below this line ===$",
    re.MULTILINE,
)


class UpstreamImportError(RuntimeError):
    """The import cannot be done as asked; nothing is written."""


def commit_for(tag: str) -> str:
    """The commit a known tag must point at."""
    try:
        return KNOWN_TAGS[tag]
    except KeyError:
        raise UpstreamImportError(
            f"unknown XGI tag {tag!r}; known tags: {', '.join(sorted(KNOWN_TAGS))}"
        ) from None


def xgi_version_of(tag: str) -> str:
    """`v1.1` -> `1.1`."""
    return tag[1:] if tag.startswith("v") else tag


def sentinel_line(xgi_version: str) -> str:
    return (
        f"# === END VERBATIM XGI {xgi_version} conftest — "
        "repo-authored code below this line ==="
    )


def _is_target_string(value: object) -> bool:
    return isinstance(value, str) and (
        value == "xgi" or _MODULE_TARGET.fullmatch(value) is not None
    )


def rewrite(source: str) -> str:
    """The only changes the importer makes to an upstream file."""
    lines = re.findall(r"[^\n]*\n|[^\n]+", source)  # physical lines, as tokenize counts them
    out: list[str] = []
    for line in lines:
        match = _IMPORT_ROOT.match(line)
        if match:
            line = f"{match.group(1)}import hypergraph_rs as xgi{match.group(2)}"
        else:
            match = _FROM_ROOT.match(line)
            if match:
                start = match.start(1) + len(match.group(1))
                line = line[:start] + "from hypergraph_rs" + line[start + len("from xgi") :]
        out.append(line)
    # Rule 3: string literal tokens. Offsets are per line, in characters.
    edits: list[tuple[int, int]] = []  # (line index, column of the "xgi" to replace)
    try:
        tokens = list(tokenize.generate_tokens(io.StringIO("".join(out)).readline))
    except (tokenize.TokenError, IndentationError, SyntaxError) as error:
        raise UpstreamImportError(f"cannot tokenize source: {error}") from error
    for token in tokens:
        if token.type != tokenize.STRING:
            continue
        try:
            value = ast.literal_eval(token.string)
        except (ValueError, SyntaxError):
            continue
        if not _is_target_string(value):
            continue
        row, col = token.start
        quote_at = min(i for i, ch in enumerate(token.string) if ch in "'\"")
        prefix = token.string[:quote_at]
        quote_len = 3 if token.string[quote_at : quote_at + 3] in ('"""', "'''") else 1
        edits.append((row - 1, col + len(prefix) + quote_len))
    for index, col in sorted(edits, reverse=True):
        line = out[index]
        assert line[col : col + 3] == "xgi", (index, col, line)
        out[index] = line[:col] + "hypergraph_rs" + line[col + 3 :]
    return "".join(out)


def leftovers(source: str) -> list[str]:
    """References to xgi that `rewrite` leaves behind, as `line N: <line>`."""
    lines = source.splitlines()
    try:
        tree = ast.parse(source)
    except SyntaxError as error:
        return [f"line {error.lineno}: unparsable ({error.msg})"]
    found: dict[int, None] = {}
    for node in ast.walk(tree):
        line_no: int | None = None
        if isinstance(node, ast.Import):
            if any(a.name == "xgi" or a.name.startswith("xgi.") for a in node.names):
                line_no = node.lineno
        elif isinstance(node, ast.ImportFrom):
            if node.module and (node.module == "xgi" or node.module.startswith("xgi.")):
                line_no = node.lineno
        elif isinstance(node, ast.Constant):
            if _is_target_string(node.value):
                line_no = node.lineno
        elif isinstance(node, ast.JoinedStr):
            for part in node.values:
                if isinstance(part, ast.Constant) and "xgi." in str(part.value):
                    line_no = node.lineno
        if line_no is not None:
            found.setdefault(line_no, None)
    return [f"line {n}: {lines[n - 1].strip()}" for n in sorted(found)]


def _git(repo: pathlib.Path, *args: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, check=False
    )
    if result.returncode != 0:
        raise UpstreamImportError(
            f"git {' '.join(args)} failed: {result.stderr.decode(errors='replace').strip()}"
        )
    return result.stdout


def ensure_repo(cache: pathlib.Path = CACHE) -> pathlib.Path:
    """Clone XGI into the cache, or fetch its tags when it is there."""
    if (cache / ".git").exists():
        _git(cache, "fetch", "--tags", "--quiet")
        return cache
    cache.parent.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        ["git", "clone", "--quiet", XGI_REPO_URL, str(cache)],
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise UpstreamImportError(
            f"git clone failed: {result.stderr.decode(errors='replace').strip()}"
        )
    return cache


def upstream_tests(repo: pathlib.Path, tag: str) -> dict[str, bytes]:
    """Every tests/**/test_*.py and tests/conftest.py at the tag, keyed relative to tests/."""
    expected = commit_for(tag)
    actual = _git(repo, "rev-parse", f"{tag}^{{commit}}").decode().strip()
    if actual != expected:
        raise UpstreamImportError(
            f"tag {tag} is commit {actual} in the clone, expected {expected}"
        )
    names = _git(repo, "ls-tree", "-r", "--name-only", tag, "tests/").decode().splitlines()
    files: dict[str, bytes] = {}
    for name in names:
        rel = name[len("tests/") :]
        base = rel.rsplit("/", 1)[-1]
        if rel != "conftest.py" and not (base.startswith("test_") and base.endswith(".py")):
            raise UpstreamImportError(
                f"{tag} has a file under tests/ that is neither conftest.py nor test_*.py: {rel}"
            )
        files[rel] = _git(repo, "show", f"{tag}:{name}")
    return files


@dataclass(frozen=True)
class UpstreamFile:
    path: str
    sha256: str


@dataclass(frozen=True)
class Lock:
    tag: str
    commit: str
    xgi_version: str
    conftest_sha256: str
    files: tuple[UpstreamFile, ...]


def read_lock(path: pathlib.Path = LOCK) -> Lock:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return Lock(
        tag=data["tag"],
        commit=data["commit"],
        xgi_version=data["xgi_version"],
        conftest_sha256=data["conftest_sha256"],
        files=tuple(UpstreamFile(f["path"], f["sha256"]) for f in data["file"]),
    )


def write_lock(lock: Lock, path: pathlib.Path = LOCK) -> None:
    out = [
        "# Written by tools/import_tests.py. Do not edit.",
        f'tag = "{lock.tag}"',
        f'commit = "{lock.commit}"',
        f'xgi_version = "{lock.xgi_version}"',
        f'conftest_sha256 = "{lock.conftest_sha256}"',
    ]
    for f in lock.files:
        out += ["", "[[file]]", f'path = "{f.path}"', f'sha256 = "{f.sha256}"']
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(out) + "\n", encoding="utf-8")


def _sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def import_tests(
    tag: str,
    repo: pathlib.Path | None = None,
    ported: pathlib.Path = PORTED,
    conftest: pathlib.Path = CONFTEST,
    lock: pathlib.Path = LOCK,
) -> Lock:
    """Write the tag's tests, rewritten, and the lock. Writes nothing on an error."""
    commit = commit_for(tag)
    if repo is None:
        repo = ensure_repo()
    files = upstream_tests(repo, tag)
    if "conftest.py" not in files:
        raise UpstreamImportError(f"{tag} has no tests/conftest.py")

    texts: dict[str, str] = {}
    problems: list[str] = []
    for rel, raw in sorted(files.items()):
        text = raw.decode("utf-8")
        rewritten = rewrite(text)
        problems += [f"{rel}: {x}" for x in leftovers(rewritten)]
        texts[rel] = rewritten
    if problems:
        raise UpstreamImportError(
            "references to xgi the rewrite leaves behind:\n  " + "\n  ".join(problems)
        )

    old = conftest.read_text(encoding="utf-8")
    sentinels = _SENTINEL.findall(old)
    if len(sentinels) != 1:
        raise UpstreamImportError(
            f"{conftest} must hold the sentinel line exactly once, found {len(sentinels)}"
        )
    sentinel_match = _SENTINEL.search(old)
    assert sentinel_match is not None
    tail = old[sentinel_match.end() :]  # starts with the newline after the sentinel
    new_conftest = (
        texts["conftest.py"] + sentinel_line(xgi_version_of(tag)) + tail
    )

    previous = read_lock(lock) if lock.exists() else None
    new_paths = {rel for rel in files if rel != "conftest.py"}
    if previous is not None:
        for old_file in previous.files:
            if old_file.path not in new_paths:
                (ported / old_file.path).unlink(missing_ok=True)
    for rel in sorted(new_paths):
        target = ported / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(texts[rel].encode("utf-8"))
    conftest.write_bytes(new_conftest.encode("utf-8"))

    result = Lock(
        tag=tag,
        commit=commit,
        xgi_version=xgi_version_of(tag),
        conftest_sha256=_sha(files["conftest.py"]),
        files=tuple(UpstreamFile(rel, _sha(files[rel])) for rel in sorted(new_paths)),
    )
    write_lock(result, lock)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m tools.import_tests")
    parser.add_argument("tag", help=f"an XGI tag: {', '.join(sorted(KNOWN_TAGS))}")
    parser.add_argument("--repo", type=pathlib.Path, default=None, help="a clone of XGI")
    args = parser.parse_args(argv)
    try:
        result = import_tests(args.tag, repo=args.repo)
    except UpstreamImportError as error:
        print(f"import_tests: {error}", file=sys.stderr)
        return 1
    print(f"imported {len(result.files)} files from XGI {result.xgi_version} ({result.commit})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
