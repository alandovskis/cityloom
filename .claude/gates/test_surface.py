"""What counts as a test in CityLoom, for the agents' red and green gates and the implementer's hook.

The test-writer agent writes the tests (the red commit); the implement agent may not change them. The test surface is:

- every file a protected glob names (`PROTECTED`): test files, the browser tests and their fixtures, what decides which
  spec runs where, the guards, and what decides a check (`.github/`, `scripts/`, `.claude/`);
- every `#[cfg(test)]` item inside any other Rust file (most of the unit tests sit in a `mod tests` at the end of the
  file they test).

The scan of Rust is lexical, not a parser: it skips strings, raw strings, chars and comments so that a brace inside
them is not counted, which is all the brace matching needs.
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

PROTECTED = (
    "**/tests.rs",
    "**/tests/**",
    "e2e/**",
    "scripts/**",
    ".github/**",
    ".claude/**",
    "sonar-project.properties",
    "crates/cityloom_editor/src/i18n_guard.rs",
    "crates/cityloom_editor/src/layout_guard.rs",
    "crates/cityloom_editor/src/shared/contrast.rs",
)

RED_TRAILER = "Red-commit"
CHANGES_TRAILER = "Changes-test"

_CFG_TEST = re.compile(r"#\[cfg\((?:[^\]\[]|\[[^\]]*\])*?\btest\b(?:[^\]\[]|\[[^\]]*\])*\)\]")
_TEST_ATTR = re.compile(r"^\s*#\[(?:[\w:]+::)?(?:test|rstest|wasm_bindgen_test)\b")
_ATTR = re.compile(r"^\s*#\[")
_FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")


def glob(pattern: str, path: str) -> bool:
    """`**` crosses folders (`**/` may be empty), `*` and `?` stay inside one (as `scripts/ci/ci_changes.py`)."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pattern.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pattern[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pattern[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pattern[i]), i + 1
    return re.fullmatch(out, path) is not None


def protected(path: str) -> bool:
    """Whether a repository-relative path is wholly test surface."""
    return any(glob(p, path) for p in PROTECTED)


# --- Rust: the #[cfg(test)] items of a file -------------------------------------------------------------------


def _skip(src: str, i: int) -> int:
    """If a comment, string or char starts at i, the index just after it; else i."""
    if src.startswith("//", i):
        j = src.find("\n", i)
        return len(src) if j < 0 else j
    if src.startswith("/*", i):
        depth, j = 1, i + 2
        while j < len(src) and depth:
            if src.startswith("/*", j):
                depth, j = depth + 1, j + 2
            elif src.startswith("*/", j):
                depth, j = depth - 1, j + 2
            else:
                j += 1
        return j
    raw = re.match(r'b?r(#*)"', src[i : i + 260])
    if raw and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
        end = src.find('"' + raw.group(1), i + raw.end())
        return len(src) if end < 0 else end + 1 + len(raw.group(1))
    if src[i] == '"' or src.startswith('b"', i) and (i == 0 or not src[i - 1].isalnum()):
        j = i + (2 if src[i] == "b" else 1)
        while j < len(src) and src[j] != '"':
            j += 2 if src[j] == "\\" else 1
        return j + 1
    if src[i] == "'":
        if src.startswith("\\", i + 1):
            end = src.find("'", i + 2)
            return len(src) if end < 0 else end + 1
        if i + 2 < len(src) and src[i + 2] == "'":
            return i + 3
        return i + 1  # a lifetime
    return i


def _is_test_cfg(attr: str) -> bool:
    """`#[cfg(test)]` or `#[cfg(all(test, …))]`, not `#[cfg(not(test))]` nor a feature named `test-…`."""
    bare = re.sub(r'"[^"]*"', "", attr)
    return re.search(r"\btest\b", bare) is not None and re.search(r"\bnot\(\s*test\s*\)", bare) is None


def test_spans(src: str) -> list[tuple[int, int]]:
    """The (start, end) of every `#[cfg(test)]` item in a Rust source, its attribute included."""
    spans, i = [], 0
    while i < len(src):
        j = _skip(src, i)
        if j != i:
            i = j
            continue
        m = _CFG_TEST.match(src, i)
        if not m or not _is_test_cfg(m.group()):
            i = m.end() if m else i + 1
            continue
        k, depth = m.end(), 0
        while k < len(src):
            j = _skip(src, k)
            if j != k:
                k = j
                continue
            c = src[k]
            if c == ";" and depth == 0:
                k += 1
                break
            if c in "{[(":
                depth += 1
            elif c in "}])":
                depth -= 1
                if depth < 0:  # the end of the enclosing item: the attribute was on a field or a last element
                    break
                if depth == 0 and c == "}":
                    k += 1
                    break
            k += 1
        spans.append((m.start(), k))
        i = k
    return spans


def test_items(src: str) -> list[str]:
    """The text of every `#[cfg(test)]` item, for comparing two versions of a file."""
    return [src[a:b] for a, b in test_spans(src)]


def touches_tests(src: str, start: int, end: int) -> bool:
    """Whether the range [start, end) of src overlaps a `#[cfg(test)]` item."""
    return any(start < b and a < end for a, b in test_spans(src))


# --- Rust: the tests a diff adds ------------------------------------------------------------------------------


@dataclass(frozen=True)
class NewTest:
    path: str  # repository-relative file
    name: str  # the function's name

    def module(self) -> str | None:
        """The module path the test's file gives, under its crate (`crates/<c>/src/a/b.rs` is `a::b`), or None for
        an integration test (`crates/<c>/tests/…`), whose binary is named by the file."""
        parts = PurePosixPath(self.path).parts
        if len(parts) < 4 or parts[0] != "crates" or parts[2] != "src":
            return None
        mods = list(parts[3:])
        mods[-1] = mods[-1][: -len(".rs")]
        if mods[-1] in ("mod", "lib", "main"):
            mods.pop()
        return "::".join(mods)

    def matches(self, result: str) -> bool:
        """Whether a cargo test result name (`a::b::tests::name`) is this test."""
        if not (result == self.name or result.endswith("::" + self.name)):
            return False
        mod = self.module()
        return mod is None or mod == "" or result.startswith(mod + "::")


def added_tests(diff: str) -> list[NewTest]:
    """The test functions a `git diff -U0` adds: a `fn` added right after an added test attribute."""
    found, path, pending = [], None, False
    for line in diff.splitlines():
        if line.startswith("+++ "):
            path = line[6:] if line.startswith("+++ b/") else None
            pending = False
        elif line.startswith("@@"):
            pending = False
        elif line.startswith("+") and path and path.endswith(".rs"):
            body = line[1:]
            if _TEST_ATTR.match(body):
                pending = True
            elif m := _FN.match(body):
                if pending:
                    found.append(NewTest(path, m.group(1)))
                pending = False
            elif body.strip() and not _ATTR.match(body) and not body.strip().startswith("//"):
                pending = False
    return found


# --- cargo test output ----------------------------------------------------------------------------------------

_RUNNING = re.compile(r"^\s*(?:Running\s+(?:unittests\s+)?\S+\s+\((?:.*/)?([\w-]+?)(?:-[0-9a-f]{8,})?\)|Doc-tests\s+(\S+))")
_RESULT = re.compile(r"^test (\S+)(?: - .*)? \.\.\. (ok|FAILED|ignored)")


def results(output: str) -> dict[str, str]:
    """`binary::test name` → ok, FAILED or ignored, from `cargo test --no-fail-fast` output."""
    out, binary = {}, "?"
    for line in output.splitlines():
        if m := _RUNNING.match(line):
            binary = m.group(1) or "doc:" + m.group(2)
        elif m := _RESULT.match(line):
            out[f"{binary}::{m.group(1)}"] = m.group(2)
    return out


def test_name(key: str) -> str:
    """The test's own name, without the binary `results` puts in front."""
    return key.split("::", 1)[1]


# --- git ------------------------------------------------------------------------------------------------------


def git(*args: str, cwd: Path | None = None) -> str:
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout


def show(rev: str, path: str, cwd: Path | None = None) -> str | None:
    """A file's content at a revision, or None when it is not there."""
    r = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=cwd, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def red_commit(base: str, cwd: Path | None = None) -> str:
    """The one commit between base and HEAD with a `Red-commit:` trailer."""
    shas = git("log", "--format=%H", f"--grep=^{RED_TRAILER}:", f"{base}..HEAD", cwd=cwd).split()
    if len(shas) != 1:
        raise SystemExit(f"expected exactly one commit with a {RED_TRAILER}: trailer since {base}, found {len(shas)}")
    return shas[0]


def changed_tests(sha: str, cwd: Path | None = None) -> list[str]:
    """The existing tests a red commit declares it changes (`Changes-test: a::b::tests::name`, one per trailer)."""
    body = git("log", "-1", "--format=%B", sha, cwd=cwd)
    return [m.group(1) for m in re.finditer(rf"^{CHANGES_TRAILER}:\s*(\S+)\s*$", body, re.M)]
