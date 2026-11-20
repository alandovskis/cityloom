#!/usr/bin/env python3
"""The red gate: the test-writer's red commit is a spec the implementer can be held to.

    python3 .claude/gates/red_gate.py [--base origin/main] [--skip-browser]

Run in the worktree, with the red commit at HEAD and nothing else on the branch. It passes when:

- HEAD is the only commit since the base and carries a `Red-commit: #<issue>` trailer, and the tree is clean;
- everything builds: the Rust tests (new APIs are stubs with `todo!()` bodies), the wasm, and `just format-check`;
- every test the commit adds fails, and so does every existing test it declares it changes (`Changes-test: <name>`
  trailers; a browser test by `e2e:<file>:<line>`);
- every other test passes: a red commit breaks nothing it does not mean to;
- the browser specs the commit adds or changes fail only in the tests it adds or declares.

A new test that already passes is refused: it asks nothing of the implementer.
Needs DATABASE_URL (`just db`), as `just test` does.
"""

from __future__ import annotations

import argparse
import sys

import runner
import test_surface
from test_surface import git


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="origin/main")
    ap.add_argument("--skip-browser", action="store_true", help="only when the commit changes no browser spec")
    a = ap.parse_args()
    report = runner.Report()
    root = runner.ROOT

    if git("status", "--porcelain", "--untracked-files=no", cwd=root).strip():
        report.fail("the worktree has uncommitted changes: the gate checks the red commit, so commit or drop them")
    base = git("merge-base", a.base, "HEAD", cwd=root).strip()
    commits = git("rev-list", f"{base}..HEAD", cwd=root).split()
    head = git("rev-parse", "HEAD", cwd=root).strip()
    if commits != [head]:
        report.fail(f"expected the red commit alone on the branch, found {len(commits)} commits since {a.base}")
    body = git("log", "-1", "--format=%B", "HEAD", cwd=root)
    if f"\n{test_surface.RED_TRAILER}:" not in "\n" + body:
        report.fail(f"HEAD has no `{test_surface.RED_TRAILER}: #<issue>` trailer")
    if report.problems:
        return report.finish("Red gate")

    diff = git("diff", "-U0", base, "HEAD", cwd=root)
    new = test_surface.added_tests(diff)
    declared = test_surface.changed_tests("HEAD", cwd=root)
    rust_declared = [d for d in declared if not d.startswith("e2e:")]
    browser_declared = {d.removeprefix("e2e:") for d in declared if d.startswith("e2e:")}
    report.note(f"red commit {head[:12]}: {len(new)} new Rust test(s), {len(declared)} declared change(s)")

    if runner.need_database(report):
        found = runner.cargo_tests(report)
        if found is not None:
            for t in new:
                mine = {k: v for k, v in found.items() if t.matches(test_surface.test_name(k))}
                if not mine:
                    report.fail(f"new test `{t.name}` ({t.path}) did not run (behind a feature or cfg the gate does not build?)")
                for k, v in mine.items():
                    if v == "ok":
                        report.fail(f"new test `{test_surface.test_name(k)}` already passes: it asks nothing")
                    elif v == "ignored":
                        report.fail(f"new test `{test_surface.test_name(k)}` is ignored")
            for d in rust_declared:
                mine = {k: v for k, v in found.items() if test_surface.test_name(k) == d}
                if not mine:
                    report.fail(f"`{test_surface.CHANGES_TRAILER}: {d}` names no test that ran")
                elif all(v == "ok" for v in mine.values()):
                    report.fail(f"changed test `{d}` still passes: either it does not change or it needs no code")
            for k, v in found.items():
                name = test_surface.test_name(k)
                if v == "FAILED" and not any(t.matches(name) for t in new) and name not in rust_declared:
                    report.fail(f"`{name}` fails but is neither new nor declared (`{test_surface.CHANGES_TRAILER}:`)")
    runner.wasm_builds(report)
    runner.formatted(report)

    specs = sorted(p for p in git("diff", "--name-only", base, "HEAD", cwd=root).split() if p.startswith("e2e/tests/") and p.endswith(".spec.ts"))
    if specs and a.skip_browser:
        report.fail(f"--skip-browser, but the commit changes {', '.join(specs)}")
    elif specs:
        added = runner.added_lines(diff)
        results = runner.browser_specs(specs, report)
        for r in results or []:
            key = f"{r.file.removeprefix('e2e/')}:{r.line}"
            is_new = r.line in added.get(r.file, set())
            if is_new and r.ok:
                report.fail(f"new browser test `{r.title}` ({key}) already passes")
            elif not is_new and not r.ok and key not in browser_declared:
                report.fail(f"browser test `{r.title}` ({key}) fails but is neither new nor declared (`e2e:{key}`)")
            elif not is_new and r.ok and key in browser_declared:
                report.fail(f"changed browser test `{r.title}` ({key}) still passes")
    return report.finish("Red gate")


if __name__ == "__main__":
    sys.exit(main())
