"""The CI selector (scripts/ci/ci_changes.py, scripts/ci/spec-map.json): `just test-scripts`.

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
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import ci_changes  # noqa: E402

MAP = json.loads((ROOT / "scripts" / "ci" / "spec-map.json").read_text(encoding="utf-8"))
TIERS = json.loads((ROOT / "e2e" / "tiers.json").read_text(encoding="utf-8"))
SPECS = sorted(p.name[: -len(".spec.ts")] for p in (ROOT / "e2e" / "tests").glob("*.spec.ts"))


def pick(*files: str) -> ci_changes.Selection:
    return ci_changes.select(MAP, SPECS, list(files), tiers=TIERS)


class Selection(unittest.TestCase):
    def test_documentation_alone_skips_everything(self):
        s = pick("website/docs/intro.md", "README.md", "website/i18n/fr-CA/x.md", "docs/superpowers/plans/a.md")
        self.assertTrue(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.browser)

    def test_documentation_with_code_is_code(self):
        s = pick("README.md", "crates/cityloom_editor/src/junction/model.rs")
        self.assertFalse(s.docs_only)
        self.assertTrue(s.ui)

    def test_one_slice_runs_its_specs_and_the_cross_ones(self):
        s = pick("crates/cityloom_editor/src/junction/vm.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("a11y", s.specs)
        self.assertNotIn("street", s.specs)
        self.assertNotIn("sync", s.specs)
        self.assertFalse(s.all)

    def test_editor_code_starts_neither_a_database_nor_the_archive(self):
        for f in ("street/vm.rs", "junction/vm.rs", "map/vm.rs", "restriction/model.rs"):
            with self.subTest(f):
                s = pick(f"crates/cityloom_editor/src/{f}")
                self.assertTrue(s.ui)
                self.assertFalse(s.integration)
                self.assertFalse(s.e2e)

    def test_the_server_and_the_sync_run_the_integration_tier(self):
        self.assertTrue(pick("crates/cityloom_server/src/app.rs").integration)
        s = pick("crates/cityloom_editor/src/sync/vm.rs")
        self.assertTrue(s.integration)
        self.assertFalse(s.e2e)
        self.assertTrue(pick("crates/cityloom_editor/src/account/vm.rs").integration)

    def test_the_builder_runs_every_spec(self):
        # it packs the archive the E2E tier serves, and makes the fixtures' network tiles every UI and integration spec
        # is served (`citymodel_builder fixture`, `just e2e-fixtures`)
        for path in ("crates/citymodel_builder/src/archive.rs", "crates/citymodel_builder/src/commands.rs"):
            with self.subTest(path):
                s = pick(path)
                self.assertTrue(s.all)
                self.assertTrue(s.ui and s.integration and s.e2e)

    def test_a_unit_test_file_runs_no_browser_test(self):
        s = pick("crates/cityloom_editor/src/street/tests.rs", "crates/cityloom_editor/src/map/tests.rs")
        self.assertFalse(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.browser)
        self.assertTrue(s.unit)

    def test_a_changed_spec_runs_itself_alone(self):
        s = pick("e2e/tests/speed.spec.ts")
        self.assertEqual(s.specs, ["speed"])

    def test_a_helper_of_the_specs_runs_all(self):
        self.assertTrue(pick("e2e/tests/fixtures.ts").all)
        self.assertTrue(pick("e2e/playwright.config.ts").all)

    def test_shared_code_and_the_build_run_all(self):
        for f in ("crates/cityloom_editor/src/shared/ports.rs", "web/app.css", "justfile", ".github/workflows/ci.yml", "Cargo.lock", "crates/cityloom_server/src/app.rs", "scripts/build.sh"):
            with self.subTest(f):
                self.assertTrue(pick(f).all)

    def test_a_file_no_rule_names_runs_all(self):
        s = pick("something-new/file.rs")
        self.assertTrue(s.all)
        self.assertFalse(s.docs_only)

    def test_a_folder_of_src_no_rule_names_runs_all(self):
        self.assertTrue(pick("crates/cityloom_editor/src/brand_new/mod.rs").all)

    def test_slices_union(self):
        s = pick("crates/cityloom_editor/src/junction/vm.rs", "crates/cityloom_editor/src/account/vm.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("account-server", s.specs)

    def test_all_wins_over_a_narrow_file(self):
        s = pick("crates/cityloom_editor/src/junction/vm.rs", "web/app.css")
        self.assertTrue(s.all)

    def test_shared_code_and_main_run_every_tier(self):
        s = pick("web/app.css")
        self.assertTrue(s.ui and s.integration and s.e2e)
        self.assertTrue(ci_changes.select(MAP, SPECS, [], tiers=TIERS).e2e)

    def test_a_changed_spec_runs_its_own_tier_alone(self):
        s = pick("e2e/tests/sync.spec.ts")
        self.assertTrue(s.integration)
        self.assertFalse(s.ui or s.e2e)
        s = pick("e2e/tests/serve.spec.ts")
        self.assertTrue(s.e2e)
        self.assertFalse(s.ui or s.integration)

    def test_no_files_runs_all(self):
        s = pick()
        self.assertTrue(s.all)

    def test_arguments_for_each_tier(self):
        s = pick("web/app.css")
        self.assertEqual((s.spec_args("ui"), s.spec_args("integration"), s.spec_args("e2e")), ("", "", ""))
        s = pick("e2e/tests/speed.spec.ts", "e2e/tests/curb.spec.ts", "e2e/tests/sync.spec.ts")
        self.assertEqual(s.spec_args("ui"), "tests/curb.spec.ts tests/speed.spec.ts")
        self.assertEqual(s.spec_args("integration"), "tests/sync.spec.ts")

    def test_the_archive_spec_repeats_the_pages_over_the_archive(self):
        s = pick("e2e/tests/archive.spec.ts")
        self.assertEqual(s.spec_args("e2e"), "tests/archive.spec.ts tests/map.spec.ts tests/street.spec.ts")
        self.assertEqual(pick("e2e/tests/serve.spec.ts").spec_args("e2e"), "tests/serve.spec.ts")


class Globs(unittest.TestCase):
    def test_star_stays_in_a_folder_and_double_star_crosses(self):
        self.assertTrue(ci_changes.glob("e2e/tests/*.spec.ts", "e2e/tests/a.spec.ts"))
        self.assertFalse(ci_changes.glob("e2e/tests/*.spec.ts", "e2e/tests/sub/a.spec.ts"))
        self.assertTrue(ci_changes.glob("**/*.md", "README.md"))
        self.assertTrue(ci_changes.glob("**/*.md", "a/b/c.md"))
        self.assertTrue(ci_changes.glob("**/tests.rs", "crates/cityloom_editor/src/street/tests.rs"))
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
        self.assertEqual([s for s in SPECS if s not in grouped], [], "a spec in no group of scripts/ci/spec-map.json")
        self.assertEqual(sorted(grouped - set(SPECS)), [], "a group names a spec that is not there")

    def test_every_spec_is_in_exactly_one_tier(self):
        tiers = {name: specs for name, specs in TIERS.items() if name in ("ui", "integration", "e2e")}
        self.assertEqual(sorted(tiers), ["e2e", "integration", "ui"])
        for spec in SPECS:
            with self.subTest(spec):
                in_tiers = [name for name, specs in tiers.items() if spec in specs]
                self.assertEqual(len(in_tiers), 1, f"{spec} is in {in_tiers or 'no tier'} of e2e/tiers.json, not exactly one")
        named = {s for specs in tiers.values() for s in specs}
        self.assertEqual(sorted(named - set(SPECS)), [], "a tier names a spec that is not there")
        for name, specs in tiers.items():
            self.assertEqual(len(specs), len(set(specs)), f"{name} lists a spec twice")

    def test_the_archive_run_repeats_ui_specs_only(self):
        for spec in TIERS["archive_also"]:
            self.assertIn(spec, TIERS["ui"], f"{spec} is repeated in archive mode but is not a UI spec")

    def test_every_source_folder_has_a_rule(self):
        for entry in sorted((ROOT / "crates/cityloom_editor" / "src").iterdir()):
            path = f"crates/cityloom_editor/src/{entry.name}/mod.rs" if entry.is_dir() else f"crates/cityloom_editor/src/{entry.name}"
            with self.subTest(path):
                self.assertTrue(ci_changes.matched(MAP, path), f"no rule in scripts/ci/spec-map.json names {path}")

    def test_every_crate_has_a_rule(self):
        for entry in sorted((ROOT / "crates").iterdir()):
            path = f"crates/{entry.name}/src/lib.rs"
            with self.subTest(path):
                self.assertTrue(ci_changes.matched(MAP, path), f"no rule in scripts/ci/spec-map.json names {path}")

    def test_every_top_level_entry_has_a_rule(self):
        for entry in sorted(ROOT.iterdir()):
            if entry.name.startswith(".git") and entry.name not in (".github", ".gitignore"):
                continue
            # src and the crates are checked on their own; the rest is not in git or is the assistant's.
            if entry.name in ("target", "node_modules", ".claude", ".impeccable", ".tools", ".env", "crates", "data"):
                continue
            path = f"{entry.name}/x" if entry.is_dir() else entry.name
            with self.subTest(path):
                self.assertTrue(ci_changes.matched(MAP, path), f"no rule in scripts/ci/spec-map.json names {path}")

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


class Revisions(unittest.TestCase):
    def test_only_a_commit_id_or_head_reaches_git(self):
        for bad in ("--output=/tmp/x", "main; rm -rf /", "-p", "a b", "../x", ""):
            self.assertIsNone(ci_changes.changed_files(bad, "HEAD"), bad)
            self.assertIsNone(ci_changes.changed_files("abcdef1234", bad), bad)

    def test_the_pattern_takes_ids_and_head(self):
        for good in ("abcdef1", "A" * 40, "HEAD"):
            self.assertTrue(ci_changes.REVISION.fullmatch(good), good)


if __name__ == "__main__":
    unittest.main()
