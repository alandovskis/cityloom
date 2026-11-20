#!/usr/bin/env python3
"""The green gate: the implementer made the red commit pass without changing what it asks.

    python3 .claude/gates/green_gate.py [--base origin/main] [--skip-browser]

Run in the worktree after the implement agent's commits. It passes when:

- exactly one commit since the base carries the `Red-commit:` trailer, and the tree is clean;
- the test surface is as the red commit left it: no protected file (`test_surface.PROTECTED`) changed since, and in
  every other Rust file the `#[cfg(test)]` items are the same, byte for byte;
- every Rust test passes, the wasm builds, `just format-check` and `scripts/check-version.sh` pass;
- every browser spec the branch adds or changes passes.

It does not say the code is good: that is the review's job. It says the code answers the tests it was given.
Needs DATABASE_URL (`just db`), as `just test` does.
"""

from __future__ import annotations

import argparse
import sys

import runner
import test_surface
from test_surface import git, show


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="origin/main")
    ap.add_argument("--skip-browser", action="store_true", help="only when the branch changes no browser spec")
    a = ap.parse_args()
    report = runner.Report()
    root = runner.ROOT

    if git("status", "--porcelain", "--untracked-files=no", cwd=root).strip():
        report.fail("the worktree has uncommitted changes: commit them, the gate checks commits")
        return report.finish("Green gate")
    base = git("merge-base", a.base, "HEAD", cwd=root).strip()
    red = test_surface.red_commit(base, cwd=root)
    report.note(f"red commit {red[:12]}, green {git('rev-parse', '--short=12', 'HEAD', cwd=root).strip()}")

    for path in git("diff", "--name-only", red, "HEAD", cwd=root).split():
        if test_surface.protected(path):
            report.fail(f"`{path}` is test surface and changed after the red commit")
        elif path.endswith(".rs"):
            before, after = show(red, path, cwd=root) or "", show("HEAD", path, cwd=root) or ""
            if test_surface.test_items(before) != test_surface.test_items(after):
                report.fail(f"a `#[cfg(test)]` item of `{path}` changed after the red commit")

    if runner.need_database(report):
        found = runner.cargo_tests(report)
        for k, v in (found or {}).items():
            if v == "FAILED":
                report.fail(f"`{test_surface.test_name(k)}` fails")
        if found is not None:
            report.note(f"{sum(v == 'ok' for v in found.values())} Rust tests pass")
    runner.wasm_builds(report)
    runner.formatted(report)
    if runner.run(["scripts/check-version.sh", a.base]).returncode != 0:
        only_config = all(
            any(test_surface.glob(g, p) for g in ("website/**", "docs/**", "**/*.md", "LICENSE*", ".github/**", "scripts/ci/**", "scripts/include/**", ".claude/**"))
            for p in git("diff", "--name-only", base, "HEAD", cwd=root).split()
        )
        if not only_config:
            report.fail("`scripts/check-version.sh` fails: raise the version (and `cargo update -p cityloom_editor`)")

    specs = sorted(p for p in git("diff", "--name-only", base, "HEAD", cwd=root).split() if p.startswith("e2e/tests/") and p.endswith(".spec.ts"))
    if specs and a.skip_browser:
        report.fail(f"--skip-browser, but the branch changes {', '.join(specs)}")
    elif specs:
        for r in runner.browser_specs(specs, report) or []:
            if not r.ok:
                report.fail(f"browser test `{r.title}` ({r.file}:{r.line}) fails")
    return report.finish("Green gate")


if __name__ == "__main__":
    sys.exit(main())
