"""The toolchain images' tag (scripts/ci/image_tag.py): `just test-scripts`.

CI pulls the images of dockerfiles/ from GHCR by this tag and builds them only when it is missing, so the tag must move
with exactly what the images are built from: the Dockerfiles and the two versions they read from the checkout. A change
to anything else must keep it, or every pull request would build the images again.
"""

from __future__ import annotations

import json
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import image_tag  # noqa: E402

CARGO = '[package]\nname = "cityloom_editor"\nversion = "0.116.10"\n\n[dependencies]\nwasm-bindgen = "0.2.130"\n'
LOCK = {
    "packages": {
        "": {"name": "cityloom-e2e"},
        "node_modules/@playwright/test": {"version": "1.65.1"},
        "node_modules/prettier": {"version": "3.10.1"},
    }
}


class ImageTag(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        (self.root / "dockerfiles").mkdir()
        (self.root / "dockerfiles" / "rust.Dockerfile").write_text("FROM rust:1.100.0-slim\n", encoding="utf-8")
        (self.root / "dockerfiles" / "docker-bake.hcl").write_text('target "rust" {}\n', encoding="utf-8")
        (self.root / "dockerfiles" / "README.md").write_text("# Build images\n", encoding="utf-8")
        (self.root / "crates" / "cityloom_editor").mkdir(parents=True)
        self.write_cargo(CARGO)
        (self.root / "tests" / "e2e").mkdir(parents=True)
        self.write_lock(LOCK)

    def write_cargo(self, text: str) -> None:
        (self.root / "crates" / "cityloom_editor" / "Cargo.toml").write_text(text, encoding="utf-8")

    def write_lock(self, lock: dict) -> None:
        (self.root / "tests" / "e2e" / "package-lock.json").write_text(json.dumps(lock), encoding="utf-8")

    def tag(self) -> str:
        return image_tag.image_tag(self.root)

    def test_the_same_tree_gives_the_same_tag(self):
        first = self.tag()
        self.assertRegex(first, r"^[0-9a-f]{16}$")
        self.assertEqual(first, self.tag())

    def test_a_change_to_a_dockerfile_changes_the_tag(self):
        before = self.tag()
        (self.root / "dockerfiles" / "rust.Dockerfile").write_text("FROM rust:1.101.0-slim\n", encoding="utf-8")
        self.assertNotEqual(before, self.tag())

    def test_a_new_file_under_dockerfiles_changes_the_tag(self):
        before = self.tag()
        (self.root / "dockerfiles" / "js.Dockerfile").write_text("FROM node:lts-slim\n", encoding="utf-8")
        self.assertNotEqual(before, self.tag())

    def test_a_change_to_the_dockerfiles_readme_keeps_the_tag(self):
        before = self.tag()
        (self.root / "dockerfiles" / "README.md").write_text("# Build images\n\nMore words.\n", encoding="utf-8")
        self.assertEqual(before, self.tag())

    def test_raising_the_editors_version_keeps_the_tag(self):
        before = self.tag()
        self.write_cargo(CARGO.replace('version = "0.116.10"', 'version = "0.116.11"'))
        self.assertEqual(before, self.tag())

    def test_moving_wasm_bindgen_changes_the_tag(self):
        before = self.tag()
        self.write_cargo(CARGO.replace('wasm-bindgen = "0.2.130"', 'wasm-bindgen = "0.2.131"'))
        self.assertNotEqual(before, self.tag())

    def test_moving_playwright_changes_the_tag(self):
        before = self.tag()
        lock = json.loads(json.dumps(LOCK))
        lock["packages"]["node_modules/@playwright/test"]["version"] = "1.66.0"
        self.write_lock(lock)
        self.assertNotEqual(before, self.tag())

    def test_moving_another_e2e_package_keeps_the_tag(self):
        before = self.tag()
        lock = json.loads(json.dumps(LOCK))
        lock["packages"]["node_modules/prettier"]["version"] = "3.11.0"
        self.write_lock(lock)
        self.assertEqual(before, self.tag())

    def test_a_missing_wasm_bindgen_pin_fails_naming_the_file(self):
        self.write_cargo(CARGO.replace('wasm-bindgen = "0.2.130"\n', ""))
        with self.assertRaises(SystemExit) as raised:
            self.tag()
        self.assertIn("crates/cityloom_editor/Cargo.toml", str(raised.exception))

    def test_a_missing_playwright_pin_fails_naming_the_file(self):
        lock = json.loads(json.dumps(LOCK))
        del lock["packages"]["node_modules/@playwright/test"]
        self.write_lock(lock)
        with self.assertRaises(SystemExit) as raised:
            self.tag()
        self.assertIn("tests/e2e/package-lock.json", str(raised.exception))

    def test_the_repositorys_own_tree_has_a_tag(self):
        # The real checkout has both pins in the form the images read them in.
        self.assertTrue(re.fullmatch(r"[0-9a-f]{16}", image_tag.image_tag(ROOT)))


if __name__ == "__main__":
    unittest.main()
