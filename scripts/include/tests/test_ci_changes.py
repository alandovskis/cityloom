"""The CI selector (scripts/ci_changes.py, scripts/ci-spec-map.json): `just test-scripts`.

A pull request skips the Rust and browser jobs when it changes documentation only, and runs only the browser specs
its files can affect. These tests pin the rules, and fail when a spec is in no group or a source folder has no rule,
so a new one cannot be left out of the selection without a failure.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import ci_changes  # noqa: E402

MAP = json.loads((ROOT / "scripts" / "ci-spec-map.json").read_text(encoding="utf-8"))
SPECS = sorted(p.name[: -len(".spec.ts")] for p in (ROOT / "e2e" / "tests").glob("*.spec.ts"))


def pick(*files: str) -> ci_changes.Selection:
    return ci_changes.select(MAP, SPECS, list(files))


class Selection(unittest.TestCase):
    def test_documentation_alone_skips_everything(self):
        s = pick("website/docs/intro.md", "README.md", "website/i18n/fr-CA/x.md", "docs/superpowers/plans/a.md")
        self.assertTrue(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.e2e)

    def test_documentation_with_code_is_code(self):
        s = pick("README.md", "src/junction/model.rs")
        self.assertFalse(s.docs_only)
        self.assertTrue(s.e2e)

    def test_one_slice_runs_its_specs_and_the_cross_ones(self):
        s = pick("src/junction/vm.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("a11y", s.specs)
        self.assertNotIn("street", s.specs)
        self.assertNotIn("sync", s.specs)
        self.assertFalse(s.all)

    def test_a_unit_test_file_runs_no_browser_test(self):
        s = pick("src/street/tests.rs", "src/map/tests.rs")
        self.assertFalse(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.e2e)
        self.assertTrue(s.unit)

    def test_a_changed_spec_runs_itself_alone(self):
        s = pick("e2e/tests/speed.spec.ts")
        self.assertEqual(s.specs, ["speed"])

    def test_a_helper_of_the_specs_runs_all(self):
        self.assertTrue(pick("e2e/tests/fixtures.ts").all)
        self.assertTrue(pick("e2e/playwright.config.ts").all)

    def test_shared_code_and_the_build_run_all(self):
        for f in ("src/shared/ports.rs", "web/app.css", "justfile", ".github/workflows/ci.yml", "Cargo.lock", "cityloom_server/src/app.rs", "scripts/build.sh"):
            with self.subTest(f):
                self.assertTrue(pick(f).all)

    def test_a_file_no_rule_names_runs_all(self):
        s = pick("something-new/file.rs")
        self.assertTrue(s.all)
        self.assertFalse(s.docs_only)

    def test_a_folder_of_src_no_rule_names_runs_all(self):
        self.assertTrue(pick("src/brand_new/mod.rs").all)

    def test_slices_union(self):
        s = pick("src/junction/vm.rs", "src/account/vm.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("account-server", s.specs)

    def test_all_wins_over_a_narrow_file(self):
        s = pick("src/junction/vm.rs", "web/app.css")
        self.assertTrue(s.all)

    def test_the_archive_run_follows_its_specs(self):
        self.assertTrue(pick("src/street/vm.rs").archive)  # the street spec is one of its three
        self.assertFalse(pick("src/account/vm.rs").archive)
        self.assertTrue(pick("web/app.css").archive)
        self.assertFalse(pick("src/street/tests.rs").archive)

    def test_no_files_runs_all(self):
        s = pick()
        self.assertTrue(s.all)

    def test_arguments_for_just(self):
        self.assertEqual(pick("web/app.css").spec_args(), "")
        self.assertEqual(pick("e2e/tests/speed.spec.ts", "e2e/tests/curb.spec.ts").spec_args(), "tests/curb.spec.ts tests/speed.spec.ts")


class Globs(unittest.TestCase):
    def test_star_stays_in_a_folder_and_double_star_crosses(self):
        self.assertTrue(ci_changes.glob("e2e/tests/*.spec.ts", "e2e/tests/a.spec.ts"))
        self.assertFalse(ci_changes.glob("e2e/tests/*.spec.ts", "e2e/tests/sub/a.spec.ts"))
        self.assertTrue(ci_changes.glob("**/*.md", "README.md"))
        self.assertTrue(ci_changes.glob("**/*.md", "a/b/c.md"))
        self.assertTrue(ci_changes.glob("**/tests.rs", "src/street/tests.rs"))
        self.assertTrue(ci_changes.glob("website/**", "website/docs/a.md"))
        self.assertFalse(ci_changes.glob("website/**", "websites/a"))


class VersionOnly(unittest.TestCase):
    TOML = 'name = "x"\nversion = "0.1.0"\n[dependencies]\na = "1"\n'
    LOCK = '[[package]]\nname = "dep"\nversion = "1.0.0"\n\n[[package]]\nname = "cityloom_editor"\nversion = "0.1.0"\n'

    def test_a_version_bump_is_not_a_change(self):
        self.assertEqual(ci_changes.normalize_toml(self.TOML), ci_changes.normalize_toml(self.TOML.replace("0.1.0", "0.2.0")))
        self.assertEqual(ci_changes.normalize_lock(self.LOCK), ci_changes.normalize_lock(self.LOCK.replace('editor"\nversion = "0.1.0', 'editor"\nversion = "0.2.0')))

    def test_a_dependency_change_is_one(self):
        self.assertNotEqual(ci_changes.normalize_toml(self.TOML), ci_changes.normalize_toml(self.TOML.replace('a = "1"', 'a = "2"')))
        self.assertNotEqual(ci_changes.normalize_lock(self.LOCK), ci_changes.normalize_lock(self.LOCK.replace("1.0.0", "1.0.1")))


class Coverage(unittest.TestCase):
    """Every spec and every source folder is accounted for."""

    def test_every_spec_is_in_a_group(self):
        grouped = {s for specs in MAP["groups"].values() for s in specs}
        self.assertEqual([s for s in SPECS if s not in grouped], [], "a spec in no group of scripts/ci-spec-map.json")
        self.assertEqual(sorted(grouped - set(SPECS)), [], "a group names a spec that is not there")

    def test_every_source_folder_has_a_rule(self):
        for entry in sorted((ROOT / "src").iterdir()):
            path = f"src/{entry.name}/mod.rs" if entry.is_dir() else f"src/{entry.name}"
            with self.subTest(path):
                self.assertTrue(ci_changes.matched(MAP, path), f"no rule in scripts/ci-spec-map.json names {path}")

    def test_every_top_level_entry_has_a_rule(self):
        for entry in sorted(ROOT.iterdir()):
            if entry.name.startswith(".git") and entry.name not in (".github", ".gitignore"):
                continue
            # src is checked folder by folder above; the rest is not in git or is the assistant's.
            if entry.name in ("target", "node_modules", ".claude", ".impeccable", ".tools", ".env", "src"):
                continue
            path = f"{entry.name}/x" if entry.is_dir() else entry.name
            with self.subTest(path):
                self.assertTrue(ci_changes.matched(MAP, path), f"no rule in scripts/ci-spec-map.json names {path}")

    def test_the_rules_are_well_formed(self):
        for rule in MAP["rules"]:
            self.assertTrue(rule["paths"])
            self.assertTrue(rule.get("docs") or "specs" in rule)
            specs = rule.get("specs", [])
            if isinstance(specs, list):
                for s in specs:
                    self.assertTrue(s.startswith("@") and s[1:] in MAP["groups"] or s in SPECS, f"unknown {s}")
            else:
                self.assertIn(specs, ("all", "none", "self"))


if __name__ == "__main__":
    unittest.main()
