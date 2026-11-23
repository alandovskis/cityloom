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

The red and green gates need DATABASE_URL (the server's tests); the docs gate does not. Exit 0 passes, 1 fails with the problems listed, 2 refuses to run (a dirty
worktree, a red commit not in the history, no database). Standard library only;
scripts/include/tests/test_factory_gate.py tests it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path
from typing import Callable, Dict, List, Optional, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts" / "ci"))

from ci_changes import ROOT, glob, rule_for  # noqa: E402

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
_RAW = re.compile(r"b?r(#*)\"")


def _code_mask(src: str) -> List[bool]:
    """True for each character of Rust code, False inside comments, strings and character literals."""
    mask = [True] * len(src)
    i, n = 0, len(src)

    def blank(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            mask[k] = False

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            end = src.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c in "br" and _RAW.match(src, i) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            raw = _RAW.match(src, i)
            close = '"' + raw.group(1)
            end = src.find(close, raw.end())
            end = n if end < 0 else end + len(close)
            blank(i, end)
            i = end
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i, j + 1)
            i = j + 1
        elif c == "'":
            if i + 1 < n and src[i + 1] == "\\":
                end = src.find("'", i + 2)
                end = n if end < 0 else end + 1
                blank(i, end)
                i = end
            elif i + 2 < n and src[i + 2] == "'":
                blank(i, i + 3)
                i += 3
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return mask


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


def run(cmd: List[str], cwd: Path, env: Optional[dict] = None) -> Tuple[int, str]:
    """Runs a command with its stderr in its stdout (cargo's `Running` lines and the tests' own lines stay in order)."""
    print("$ " + " ".join(cmd), file=sys.stderr, flush=True)
    try:
        done = subprocess.run(
            cmd, cwd=cwd, env=dict(os.environ, **(env or {})), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )
    except FileNotFoundError:
        return 127, f"{cmd[0]}: command not found"  # as a shell answers, which `Gate._tool` refuses
    return done.returncode, done.stdout


class Refused(Exception):
    """The gate cannot judge: say why and change nothing."""


@dataclass
class Report:
    gate: str
    problems: List[str] = field(default_factory=list)
    new_tests: List[str] = field(default_factory=list)
    pages: List[str] = field(default_factory=list)

    def text(self) -> str:
        lines = [f"{'PASS' if not self.problems else 'FAIL'} {self.gate} gate"]
        lines += [f"  new test: {t}" for t in self.new_tests]
        lines += [f"  page: {p}" for p in self.pages]
        lines += [f"  problem: {p}" for p in self.problems]
        return "\n".join(lines)


class Gate:
    def __init__(self, repo: Path, runner: Runner = run):
        self.repo = Path(repo)
        self.run = runner

    # git, always for real: the gates' own reading of the history must not be faked by what they run.
    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True, text=True).stdout.strip()

    def _tool(self, cmd: List[str], cwd: Path, env: Optional[dict] = None) -> Tuple[int, str]:
        """Runs a command; a command or tool not found (exit 127) is the machine's, not the change's: refuse."""
        code, out = self.run(cmd, cwd, env)
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

    def _playwright_report(self, files: List[str], listing: bool) -> dict:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "report.json"
            args = ["npx", "playwright", "test"] + (["--list"] if listing else files) + ["--reporter=json"]
            self._tool(args, self.repo / "e2e", {"PLAYWRIGHT_JSON_OUTPUT_FILE": str(out), "PLAYWRIGHT_JSON_OUTPUT_NAME": str(out)})
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

    def _blocks_at(self, rev: str, path: str) -> List[str]:
        shown = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=self.repo, capture_output=True, text=True)
        return cfg_test_blocks(shown.stdout) if shown.returncode == 0 else []


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
    parser.add_argument("gate", choices=["red", "green", "docs"])
    parser.add_argument("--repo", required=True, help="the issue's worktree")
    parser.add_argument("--base", required=True, help="where the branch started (origin/main)")
    parser.add_argument("--red", help="the red commit (green only)")
    args = parser.parse_args(argv)
    try:
        gate = Gate(Path(args.repo))
        if args.gate == "red":
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
