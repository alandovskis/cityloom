#!/usr/bin/env python3
"""Decides what CI runs for a change (`.github/workflows/ci.yml`'s `changes` job).

    scripts/ci/ci_changes.py --event pull_request --base <sha> [--head HEAD]

Reads the files the change touches (git diff base..head) and the rules in scripts/ci/spec-map.json, prints what it
decided and, when GITHUB_OUTPUT is set, writes the job's outputs:

    docs_only  true when every file is documentation: the Rust and browser jobs and the Railway deploy are skipped
    unit       true unless docs_only: the unit tests run in full
    e2e        true when some browser spec runs
    specs      the Playwright arguments ("" runs every spec)
    archive    true when `just e2e-archive` runs (its specs: archive, map, street)

Only a pull request is narrowed. A push to main and a manual run run every spec (a push may still be docs_only),
and so does anything doubtful: no files, a base that is not there, a file no rule names. A version bump alone
(Cargo.toml's version and the editor's line of Cargo.lock) is not a change to those files.

Standard library only; scripts/include/tests/test_ci_changes.py tests it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE_SPECS = {"archive", "map", "street"}  # what scripts/e2e-archive.sh runs


def glob(pattern: str, path: str) -> bool:
    """`**` crosses folders (`**/` may be empty), `*` and `?` stay inside one."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif pattern.startswith("**", i):
            out += ".*"
            i += 2
        elif pattern[i] == "*":
            out += "[^/]*"
            i += 1
        elif pattern[i] == "?":
            out += "[^/]"
            i += 1
        else:
            out += re.escape(pattern[i])
            i += 1
    return re.fullmatch(out, path) is not None


def rule_for(rules: dict, path: str) -> dict | None:
    return next((r for r in rules["rules"] if any(glob(p, path) for p in r["paths"])), None)


def matched(rules: dict, path: str) -> bool:
    return rule_for(rules, path) is not None


@dataclass
class Selection:
    docs_only: bool = False
    all: bool = False
    specs: list[str] = field(default_factory=list)

    @property
    def unit(self) -> bool:
        return not self.docs_only

    @property
    def e2e(self) -> bool:
        return not self.docs_only and (self.all or bool(self.specs))

    @property
    def archive(self) -> bool:
        return self.e2e and (self.all or bool(ARCHIVE_SPECS & set(self.specs)))

    def spec_args(self) -> str:
        return "" if self.all else " ".join(f"tests/{s}.spec.ts" for s in sorted(self.specs))


def specs_for(rules: dict, every_spec: list[str], f: str) -> set[str] | None:
    """The specs one changed file asks for; None when it asks for every spec."""
    specs = rule_for(rules, f)["specs"]
    if specs == "all":
        return None
    if specs == "self":
        name = Path(f).name[: -len(".spec.ts")]
        return {name} if name in every_spec else set()
    chosen: set[str] = set()
    if specs != "none":
        for s in specs:
            chosen.update(rules["groups"][s[1:]] if s.startswith("@") else [s])
    return chosen


def select(rules: dict, every_spec: list[str], files: list[str], empty_means_none: bool = False) -> Selection:
    if not files:
        return Selection(all=not empty_means_none)
    docs = 0
    chosen: set[str] = set()
    everything = False
    for f in files:
        rule = rule_for(rules, f)
        if rule is None:
            everything = True
        elif rule.get("docs"):
            docs += 1
        else:
            asked = specs_for(rules, every_spec, f)
            if asked is None:
                everything = True
            else:
                chosen |= asked
    return Selection(
        docs_only=docs == len(files),
        all=everything,
        specs=sorted(every_spec if everything else chosen),
    )


def normalize_toml(text: str) -> str:
    return re.sub(r'(?m)^version = ".*"$', 'version = "-"', text, count=1)


def normalize_lock(text: str) -> str:
    return re.sub(r'(name = "cityloom_editor"\nversion = ")[^"]*', r"\1-", text)


REVISION = re.compile(r"[0-9a-fA-F]{7,64}|HEAD")


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def changed_files(base: str, head: str) -> list[str] | None:
    """The files that differ, a bare version bump left out; None when the base is not known."""
    if not base or set(base) == {"0"}:
        return None
    if not (REVISION.fullmatch(base) and REVISION.fullmatch(head)):
        return None  # only a commit id or HEAD goes to git
    try:
        files = [f for f in git("diff", "--name-only", "--no-renames", base, head, "--").splitlines() if f]
        for name, norm in (("Cargo.toml", normalize_toml), ("Cargo.lock", normalize_lock)):
            if name in files and norm(git("show", f"{base}:{name}")) == norm(git("show", f"{head}:{name}")):
                files.remove(name)
        return files
    except subprocess.CalledProcessError:
        return None


def describe(rule: dict | None) -> str:
    if rule is None:
        return "no rule: all"
    return "docs" if rule.get("docs") else str(rule["specs"])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--event", default="pull_request")
    ap.add_argument("--base", default="")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("files", nargs="*", help="the files, instead of git diff (for trying)")
    a = ap.parse_args()

    rules = json.loads((ROOT / "scripts" / "ci" / "spec-map.json").read_text(encoding="utf-8"))
    every = sorted(p.name[: -len(".spec.ts")] for p in (ROOT / "e2e" / "tests").glob("*.spec.ts"))
    files = a.files or changed_files(a.base, a.head)
    if files is None:
        print("The change is not known: everything runs.")
        s = Selection(all=True)
    else:
        # A change of a version number alone leaves nothing to test but the unit tests.
        s = select(rules, every, files, empty_means_none=True)
        for f in files:
            print(f"  {f}: {describe(rule_for(rules, f))}")
    if a.event != "pull_request" and not s.docs_only:
        s = Selection(all=True)
    out = {
        "docs_only": str(s.docs_only).lower(),
        "unit": str(s.unit).lower(),
        "e2e": str(s.e2e).lower(),
        "specs": s.spec_args(),
        "archive": str(s.archive).lower(),
    }
    print(json.dumps(out))
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as fh:
            fh.writelines(f"{k}={v}\n" for k, v in out.items())
    return 0


if __name__ == "__main__":
    sys.exit(main())
