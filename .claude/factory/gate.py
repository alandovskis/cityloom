#!/usr/bin/env python3
"""The software factory's red and green gates (CLM-27).

    gate.py red   --repo <worktree> --base <rev>                 the test-writer's red commit is HEAD
    gate.py green --repo <worktree> --base <rev> --red <sha>     the implementer's green head is HEAD
    gate.py docs  --repo <worktree> --base <rev>                 the writer's documentation head is HEAD

The red gate passes a red commit when its new tests compile and every one of them fails, and every test that was there
before still passes. A new test that already passes proves nothing and is refused, as is a red commit with no new test.
It lists the tests at the base by checking the base out in the same worktree (one target directory, built
incrementally) and comes back to where it started.

The green gate passes a head when every test passes, built as CI builds it (RUSTFLAGS=-D warnings), formatting and the
version bump hold, and nothing the test-writer owns changed since the red commit: no test file (`is_test_file`), no
`#[cfg(test)]` block, and nothing else of the guarded set (`is_guarded`: the checks and the factory itself).

The docs gate is the documentation flow's, which has no red commit: it passes a head when every file the branch changes
is documentation (the `docs` rule of scripts/ci/spec-map.json) and none is guarded or the site's configuration,
code or dependencies (`SITE_CONFIGURATION`), each page of the site's docs has its
other language's page (`website/docs/` and fr-CA's), and, when the branch changes website/, the site passes the
Documentation workflow's Prettier check and builds in both languages (a broken link fails the build).

The tests gate is the tests flow's (CLM-100), for an issue that asks only for tests of code that already works: there is
no red commit, so each new test is proved by mutation instead. It passes a head when the branch changes only tests (and
the version line), keeps every existing test as it was, adds at least one test, every test passes as the green gate
runs them, and each new test catches at least one fault of the code the plan's `## Faults` names (faults.py): the gate
applies each fault, runs the new tests, and takes it back. `--proved <sha>` keeps the proof made at that head over a
rebase that changed nothing it rests on; `--brief` leaves the faults out of the report (for the test-writer).

    gate.py tests  --repo <worktree> --base <rev> --faults <file> [--proved <sha>] [--brief]
    gate.py faults --repo <main checkout> --faults <file> [--browser]   the plan's faults, before it is posted

The red, green and tests gates need DATABASE_URL (the server's tests); the docs gate does not. Exit 0 passes, 1 fails
with the problems listed, 2 refuses to run (a dirty worktree, a red commit not in the history, no database, no pinned
cargo-mutants, a fault left by a stopped run). Standard library only; scripts/include/tests/test_factory_gate.py tests
it.
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from collections import Counter
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path
from typing import Callable, Dict, List, Optional, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts" / "ci"))

from ci_changes import ROOT, glob, rule_for  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))

import faults as fl  # noqa: E402

# What `just test` runs (the justfile is the source; test_factory_gate.py checks they agree).
CARGO_TEST = ["cargo", "test", "--workspace", "--features", "cityloom_server/test-provider"]
WASM_BUILD = [
    "cargo", "build", "--target", "wasm32-unknown-unknown", "--workspace",
    "--exclude", "cityloom_server", "--exclude", "citymodel_builder",
]

# Only the test-writer writes these: the tests, their fixtures, which tier runs a spec and which specs a change runs.
TEST_FILES = (
    "**/tests.rs",
    "**/tests/**",
    "e2e/tests/**",
    "e2e/fixtures/**",
    "e2e/tiers.json",
    "scripts/ci/spec-map.json",
)
# Nor may the implementer change what decides whether a check passes, or the factory that judges it.
GUARDED = TEST_FILES + (
    ".github/**",
    "scripts/**",
    ".claude/**",
    "crates/cityloom_editor/src/i18n_guard.rs",
    "crates/cityloom_editor/src/layout_guard.rs",
    "crates/cityloom_editor/src/dependency_guard.rs",
    "crates/cityloom_editor/src/shared/contrast.rs",
)

# What the Documentation workflow (.github/workflows/docs.yml) runs, from e2e/ and website/ (test_factory_gate.py checks they agree).
DOCS_PRETTIER = ("npx", "prettier", "--ignore-path", "../.prettierignore", "--check", "../website")
DOCS_BUILD = ("npm", "run", "build")
# A page of the site in English, and the same page in Canadian French.
DOCS_EN = "website/docs/"
DOCS_FR = "website/i18n/fr-CA/docusaurus-plugin-content-docs/current/"
_SPEC_MAP = ROOT / "scripts" / "ci" / "spec-map.json"
# CI counts all of website/ as documentation, but these are the site's configuration, code and dependencies: a change to
# them is a decision for a person, not pages (a new page's place in website/sidebars.ts is pages).
SITE_CONFIGURATION = (
    "website/docusaurus.config.ts",
    "website/package.json",
    "website/package-lock.json",
    "website/tsconfig.json",
    "website/src/**",
)


@lru_cache(maxsize=1)
def _spec_map() -> dict:
    return json.loads(_SPEC_MAP.read_text(encoding="utf-8"))


def is_documentation(path: str) -> bool:
    """Documentation as CI counts it: the first rule of the spec map naming the path is its `docs` rule."""
    rule = rule_for(_spec_map(), path)
    return bool(rule and rule.get("docs"))


def other_language(path: str) -> Optional[Tuple[str, str]]:
    """For a page of the site's docs, the same page in the other language and that language's name; else None."""
    if not path.endswith((".md", ".mdx")):
        return None
    if path.startswith(DOCS_EN):
        return DOCS_FR + path[len(DOCS_EN) :], "fr-CA"
    if path.startswith(DOCS_FR):
        return DOCS_EN + path[len(DOCS_FR) :], "English"
    return None


def is_test_file(path: str) -> bool:
    return any(glob(p, path) for p in TEST_FILES)


def is_guarded(path: str) -> bool:
    return any(glob(p, path) for p in GUARDED)


# ---- cargo's output -------------------------------------------------------------------------------------------------

_RUNNING = re.compile(r"^\s+Running (.+?) \((.+)\)\s*$")
_DOCTESTS = re.compile(r"^\s+Doc-tests (\S+)\s*$")
_LISTED = re.compile(r"^(.+): (?:test|benchmark)$")
_RESULT = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored)\b")
_HASH = re.compile(r"-[0-9a-f]{16}$")


def _binary(line: str) -> Optional[str]:
    """The test binary a `Running` or `Doc-tests` line starts, by crate and source, without the build's hash."""
    running = _RUNNING.match(line)
    if running:
        source, path = running.groups()
        return f"{_HASH.sub('', Path(path).name)} {source}"
    doc = _DOCTESTS.match(line)
    if doc:
        return f"doc {doc.group(1)}"
    return None


def cargo_listing(output: str) -> set:
    """Every test `cargo test -- --list` names, as `<binary>::<name>`."""
    tests, binary = set(), None
    for line in output.splitlines():
        binary = _binary(line) or binary
        listed = _LISTED.match(line)
        if binary and listed:
            tests.add(f"{binary}::{listed.group(1)}")
    return tests


def cargo_results(output: str) -> Dict[str, str]:
    """Every test a `cargo test` run reports, as `<binary>::<name>`, with `ok`, `failed` or `ignored`."""
    results, binary = {}, None
    for line in output.splitlines():
        binary = _binary(line) or binary
        result = _RESULT.match(line)
        if binary and result:
            name = result.group(1)
            if name.endswith(" - should panic"):
                name = name[: -len(" - should panic")]
            results[f"{binary}::{name}"] = {"ok": "ok", "FAILED": "failed", "ignored": "ignored"}[result.group(2)]
    return results


def did_not_compile(code: int, output: str) -> bool:
    return code != 0 and bool(re.search(r"^error(\[E\d+\])?: (?!test failed|\d+ targets? failed)", output, re.M))


# ---- Playwright's output --------------------------------------------------------------------------------------------


def playwright_results(report: dict) -> Dict[str, Optional[str]]:
    """Every test of a Playwright JSON report, as `<file>::<title path>::<project>`, with its status (None in a listing)."""
    results: Dict[str, Optional[str]] = {}

    def walk(suite: dict, titles: List[str]) -> None:
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                name = " > ".join(titles + [spec["title"]])
                results[f"{spec['file']}::{name}::{test.get('projectName', '')}"] = test.get("status")
        for child in suite.get("suites", []):
            walk(child, titles + [child["title"]])

    for top in report.get("suites", []):
        walk(top, [])
    return results


def spec_of(name: str) -> str:
    """The spec file (relative to e2e/tests) of a test named as `playwright_results` names it."""
    return name.split("::", 1)[0]


def playwright_load_errors(report: dict) -> Dict[str, str]:
    """The files a Playwright report could not load (relative to e2e/tests when they are there), each with its error's
    first line. Playwright leaves such a file's tests out of the report and says why in its top-level `errors`."""
    errors: Dict[str, str] = {}
    for error in report.get("errors", []):
        path = (error.get("location") or {}).get("file", "")
        name = path.split("/e2e/tests/", 1)[1] if "/e2e/tests/" in path else (path or "the specs")
        message = (error.get("message") or "").strip()
        errors.setdefault(name, message.splitlines()[0] if message else "")
    return errors


# ---- #[cfg(test)] blocks --------------------------------------------------------------------------------------------

_CFG_TEST = re.compile(r"#\[cfg\((?:test|(?:all|any)\((?:[^\[\]]*[\s(,])?test(?![\w-])[^\[\]]*\))\)\]")
_code_mask = fl.code_mask


def cfg_test_blocks(src: str) -> List[str]:
    """The text of every item `#[cfg(test)]` (or a cfg holding `test`) guards, from the attribute to its end."""
    mask = _code_mask(src)
    blocks = []
    for found in _CFG_TEST.finditer(src):
        if not mask[found.start()]:
            continue
        depth, j = 0, found.end()
        while j < len(src):
            if mask[j]:
                ch = src[j]
                if ch in "{[(":
                    depth += 1
                elif ch in "}])":
                    depth -= 1
                    if depth == 0 and ch == "}":
                        j += 1
                        break
                elif ch == ";" and depth == 0:
                    j += 1
                    break
            j += 1
        blocks.append(src[found.start() : j])
    return blocks


# ---- judging --------------------------------------------------------------------------------------------------------

_PASSED = {"ok", "expected", "flaky"}
_FAILED = {"failed", "unexpected"}


def judge_red(before: set, red: Dict[str, Optional[str]], kind: str) -> Tuple[List[str], List[str]]:
    """The problems of a red run against the tests listed at the base, and the new tests (sorted)."""
    problems, new = [], []
    for name in sorted(red):
        outcome = red[name]
        if name in before:
            if outcome in _FAILED:
                problems.append(f"{kind}: the existing test {name} fails")
        else:
            new.append(name)
            if outcome in _PASSED:
                problems.append(f"{kind}: the new test {name} already passes: it does not prove the change")
            elif outcome not in _FAILED:
                problems.append(f"{kind}: the new test {name} is ignored or skipped")
    for name in sorted(before - set(red)):
        problems.append(f"{kind}: the test {name} is gone")
    return problems, new


# ---- running --------------------------------------------------------------------------------------------------------

Runner = Callable[..., Tuple[int, str]]
TIMED_OUT = 124  # as `timeout` answers


def run(cmd: List[str], cwd: Path, env: Optional[dict] = None, timeout: Optional[float] = None) -> Tuple[int, str]:
    """Runs a command with its stderr in its stdout (cargo's `Running` lines and the tests' own lines stay in order).
    With a timeout, the command and everything it started are killed when it runs out, and it answers TIMED_OUT."""
    print("$ " + " ".join(cmd), file=sys.stderr, flush=True)
    try:
        proc = subprocess.Popen(
            cmd,
            cwd=cwd,
            env=dict(os.environ, **(env or {})),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=timeout is not None,
        )
    except FileNotFoundError:
        return 127, f"{cmd[0]}: command not found"  # as a shell answers, which `Gate._tool` refuses
    try:
        out, _ = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        out, _ = proc.communicate()
        return TIMED_OUT, (out or "") + f"\n[killed after {timeout:.0f} s]"
    return proc.returncode, out


# ---- the tests flow: what is test code, and what an existing test is ------------------------------------------------

# Besides the test files, what the tests flow may change: the feature list (a new test file's feature), the contrast
# measures (a test file in all but name), and the editor's version line (`VERSION_FILES`, checked line by line).
TESTS_FLOW_FILES = (
    "scripts/ci/features.json",
    "scripts/ci/spec-map.json",
    "e2e/tiers.json",
    "crates/cityloom_editor/src/shared/contrast.rs",
)
VERSION_FILES = ("crates/cityloom_editor/Cargo.toml", "Cargo.lock")

# The faults: cargo-mutants, pinned and installed once in the main checkout's .tools (the README says how), and the
# limits of a run (CLM-100's design measured a native fault at about 16 s and a browser one at about 64 s).
MUTANTS = ROOT / ".tools" / "bin" / "cargo-mutants"
MUTANTS_VERSION = "27.1.0"
LIMITS = {"native": 40, "browser": 12}  # faults run, and entries a plan may name
DEADLINE = 30 * 60  # no fault starts later than this after the first of its kind
CAUGHT_ENOUGH = 3  # a test that caught this many is not run again
NATIVE_TIMEOUT = 60  # a native test under a fault is killed after this, or four times its own time if longer
BASELINE_TIMEOUT = 10 * 60  # a new test run alone with no fault
SPEC_TIMEOUT_MS = 30_000  # a browser test's own timeout under a fault
BROWSER_RUN = 5 * 60  # a browser run as a whole; the tests not ended by then are hung
LEVEL_1 = {"CARGO_PROFILE_DEV_OPT_LEVEL": "1", "CARGO_PROFILE_DEV_DEBUG": "0"}  # the wasm the faults are built into
NOT_BROWSER = {"cityloom_server", "citymodel_builder", "cityloom_report_pdf"}  # crates whose code no browser test runs


def without_test_code(src: str) -> str:
    """A Rust file without the items built only for tests (`faults.is_test_cfg`: `test` or `all(test, …)`, never
    `any(test, …)`, which is product code too)."""
    kept, at = [], 0
    for start, end in fl.test_items(src):
        kept.append(src[at:start])
        at = end
    kept.append(src[at:])
    return "".join(kept)


_TEST_ATTR = re.compile(r"#\[(?:[\w:]+::)?test(?:\s*\([^\]]*\))?\]|#\[wasm_bindgen_test\]")


def test_functions(src: str) -> List[Tuple[str, str]]:
    """Each Rust test function, by name, with its text from its first attribute or doc comment to its closing brace."""
    mask, found = fl.code_mask(src), []
    for attr in _TEST_ATTR.finditer(src):
        if not mask[attr.start()]:
            continue
        name = re.compile(r"\bfn\s+(\w+)").search(src, attr.end())
        if not name:
            continue
        start = attr.start()
        while True:
            line_start = src.rfind("\n", 0, start) + 1
            if line_start == 0:
                break
            previous_start = src.rfind("\n", 0, line_start - 1) + 1
            previous = src[previous_start : line_start - 1]
            if not previous.strip().startswith(("#[", "///")):
                break
            start = previous_start + len(previous) - len(previous.lstrip())
        found.append((name.group(1), src[start : fl.item_end(src, mask, attr.start())]))
    return found


def _js_mask(src: str) -> List[bool]:
    """True for each character of JavaScript code, False inside comments and string or template literals."""
    mask, i, n = [True] * len(src), 0, len(src)
    while i < n:
        if src.startswith("//", i):
            end = src.find("\n", i)
            end = n if end < 0 else end
        elif src.startswith("/*", i):
            end = src.find("*/", i + 2)
            end = n if end < 0 else end + 2
        elif src[i] in "'\"`":
            quote, end = src[i], i + 1
            while end < n and src[end] != quote:
                end += 2 if src[end] == "\\" else 1
            end += 1
        else:
            i += 1
            continue
        for k in range(i, min(end, n)):
            mask[k] = False
        i = end
    return mask


_PLAYWRIGHT_TEST = re.compile(r"(?<![\w.$])test(?:\.(?:only|skip|fixme|fail|slow))?\s*\(")


def playwright_tests(src: str) -> List[Tuple[str, str]]:
    """Each Playwright test of a spec, by title, with its text from `test` to its closing parenthesis. A call with no
    title (`test.skip(condition)` inside a test) is part of its test, not one."""
    mask, found = _js_mask(src), []
    for call in _PLAYWRIGHT_TEST.finditer(src):
        if not mask[call.start()]:
            continue
        first = call.end()
        while first < len(src) and src[first].isspace():
            first += 1
        if first >= len(src) or src[first] not in "'\"`":
            continue
        close = first + 1
        while close < len(src) and src[close] != src[first]:
            close += 2 if src[close] == "\\" else 1
        depth, j = 0, call.end() - 1
        while j < len(src):
            if mask[j]:
                if src[j] in "([{":
                    depth += 1
                elif src[j] in ")]}":
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        found.append((src[first + 1 : close], src[call.start() : j + 1]))
    return found


def version_only(path: str, before: str, after: str) -> bool:
    """Whether a version file changed in the editor's version line alone."""

    def strip(text: str) -> str:
        if path == "Cargo.lock":
            return re.sub(r'(name = "cityloom_editor"\r?\nversion = )"[^"]*"', r'\1""', text)
        return re.sub(r'(?m)^version\s*=\s*"[^"]*"', 'version = ""', text, count=1)

    return strip(before) == strip(after)


def only_added(before: str, after: str) -> bool:
    """Whether `after` is `before` with lines added and none changed or removed."""
    matcher = difflib.SequenceMatcher(a=before.splitlines(), b=after.splitlines(), autojunk=False)
    return all(op in ("equal", "insert") for op, *_ in matcher.get_opcodes())


@dataclass(frozen=True)
class Binary:
    """A test binary cargo built: where it is, where cargo runs it from (its package's folder), and its package."""

    path: str
    cwd: str
    package: str


_UNIT_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro", "bin"}


def test_binaries(output: str) -> Dict[str, Binary]:
    """The test binaries of a `cargo test --no-run --message-format=json` build, by the label `cargo test` gives them
    (`cargo_listing`'s `<binary>`)."""
    found: Dict[str, Binary] = {}
    for line in output.splitlines():
        if not line.startswith("{"):
            continue
        try:
            artifact = json.loads(line)
        except ValueError:
            continue
        if artifact.get("reason") != "compiler-artifact" or not artifact.get("executable"):
            continue
        if not (artifact.get("profile") or {}).get("test"):
            continue
        target, root = artifact["target"], Path(artifact["manifest_path"]).parent
        unit = "unittests " if _UNIT_KINDS & set(target.get("kind", [])) else ""
        label = f"{_HASH.sub('', Path(artifact['executable']).name)} {unit}{os.path.relpath(target['src_path'], root)}"
        package_id = artifact.get("package_id", "")
        fragment = package_id.split("#", 1)[1] if "#" in package_id else ""
        package = fragment.split("@", 1)[0] if "@" in fragment else package_id.split("#", 1)[0].rstrip("/").rsplit("/", 1)[-1]
        found[label] = Binary(artifact["executable"], str(root), package)
    return found


def playwright_locations(report: dict) -> Dict[str, Tuple[str, int, str]]:
    """Every test of a Playwright listing, named as `playwright_results` names it, with its file, line and project."""
    found: Dict[str, Tuple[str, int, str]] = {}

    def walk(suite: dict, titles: List[str]) -> None:
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                name = " > ".join(titles + [spec["title"]])
                project = test.get("projectName", "")
                found[f"{spec['file']}::{name}::{project}"] = (spec["file"], spec.get("line", 0), project)
        for child in suite.get("suites", []):
            walk(child, titles + [child["title"]])

    for top in report.get("suites", []):
        walk(top, [])
    return found


class Refused(Exception):
    """The gate cannot judge: say why and change nothing."""


@dataclass
class Report:
    gate: str
    problems: List[str] = field(default_factory=list)
    new_tests: List[str] = field(default_factory=list)
    pages: List[str] = field(default_factory=list)
    proof: List[str] = field(default_factory=list)  # the tests gate's faults, and the faults gate's entries

    def text(self) -> str:
        lines = [f"{'PASS' if not self.problems else 'FAIL'} {self.gate} gate"]
        lines += [f"  new test: {t}" for t in self.new_tests]
        lines += [f"  page: {p}" for p in self.pages]
        lines += [f"  {p}" for p in self.proof]
        lines += [f"  problem: {p}" for p in self.problems]
        return "\n".join(lines)


class Gate:
    def __init__(self, repo: Path, runner: Runner = run, clock: Callable[[], float] = time.monotonic):
        self.repo = Path(repo)
        self.run = runner
        self.clock = clock

    # git, always for real: the gates' own reading of the history must not be faked by what they run.
    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True, text=True).stdout.strip()

    def _tool(self, cmd: List[str], cwd: Path, env: Optional[dict] = None, timeout: Optional[float] = None) -> Tuple[int, str]:
        """Runs a command; a command or tool not found (exit 127) is the machine's, not the change's: refuse."""
        code, out = self.run(cmd, cwd, env, timeout=timeout) if timeout else self.run(cmd, cwd, env)
        if code == 127 or "with exit code 127" in out:
            raise Refused(
                f"`{' '.join(cmd)}` could not run a tool ({_last_error(out)}): set the worktree up first "
                "(`npm ci` at its root and in e2e/, or `just setup`; `just setup-docs` for the documentation site)"
            )
        return code, out

    def _is_ancestor(self, older: str, newer: str) -> bool:
        return subprocess.run(["git", "merge-base", "--is-ancestor", older, newer], cwd=self.repo).returncode == 0

    def _changed(self, a: str, b: str) -> List[str]:
        # Both sides of a rename: a file renamed away is a change to the path it had.
        return [p for p in self.git("diff", "--name-only", "--no-renames", f"{a}..{b}").splitlines() if p]

    def _ready(self, database: bool = True) -> None:
        if database and not os.environ.get("DATABASE_URL"):
            raise Refused("DATABASE_URL is not set: the server's tests need a Postgres (`just db`)")
        if self.git("status", "--porcelain"):
            raise Refused("the worktree has uncommitted changes: commit or remove them first")

    def _cargo(self, env: Optional[dict] = None) -> Tuple[Optional[Dict[str, str]], List[str]]:
        code, out = self.run(CARGO_TEST + ["--no-fail-fast"], self.repo, env)
        if did_not_compile(code, out):
            return None, []
        problems = []
        code, wasm = self.run(WASM_BUILD, self.repo, env)
        if code != 0:
            problems.append("the wasm32 build fails: " + _last_error(wasm))
        return cargo_results(out), problems

    def _playwright(self, files: List[str], listing: bool) -> Dict[str, Optional[str]]:
        return playwright_results(self._playwright_report(files, listing))

    def _playwright_report(self, files: List[str], listing: bool, timeout: Optional[float] = None) -> dict:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "report.json"
            args = ["npx", "playwright", "test"] + (["--list"] if listing else files) + ["--reporter=json"]
            env = {"PLAYWRIGHT_JSON_OUTPUT_FILE": str(out), "PLAYWRIGHT_JSON_OUTPUT_NAME": str(out)}
            self._tool(args, self.repo / "e2e", env, timeout=timeout)
            if not out.exists():
                raise Refused("Playwright wrote no report: run `just e2e` by hand to see why")
            return json.loads(out.read_text(encoding="utf-8"))

    def _prepare_browser(self) -> None:
        for recipe in ("build", "server", "e2e-fixtures"):
            code, out = self._tool(["just", recipe], self.repo)
            if code != 0:
                raise Refused(f"`just {recipe}` fails: {_last_error(out)}")

    @staticmethod
    def _specs(paths: List[str]) -> List[str]:
        return sorted(p[len("e2e/") :] for p in paths if glob("e2e/tests/*.spec.ts", p))

    def red(self, base: str) -> Report:
        self._ready()
        base, red = self.git("rev-parse", base), self.git("rev-parse", "HEAD")
        if base == red or not self._is_ancestor(base, red):
            raise Refused(f"HEAD {red[:12]} is not a commit after the base {base[:12]}")
        report = Report("red")
        changed = self._changed(base, red)
        rust = any(p.endswith(".rs") or p.endswith("Cargo.toml") for p in changed)
        specs = self._specs(changed)
        # Any change under e2e/ touches the browser tests. One that is not a spec (a helper every spec imports, the
        # fixtures, the configuration, the packages) can change any spec's outcome; the tiers and the notes cannot,
        # and every new spec is listed in the tiers.
        browser = [p for p in changed if p.startswith("e2e/")]
        shared = any(
            not glob("e2e/tests/*.spec.ts", p) and p != "e2e/tiers.json" and not p.endswith(".md") for p in browser
        )
        if browser:
            self._prepare_browser()

        # What the base has, listed in the same worktree; always back to where the gate started.
        start = self.git("rev-parse", "--abbrev-ref", "HEAD")
        start = red if start == "HEAD" else start
        rust_before: set = set()
        browser_before: set = set()
        self.git("checkout", "-q", "--detach", base)
        try:
            if rust:
                _, listed = self.run(CARGO_TEST + ["--", "--list"], self.repo)
                rust_before = cargo_listing(listed)
            if browser:
                browser_before = set(self._playwright([], listing=True))
        finally:
            self.git("checkout", "-q", start)

        if rust:
            results, problems = self._cargo()
            if results is None:
                report.problems.append(
                    "Rust: the tests do not compile at the red commit: add the new API as signatures with todo!() bodies"
                )
            else:
                found, new = judge_red(rust_before, results, "Rust")
                report.problems += found + problems
                report.new_tests += new
        if browser:
            listing = self._playwright_report([], listing=True)
            # One file that does not load empties Playwright's whole listing: say which and why, and judge nothing
            # else from a listing that holds no test.
            unloaded = playwright_load_errors(listing)
            if unloaded:
                report.problems += [f"Playwright: {spec} does not load: {error}" for spec, error in sorted(unloaded.items())]
                return report
            listed = set(playwright_results(listing))
            # A test of the base the red commit no longer lists is gone, in whichever spec it was.
            report.problems += [f"Playwright: the test {name} is gone" for name in sorted(browser_before - listed)]
            # Run every spec after a shared change; otherwise the specs that gain a test and the changed specs that
            # still hold one, since an unchanged spec with unchanged helpers runs as it did at the base.
            listed_specs = {spec_of(n) for n in listed}
            changed_specs = {spec[len("tests/") :] for spec in specs}
            to_run = listed_specs if shared else ({spec_of(n) for n in listed - browser_before} | changed_specs) & listed_specs
            if to_run:
                files = [] if shared else sorted("tests/" + spec for spec in to_run)
                results = self._playwright(files, listing=False)
                before = {name for name in browser_before & listed if spec_of(name) in to_run}
                found, new = judge_red(before, results, "Playwright")
                report.problems += found
                report.new_tests += new
        if not report.new_tests and not report.problems:
            report.problems.append("the red commit adds no test")
        return report

    def green(self, base: str, red: str) -> Report:
        self._ready()
        base, red, head = self.git("rev-parse", base), self.git("rev-parse", red), self.git("rev-parse", "HEAD")
        if head == red or not self._is_ancestor(red, head):
            raise Refused(f"the red commit {red[:12]} is not an earlier commit of HEAD {head[:12]}")
        report = Report("green")

        changed = self._changed(red, head)
        for path in sorted(changed):
            if is_guarded(path):
                report.problems.append(f"{path}: changed since the red commit, and only the test-writer may change it")
            elif path.endswith(".rs") and self._blocks_at(red, path) != self._blocks_at(head, path):
                report.problems.append(f"{path}: a #[cfg(test)] block changed since the red commit")

        strict = {"RUSTFLAGS": "-D warnings"}
        results, problems = self._cargo(strict)
        if results is None:
            report.problems.append("Rust: the tests do not compile with warnings denied")
        else:
            report.problems += [f"Rust: {name} fails" for name, outcome in sorted(results.items()) if outcome == "failed"]
            report.problems += problems
        for cmd in (["just", "format-check"], ["scripts/check-version.sh", base]):
            code, out = self._tool(cmd, self.repo)
            if code != 0:
                report.problems.append(f"`{' '.join(cmd)}` fails: {_last_error(out)}")

        specs = self._specs(self._changed(base, red))
        if specs:
            self._prepare_browser()
            results = self._playwright(specs, listing=False)
            report.problems += [f"Playwright: {name} fails" for name, s in sorted(results.items()) if s in _FAILED]
        return report

    def docs(self, base: str) -> Report:
        self._ready(database=False)
        base, head = self.git("rev-parse", base), self.git("rev-parse", "HEAD")
        if base == head or not self._is_ancestor(base, head):
            raise Refused(f"HEAD {head[:12]} is not a commit after the base {base[:12]}")
        report = Report("docs")
        changed = sorted(self._changed(base, head))
        report.pages = changed

        def exists(path: str) -> bool:
            return (self.repo / path).is_file()

        missing = set()
        for path in changed:
            if is_guarded(path):
                report.problems.append(f"{path}: a check or the factory's own file, which the documentation flow may not change")
            elif any(glob(pattern, path) for pattern in SITE_CONFIGURATION):
                report.problems.append(
                    f"{path}: the documentation site's configuration, code or dependencies, which the documentation flow may not change"
                )
            elif not is_documentation(path):
                report.problems.append(f"{path}: not documentation, and the documentation flow changes only documentation")
            pair = other_language(path)
            if pair and exists(path) != exists(pair[0]):
                missing.add((path, *pair) if exists(path) else (pair[0], path, "fr-CA" if path.startswith(DOCS_FR) else "English"))
        for present, absent, language in sorted(missing):
            report.problems.append(f"{present}: its {language} page {absent} is missing")

        if any(p.startswith("website/") for p in changed):
            checks = (
                (DOCS_PRETTIER, "e2e", "the Prettier check of website/ fails", _last_error),
                (DOCS_BUILD, "website", "the documentation site does not build", build_error),
            )
            for cmd, folder, failure, cause in checks:
                code, out = self._tool(list(cmd), self.repo / folder)
                if code != 0:
                    report.problems.append(f"{failure}: {cause(out)}")
        return report

    # ---- the tests flow -------------------------------------------------------------------------------------------

    def note_path(self) -> Path:
        """Where the tests gate notes the fault it holds: in the worktree's git folder, outside the worktree."""
        return Path(self.git("rev-parse", "--absolute-git-dir")) / "factory-faults.json"

    def _note(self, **fields: Optional[str]) -> None:
        path = self.note_path()
        note = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
        note.update(fields)
        path.write_text(json.dumps(note), encoding="utf-8")

    def _no_note(self) -> None:
        path = self.note_path()
        if not path.exists():
            return
        try:
            note = json.loads(path.read_text(encoding="utf-8"))
        except ValueError:
            note = {}
        held = note.get("fault") or "no file (it stopped between faults)"
        raise Refused(
            f"a tests gate was stopped while it held a fault in {held}: put that file back "
            f"(`git -C {self.repo} checkout -- <file>`), run `just build` in the worktree (web/pkg, which git ignores, "
            f"may hold a faulted build{'; a copy of the good one is in ' + note['pkg'] if note.get('pkg') else ''}), "
            f"then delete {path}"
        )

    def _mutants_ready(self) -> None:
        code, out = self.run([str(MUTANTS), "mutants", "--version"], self.repo)
        if code != 0 or f"cargo-mutants {MUTANTS_VERSION}" not in out:
            raise Refused(
                f"cargo-mutants {MUTANTS_VERSION} is not at {MUTANTS} ({_last_error(out)}): in the main checkout, run "
                f"`cargo install cargo-mutants --version {MUTANTS_VERSION} --locked --root .tools`"
            )

    def _spec(self, faults_file: Path) -> dict:
        try:
            return fl.read_spec(Path(faults_file).read_text(encoding="utf-8"))
        except (OSError, fl.SpecError) as why:
            raise Refused(f"the plan's faults: {why}") from None

    def _lister(self, files: List[str]) -> List[dict]:
        cmd = [str(MUTANTS), "mutants", "--list", "--json"] + [a for f in files for a in ("--file", f)]
        code, out = self.run(cmd, self.repo)
        start = re.search(r"(?m)^\[", out)
        if code != 0 or not start:
            raise Refused(f"`cargo mutants --list` fails: {_last_error(out)}")
        return json.JSONDecoder().raw_decode(out, start.start())[0]

    def _show(self, rev: str, path: str) -> Optional[str]:
        shown = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=self.repo, capture_output=True, text=True)
        return shown.stdout if shown.returncode == 0 else None

    def _declared_for_tests(self, rev: str, path: str) -> bool:
        """Whether a Rust file is a module its parent declares for tests alone (`#[cfg(test)] mod <name>;`)."""
        file = Path(path)
        name, folder = (file.parent.name, file.parent.parent) if file.stem == "mod" else (file.stem, file.parent)
        for parent in (folder / "mod.rs", folder.with_suffix(".rs"), folder / "lib.rs", folder / "main.rs"):
            src = self._show(rev, parent.as_posix())
            if src and any(re.search(r"\bmod\s+" + re.escape(name) + r"\s*;", src[a:b]) for a, b in fl.test_items(src)):
                return True
        return False

    def _only_tests(self, base: str, head: str, changed: List[str]) -> List[str]:
        """Rule 1: every changed file is a test, or test code, or one of the few files a new test may need."""
        problems = []
        checks = [p for p in GUARDED if p not in TEST_FILES]
        for path in sorted(changed):
            before, after = self._show(base, path), self._show(head, path)
            if path in VERSION_FILES:
                if not version_only(path, before or "", after or ""):
                    problems.append(f"{path}: changes more than the version line of cityloom_editor")
            elif path in TESTS_FLOW_FILES:
                continue
            elif any(glob(p, path) for p in checks):
                problems.append(f"{path}: a check or the factory's own file, which the tests flow may not change")
            elif path.startswith("website/") or is_documentation(path):
                problems.append(f"{path}: documentation, which the tests flow may not change")
            elif glob("e2e/tests/**", path) and not glob("e2e/tests/**/*.spec.ts", path) and before is not None:
                if not only_added(before, after or ""):
                    problems.append(f"{path}: a helper of the specs, which may gain lines but not change one")
            elif is_test_file(path):
                continue
            elif path.endswith(".rs") and before is not None:
                if after is None or _squash(without_test_code(before)) != _squash(without_test_code(after)):
                    problems.append(f"{path}: product code changed, and only test code (#[cfg(test)]) may")
            elif path.endswith(".rs") and after is not None and (
                not _squash(without_test_code(after)) or self._declared_for_tests(head, path)
            ):
                continue
            else:
                problems.append(f"{path}: not a test, and the tests flow changes only tests")
        return problems

    def _tests_kept(self, base: str, head: str, changed: List[str]) -> List[str]:
        """Rule 2: every test of the base is in the head with the same text (attributes included: not ignored)."""
        problems = []
        for path in sorted(changed):
            before = self._show(base, path)
            if before is None:
                continue
            if path.endswith(".rs"):
                read = test_functions
            elif glob("e2e/tests/**/*.spec.ts", path):
                read = playwright_tests
            else:
                continue
            left = Counter(text for _, text in read(self._show(head, path) or ""))
            for name, text in read(before):
                if left[text]:
                    left[text] -= 1
                else:
                    problems.append(f"{path}: the existing test {name} is changed or gone")
        return problems

    def _proof_kept(self, base: str, head: str, proved: str, spec: dict) -> Optional[str]:
        """None when the proof made at `proved` holds for `head`: the branch's commits are the same but for the version
        lines, and no file the plan faults changed between the two bases. Otherwise why the faults run again."""
        try:
            proved = self.git("rev-parse", "--verify", f"{proved}^{{commit}}")
        except subprocess.CalledProcessError:
            return f"{proved} is not a commit of this repository"
        old_base = self.git("merge-base", proved, base)
        moved = [f for f in sorted({e.file for e in fl.entries(spec)}) if self.git("diff", "--name-only", old_base, base, "--", f)]
        if moved:
            return f"{', '.join(moved)} changed between the two bases"
        if self._patch(old_base, proved) != self._patch(base, head):
            return "the branch's changes differ from those proved, beyond the version lines"
        return None

    def _patch(self, a: str, b: str) -> List[str]:
        out = self.git("diff", a, b, "--", ".", *[f":(exclude){f}" for f in VERSION_FILES])
        return [re.sub(r"^@@ .*? @@", "@@", line) for line in out.splitlines() if not line.startswith("index ")]

    def tests(self, base: str, faults_file: Path, proved: Optional[str] = None, brief: bool = False) -> Report:
        self._ready()
        self._mutants_ready()
        self._no_note()
        spec = self._spec(faults_file)
        base, head = self.git("rev-parse", base), self.git("rev-parse", "HEAD")
        if base == head or not self._is_ancestor(base, head):
            raise Refused(f"HEAD {head[:12]} is not a commit after the base {base[:12]}")
        report = Report("tests")
        changed = self._changed(base, head)

        # Rules 1 and 2, from the history alone.
        report.problems += self._only_tests(base, head, changed)
        report.problems += self._tests_kept(base, head, changed)
        if report.problems:
            return report

        # Rules 3 and 4: what the base and the head list, and every test passing at the head as green runs them.
        rust_new, browser_new, listing = self._run_tests(base, head, changed, report)
        if report.problems:
            return report
        if not rust_new and not browser_new:
            report.problems.append("the branch adds no test")
            return report
        report.new_tests = rust_new + browser_new

        kept = self._proof_kept(base, head, proved, spec) if proved else "no earlier proof"
        notes = []
        if proved and kept is None:
            notes.append(f"the faults were run at {proved}: the branch is the same since, but for the version lines, and no faulted file changed")
        elif proved:
            notes.append(f"the faults run again: {kept}")
        made: List[Tuple[fl.Entry, List[fl.Fault]]] = []
        if kept is not None:
            made, spec_problems = fl.make(self.repo, spec, self._lister)
            if spec_problems:
                report.problems += [f"the plan's faults: {p}" for p in spec_problems]
                return report

        proof = _Proof(report.new_tests)
        self._note(head=head, fault=None, pkg=None)
        try:
            if rust_new:
                self._prove_native(rust_new, made, proof, report, prove=kept is not None)
            if browser_new and not report.problems:
                self._prove_browser(browser_new, listing, made, proof, report, prove=kept is not None)
        finally:
            self.note_path().unlink()
        if report.problems or kept is None:
            report.proof = notes
            return report
        for test in report.new_tests:
            if not proof.caught[test]:
                report.problems.append(f"the new test {test} catches none of the {proof.ran[test]} faults run")
        report.proof = notes + proof.lines(brief)
        return report

    def _run_tests(self, base: str, head: str, changed: List[str], report: Report) -> Tuple[List[str], List[str], dict]:
        """Rules 3 and 4. The new Rust and browser tests (head's listing less the base's), and the head's Playwright
        listing; each failure goes into the report."""
        browser = [p for p in changed if p.startswith("e2e/")]
        shared = any(not glob("e2e/tests/*.spec.ts", p) and p != "e2e/tiers.json" and not p.endswith(".md") for p in browser)
        if browser:
            self._prepare_browser()
        start = self.git("rev-parse", "--abbrev-ref", "HEAD")
        start = head if start == "HEAD" else start
        browser_before: set = set()
        self.git("checkout", "-q", "--detach", base)
        try:
            _, listed = self.run(CARGO_TEST + ["--", "--list"], self.repo)
            rust_before = cargo_listing(listed)
            if browser:
                browser_before = set(self._playwright([], listing=True))
        finally:
            self.git("checkout", "-q", start)

        rust_new: List[str] = []
        results, problems = self._cargo({"RUSTFLAGS": "-D warnings"})
        if results is None:
            report.problems.append("Rust: the tests do not compile with warnings denied")
        else:
            report.problems += [f"Rust: {name} fails" for name, outcome in sorted(results.items()) if outcome == "failed"]
            report.problems += [f"Rust: the test {name} is gone" for name in sorted(rust_before - set(results))]
            rust_new = sorted(set(results) - rust_before)
            report.problems += [f"Rust: the new test {name} is ignored" for name in rust_new if results[name] == "ignored"]
            report.problems += problems
        for cmd in (["just", "format-check"], ["scripts/check-version.sh", base], ["just", "test-scripts"]):
            code, out = self._tool(cmd, self.repo)
            if code != 0:
                report.problems.append(f"`{' '.join(cmd)}` fails: {_last_error(out)}")

        browser_new: List[str] = []
        listing: dict = {}
        if browser:
            listing = self._playwright_report([], listing=True)
            unloaded = playwright_load_errors(listing)
            if unloaded:
                report.problems += [f"Playwright: {spec} does not load: {error}" for spec, error in sorted(unloaded.items())]
                return rust_new, browser_new, listing
            listed = set(playwright_results(listing))
            report.problems += [f"Playwright: the test {name} is gone" for name in sorted(browser_before - listed)]
            browser_new = sorted(listed - browser_before)
            changed_specs = {p[len("e2e/tests/") :] for p in browser if glob("e2e/tests/*.spec.ts", p)}
            to_run = {spec_of(n) for n in listed} if shared else changed_specs & {spec_of(n) for n in listed}
            if to_run:
                files = [] if shared else sorted("tests/" + spec for spec in to_run)
                results = self._playwright(files, listing=False)
                report.problems += [f"Playwright: {name} fails" for name, s in sorted(results.items()) if s in _FAILED]
                report.problems += [f"Playwright: the new test {name} is skipped" for name in browser_new if results.get(name) == "skipped"]
        return rust_new, browser_new, listing

    def _deadline_passed(self, phase: dict) -> bool:
        now = self.clock()
        if "start" not in phase:
            phase["start"] = now
            return False
        return now - phase["start"] > DEADLINE

    def _prove(self, tests: List[str], faults: List[fl.Fault], limit: int, build, check, proof: "_Proof") -> None:
        """Runs faults (up to `limit`, none starting after DEADLINE) for the tests: each applied, built (`build`: False
        when it does not compile), the tests that have not caught three run (`check`), and taken back."""
        phase: dict = {}
        chosen = faults[:limit]
        proof.left_out += [f.name for f in faults[limit:]]
        for fault in chosen:
            if self._deadline_passed(phase):
                proof.left_out.append(fault.name)
                continue
            pending = [t for t in tests if len(proof.caught[t]) < CAUGHT_ENOUGH]
            if not pending:
                proof.not_run += 1
                continue
            self._note(fault=fault.file)
            with fl.applied(self.repo, fault):
                built = build(fault)
                outcomes = check(built, pending) if built is not False else None
            self._note(fault=None)
            if outcomes is None:
                proof.no_compile.append(fault.name)
                continue
            for test, outcome in outcomes.items():
                proof.ran[test] += 1
                if outcome == "failed":
                    proof.caught[test].append(fault.name)
                elif outcome in ("hung", "crashed"):
                    getattr(proof, outcome).append(f"{fault.name} ({test})")
            if all(o == "ok" for o in outcomes.values()):
                proof.uncaught.append(fault.name)

    def _ordered(self, made: List[Tuple[fl.Entry, List[fl.Fault]]], keep) -> List[fl.Fault]:
        """The faults in the order they are run: in turn, one from each entry of the plan, then the next of each."""
        groups = [[f for f in found if keep(f)] for _, found in made]
        return fl.take(groups, sum(len(g) for g in groups))

    def _prove_native(self, tests: List[str], made, proof: "_Proof", report: Report, prove: bool) -> None:
        # Built without denying warnings: a fault often leaves a parameter unused. The first build is the workspace's,
        # to find the binaries; the faults rebuild only the packages that hold the new tests.
        code, out = self.run(CARGO_TEST + ["--no-run", "--message-format=json"], self.repo)
        binaries = test_binaries(out)
        missing = [t for t in tests if t.split("::", 1)[0] not in binaries]
        if code != 0 or missing:
            report.problems.append(f"the new tests' binaries could not be built for the faults: {_last_error(out) if code else ', '.join(missing)}")
            return
        packages = sorted({binaries[t.split("::", 1)[0]].package for t in tests})
        build = ["cargo", "test", "--no-run", "--message-format=json"] + [a for p in packages for a in ("-p", p)]
        if "cityloom_server" in packages:
            build += ["--features", "cityloom_server/test-provider"]
        code, out = self.run(build, self.repo)
        binaries = test_binaries(out) if code == 0 else binaries
        limits: Dict[str, float] = {}
        for test in tests:
            begun = self.clock()
            outcome = self._run_alone(binaries, test, BASELINE_TIMEOUT)
            limits[test] = max(NATIVE_TIMEOUT, 4 * (self.clock() - begun))
            if outcome != "ok":
                report.problems.append(f"the new test {test} does not pass when it runs alone")
        if report.problems or not prove:
            return

        def built(fault: fl.Fault):
            code, out = self.run(build, self.repo)
            return test_binaries(out) if code == 0 else False

        def check(bins: Dict[str, Binary], pending: List[str]) -> Dict[str, str]:
            return {t: self._run_alone(bins, t, limits[t]) for t in pending}

        self._prove(tests, self._ordered(made, lambda f: True), LIMITS["native"], built, check, proof)

    def _run_alone(self, binaries: Dict[str, Binary], test: str, timeout: float) -> str:
        """One Rust test, by its full name, in a process of its own: ok, failed (an assertion or a panic), hung (killed
        at the timeout), crashed (ended any other way) or missing (the name ran no test)."""
        label, name = test.split("::", 1)
        binary = binaries.get(label)
        if binary is None:
            return "missing"
        code, out = self.run(
            [binary.path, name, "--exact", "--test-threads=1"], Path(binary.cwd), {"CARGO_MANIFEST_DIR": binary.cwd}, timeout=timeout
        )
        if code == TIMED_OUT:
            return "hung"
        if code == 0:
            return "ok" if re.search(r"test result: ok\. 1 passed", out) else "missing"
        if "test result: FAILED" in out or "panicked" in out:
            return "failed"
        return "crashed"

    def _prove_browser(self, tests: List[str], listing: dict, made, proof: "_Proof", report: Report, prove: bool) -> None:
        tiers = json.loads((self.repo / "e2e" / "tiers.json").read_text(encoding="utf-8")) if (self.repo / "e2e" / "tiers.json").exists() else {}
        archived = set(tiers.get("e2e", []))
        for test in tests:
            if Path(spec_of(test)).name.removesuffix(".spec.ts") in archived:
                report.problems.append(f"the new test {test} is in the e2e tier (E2E_ARCHIVE=1), which the gate cannot prove against faults")
        if report.problems:
            return
        where = playwright_locations(listing)
        pkg, saved = self.repo / "web" / "pkg", Path(self.git("rev-parse", "--absolute-git-dir")) / "factory-pkg"
        if saved.exists():
            shutil.rmtree(saved)
        if pkg.exists():
            shutil.copytree(pkg, saved)
        self._note(pkg=str(saved))
        try:
            failure = self._build_level_1()
            if failure:
                raise Refused(f"the wasm does not build at optimisation level 1: {failure}")
            first = self._run_browser(where, tests)
            failing = [t for t in tests if first[t] != "ok"]
            if failing:
                raise Refused(f"the new browser tests fail at optimisation level 1 with no fault: {', '.join(failing)}")
            if not prove:
                return

            def built(fault: fl.Fault):
                if fault.file.startswith("web/") and fault.file.endswith(".css"):
                    return True  # served as it is
                return False if self._build_level_1() else True

            def check(_, pending: List[str]) -> Dict[str, str]:
                return self._run_browser(where, pending)

            def unreachable(f: fl.Fault) -> bool:
                return f.kind != "css" and f.file.startswith("crates/") and f.file.split("/")[1] in NOT_BROWSER

            proof.unreachable += [f.name for f in self._ordered(made, unreachable)]
            self._prove(tests, self._ordered(made, lambda f: not unreachable(f)), LIMITS["browser"], built, check, proof)
        finally:
            if pkg.exists():
                shutil.rmtree(pkg)
            if saved.exists():
                shutil.copytree(saved, pkg)
                shutil.rmtree(saved)
            self._note(pkg=None)

    def _build_level_1(self) -> Optional[str]:
        """The editor's wasm at optimisation level 1 into web/pkg, as `just build` makes it in release; why not, or None."""
        env = dict(LEVEL_1)
        if platform.system() == "Darwin":  # as scripts/build.sh: rust-lld may not find libLLVM beside itself
            _, sysroot = self.run(["rustc", "--print", "sysroot"], self.repo)
            env["DYLD_FALLBACK_LIBRARY_PATH"] = sysroot.strip() + "/lib"
        code, out = self.run(["cargo", "build", "--target", "wasm32-unknown-unknown", "-p", "cityloom_editor"], self.repo, env)
        if code != 0:
            return _last_error(out)
        _, target = self.run(["scripts/target-dir.sh"], self.repo)
        wasm = f"{target.strip().splitlines()[-1]}/wasm32-unknown-unknown/debug/cityloom_editor.wasm"
        bindgen = [str(self.repo / ".tools" / "bin" / "wasm-bindgen"), "--target", "web", "--no-typescript", "--out-dir", "web/pkg", wasm]
        code, out = self._tool(bindgen, self.repo)
        return _last_error(out) if code != 0 else None

    def _run_browser(self, where: Dict[str, Tuple[str, int, str]], tests: List[str]) -> Dict[str, str]:
        """The new browser tests, by file and line: ok, failed (its own assertion or timeout) or hung (the run's time
        ran out first)."""
        places = sorted({f"tests/{where[t][0]}:{where[t][1]}" for t in tests})
        projects = sorted({f"--project={where[t][2]}" for t in tests if where[t][2]})
        flags = [f"--timeout={SPEC_TIMEOUT_MS}", f"--global-timeout={BROWSER_RUN * 1000}"]
        results = playwright_results(self._playwright_report(places + projects + flags, listing=False, timeout=BROWSER_RUN + 60))
        outcome = {"expected": "ok", "flaky": "ok", "unexpected": "failed"}
        return {t: outcome.get(results.get(t) or "", "hung") for t in tests}

    def faults(self, faults_file: Path, browser: bool = False) -> Report:
        """The plan's faults against a checkout, before the plan is posted: every file, function, message and
        declaration there, each entry yielding a fault, no more entries than the limit. Builds nothing."""
        spec = self._spec(faults_file)
        if spec.get("rust"):
            self._mutants_ready()
        made, problems = fl.make(self.repo, spec, self._lister)
        report = Report("faults", problems=problems)
        for entry, found in made:
            report.proof.append(f"entry: {entry.kind} {entry.file} {entry.name}: {_count(len(found), 'fault')}")
        kind = "browser" if browser else "native"
        if len(made) > LIMITS[kind]:
            report.problems.append(f"{len(made)} entries: more than the {LIMITS[kind]} faults a {kind} test can be proved against")
        if browser:
            for entry in fl.entries(spec):
                crate = entry.file.split("/")[1] if entry.file.startswith("crates/") else ""
                problem = f"{entry.kind} {entry.file}: {crate} does not reach the browser, so a browser test cannot catch its faults"
                if crate in NOT_BROWSER and problem not in report.problems:
                    report.problems.append(problem)
        return report

    def _blocks_at(self, rev: str, path: str) -> List[str]:
        shown = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=self.repo, capture_output=True, text=True)
        return cfg_test_blocks(shown.stdout) if shown.returncode == 0 else []


def _squash(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def _count(n: int, word: str) -> str:
    return f"{n} {word}{'' if n == 1 else 's'}"


class _Proof:
    """What the faults showed, test by test and fault by fault."""

    def __init__(self, tests: List[str]):
        self.tests = tests
        self.caught: Dict[str, List[str]] = {t: [] for t in tests}
        self.ran: Dict[str, int] = {t: 0 for t in tests}
        self.uncaught: List[str] = []
        self.no_compile: List[str] = []
        self.hung: List[str] = []
        self.crashed: List[str] = []
        self.unreachable: List[str] = []
        self.left_out: List[str] = []
        self.not_run = 0

    def lines(self, brief: bool) -> List[str]:
        """The report's lines. Brief (for the test-writer) names the tests that caught nothing, and no fault."""
        lines = []
        for test in self.tests:
            found = self.caught[test]
            if not found:
                lines.append(f"caught nothing: {test}")
            elif not brief:
                lines.append(f"caught: {test}: {_count(len(found), 'fault')}, the first: " + "; ".join(found[:CAUGHT_ENOUGH]))
        if brief:
            return lines
        lines += [f"not caught by any new test: {f}" for f in self.uncaught]
        lines += [f"did not compile: {f}" for f in self.no_compile]
        lines += [f"hung: {f}" for f in self.hung]
        lines += [f"crashed: {f}" for f in self.crashed]
        lines += [f"not run in the browser, which its code does not reach: {f}" for f in self.unreachable]
        if self.left_out:
            lines.append(f"left out by the limit: {_count(len(self.left_out), 'fault')}: " + "; ".join(self.left_out))
        if self.not_run:
            lines.append(f"not run, every new test had caught three: {_count(self.not_run, 'fault')}")
        return lines


def _last_error(output: str) -> str:
    lines = [line for line in output.splitlines() if line.strip()]
    errors = [line for line in lines if line.lstrip().startswith("error")]
    return (errors or lines or ["no output"])[-1].strip()


_BOX = " \t│╰╭─▶×┌└├"


def build_error(output: str, limit: int = 12) -> str:
    """What a failed Docusaurus build says went wrong: from its first error on, without the box drawing, blank lines and
    stack frames that bury the cause, at most `limit` lines."""
    lines = [line.lstrip(_BOX) for line in output.splitlines()]
    start = next((i for i, line in enumerate(lines) if line.startswith(("[ERROR]", "Error"))), None)
    if start is None:
        return _last_error(output)
    kept = [line.rstrip() for line in lines[start:] if line.strip() and not line.startswith("at ")][:limit]
    return "\n    ".join(kept)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("gate", choices=["red", "green", "docs", "tests", "faults"])
    parser.add_argument("--repo", required=True, help="the issue's worktree (faults: the main checkout)")
    parser.add_argument("--base", help="where the branch started (origin/main); all but faults")
    parser.add_argument("--red", help="the red commit (green only)")
    parser.add_argument("--faults", help="the plan's faults: its ## Faults JSON, or the plan (tests and faults)")
    parser.add_argument("--proved", help="tests: the head the faults were last run at, kept over a rebase that changed nothing they rest on")
    parser.add_argument("--brief", action="store_true", help="tests: name no fault in the report (for the test-writer)")
    parser.add_argument("--browser", action="store_true", help="faults: the plan has browser tests (a lower limit)")
    args = parser.parse_args(argv)
    if args.gate != "faults" and not args.base:
        parser.error(f"{args.gate} needs --base")
    if args.gate in ("tests", "faults") and not args.faults:
        parser.error(f"{args.gate} needs --faults")
    try:
        gate = Gate(Path(args.repo))
        if args.gate == "tests":
            report = gate.tests(args.base, Path(args.faults), proved=args.proved, brief=args.brief)
        elif args.gate == "faults":
            report = gate.faults(Path(args.faults), browser=args.browser)
        elif args.gate == "red":
            report = gate.red(args.base)
        elif args.gate == "docs":
            report = gate.docs(args.base)
        else:
            if not args.red:
                parser.error("green needs --red")
            report = gate.green(args.base, args.red)
    except Refused as why:
        print(f"REFUSED {args.gate} gate: {why}")
        return 2
    print(report.text())
    return 1 if report.problems else 0


if __name__ == "__main__":
    sys.exit(main())
