#!/usr/bin/env python3
"""Prints the tag of the toolchain images of dockerfiles/ (`.github/workflows/images.yml`).

    scripts/ci/image_tag.py [--root <checkout>]

CI pulls ghcr.io/<owner>/cityloom-<image>:<tag> and builds the images only when that tag is missing, so the tag is a
hash of exactly what they are built from:

    every file under dockerfiles/ but its Markdown (the README does not change an image)
    the `wasm-bindgen = "<version>"` line of crates/cityloom_editor/Cargo.toml (rust.Dockerfile installs that CLI)
    @playwright/test's version in tests/e2e/package-lock.json (e2e.Dockerfile installs its Chromium)

The editor's own version, and every other dependency, are left out: every pull request raises the first, and neither is
in an image. When GITHUB_OUTPUT is set, `tag=<tag>` is also written there. Standard library only.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path

CARGO = Path("crates/cityloom_editor/Cargo.toml")
LOCK = Path("tests/e2e/package-lock.json")
# The same reading as rust.Dockerfile's `sed`, so the tag and the image agree on the version.
WASM_BINDGEN = re.compile(r'^wasm-bindgen = "(.*)"$', re.MULTILINE)


def wasm_bindgen_version(root: Path) -> str:
    found = WASM_BINDGEN.search((root / CARGO).read_text(encoding="utf-8"))
    if not found:
        raise SystemExit(f"image_tag.py: no 'wasm-bindgen = \"<version>\"' line in {CARGO.as_posix()}")
    return found.group(1)


def playwright_version(root: Path) -> str:
    lock = json.loads((root / LOCK).read_text(encoding="utf-8"))
    version = lock.get("packages", {}).get("node_modules/@playwright/test", {}).get("version")
    if not version:
        raise SystemExit(f"image_tag.py: no @playwright/test version in {LOCK.as_posix()}")
    return version


def image_tag(root: Path) -> str:
    """The first 16 hex characters of a sha256 over the Dockerfiles and the two versions."""
    digest = hashlib.sha256()
    dockerfiles = root / "dockerfiles"
    files = {p.relative_to(dockerfiles).as_posix(): p for p in dockerfiles.rglob("*") if p.is_file() and p.suffix != ".md"}
    for name in sorted(files):
        digest.update(name.encode() + b"\0")
        digest.update(files[name].read_bytes() + b"\0")
    digest.update(f"wasm-bindgen={wasm_bindgen_version(root)}\n".encode())
    digest.update(f"playwright={playwright_version(root)}\n".encode())
    return digest.hexdigest()[:16]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    tag = image_tag(parser.parse_args().root)
    print(tag)
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a", encoding="utf-8") as f:
            f.write(f"tag={tag}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
