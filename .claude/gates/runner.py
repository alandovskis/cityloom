"""Running the checks the red and green gates read: the Rust tests, the wasm type-check, formatting, browser specs."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import test_surface

ROOT = Path(__file__).resolve().parents[2]

CARGO_TEST = ["cargo", "test", "--workspace", "--features", "cityloom_server/test-provider", "--no-fail-fast"]
WASM_CHECK = [
    "cargo", "build", "--target", "wasm32-unknown-unknown", "--workspace",
    "--exclude", "cityloom_server", "--exclude", "citymodel_builder",
]  # fmt: skip
ENV = {**os.environ, "RUSTFLAGS": "-D warnings"}


@dataclass
class Report:
    problems: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def fail(self, text: str) -> None:
        self.problems.append(text)

    def note(self, text: str) -> None:
        self.notes.append(text)

    def finish(self, gate: str) -> int:
        for n in self.notes:
            print(f"  {n}")
        if self.problems:
            print(f"{gate}: REFUSED")
            for p in self.problems:
                print(f"  - {p}")
            return 1
        print(f"{gate}: PASSED")
        return 0


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, env=ENV, capture_output=True, text=True)


def need_database(report: Report) -> bool:
    if not os.environ.get("DATABASE_URL"):
        report.fail("DATABASE_URL is not set: the server's tests need a Postgres (`just db`, then export the URL)")
        return False
    return True


def cargo_tests(report: Report) -> dict[str, str] | None:
    """Every Rust test's result, or None (and a problem) when the tests did not build."""
    done = run(CARGO_TEST)
    out = done.stdout + done.stderr
    found = test_surface.results(out)
    if not found:  # cargo builds every test binary before it runs one: a build error leaves no result at all
        tail = "\n".join(out.strip().splitlines()[-25:])
        report.fail(f"the Rust tests do not build:\n{tail}")
        return None
    return found


def wasm_builds(report: Report) -> None:
    done = run(WASM_CHECK)
    if done.returncode != 0:
        tail = "\n".join((done.stdout + done.stderr).strip().splitlines()[-25:])
        report.fail(f"the wasm build (cfg(wasm32) code) fails:\n{tail}")


def formatted(report: Report) -> None:
    if run(["just", "format-check"]).returncode != 0:
        report.fail("`just format-check` fails: run `just format` and amend")


@dataclass(frozen=True)
class SpecResult:
    file: str  # e2e/tests/<name>.spec.ts
    line: int
    title: str
    ok: bool


def browser_specs(specs: list[str], report: Report) -> list[SpecResult] | None:
    """Run the named specs (`e2e/tests/x.spec.ts`) through `just e2e` and read each test's result."""
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "results.json"
        args = [s.removeprefix("e2e/") for s in specs]
        done = subprocess.run(
            ["just", "e2e", *args, "--reporter=json"],
            cwd=ROOT,
            env={**ENV, "PLAYWRIGHT_JSON_OUTPUT_NAME": str(out)},
            capture_output=True,
            text=True,
        )
        if not out.exists():
            tail = "\n".join((done.stdout + done.stderr).strip().splitlines()[-25:])
            report.fail(f"the browser specs did not run:\n{tail}")
            return None
        data = json.loads(out.read_text(encoding="utf-8"))
    found: list[SpecResult] = []

    def walk(suite: dict) -> None:
        for spec in suite.get("specs", []):
            name = Path(spec.get("file", "")).name
            found.append(SpecResult(f"e2e/tests/{name}", spec.get("line", 0), spec.get("title", ""), spec.get("ok", False)))
        for child in suite.get("suites", []):
            walk(child)

    for s in data.get("suites", []):
        walk(s)
    return found


def added_lines(diff: str) -> dict[str, set[int]]:
    """Each file's added line numbers in a `git diff -U0`."""
    lines: dict[str, set[int]] = {}
    path = None
    for line in diff.splitlines():
        if line.startswith("+++ "):
            path = line[6:] if line.startswith("+++ b/") else None
        elif path and (m := re.match(r"@@ -\S+ \+(\d+)(?:,(\d+))? @@", line)):
            start, count = int(m.group(1)), int(m.group(2) or 1)
            lines.setdefault(path, set()).update(range(start, start + count))
    return lines
