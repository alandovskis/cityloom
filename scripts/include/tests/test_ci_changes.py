"""The CI selector (scripts/ci/ci_changes.py, scripts/ci/spec-map.json): `just test-scripts`.

A pull request skips the Rust and browser jobs when it changes documentation only, and runs only the browser specs
its files can affect. These tests pin the rules, and fail when a spec is in no group or a source folder has no rule,
so a new one cannot be left out of the selection without a failure.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import ci_changes  # noqa: E402

MAP = json.loads((ROOT / "scripts" / "ci" / "spec-map.json").read_text(encoding="utf-8"))
TIERS = json.loads((ROOT / "tests" / "e2e" / "tiers.json").read_text(encoding="utf-8"))
SPECS = sorted(p.name[: -len(".spec.ts")] for p in (ROOT / "tests" / "e2e" / "tests").glob("*.spec.ts"))


def paths_ignore(workflow: Path) -> dict[str, list[str] | None]:
    """Every event of a workflow's `on:` block, with its `paths-ignore` list or None when it has none.

    A line scan of the block-style YAML the workflows use: the standard library has no YAML parser.
    """
    lists: dict[str, list[str] | None] = {}
    event, inside, collecting = None, False, False
    for line in workflow.read_text(encoding="utf-8").splitlines():
        if not inside:
            inside = line.startswith("on:")
        elif line and not line.startswith((" ", "#")):
            break
        elif m := re.fullmatch(r"  ([\w-]+):.*", line):
            event, collecting = m.group(1), False
            lists[event] = None
        elif m := re.fullmatch(r"    ([\w-]+):.*", line):
            collecting = m.group(1) == "paths-ignore"
            if collecting:
                lists[event] = []
        elif collecting and (m := re.fullmatch(r"      - (.*)", line)):
            lists[event].append(m.group(1).strip().strip('"'))
    return lists


def pick(*files: str) -> ci_changes.Selection:
    return ci_changes.select(MAP, SPECS, list(files), tiers=TIERS)


class Selection(unittest.TestCase):
    def test_documentation_alone_skips_everything(self):
        s = pick("website/docs/intro.md", "README.md", "website/i18n/fr-CA/x.md", "docs/superpowers/plans/a.md")
        self.assertTrue(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.browser)

    def test_documentation_and_ci_configuration_need_no_version_bump(self):
        # Neither changes anything the pages show (the version is in their menu).
        for files in (
            ["website/docs/intro.md"],
            [".github/workflows/presubmit.yml"],
            [".github/workflows/docs.yml", "website/docs/intro.md", "CLAUDE.md"],
            [".github/labels.yml", ".github/ISSUE_TEMPLATE/bug.yml"],
            ["scripts/ci/ci_changes.py", "scripts/ci/spec-map.json", "scripts/include/tests/test_ci_changes.py"],
            [".claude/agents/deploy.md", ".github/workflows/presubmit.yml"],
        ):
            with self.subTest(files):
                self.assertFalse(pick(*files).version)

    def test_ci_configuration_alone_still_runs_every_test(self):
        s = pick(".github/workflows/presubmit.yml", "website/docs/intro.md")
        self.assertFalse(s.version)
        self.assertTrue(s.all)
        self.assertTrue(s.unit)

    def test_anything_else_needs_a_version_bump(self):
        for files in (
            ["crates/cityloom_editor/src/junction/model.rs"],
            [".github/workflows/presubmit.yml", "web/app.css"],
            ["scripts/build.sh"],
            ["something-new/file.rs"],
        ):
            with self.subTest(files):
                self.assertTrue(pick(*files).version)

    def test_an_unknown_change_needs_a_version_bump(self):
        self.assertTrue(ci_changes.select(MAP, SPECS, [], tiers=TIERS).version)

    def test_documentation_with_code_is_code(self):
        s = pick("README.md", "crates/cityloom_editor/src/junction/model/mod.rs")
        self.assertFalse(s.docs_only)
        self.assertTrue(s.ui)

    def test_one_slice_runs_its_specs_and_the_cross_ones(self):
        s = pick("crates/cityloom_editor/src/junction/vm/mod.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("a11y", s.specs)
        self.assertNotIn("street", s.specs)
        self.assertNotIn("sync", s.specs)
        self.assertFalse(s.all)

    def test_editor_code_starts_neither_a_database_nor_the_archive(self):
        for f in ("street/vm/mod.rs", "junction/view/iso/mod.rs", "map/model/network.rs", "restriction/model.rs"):
            with self.subTest(f):
                s = pick(f"crates/cityloom_editor/src/{f}")
                self.assertTrue(s.ui)
                self.assertFalse(s.integration)
                self.assertFalse(s.e2e)

    def test_the_list_of_projects_runs_the_home_specs_and_the_project_store_runs_every_page_that_opens_one(self):
        for f in ("vm.rs", "view.rs", "text.rs", "i18n/en.ftl"):
            with self.subTest(f):
                s = pick(f"crates/cityloom_editor/src/project/{f}")
                self.assertIn("projects", s.specs)
                self.assertIn("a11y", s.specs)
                self.assertNotIn("map", s.specs)
                self.assertNotIn("sync", s.specs)
        # Which project a page opens, the list's store and the move of the old work: every page and the sync use them.
        for f in ("context.rs", "store.rs", "model.rs", "migrate.rs", "mod.rs"):
            with self.subTest(f):
                s = pick(f"crates/cityloom_editor/src/project/{f}")
                for spec in ("projects", "home", "map", "street", "junction", "sync", "account", "archive", "a11y"):
                    self.assertIn(spec, s.specs)
                self.assertTrue(s.integration)
                self.assertTrue(s.e2e)

    def test_the_projects_their_copies_and_the_sync_run_the_share_spec(self):
        # The home page's row links to sharing; a shared design opens as a copy (`project/copies.rs`), which the
        # pages' lookup and the sync keep apart from the person's projects (#221).
        for f in ("project/vm.rs", "project/copies.rs", "project/context.rs", "project/store.rs", "sync/local.rs"):
            with self.subTest(f):
                self.assertIn("share", pick(f"crates/cityloom_editor/src/{f}").specs)

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
        s = pick("crates/cityloom_editor/src/street/view/tests.rs", "crates/cityloom_editor/src/map/view/tests.rs")
        self.assertFalse(s.docs_only)
        self.assertEqual(s.specs, [])
        self.assertFalse(s.browser)
        self.assertTrue(s.unit)

    def test_a_changed_spec_runs_itself_alone(self):
        s = pick("tests/e2e/tests/speed.spec.ts")
        self.assertEqual(s.specs, ["speed"])

    def test_a_helper_of_the_specs_runs_all(self):
        self.assertTrue(pick("tests/e2e/tests/fixtures.ts").all)
        self.assertTrue(pick("tests/e2e/playwright.config.ts").all)

    def test_shared_code_and_the_build_run_all(self):
        for f in ("crates/cityloom_editor/src/shared/ports.rs", "web/app.css", "justfile", ".github/workflows/presubmit.yml", "Cargo.lock", "crates/cityloom_server/src/app.rs", "scripts/build.sh"):
            with self.subTest(f):
                self.assertTrue(pick(f).all)

    def test_a_file_no_rule_names_runs_all(self):
        s = pick("something-new/file.rs")
        self.assertTrue(s.all)
        self.assertFalse(s.docs_only)

    def test_a_folder_of_src_no_rule_names_runs_all(self):
        self.assertTrue(pick("crates/cityloom_editor/src/brand_new/mod.rs").all)

    def test_slices_union(self):
        s = pick("crates/cityloom_editor/src/junction/vm/mod.rs", "crates/cityloom_editor/src/account/vm.rs")
        self.assertIn("junction", s.specs)
        self.assertIn("account-server", s.specs)

    def test_all_wins_over_a_narrow_file(self):
        s = pick("crates/cityloom_editor/src/junction/vm/mod.rs", "web/app.css")
        self.assertTrue(s.all)

    def test_shared_code_and_main_run_every_tier(self):
        s = pick("web/app.css")
        self.assertTrue(s.ui and s.integration and s.e2e)
        self.assertTrue(ci_changes.select(MAP, SPECS, [], tiers=TIERS).e2e)

    def test_a_changed_spec_runs_its_own_tier_alone(self):
        s = pick("tests/e2e/tests/sync.spec.ts")
        self.assertTrue(s.integration)
        self.assertFalse(s.ui or s.e2e)
        s = pick("tests/e2e/tests/serve.spec.ts")
        self.assertTrue(s.e2e)
        self.assertFalse(s.ui or s.integration)

    def test_no_files_runs_all(self):
        s = pick()
        self.assertTrue(s.all)

    def test_arguments_for_each_tier(self):
        s = pick("web/app.css")
        self.assertEqual((s.spec_args("ui"), s.spec_args("integration"), s.spec_args("e2e")), ("", "", ""))
        s = pick("tests/e2e/tests/speed.spec.ts", "tests/e2e/tests/curb.spec.ts", "tests/e2e/tests/sync.spec.ts")
        self.assertEqual(s.spec_args("ui"), "tests/curb.spec.ts tests/speed.spec.ts")
        self.assertEqual(s.spec_args("integration"), "tests/sync.spec.ts")

    def test_the_archive_spec_repeats_the_pages_over_the_archive(self):
        s = pick("tests/e2e/tests/archive.spec.ts")
        self.assertEqual(s.spec_args("e2e"), "tests/archive.spec.ts tests/map.spec.ts tests/street.spec.ts")
        self.assertEqual(pick("tests/e2e/tests/serve.spec.ts").spec_args("e2e"), "tests/serve.spec.ts")


class Crates(unittest.TestCase):
    """A pull request runs the Rust tests of the crates it touches and of the crates that depend on them."""

    def test_the_dependents_are_read_from_the_manifests(self):
        d = ci_changes.crate_dependents()
        self.assertEqual(
            d["cityloom_model"], {"cityloom_editor", "citymodel_builder"}, "the crates that name it as a path dependency"
        )
        self.assertEqual(d["cityloom_server"], set(), "nothing depends on the server")
        self.assertIn("cityloom_report_pdf", d["cityloom_report"])
        # The editor's controls depend on no crate of the workspace (so on no wording), and a change to them runs every spec.
        self.assertEqual(d["cityloom_ui"], {"cityloom_editor"}, "only the editor uses the controls")
        self.assertEqual([name for name, users in d.items() if "cityloom_ui" in users], [], "cityloom_ui depends on none")
        self.assertTrue(pick("crates/cityloom_ui/src/radio.rs").all)

    def test_a_crate_runs_with_what_depends_on_it_however_far(self):
        # report -> report_pdf and editor; editor -> citymodel_builder; builder -> server
        self.assertEqual(
            pick("crates/cityloom_report/src/lib.rs").crates,
            ["cityloom_editor", "cityloom_report", "cityloom_report_pdf", "cityloom_server", "citymodel_builder"],
        )

    def test_the_server_alone_runs_alone(self):
        s = pick("crates/cityloom_server/src/app.rs")
        self.assertEqual(s.crates, ["cityloom_server"])
        self.assertEqual(s.rust_args, "-p cityloom_server")
        self.assertTrue(s.server_release)
        self.assertFalse(s.wasm, "the server is not built for the browser")
        self.assertEqual(s.wasm_args, "")

    def test_the_wasm_check_leaves_out_what_the_browser_does_not_load(self):
        s = pick("crates/cityloom_report/src/lib.rs")
        self.assertEqual(s.wasm_args, "-p cityloom_editor -p cityloom_report -p cityloom_report_pdf")
        self.assertTrue(s.wasm)

    def test_a_citys_config_runs_every_crate_and_no_browser_spec(self):
        # The builder's tests load every file of cities/; the specs' road data is the e2e fixtures', not a city's.
        files = sorted(p.relative_to(ROOT).as_posix() for p in (ROOT / "cities").glob("*.toml"))
        self.assertIn("cities/montreal.toml", files, "cities/ has no config of Montréal")
        for f in files:
            with self.subTest(f):
                s = pick(f)
                self.assertIsNone(s.crates)
                self.assertTrue(s.rust and s.version)
                self.assertEqual(s.specs, [])
                self.assertFalse(s.browser)

    def test_a_change_no_crate_reads_runs_no_crate(self):
        for f in ("tests/e2e/tests/street.spec.ts", "scripts/include/tests/test_ci_changes.py", "website/docs/a.md"):
            with self.subTest(f):
                s = pick(f, "scripts/ci/spec-map.json")
                self.assertEqual(s.crates, [])
                self.assertFalse(s.rust or s.wasm or s.server_release)

    def test_what_every_crate_reads_runs_every_crate(self):
        # The tests include the pages and fonts of web/, read the fixtures and are built by the manifests and the workflows.
        for f in (
            "web/street.html",
            "tests/e2e/fixtures/plateau.osm.pbf",
            "Cargo.lock",
            "Cargo.toml",
            "justfile",
            ".github/workflows/presubmit.yml",
            "scripts/build.sh",
            "rust-toolchain.toml",
            "crates/new_crate/src/lib.rs",
        ):
            with self.subTest(f):
                s = pick(f)
                self.assertIsNone(s.crates)
                self.assertEqual((s.rust_args, s.wasm_args), ("", ""))
                self.assertTrue(s.rust and s.wasm and s.server_release)

    def test_a_file_among_changed_crates_does_not_narrow_a_wider_change(self):
        s = pick("crates/cityloom_server/src/app.rs", "web/app.css")
        self.assertIsNone(s.crates)

    def test_a_bare_version_bump_runs_every_crate(self):
        # `changed_files` leaves the bump out, so nothing is left to narrow by: the doubtful case runs everything.
        s = ci_changes.select(MAP, SPECS, [], tiers=TIERS, empty_means_none=True)
        self.assertIsNone(s.crates)

    def test_the_wasm_exclusions_are_the_justfiles(self):
        just = (ROOT / "justfile").read_text(encoding="utf-8")
        recipe = re.search(r"(?m)^test-wasm.*\n(?:    .*\n)+", just)
        self.assertIsNotNone(recipe)
        for name in ci_changes.NOT_WASM:
            self.assertIn(f"--exclude {name}", recipe.group(0))
        crates = {d.name for d in (ROOT / "crates").iterdir() if (d / "Cargo.toml").is_file()}
        self.assertLessEqual(set(ci_changes.NOT_WASM), crates)

    def test_the_server_feature_is_only_asked_when_the_server_is_tested(self):
        # `cargo test -p other --features cityloom_server/test-provider` is an error: the recipe adds it for the server alone.
        just = (ROOT / "justfile").read_text(encoding="utf-8")
        self.assertRegex(just, r'\*"-p cityloom_server "\*\) exec cargo test \{\{crates\}\} --features cityloom_server/test-provider')
        self.assertRegex(just, r"\*\) exec cargo test \{\{crates\}\} ;;")


class PageFiles(unittest.TestCase):
    """A page's own files run that page's specs and the cross-page ones, not every spec."""

    def test_a_page_runs_its_own_specs_and_the_cross_ones(self):
        for f, own, other in (
            ("web/account.js", ("account", "account-server"), ("map", "street", "junction", "sync")),
            ("web/map.js", ("map", "world"), ("account", "street", "home")),
            ("web/street.html", ("street", "curb"), ("map", "account")),
            ("web/intersection.html", ("junction",), ("street", "map")),
            ("web/timeline.css", ("timeline",), ("map", "account")),
            ("web/share.js", ("share",), ("map", "timeline")),
            ("web/home.css", ("home", "projects", "about"), ("map", "street")),
        ):
            with self.subTest(f):
                s = pick(f)
                self.assertFalse(s.all)
                for spec in own + ("a11y", "shell"):
                    self.assertIn(spec, s.specs)
                for spec in other:
                    self.assertNotIn(spec, s.specs)

    def test_every_html_page_names_its_own_specs(self):
        for page in sorted((ROOT / "web").glob("*.html")):
            with self.subTest(page.name):
                self.assertFalse(pick(f"web/{page.name}").all, "a page with no rule of its own runs every spec")

    def test_what_every_page_loads_runs_every_spec(self):
        for f in ("web/app.css", "web/boot.js", "web/city.js", "web/brand.svg", "web/fonts/NotoSans-Regular.ttf"):
            with self.subTest(f):
                self.assertTrue(pick(f).all)

    def test_the_map_runtime_runs_the_map_and_the_about_page(self):
        # The About page draws a city of the project opened last with the same map code.
        s = pick("web/mapcanvas.js")
        self.assertTrue({"map", "about", "a11y"} <= set(s.specs))
        self.assertFalse(s.all)

    def test_a_helper_runs_the_specs_that_import_it(self):
        helpers = ROOT / "tests" / "e2e" / "tests"
        users = sorted(
            p.name[: -len(".spec.ts")] for p in helpers.glob("*.spec.ts") if "account-helpers" in p.read_text(encoding="utf-8")
        )
        self.assertEqual(pick("tests/e2e/tests/account-helpers.ts").specs, users)

    def test_the_scripts_that_only_the_ci_jobs_run_need_no_browser_spec(self):
        for f in ("scripts/check-no-fake.sh", "scripts/check-version.sh", "deploy/Dockerfile", ".github/workflows/docs.yml"):
            with self.subTest(f):
                self.assertEqual(pick(f).specs, [])
        for f in ("scripts/build.sh", "scripts/pin-server.sh", ".github/workflows/presubmit.yml"):
            with self.subTest(f):
                self.assertTrue(pick(f).all)


class Globs(unittest.TestCase):
    def test_star_stays_in_a_folder_and_double_star_crosses(self):
        self.assertTrue(ci_changes.glob("tests/e2e/tests/*.spec.ts", "tests/e2e/tests/a.spec.ts"))
        self.assertFalse(ci_changes.glob("tests/e2e/tests/*.spec.ts", "tests/e2e/tests/sub/a.spec.ts"))
        self.assertTrue(ci_changes.glob("**/*.md", "README.md"))
        self.assertTrue(ci_changes.glob("**/*.md", "a/b/c.md"))
        self.assertTrue(ci_changes.glob("**/tests.rs", "crates/cityloom_editor/src/street/view/tests.rs"))
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
                self.assertEqual(len(in_tiers), 1, f"{spec} is in {in_tiers or 'no tier'} of tests/e2e/tiers.json, not exactly one")
        named = {s for specs in tiers.values() for s in specs}
        self.assertEqual(sorted(named - set(SPECS)), [], "a tier names a spec that is not there")
        for name, specs in tiers.items():
            self.assertEqual(len(specs), len(set(specs)), f"{name} lists a spec twice")

    def test_the_archive_run_repeats_ui_specs_only(self):
        for spec in TIERS["archive_also"]:
            self.assertIn(spec, TIERS["ui"], f"{spec} is repeated in archive mode but is not a UI spec")

    def test_the_browser_tests_live_under_tests(self):
        self.assertTrue((ROOT / "tests" / "e2e" / "tiers.json").is_file())
        # A checkout from before the move keeps an untracked e2e/ (node_modules, generated fixtures): only git's files count.
        self.assertEqual(ci_changes.git("ls-files", "e2e").splitlines(), [])

    def test_nothing_names_the_old_folder(self):
        # The plans and designs under docs/superpowers record what was true when they were written.
        old = re.compile(r"(?<![\w/.-])e2e/|\bcd e2e\b|in `e2e`|et dans `e2e`|in e2e\b|<wt>/e2e\b")
        found = []
        for f in ci_changes.git("ls-files").splitlines():
            if f.startswith("docs/superpowers/") or f.endswith("package-lock.json") or f == "scripts/include/tests/test_ci_changes.py" or not (ROOT / f).is_file():
                continue
            try:
                text = (ROOT / f).read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            found += [f"{f}:{n}" for n, line in enumerate(text.splitlines(), 1) if old.search(line)]
        self.assertEqual(found, [], "these still name e2e/, which is tests/e2e/ now")

    def test_the_prettier_commands_run_from_tests_e2e_reach_the_root(self):
        command = re.compile(r"cd tests/e2e && npx prettier --ignore-path (\S+)[^`\n]*?((?:\.\./)+[\w.*/-]+)?[`\n]")
        checked = 0
        for f in ci_changes.git("ls-files", "*.md", "justfile").splitlines():
            for m in command.finditer((ROOT / f).read_text(encoding="utf-8")):
                for target in filter(None, m.groups()):
                    checked += 1
                    with self.subTest(f"{f}: {target}"):
                        self.assertTrue((ROOT / "tests" / "e2e" / target.split("*")[0]).resolve().exists(), f"{target} from tests/e2e/")
        self.assertGreater(checked, 0)

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
        # What git tracks: an untracked folder (a build's output, an old checkout's e2e/) is no change a PR can make.
        tracked = ci_changes.git("ls-files").splitlines()
        entries = sorted({f.split("/", 1)[0] + ("/x" if "/" in f else "") for f in tracked})
        for path in entries:
            name = path.removesuffix("/x")
            # The crates are checked on their own; the rest is the assistant's.
            if name in (".claude", ".impeccable", "crates"):
                continue
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

    def test_the_workflows_ignore_exactly_the_documentation(self):
        # presubmit.yml, postsubmit.yml and sonar.yml do not start for documentation alone; their `paths-ignore` lists
        # must be the `docs` rule, or a file would start no test at all, or start every job for a change the selector
        # calls documentation. Every event a change starts (all but a manual run) has the list; the order does not matter.
        docs = sorted(next(r["paths"] for r in MAP["rules"] if r.get("docs")))
        events = {
            "presubmit.yml": {"pull_request"},
            "postsubmit.yml": {"push"},
            "sonar.yml": {"push", "pull_request"},
        }
        for workflow, started_by in events.items():
            with self.subTest(workflow):
                lists = paths_ignore(ROOT / ".github" / "workflows" / workflow)
                self.assertTrue(started_by <= set(lists), f"{workflow}: {sorted(lists)}")
                for event, paths in lists.items():
                    if event != "workflow_dispatch":
                        self.assertEqual(sorted(paths or []), docs, f"{workflow} {event}")


class Pipelines(unittest.TestCase):
    """CI is two workflows: presubmit.yml checks a pull request, postsubmit.yml checks main and deploys it."""

    WORKFLOWS = ROOT / ".github" / "workflows"

    def text(self, name: str) -> str:
        return (self.WORKFLOWS / name).read_text(encoding="utf-8")

    def jobs(self, name: str) -> dict[str, str]:
        """The top-level jobs of a workflow, each with its own text."""
        found: dict[str, list[str]] = {}
        inside, job = False, None
        for line in self.text(name).splitlines():
            if not inside:
                inside = line.startswith("jobs:")
            elif m := re.fullmatch(r"  ([\w-]+):\s*", line):
                job = m.group(1)
                found[job] = []
            elif job is not None:
                found[job].append(line)
        return {k: "\n".join(v) for k, v in found.items()}

    def test_ci_yml_is_gone(self):
        self.assertFalse((self.WORKFLOWS / "ci.yml").exists())

    def test_presubmit_runs_on_pull_requests_only(self):
        self.assertEqual(set(paths_ignore(self.WORKFLOWS / "presubmit.yml")), {"pull_request"})

    def test_postsubmit_runs_on_main_and_by_hand_only(self):
        self.assertEqual(set(paths_ignore(self.WORKFLOWS / "postsubmit.yml")), {"push", "workflow_dispatch"})

    def test_only_postsubmit_deploys(self):
        self.assertNotIn("deploy", self.jobs("presubmit.yml"))
        self.assertIn("deploy", self.jobs("postsubmit.yml"))
        self.assertNotIn("RAILWAY_TOKEN", self.text("presubmit.yml"))

    def test_the_deploy_waits_for_every_check(self):
        jobs = self.jobs("postsubmit.yml")
        self.assertRegex(jobs["deploy"], r"needs: \[changes, ci\]")
        checks = set(jobs) - {"ci", "deploy"}
        needed = re.search(r"needs: \[([^\]]*)\]", jobs["ci"])
        self.assertIsNotNone(needed)
        self.assertEqual(checks, {n.strip() for n in needed.group(1).split(",")})

    def test_main_decides_whether_to_deploy(self):
        jobs = self.jobs("postsubmit.yml")
        self.assertIn("scripts/ci/last_deploy.py", jobs["changes"])
        self.assertIn("ci_changes.py --event push", jobs["changes"])
        self.assertIn("actions: read", jobs["changes"])
        self.assertIn("needs.changes.outputs.deploy == 'true'", jobs["deploy"])

    def test_no_check_waits_for_the_decision(self):
        # Every check runs in full on main; only the deploy reads what the Changes job decided.
        for name, text in self.jobs("postsubmit.yml").items():
            needs = re.search(r"needs: (.*)", text)
            if name not in ("ci", "deploy") and needs:
                with self.subTest(name):
                    self.assertNotIn("changes", needs.group(1))

    def test_presubmit_has_the_one_check_to_require(self):
        jobs = self.jobs("presubmit.yml")
        self.assertIn("name: CI", jobs["ci"])
        needed = re.search(r"needs: \[([^\]]*)\]", jobs["ci"])
        self.assertIsNotNone(needed)
        self.assertEqual(set(jobs) - {"ci"}, {n.strip() for n in needed.group(1).split(",")})

    def test_only_presubmit_checks_the_version(self):
        self.assertIn("version", self.jobs("presubmit.yml"))
        self.assertNotIn("version", self.jobs("postsubmit.yml"))

    def test_the_ui_tier_runs_the_specs_the_change_selects(self):
        # The tier is narrowed by what the change touches (`ui_specs`), not split over runners.
        ui = self.jobs("presubmit.yml")["ui"]
        self.assertIn("npx playwright test --project=ui $SPECS", ui)
        self.assertIn("needs.changes.outputs.ui_specs", ui)
        self.assertNotIn("shard", ui)

    def test_the_wasm_check_runs_beside_the_host_tests(self):
        # `just test` (the local recipe) is both; in CI they are two jobs, so neither waits for the other.
        jobs = self.jobs("presubmit.yml")
        self.assertIn("just test-host", jobs["test"])
        self.assertNotIn("just test-wasm", jobs["test"])
        self.assertIn("just test-wasm", jobs["wasm"])
        self.assertNotIn("just test-host", jobs["wasm"])

    def test_the_test_builds_carry_no_debug_info(self):
        # Debug info is most of the time the linker spends on the test binaries, and a CI log never shows it.
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', self.text("presubmit.yml"))

    def test_the_rust_jobs_run_what_the_change_selects(self):
        jobs = self.jobs("presubmit.yml")
        self.assertIn("rust == 'true'", jobs["test"])
        self.assertIn("just test-host $CRATES", jobs["test"])
        self.assertIn("rust_args", jobs["test"])
        self.assertIn("wasm == 'true'", jobs["wasm"])
        self.assertIn("just test-wasm $CRATES", jobs["wasm"])
        self.assertIn("server_release == 'true'", jobs["server-release"])
        # The script tests need no Rust, so a change without any crate still runs them.
        self.assertIn("just test-scripts", jobs["scripts"])
        self.assertNotIn("just test-scripts", jobs["test"])
        outputs = self.text("presubmit.yml").split("steps:")[0]
        for name in ("rust", "rust_args", "wasm", "wasm_args", "server_release"):
            self.assertIn(f"{name}: ${{{{ steps.select.outputs.{name} }}}}", outputs)

    def test_main_runs_every_crate(self):
        post = self.text("postsubmit.yml")
        self.assertNotIn("$CRATES", post)
        self.assertNotIn("rust_args", post)

    def test_the_e2e_tier_runs_only_after_the_merge(self):
        # The E2E tier (an archive packed and served, the real server's headers) is the slowest to set up and proves what
        # only main needs to: postsubmit runs it before the deploy, presubmit never does.
        self.assertNotIn("e2e", self.jobs("presubmit.yml"))
        self.assertNotIn("scripts/e2e-archive.sh", self.text("presubmit.yml"))
        self.assertIn("e2e", self.jobs("postsubmit.yml"))

    def test_main_splits_the_ui_tier_over_runners(self):
        # Main runs every spec, so the UI tier (the longest job) is split: Playwright gives each shard a share of the tests.
        ui = self.jobs("postsubmit.yml")["ui"]
        self.assertIn("npx playwright test --project=ui --shard=${{ matrix.shard }}/6", ui)
        self.assertIn("shard: [1, 2, 3, 4, 5, 6]", ui)
        self.assertIn("fail-fast: false", ui)
        self.assertIn("playwright-report-ui-${{ matrix.shard }}", ui)

    def test_main_runs_the_host_tests_the_wasm_check_and_the_script_tests_side_by_side(self):
        # What `just test` and `just test-scripts` run, as three jobs, so none waits for another.
        jobs = self.jobs("postsubmit.yml")
        self.assertIn("just test-host", jobs["test"])
        self.assertNotIn("just test-wasm", jobs["test"])
        self.assertNotIn("just test-scripts", jobs["test"])
        self.assertIn("just test-wasm", jobs["wasm"])
        self.assertNotIn("just test-host", jobs["wasm"])
        self.assertIn("just test-scripts", jobs["scripts"])

    def test_main_builds_its_tests_without_debug_info(self):
        self.assertIn('CARGO_PROFILE_TEST_DEBUG: "0"', self.text("postsubmit.yml"))
        self.assertIn('CARGO_PROFILE_DEV_DEBUG: "0"', self.text("postsubmit.yml"))

    def test_only_a_pull_request_is_cancelled_by_a_newer_push(self):
        self.assertIn("cancel-in-progress: true", self.text("presubmit.yml"))
        self.assertIn("cancel-in-progress: false", self.text("postsubmit.yml"))


class Deploy(unittest.TestCase):
    """A push to main deploys only when it changed a file the deployed site is built from (DEPLOY_INPUTS), since the
    last deploy: postsubmit.yml's Changes job passes that commit as the base."""

    def test_tests_alone_do_not_deploy(self):
        self.assertFalse(
            ci_changes.deploys(
                [
                    "tests/e2e/tests/map.spec.ts",
                    "crates/cityloom_editor/src/street/view/tests.rs",
                    "crates/citymodel_builder/tests/tiles.rs",
                    "crates/cityloom_editor/src/ui_guard.rs",
                ]
            )
        )

    def test_ci_configuration_alone_does_not_deploy(self):
        self.assertFalse(ci_changes.deploys([".claude/agents/deploy.md", "scripts/ci/spec-map.json", ".github/workflows/presubmit.yml"]))

    def test_one_input_among_tests_deploys(self):
        self.assertTrue(ci_changes.deploys(["tests/e2e/tests/map.spec.ts", "web/app.css"]))

    def test_each_kind_of_input_deploys(self):
        for f in (
            "crates/cityloom_server/src/main.rs",
            "crates/cityloom_server/migrations/0007_x.sql",
            "web/map.html",
            "deploy/Dockerfile",
            "cities/montreal.toml",
            "Cargo.lock",
            "package-lock.json",
            "justfile",
            "scripts/deploy.sh",
            "scripts/precompress.mjs",
            ".github/workflows/postsubmit.yml",
        ):
            with self.subTest(f):
                self.assertTrue(ci_changes.deploys([f]))

    def test_an_unknown_base_deploys(self):
        self.assertTrue(ci_changes.deploys(None))

    def test_a_bare_version_bump_does_not_deploy(self):
        self.assertFalse(ci_changes.deploys([]))  # changed_files has already left the bump out

    def test_a_site_change_behind_a_tests_only_one_deploys(self):
        # The base is the last deployed commit, so a web/ change whose own run failed is still in the diff when a later
        # merge adds only tests.
        self.assertTrue(ci_changes.deploys(["web/app.js", "tests/e2e/tests/street.spec.ts"]))

    def test_a_base_not_behind_head_deploys(self):
        # A repository of its own: CI's checkouts are shallow, so this one's history cannot be counted on.
        with tempfile.TemporaryDirectory() as tmp:
            run = lambda *a: subprocess.run(["git", "-C", tmp, *a], check=True, capture_output=True, text=True).stdout.strip()
            run("init", "-q")
            for n in ("1", "2"):
                (Path(tmp) / "f").write_text(n, encoding="utf-8")
                run("add", "f")
                run("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", n)
            parent, head = run("rev-parse", "HEAD~1"), run("rev-parse", "HEAD")
            with mock.patch.object(ci_changes, "ROOT", Path(tmp)):
                self.assertTrue(ci_changes.behind(parent, head))
                self.assertTrue(ci_changes.behind(head, head))
                self.assertFalse(ci_changes.behind(head, parent))
                self.assertFalse(ci_changes.behind("0" * 40, head))
                self.assertFalse(ci_changes.behind("HEAD~1; rm -rf /", head))

    def test_every_tracked_file_is_classified(self):
        def known(f: str) -> bool:
            return ci_changes.served(f) or any(ci_changes.glob(p, f) for p in ci_changes.NOT_SERVED + ci_changes.CARVED_OUT)

        loose = [f for f in ci_changes.git("ls-files").splitlines() if not known(f)]
        self.assertEqual(loose, [], "classify these in DEPLOY_INPUTS or NOT_SERVED (scripts/ci/ci_changes.py)")

    def test_what_is_not_served_is_no_input(self):
        both = [
            f
            for f in ci_changes.git("ls-files").splitlines()
            if any(ci_changes.glob(p, f) for p in ci_changes.NOT_SERVED) and ci_changes.served(f)
        ]
        self.assertEqual(both, [])

    def test_the_tile_cache_key_reads_only_inputs(self):
        # Every file the deploy job's tile cache hashes is a deploy input.
        workflow = (ROOT / ".github" / "workflows" / "postsubmit.yml").read_text(encoding="utf-8")
        key = re.search(r"key: tiles-\$\{\{ hashFiles\(([^)]*)\)", workflow)
        self.assertIsNotNone(key)
        for pattern in [p for p in re.findall(r"'([^']*)'", key.group(1)) if not p.startswith("!")]:
            with self.subTest(pattern):
                self.assertTrue(ci_changes.served(pattern.replace("**", "x.rs")))

    # A served Rust file whose only change is inside #[cfg(test)] items is not built into the site.
    CODE = "pub fn add(a: u8, b: u8) -> u8 {\n    a + b\n}\n"
    TESTS = "\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn adds() {\n        assert_eq!(add(1, 2), 3);\n    }\n}\n"
    MORE = "\n    #[test]\n    fn adds_zero() {\n        assert_eq!(add(0, 0), 0);\n    }\n}\n"

    def same(self, base: str, head: str) -> bool:
        old, new = ci_changes.outside_tests(base), ci_changes.outside_tests(head)
        self.assertIsNotNone(old)
        self.assertIsNotNone(new)
        return old == new

    def test_a_test_added_to_the_tests_module_does_not_count(self):
        base = self.CODE + self.TESTS
        head = self.CODE + self.TESTS[: self.TESTS.rindex("}")] + self.MORE
        self.assertTrue(self.same(base, head))

    def test_a_new_tests_module_does_not_count(self):
        self.assertTrue(self.same(self.CODE, self.CODE + self.TESTS))

    def test_whitespace_and_comments_do_not_count(self):
        head = "// Adds.\n/// Two numbers.\npub fn add(a: u8,\n b: u8) -> u8 { /* sum */ a + b }\n"
        self.assertTrue(self.same(self.CODE, head))

    def test_a_change_outside_the_tests_counts(self):
        head = self.CODE.replace("a + b", "a.saturating_add(b)") + self.TESTS
        self.assertFalse(self.same(self.CODE + self.TESTS, head))

    def test_removing_the_cfg_deploys(self):
        base = self.CODE + "#[cfg(test)]\nfn helper() -> u8 { 1 }\n"
        self.assertFalse(self.same(base, base.replace("#[cfg(test)]\n", "")))

    def test_each_kind_of_item_is_removed(self):
        marked = (
            "#[cfg(test)]\nuse std::fmt;\n",
            "#[cfg(test)]\nconst N: usize = 3;\n",
            "#[cfg(test)]\nconst P: Point = Point { x: 1 };\n",
            "#[cfg(test)]\nfn bytes() -> [u8; 3] { [1, 2, 3] }\n",
            "#[cfg(test)]\nimpl Point { fn origin() -> Self { Point { x: 0 } } }\n",
            "#[cfg(test)]\n#[derive(Debug)]\nstruct S { a: u8 }\n",
            "#[cfg(test)]\nmod fixtures;\n",
            "#[cfg(test)]\npub fn split(&mut self, at: Option<i32>) -> Result<(), Refusal> { Ok(()) }\n",
            "#[cfg(test)]\nfn pairs<A, B>(m: HashMap<A, Vec<B>>) where A: Ord, B: Clone {}\n",
            "#[ cfg ( test ) ]\nmod spaced {}\n",
        )
        for item in marked:
            with self.subTest(item):
                self.assertTrue(self.same(self.CODE + "fn last() {}\n", self.CODE + item + "fn last() {}\n"))

    def test_other_conditions_are_code(self):
        for attr in ("#[cfg(not(test))]", "#[cfg(any(test, feature = \"x\"))]", "#[cfg(all(test, unix))]", "#![cfg(test)]"):
            with self.subTest(attr):
                self.assertFalse(self.same(self.CODE, self.CODE + attr + "\nfn f() {}\n"))

    def test_chars_and_lifetimes_do_not_unbalance(self):
        tricky = (
            "\n#[cfg(test)]\nmod tests {\n"
            "    fn a<'a>(s: &'a str) -> &'static str { let _ = ('{', '\\'', '}', b'{', '\\u{7b}'); \"}\" }\n"
            "    const R: &str = r#\"}\"# ;\n"
            "    const B: &[u8] = br\"{\";\n"
            "    /* nested /* } */ still a comment { */\n"
            "    // }\n"
            "}\n"
        )
        self.assertTrue(self.same(self.CODE + "fn last() {}\n", self.CODE + tricky + "fn last() {}\n"))
        self.assertNotIn("last", ci_changes.outside_tests(self.CODE + tricky)[:-1])

    def test_strings_are_code(self):
        self.assertFalse(self.same('const S: &str = "a b";\n', 'const S: &str = "a  b";\n'))
        self.assertFalse(self.same('const S: &str = "// x";\n', 'const S: &str = "// y";\n'))

    def test_what_cannot_be_read_is_none(self):
        for text in (
            'const S: &str = "open;\n',
            "fn f() {} /* open\n",
            "fn f() { (] }\n",
            "fn f() {} }\n",
            "fn f() {\n",
            "struct S {\n    #[cfg(test)]\n    a: u8,\n}\n",
            "fn f(#[cfg(test)] a: u8) {}\n",
            "struct S {\n    #[cfg(test)]\n    a: Vec<u8>,\n    b: u8,\n}\n",
            # An arm is not taken apart: past its `,` (hidden by the `<`) is the next arm's real code.
            "fn f(x: u8) -> u8 { match x { #[cfg(test)] 0 if x < 1 => 1, _ => { real() } } }\n",
            "#[cfg(test)]\nfn f() -> u8 { 1 }\n}\n",
            "#[cfg(test)]\nfn f()",
            "#[cfg(test)]\n",
        ):
            with self.subTest(text):
                self.assertIsNone(ci_changes.outside_tests(text))


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
