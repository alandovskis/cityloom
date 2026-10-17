#!/usr/bin/env python3
"""Says what a push to main has to deploy: the site (the Railway deploy) and the documentation (GitHub Pages).

    scripts/what_to_deploy.py BEFORE AFTER [--repo DIR]

Prints `site=true|false` and `docs=true|false`, for $GITHUB_OUTPUT, and on stderr why. The files that cannot
change what is deployed are listed below; every other file deploys the site, so a file nobody listed (a new crate,
a new script) deploys rather than being left out. A change to Cargo.toml or Cargo.lock that is only the root
package's version deploys nothing: every pull request raises it. When BEFORE is empty, all zeros or not a commit
of the repository (a branch's first push, a force-push), everything deploys.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Callable, Optional

# Paths under these hold documents, tests or tools that no deploy builds or uploads.
NOT_SITE_PREFIXES = (
    "website/",
    "docs/",
    "e2e/",
    ".claude/",
    ".impeccable/",
    ".github/codeql/",
    "cityloom_server/tests/",
    "osm_import/tests/",
    "scripts/include/tests/",
)
NOT_SITE_FILES = {
    "CLAUDE.md",
    "README.md",
    "DESIGN.md",
    "PRODUCT.md",
    ".coveragerc",
    ".env.example",
    ".gitignore",
    ".prettierignore",
    ".prettierrc.json",
    "rustfmt.toml",
    "sonar-project.properties",
    ".github/dependabot.yml",
    ".github/workflows/codeql.yml",
    ".github/workflows/sonar.yml",
    "scripts/check-version.sh",
    # It writes web/basemap-*.json, which are committed, so a change to the styles comes as a change to web/.
    "scripts/make_basemap_style.mjs",
}
DOCS_FILES = {"justfile", ".github/workflows/ci.yml"}
VERSIONED = {"Cargo.toml", "Cargo.lock"}

Show = Callable[[str, str], Optional[str]]


def site_needs(path: str) -> bool:
    """Whether a change to `path` can change the site or the server that is deployed."""
    if path in NOT_SITE_FILES or path.startswith(NOT_SITE_PREFIXES):
        return False
    # A module's unit tests (`#[cfg(test)] mod tests;`) are compiled into no build that is deployed.
    return not (path.endswith("/tests.rs") or path == "tests.rs")


def docs_needs(path: str) -> bool:
    """Whether a change to `path` can change the documentation site."""
    return path.startswith("website/") or path in DOCS_FILES


def _package_name(toml: str) -> str | None:
    section = None
    for line in toml.splitlines():
        if line.startswith("["):
            section = line.strip()
        elif section == "[package]" and line.startswith("name = "):
            return line.split("=", 1)[1].strip().strip('"')
    return None


def _without_version(path: str, text: str, package: str | None) -> str:
    """`text` without the root package's version line: in Cargo.toml the one of [package], in Cargo.lock the one of
    that package's [[package]] entry."""
    out, section, block_name = [], None, None
    for line in text.splitlines():
        if line.startswith("["):
            section, block_name = line.strip(), None
        elif line.startswith("name = "):
            block_name = line.split("=", 1)[1].strip().strip('"')
        if line.startswith("version = "):
            if path == "Cargo.toml" and section == "[package]":
                continue
            if path == "Cargo.lock" and section == "[[package]]" and block_name == package:
                continue
        out.append(line)
    return "\n".join(out)


def only_version(path: str, old: str | None, new: str | None) -> bool:
    """Whether Cargo.toml or Cargo.lock changed from `old` to `new` in nothing but the root package's version."""
    if path not in VERSIONED or old is None or new is None:
        return False
    package = _package_name(new) if path == "Cargo.toml" else "cityloom_editor"
    return _without_version(path, old, package) == _without_version(path, new, package)


def targets(paths: list[str], show: Show) -> dict[str, list[str]]:
    """Which of the changed `paths` deploy the site and which the documentation (an empty list: no deploy);
    `show(side, path)` gives a file's text before ("old") or after ("new")."""

    def deploys(path: str) -> bool:
        return site_needs(path) and not (path in VERSIONED and only_version(path, show("old", path), show("new", path)))

    return {"site": [p for p in paths if deploys(p)], "docs": [p for p in paths if docs_needs(p)]}


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=False)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args(argv)

    known = args.before.strip("0") and _git(args.repo, "cat-file", "-e", f"{args.before}^{{commit}}").returncode == 0
    if not known:
        print("site=true\ndocs=true")
        print(f"no earlier commit to compare with ({args.before or 'none'}): deploying everything", file=sys.stderr)
        return 0

    diff = _git(args.repo, "diff", "--name-only", "--no-renames", args.before, args.after)
    if diff.returncode != 0:
        print(diff.stderr, file=sys.stderr)
        return 1
    paths = diff.stdout.split()

    def show(side: str, path: str) -> str | None:
        shown = _git(args.repo, "show", f"{args.before if side == 'old' else args.after}:{path}")
        return shown.stdout if shown.returncode == 0 else None

    got = targets(paths, show)
    print(f"site={str(bool(got['site'])).lower()}\ndocs={str(bool(got['docs'])).lower()}")
    for name, why in got.items():
        said = f"deploys ({', '.join(why[:10])}{', ...' if len(why) > 10 else ''})" if why else "nothing to deploy"
        print(f"{name}: {said}, of {len(paths)} changed files", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
