"""The software factory's red and green gates (.claude/factory/gate.py): `just test-scripts`.

The gates decide whether the test-writer's red commit and the implementer's green head may go on. These tests pin
what each gate reads from cargo's and Playwright's output, what it counts as a test file, and what it refuses. The
cargo and Playwright runs are faked; git is real, in a throwaway repository.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / ".claude" / "factory"))

import gate  # noqa: E402

CARGO_LIST_BASE = """\
   Compiling cityloom_model v0.1.0
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.2s
     Running unittests src/lib.rs (target/debug/deps/cityloom_model-0a1b2c3d4e5f6789)
street::tests::keeps_lanes: test
street::tests::drops_empty: test

2 tests, 0 benchmarks
     Running tests/import.rs (target/debug/deps/import-89abcdef01234567)
reads_a_way: test

1 test, 0 benchmarks
   Doc-tests cityloom_model
src/lib.rs - street::Street (line 12): test

1 test, 0 benchmarks
"""

CARGO_RUN_RED = """\
     Running unittests src/lib.rs (target/debug/deps/cityloom_model-0a1b2c3d4e5f6789)

running 3 tests
test street::tests::keeps_lanes ... ok
test street::tests::drops_empty ... ok
test street::tests::splits_at_a_junction ... FAILED

failures:

---- street::tests::splits_at_a_junction stdout ----
thread 'street::tests::splits_at_a_junction' panicked at src/street.rs:40:9:
not yet implemented

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

error: test failed, to rerun pass `-p cityloom_model --lib`
     Running tests/import.rs (target/debug/deps/import-89abcdef01234567)

running 1 test
test reads_a_way ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests cityloom_model

running 1 test
test src/lib.rs - street::Street (line 12) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
error: 1 target failed:
    `-p cityloom_model --lib`
"""


def playwright_report(*tests: tuple[str, list[str], str]) -> dict:
    """A Playwright JSON report: each test as (file, title path, status), the path's head a describe block."""
    suites: dict[str, dict] = {}
    for file, titles, status in tests:
        top = suites.setdefault(file, {"title": file, "file": file, "specs": [], "suites": []})
        parent = top
        for title in titles[:-1]:
            child = next((s for s in parent["suites"] if s["title"] == title), None)
            if child is None:
                child = {"title": title, "file": file, "specs": [], "suites": []}
                parent["suites"].append(child)
            parent = child
        test = {"projectName": "ui", "status": status, "results": []}
        parent["specs"].append({"title": titles[-1], "file": file, "tests": [test]})
    return {"suites": list(suites.values())}


class CargoOutput(unittest.TestCase):
    def test_a_listing_names_each_test_by_its_binary(self):
        self.assertEqual(
            gate.cargo_listing(CARGO_LIST_BASE),
            {
                "cityloom_model unittests src/lib.rs::street::tests::keeps_lanes",
                "cityloom_model unittests src/lib.rs::street::tests::drops_empty",
                "import tests/import.rs::reads_a_way",
                "doc cityloom_model::src/lib.rs - street::Street (line 12)",
            },
        )

    def test_a_run_gives_each_test_its_outcome(self):
        self.assertEqual(
            gate.cargo_results(CARGO_RUN_RED),
            {
                "cityloom_model unittests src/lib.rs::street::tests::keeps_lanes": "ok",
                "cityloom_model unittests src/lib.rs::street::tests::drops_empty": "ok",
                "cityloom_model unittests src/lib.rs::street::tests::splits_at_a_junction": "failed",
                "import tests/import.rs::reads_a_way": "ok",
                "doc cityloom_model::src/lib.rs - street::Street (line 12)": "ignored",
            },
        )

    def test_should_panic_and_ignore_reasons_are_not_part_of_the_name(self):
        out = (
            "     Running unittests src/lib.rs (target/debug/deps/a-0123456789abcdef)\n"
            "test x::panics - should panic ... ok\n"
            "test x::slow ... ignored, needs a database\n"
        )
        self.assertEqual(
            gate.cargo_results(out),
            {"a unittests src/lib.rs::x::panics": "ok", "a unittests src/lib.rs::x::slow": "ignored"},
        )

    def test_the_hash_of_the_binary_is_not_part_of_the_name(self):
        one = "     Running unittests src/lib.rs (target/debug/deps/a-0123456789abcdef)\nt: test\n"
        two = "     Running unittests src/lib.rs (/abs/target/debug/deps/a-fedcba9876543210)\nt: test\n"
        self.assertEqual(gate.cargo_listing(one), gate.cargo_listing(two))

    def test_a_compile_error_is_told_apart_from_a_failing_test(self):
        self.assertTrue(gate.did_not_compile(1, "error[E0425]: cannot find function `split` in this scope\n"))
        self.assertTrue(gate.did_not_compile(101, "error: could not compile `cityloom_model` (lib test)\n"))
        self.assertFalse(gate.did_not_compile(101, CARGO_RUN_RED))
        self.assertFalse(gate.did_not_compile(0, "warning: unused variable\n"))


class PlaywrightOutput(unittest.TestCase):
    def test_each_test_is_named_by_its_file_title_path_and_project(self):
        report = playwright_report(
            ("street.spec.ts", ["Street", "adds a lane"], "expected"),
            ("street.spec.ts", ["Street", "splits", "at a junction"], "unexpected"),
            ("map.spec.ts", ["opens"], "skipped"),
        )
        self.assertEqual(
            gate.playwright_results(report),
            {
                "street.spec.ts::Street > adds a lane::ui": "expected",
                "street.spec.ts::Street > splits > at a junction::ui": "unexpected",
                "map.spec.ts::opens::ui": "skipped",
            },
        )

    def test_a_listing_has_no_outcome_but_the_same_names(self):
        report = playwright_report(("street.spec.ts", ["Street", "adds a lane"], "expected"))
        for spec in report["suites"][0]["suites"][0]["specs"]:
            for test in spec["tests"]:
                del test["status"]
        self.assertEqual(gate.playwright_results(report), {"street.spec.ts::Street > adds a lane::ui": None})


class TestFiles(unittest.TestCase):
    def test_what_only_the_test_writer_may_change(self):
        for path in [
            "crates/cityloom_editor/src/street/view/tests.rs",
            "crates/cityloom_model/tests/import.rs",
            "crates/cityloom_server/tests/fixtures/a.json",
            "e2e/tests/street.spec.ts",
            "e2e/fixtures/plateau.osm.pbf",
            "e2e/tiers.json",
            "scripts/ci/spec-map.json",
        ]:
            with self.subTest(path):
                self.assertTrue(gate.is_test_file(path))

    def test_what_the_implementer_writes(self):
        for path in [
            "crates/cityloom_editor/src/street/model/mod.rs",
            "crates/cityloom_editor/Cargo.toml",
            "Cargo.lock",
            "crates/cityloom_editor/src/street/i18n/en.ftl",
            "e2e/helpers.ts",
            "web/app.css",
        ]:
            with self.subTest(path):
                self.assertFalse(gate.is_test_file(path))
                self.assertFalse(gate.is_guarded(path))

    def test_the_guarded_set_holds_the_tests_the_checks_and_the_factory(self):
        for path in [
            "e2e/tests/street.spec.ts",
            ".github/workflows/presubmit.yml",
            "scripts/check-version.sh",
            ".claude/factory/gate.py",
            ".claude/settings.json",
            "crates/cityloom_editor/src/i18n_guard.rs",
            "crates/cityloom_editor/src/layout_guard.rs",
            "crates/cityloom_editor/src/dependency_guard.rs",
            "crates/cityloom_editor/src/shared/contrast.rs",
        ]:
            with self.subTest(path):
                self.assertTrue(gate.is_guarded(path))


class CfgTestBlocks(unittest.TestCase):
    def test_a_test_module_is_one_block_to_its_closing_brace(self):
        src = 'fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn b() { assert_eq!(1, 1); }\n}\n\nfn c() {}\n'
        self.assertEqual(gate.cfg_test_blocks(src), ['#[cfg(test)]\nmod tests {\n    #[test]\n    fn b() { assert_eq!(1, 1); }\n}'])

    def test_a_test_only_item_ends_at_its_semicolon(self):
        src = "#[cfg(test)]\nuse crate::shared::testing::fake;\nfn a() {}\n"
        self.assertEqual(gate.cfg_test_blocks(src), ["#[cfg(test)]\nuse crate::shared::testing::fake;"])

    def test_further_attributes_belong_to_the_item(self):
        src = '#[cfg(test)]\n#[path = "x.rs"]\nmod x;\n'
        self.assertEqual(gate.cfg_test_blocks(src), ['#[cfg(test)]\n#[path = "x.rs"]\nmod x;'])

    def test_braces_in_strings_chars_and_comments_do_not_count(self):
        src = (
            "#[cfg(test)]\nmod tests {\n"
            '    const A: &str = "}";\n'
            "    const B: char = '}';\n"
            '    const C: &str = r#"}"}"#;\n'
            "    // }\n"
            "    /* } /* } */ */\n"
            "    fn life<'a>(x: &'a str) -> &'a str { x }\n"
            "}\nfn after() {}\n"
        )
        blocks = gate.cfg_test_blocks(src)
        self.assertEqual(len(blocks), 1)
        self.assertTrue(blocks[0].endswith("x }\n}"))

    def test_an_attribute_in_a_string_or_comment_is_not_one(self):
        src = 'const S: &str = "#[cfg(test)] mod x {}";\n// #[cfg(test)]\nfn a() {}\n'
        self.assertEqual(gate.cfg_test_blocks(src), [])

    def test_test_inside_all_or_any_counts(self):
        src = "#[cfg(all(test, not(target_arch = \"wasm32\")))]\nmod native { fn t() {} }\n"
        self.assertEqual(len(gate.cfg_test_blocks(src)), 1)

    def test_a_cfg_that_only_mentions_a_test_feature_does_not(self):
        for attr in ['#[cfg(feature = "test-provider")]', '#[cfg(any(feature = "test-provider"))]']:
            with self.subTest(attr):
                self.assertEqual(gate.cfg_test_blocks(attr + "\nmod fake { }\n"), [])


class JudgeRed(unittest.TestCase):
    def test_new_tests_that_fail_and_old_ones_that_pass_go_on(self):
        base = {"a", "b"}
        red = {"a": "ok", "b": "ignored", "c": "failed"}
        self.assertEqual(gate.judge_red(base, red, "Rust"), ([], ["c"]))

    def test_a_new_test_that_already_passes_is_refused(self):
        problems, _ = gate.judge_red({"a"}, {"a": "ok", "c": "ok"}, "Rust")
        self.assertEqual(problems, ["Rust: the new test c already passes: it does not prove the change"])

    def test_a_new_test_that_is_ignored_is_refused(self):
        problems, _ = gate.judge_red(set(), {"c": "ignored"}, "Rust")
        self.assertEqual(problems, ["Rust: the new test c is ignored or skipped"])

    def test_an_old_test_that_now_fails_is_refused(self):
        problems, _ = gate.judge_red({"a"}, {"a": "failed", "c": "failed"}, "Rust")
        self.assertEqual(problems, ["Rust: the existing test a fails"])

    def test_a_removed_test_is_refused(self):
        problems, _ = gate.judge_red({"a", "gone"}, {"a": "ok", "c": "failed"}, "Rust")
        self.assertEqual(problems, ["Rust: the test gone is gone"])

    def test_playwright_outcomes_read_the_same(self):
        red = {"s::old::ui": "expected", "s::new::ui": "unexpected", "s::skip::ui": "skipped"}
        self.assertEqual(gate.judge_red({"s::old::ui", "s::skip::ui"}, red, "Playwright"), ([], ["s::new::ui"]))


def git(cwd: Path, *args: str) -> str:
    env = dict(
        os.environ,
        GIT_AUTHOR_NAME="t",
        GIT_AUTHOR_EMAIL="t@example.invalid",
        GIT_COMMITTER_NAME="t",
        GIT_COMMITTER_EMAIL="t@example.invalid",
    )
    return subprocess.run(["git", *args], cwd=cwd, env=env, check=True, capture_output=True, text=True).stdout.strip()


def commit(cwd: Path, files: dict[str, str], message: str) -> str:
    for name, text in files.items():
        path = cwd / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    git(cwd, "add", "-A")
    git(cwd, "commit", "-q", "-m", message)
    return git(cwd, "rev-parse", "HEAD")


class FakeRunner:
    """Answers cargo and Playwright by the commit checked out, records every command, runs git for real."""

    def __init__(self, repo: Path, listings: dict[str, str], runs: dict[str, tuple[int, str]], playwright=None):
        self.repo = repo
        self.listings = listings
        self.runs = runs
        self.playwright = playwright or {}
        self.commands: list[tuple[str, ...]] = []
        self.envs: list[dict] = []

    def __call__(self, cmd, cwd, env=None):
        self.commands.append(tuple(cmd))
        self.envs.append(env or {})
        head = git(self.repo, "rev-parse", "HEAD")
        if cmd[:2] == ["cargo", "test"]:
            if "--list" in cmd:
                return 0, self.listings[head]
            return self.runs[head]
        if cmd[:3] == ["npx", "playwright", "test"]:
            report = self.playwright[(head, "--list" in cmd)]
            Path(env["PLAYWRIGHT_JSON_OUTPUT_FILE"]).write_text(json.dumps(report), encoding="utf-8")
            failed = any(s == "unexpected" for s in gate.playwright_results(report).values())
            return (1 if failed else 0), ""
        return 0, ""


class Gates(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        git(self.repo, "init", "-q", "-b", "work")
        self.base = commit(
            self.repo,
            {
                "crates/m/src/street.rs": "pub fn keep() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn keeps() {}\n}\n",
                "crates/m/Cargo.toml": '[package]\nname = "m"\nversion = "0.1.0"\n',
            },
            "base",
        )
        os.environ.setdefault("DATABASE_URL", "postgres://unused")

    def tearDown(self):
        self.tmp.cleanup()

    def red_commit(self, extra: dict[str, str] | None = None) -> str:
        files = {
            "crates/m/src/street.rs": (
                "pub fn keep() {}\npub fn split() -> usize { todo!() }\n\n#[cfg(test)]\nmod tests {\n"
                "    #[test]\n    fn keeps() {}\n    #[test]\n    fn splits() { assert_eq!(super::split(), 2); }\n}\n"
            )
        }
        files.update(extra or {})
        return commit(self.repo, files, "Red: splits\n\nFactory-Red: CLM-1")

    def test_the_red_gate_passes_a_red_commit_whose_new_test_fails(self):
        red = self.red_commit()
        runner = FakeRunner(
            self.repo,
            listings={self.base: "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntests::keeps: test\n"},
            runs={
                red: (
                    101,
                    "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\n"
                    "test tests::keeps ... ok\ntest tests::splits ... FAILED\n",
                )
            },
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, [])
        self.assertEqual(report.new_tests, ["m unittests src/lib.rs::tests::splits"])
        # It lists the tests at the base, comes back to the branch it started on, and leaves no change behind.
        self.assertEqual(git(self.repo, "rev-parse", "--abbrev-ref", "HEAD"), "work")
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")
        self.assertIn(("cargo", "build", "--target", "wasm32-unknown-unknown", "--workspace", "--exclude", "cityloom_server", "--exclude", "citymodel_builder"), runner.commands)

    def test_the_red_gate_refuses_a_red_commit_that_does_not_compile(self):
        red = self.red_commit()
        runner = FakeRunner(
            self.repo,
            listings={self.base: "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntests::keeps: test\n"},
            runs={red: (101, "error[E0425]: cannot find function `split`\nerror: could not compile `m` (lib test)\n")},
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Rust: the tests do not compile at the red commit: add the new API as signatures with todo!() bodies"])

    def test_the_red_gate_refuses_a_commit_with_no_new_test(self):
        red = commit(self.repo, {"crates/m/src/other.rs": "pub fn x() {}\n"}, "Red: nothing")
        runner = FakeRunner(
            self.repo,
            listings={self.base: "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntests::keeps: test\n"},
            runs={red: (0, "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntest tests::keeps ... ok\n")},
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["the red commit adds no test"])

    def test_the_red_gate_refuses_a_dirty_worktree(self):
        self.red_commit()
        (self.repo / "loose.rs").write_text("x", encoding="utf-8")
        g = gate.Gate(self.repo, FakeRunner(self.repo, {}, {}))
        with self.assertRaises(gate.Refused):
            g.red(self.base)

    def test_the_red_gate_runs_the_specs_with_new_browser_tests(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n"})
        listing = "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntests::keeps: test\n"
        runner = FakeRunner(
            self.repo,
            listings={self.base: listing},
            runs={red: (101, "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntest tests::keeps ... ok\ntest tests::splits ... FAILED\n")},
            playwright={
                (self.base, True): playwright_report(("street.spec.ts", ["adds a lane"], "expected")),
                (red, True): playwright_report(("street.spec.ts", ["adds a lane"], "expected"), ("street.spec.ts", ["splits"], "expected")),
                (red, False): playwright_report(("street.spec.ts", ["adds a lane"], "expected"), ("street.spec.ts", ["splits"], "unexpected")),
            },
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, [])
        self.assertIn("street.spec.ts::splits::ui", report.new_tests)
        self.assertIn(("npx", "playwright", "test", "tests/street.spec.ts", "--reporter=json"), runner.commands)

    BASE_SPECS = (
        ("street.spec.ts", ["adds a lane"], "expected"),
        ("street.spec.ts", ["drops a lane"], "expected"),
        ("map.spec.ts", ["opens the city"], "expected"),
        ("map.spec.ts", ["zooms"], "expected"),
        ("home.spec.ts", ["lists the projects"], "expected"),
    )

    def browser_red_runner(self, red: str, red_listing: tuple | dict, red_run: tuple) -> FakeRunner:
        """A base with three specs; the red commit's listing (tests, or a whole report) and the run the gate asks for."""
        listing = "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntests::keeps: test\n"
        return FakeRunner(
            self.repo,
            listings={self.base: listing},
            runs={red: (101, "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntest tests::keeps ... ok\ntest tests::splits ... FAILED\n")},
            playwright={
                (self.base, True): playwright_report(*self.BASE_SPECS),
                (red, True): red_listing if isinstance(red_listing, dict) else playwright_report(*red_listing),
                (red, False): playwright_report(*red_run),
            },
        )

    def test_the_red_gate_runs_every_spec_when_a_shared_helper_changed(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n", "e2e/tests/fixtures.ts": "// a changed helper\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        broken = ("home.spec.ts", ["lists the projects"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS + (("street.spec.ts", ["splits"], "expected"),),
            red_run=self.BASE_SPECS[:4] + (broken, splits),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: the existing test home.spec.ts::lists the projects::ui fails"])
        self.assertEqual(self.playwright_runs(runner), [("npx", "playwright", "test", "--reporter=json")])

    def test_the_red_gate_checks_the_browser_tests_when_only_a_helper_changed(self):
        red = self.red_commit({"e2e/tests/fixtures.ts": "// a changed helper\n"})
        broken = ("home.spec.ts", ["lists the projects"], "unexpected")
        runner = self.browser_red_runner(red, red_listing=self.BASE_SPECS, red_run=self.BASE_SPECS[:4] + (broken,))
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: the existing test home.spec.ts::lists the projects::ui fails"])

    def test_the_red_gate_says_a_spec_does_not_load_instead_of_naming_its_tests_gone(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n", "e2e/tests/map.spec.ts": "// a bad import\n"})
        # One file that does not load empties Playwright's whole listing (1.65): no suite, only the error.
        listing = {
            "suites": [],
            "errors": [
                {
                    "message": "Error: Cannot find module './missing'\n\n   at map.spec.ts:3",
                    "location": {"file": str(self.repo / "e2e/tests/map.spec.ts"), "line": 3, "column": 1},
                }
            ],
        }
        runner = self.browser_red_runner(red, red_listing=listing, red_run=())
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: map.spec.ts does not load: Error: Cannot find module './missing'"])
        self.assertEqual(self.playwright_runs(runner), [])

    def test_a_shared_file_renamed_to_a_note_still_runs_every_spec(self):
        self.base = commit(self.repo, {"e2e/fixtures/plateau.osm": "<osm/>\n"}, "a fixture")
        git(self.repo, "mv", "e2e/fixtures/plateau.osm", "e2e/fixtures/plateau.md")
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS + (("street.spec.ts", ["splits"], "expected"),),
            red_run=self.BASE_SPECS + (splits,),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, [])
        self.assertEqual(self.playwright_runs(runner), [("npx", "playwright", "test", "--reporter=json")])

    def test_a_new_spec_listed_in_the_tiers_runs_alone(self):
        red = self.red_commit({"e2e/tests/split.spec.ts": "// a new spec\n", "e2e/tiers.json": "{}\n", "e2e/CLAUDE.md": "notes\n"})
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS + (("split.spec.ts", ["splits"], "expected"),),
            red_run=(("split.spec.ts", ["splits"], "unexpected"),),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, [])
        self.assertEqual(self.playwright_runs(runner), [("npx", "playwright", "test", "tests/split.spec.ts", "--reporter=json")])

    def playwright_runs(self, runner: FakeRunner) -> list[tuple[str, ...]]:
        return [c for c in runner.commands if c[:3] == ("npx", "playwright", "test") and "--list" not in c]

    def test_the_red_gate_does_not_count_the_tests_of_specs_it_did_not_run_as_gone(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n"})
        splits = ("street.spec.ts", ["splits"], "expected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS + (splits,),
            red_run=self.BASE_SPECS[:2] + (("street.spec.ts", ["splits"], "unexpected"),),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, [])
        self.assertIn("street.spec.ts::splits::ui", report.new_tests)
        self.assertEqual(self.playwright_runs(runner), [("npx", "playwright", "test", "tests/street.spec.ts", "--reporter=json")])

    def test_the_red_gate_still_refuses_a_test_gone_from_a_spec_it_ran(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=(self.BASE_SPECS[0], splits) + self.BASE_SPECS[2:],
            red_run=(self.BASE_SPECS[0], splits),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: the test street.spec.ts::drops a lane::ui is gone"])

    def test_the_red_gate_refuses_a_test_gone_from_a_changed_spec_with_no_new_test(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n", "e2e/tests/map.spec.ts": "// one test fewer\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS[:3] + (self.BASE_SPECS[4], splits),
            red_run=self.BASE_SPECS[:3] + (splits,),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: the test map.spec.ts::zooms::ui is gone"])

    def test_the_red_gate_refuses_a_spec_deleted_whole(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS[:2] + (self.BASE_SPECS[4], splits),
            red_run=self.BASE_SPECS[:2] + (splits,),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(
            report.problems,
            ["Playwright: the test map.spec.ts::opens the city::ui is gone", "Playwright: the test map.spec.ts::zooms::ui is gone"],
        )
        # A spec that has no test left is not run.
        self.assertEqual(self.playwright_runs(runner), [("npx", "playwright", "test", "tests/street.spec.ts", "--reporter=json")])

    def test_the_red_gate_runs_a_changed_spec_and_refuses_an_old_test_that_now_fails(self):
        red = self.red_commit({"e2e/tests/street.spec.ts": "// a new test\n", "e2e/tests/map.spec.ts": "// a broken test\n"})
        splits = ("street.spec.ts", ["splits"], "unexpected")
        runner = self.browser_red_runner(
            red,
            red_listing=self.BASE_SPECS + (splits,),
            red_run=self.BASE_SPECS[:2] + (("map.spec.ts", ["opens the city"], "unexpected"), self.BASE_SPECS[3], splits),
        )
        report = gate.Gate(self.repo, runner).red(self.base)
        self.assertEqual(report.problems, ["Playwright: the existing test map.spec.ts::opens the city::ui fails"])
        self.assertEqual(
            self.playwright_runs(runner),
            [("npx", "playwright", "test", "tests/map.spec.ts", "tests/street.spec.ts", "--reporter=json")],
        )

    def green_setup(self):
        red = self.red_commit()
        green = commit(self.repo, {"crates/m/src/street.rs": (self.repo / "crates/m/src/street.rs").read_text().replace("todo!()", "2"), "crates/m/Cargo.toml": '[package]\nname = "m"\nversion = "0.2.0"\n'}, "Split")
        return red, green

    def passing_runner(self, *heads: str) -> FakeRunner:
        out = "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntest tests::keeps ... ok\ntest tests::splits ... ok\n"
        return FakeRunner(self.repo, {}, {h: (0, out) for h in heads})

    def test_the_green_gate_passes_when_every_test_passes_and_no_test_changed(self):
        red, green = self.green_setup()
        runner = self.passing_runner(green)
        report = gate.Gate(self.repo, runner).green(self.base, red)
        self.assertEqual(report.problems, [])
        # Green is built as CI builds it: warnings are errors.
        cargo_envs = [e for c, e in zip(runner.commands, runner.envs) if c[0] == "cargo"]
        self.assertTrue(cargo_envs and all(e.get("RUSTFLAGS") == "-D warnings" for e in cargo_envs))
        self.assertIn(("just", "format-check"), runner.commands)
        self.assertIn(("scripts/check-version.sh", self.base), runner.commands)

    def test_the_green_gate_refuses_a_change_to_a_test_block(self):
        red, _ = self.green_setup()
        path = self.repo / "crates/m/src/street.rs"
        head = commit(self.repo, {"crates/m/src/street.rs": path.read_text().replace("assert_eq!(super::split(), 2)", "assert!(true)")}, "Loosen")
        report = gate.Gate(self.repo, self.passing_runner(head)).green(self.base, red)
        self.assertEqual(report.problems, ["crates/m/src/street.rs: a #[cfg(test)] block changed since the red commit"])

    def test_the_green_gate_refuses_a_change_to_a_guarded_file(self):
        red, _ = self.green_setup()
        head = commit(self.repo, {"e2e/tiers.json": "{}\n", ".github/workflows/x.yml": "on: push\n"}, "Touch")
        report = gate.Gate(self.repo, self.passing_runner(head)).green(self.base, red)
        self.assertEqual(
            report.problems,
            [
                ".github/workflows/x.yml: changed since the red commit, and only the test-writer may change it",
                "e2e/tiers.json: changed since the red commit, and only the test-writer may change it",
            ],
        )

    def test_the_green_gate_refuses_a_change_to_the_dependency_guards_tables(self):
        # The tables are outside the guard's #[cfg(test)] block, so only its place in the guarded set protects them.
        guard = "crates/cityloom_editor/src/dependency_guard.rs"
        for table in ("ALLOWED", "EXCEPTIONS"):
            with self.subTest(table):
                red = self.red_commit({guard: f"const {table}: &[Entry] = &[];\n"})
                head = commit(self.repo, {guard: f"const {table}: &[Entry] = &[ENTRY];\n"}, "Allow")
                report = gate.Gate(self.repo, self.passing_runner(head)).green(self.base, red)
                self.assertEqual(report.problems, [f"{guard}: changed since the red commit, and only the test-writer may change it"])

    def test_the_green_gate_refuses_a_failing_test(self):
        red, green = self.green_setup()
        out = "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\ntest tests::keeps ... ok\ntest tests::splits ... FAILED\n"
        runner = FakeRunner(self.repo, {}, {green: (101, out)})
        report = gate.Gate(self.repo, runner).green(self.base, red)
        self.assertEqual(report.problems, ["Rust: m unittests src/lib.rs::tests::splits fails"])

    def test_a_missing_tool_is_the_environment_not_the_change(self):
        red, green = self.green_setup()
        runner = self.passing_runner(green)
        plain = runner.__call__

        def missing_prettier(cmd, cwd, env=None):
            code, out = plain(cmd, cwd, env)
            return (127, "sh: prettier: command not found") if cmd == ["just", "format-check"] else (code, out)

        g = gate.Gate(self.repo, missing_prettier)
        with self.assertRaises(gate.Refused) as caught:
            g.green(self.base, red)
        self.assertIn("just setup", str(caught.exception))

    def test_the_green_gate_refuses_a_red_commit_that_is_not_in_the_history(self):
        red, _ = self.green_setup()
        git(self.repo, "checkout", "-q", "-b", "other", self.base)
        commit(self.repo, {"crates/m/src/x.rs": "\n"}, "Elsewhere")
        g = gate.Gate(self.repo, self.passing_runner())
        with self.assertRaises(gate.Refused):
            g.green(self.base, red)

    def test_the_green_gate_refuses_the_red_commit_itself(self):
        red = self.red_commit()
        g = gate.Gate(self.repo, self.passing_runner(red))
        with self.assertRaises(gate.Refused):
            g.green(self.base, red)


EN = "website/docs"
FR = "website/i18n/fr-CA/docusaurus-plugin-content-docs/current"


class DocsGate(unittest.TestCase):
    """The documentation flow's gate: the writer's branch changes only documentation, each page in both languages, and
    the site passes CI's Prettier check and builds. No Rust, no Playwright and no database."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        git(self.repo, "init", "-q", "-b", "work")
        self.base = commit(
            self.repo,
            {
                f"{EN}/intro.md": "# Intro\n",
                f"{FR}/intro.md": "# Introduction\n",
                "crates/m/src/street.rs": "pub fn keep() {}\n",
            },
            "base",
        )
        saved = os.environ.pop("DATABASE_URL", None)
        self.addCleanup(lambda: saved is None or os.environ.__setitem__("DATABASE_URL", saved))

    def tearDown(self):
        self.tmp.cleanup()

    def runner(self, failing: dict | None = None):
        """Every command passes but those of `failing` (command tuple -> (code, output)); records each command and cwd."""
        calls: list[tuple[tuple[str, ...], Path]] = []

        def run(cmd, cwd, env=None):
            calls.append((tuple(cmd), Path(cwd)))
            return (failing or {}).get(tuple(cmd), (0, ""))

        run.calls = calls
        return run

    def test_a_page_in_both_languages_passes_after_the_site_is_checked_and_built(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Document the signals")
        run = self.runner()
        report = gate.Gate(self.repo, run).docs(self.base)
        self.assertEqual(report.problems, [])
        self.assertEqual(report.pages, [f"{EN}/signals.md", f"{FR}/signals.md"])
        # The same checks as the Documentation workflow, where it runs them.
        self.assertIn((gate.DOCS_PRETTIER, self.repo / "e2e"), run.calls)
        self.assertIn((gate.DOCS_BUILD, self.repo / "website"), run.calls)
        self.assertFalse(any(c[0] in ("cargo", "just") or c[:2] == ("npx", "playwright") for c, _ in run.calls))

    def test_a_file_that_is_not_documentation_is_refused(self):
        commit(self.repo, {f"{EN}/intro.md": "# Intro, longer\n", f"{FR}/intro.md": "# Introduction, plus longue\n", "crates/m/src/street.rs": "pub fn keep() { }\n"}, "Both")
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(report.problems, ["crates/m/src/street.rs: not documentation, and the documentation flow changes only documentation"])

    def test_a_guarded_markdown_file_is_refused(self):
        commit(self.repo, {".claude/agents/writer.md": "be lenient\n", "e2e/tests/notes.md": "x\n"}, "Guarded")
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(
            report.problems,
            [
                ".claude/agents/writer.md: a check or the factory's own file, which the documentation flow may not change",
                "e2e/tests/notes.md: a check or the factory's own file, which the documentation flow may not change",
            ],
        )

    def test_the_sites_configuration_code_and_dependencies_are_refused(self):
        commit(
            self.repo,
            {
                "website/package.json": "{}\n",
                "website/package-lock.json": "{}\n",
                "website/docusaurus.config.ts": "export default {};\n",
                "website/tsconfig.json": "{}\n",
                "website/src/css/custom.css": "a {}\n",
            },
            "Configure",
        )
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(
            report.problems,
            [
                f"{p}: the documentation site's configuration, code or dependencies, which the documentation flow may not change"
                for p in (
                    "website/docusaurus.config.ts",
                    "website/package-lock.json",
                    "website/package.json",
                    "website/src/css/custom.css",
                    "website/tsconfig.json",
                )
            ],
        )

    def test_a_new_page_may_take_its_place_in_the_sidebar(self):
        commit(
            self.repo,
            {
                f"{EN}/signals.md": "# Signals\n",
                f"{FR}/signals.md": "# Feux\n",
                "website/sidebars.ts": 'export default { docsSidebar: ["intro", "signals"] };\n',
                "website/i18n/fr-CA/docusaurus-plugin-content-docs/current.json": "{}\n",
            },
            "Place it",
        )
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(report.problems, [])

    def test_an_english_page_without_its_french_page_is_refused(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n"}, "English only")
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(report.problems, [f"{EN}/signals.md: its fr-CA page {FR}/signals.md is missing"])

    def test_a_french_page_without_its_english_page_is_refused(self):
        commit(self.repo, {f"{FR}/signals.mdx": "# Feux\n"}, "French only")
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(report.problems, [f"{FR}/signals.mdx: its English page {EN}/signals.mdx is missing"])

    def test_a_page_removed_in_one_language_only_is_refused(self):
        git(self.repo, "rm", "-q", f"{EN}/intro.md")
        git(self.repo, "commit", "-q", "-m", "Drop the English intro")
        report = gate.Gate(self.repo, self.runner()).docs(self.base)
        self.assertEqual(report.problems, [f"{FR}/intro.md: its English page {EN}/intro.md is missing"])

    def test_a_site_that_does_not_build_or_is_not_formatted_is_refused(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Both")
        run = self.runner(
            {
                gate.DOCS_PRETTIER: (1, "[warn] ../website/docs/signals.md\n[warn] Code style issues found in the above file."),
                gate.DOCS_BUILD: (1, "Error: Docusaurus found broken links!\n"),
            }
        )
        report = gate.Gate(self.repo, run).docs(self.base)
        self.assertEqual(
            report.problems,
            [
                "the Prettier check of website/ fails: [warn] Code style issues found in the above file.",
                "the documentation site does not build: Error: Docusaurus found broken links!",
            ],
        )

    def test_a_failed_build_is_told_by_its_cause_not_its_stack(self):
        out = (
            "[INFO] [en] Creating an optimized production build...\n"
            "[ERROR] Client bundle compiled with errors therefore further build is impossible.\n"
            '╰─▶   × Error: MDX compilation failed for file "/w/website/docs/smoke.md"\n'
            '        │ Cause: Markdown link with URL `./nowhere.md` in source file "docs/smoke.md" (3:1) couldn\'t be resolved.\n'
            "        │\n"
            "        │     at throwError (/w/website/node_modules/@docusaurus/logger/lib/logger.js:80:11)\n"
            "        │     at onBrokenMarkdownLinks (/w/website/node_modules/@docusaurus/mdx-loader/lib/x.js:20:60)\n"
        )
        self.assertEqual(
            gate.build_error(out),
            "[ERROR] Client bundle compiled with errors therefore further build is impossible.\n"
            '    Error: MDX compilation failed for file "/w/website/docs/smoke.md"\n'
            '    Cause: Markdown link with URL `./nowhere.md` in source file "docs/smoke.md" (3:1) couldn\'t be resolved.',
        )

    def test_documentation_outside_the_site_needs_no_site_check(self):
        commit(self.repo, {"docs/glossary-fr-ca.md": "# Glossaire\n", "crates/m/CLAUDE.md": "# m\n"}, "Notes")
        run = self.runner()
        report = gate.Gate(self.repo, run).docs(self.base)
        self.assertEqual(report.problems, [])
        self.assertEqual(run.calls, [])

    def test_a_branch_with_no_commit_is_refused(self):
        g = gate.Gate(self.repo, self.runner())
        with self.assertRaises(gate.Refused):
            g.docs(self.base)

    def test_a_dirty_worktree_is_refused(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Both")
        (self.repo / "loose.md").write_text("x", encoding="utf-8")
        g = gate.Gate(self.repo, self.runner())
        with self.assertRaises(gate.Refused):
            g.docs(self.base)

    def test_a_site_not_set_up_is_the_environment_not_the_change(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Both")
        g = gate.Gate(self.repo, self.runner({gate.DOCS_BUILD: (127, "sh: docusaurus: command not found")}))
        with self.assertRaises(gate.Refused) as caught:
            g.docs(self.base)
        self.assertIn("just setup-docs", str(caught.exception))

    def test_a_tool_not_on_the_path_is_refused_not_a_crash(self):
        # The shell's answer to a command it cannot find (127), which the gates refuse as the machine's.
        code, out = gate.run(["cityloom-no-such-tool"], self.repo)
        self.assertEqual(code, 127)
        self.assertIn("cityloom-no-such-tool", out)

    def test_the_report_names_the_gate_and_each_page(self):
        report = gate.Report("docs", pages=[f"{EN}/signals.md"])
        self.assertEqual(report.text(), f"PASS docs gate\n  page: {EN}/signals.md")


class TestCode(unittest.TestCase):
    """What the tests flow counts as test code (rule 1) and as an existing test (rule 2)."""

    def test_only_items_built_for_tests_alone_are_test_code(self):
        src = (
            "pub fn a() {}\n"
            "#[cfg(test)]\nmod tests { fn t() {} }\n"
            "#[cfg(all(test, not(target_arch = \"wasm32\")))]\nmod native { fn n() {} }\n"
            '#[cfg(any(test, feature = "test-provider"))]\npub mod fake { pub fn f() {} }\n'
            "#[cfg(not(test))]\nfn real() {}\n"
            "#[cfg(all(not(test), unix))]\nfn unix() {}\n"
        )
        kept = gate.without_test_code(src)
        self.assertNotIn("mod tests", kept)
        self.assertNotIn("mod native", kept)
        self.assertIn("pub mod fake", kept)
        self.assertIn("fn real", kept)
        self.assertIn("fn unix", kept)

    def test_each_test_function_with_its_attributes(self):
        src = (
            "#[cfg(test)]\nmod tests {\n"
            "    #[test]\n    fn one() { assert!(true); }\n\n"
            "    /// Slow.\n    #[ignore]\n    #[test]\n    fn two() {}\n\n"
            "    #[sqlx::test]\n    async fn three(pool: PgPool) { let _ = pool; }\n"
            "    fn helper() {}\n}\n"
        )
        self.assertEqual(
            gate.test_functions(src),
            [
                ("one", "#[test]\n    fn one() { assert!(true); }"),
                ("two", "/// Slow.\n    #[ignore]\n    #[test]\n    fn two() {}"),
                ("three", "#[sqlx::test]\n    async fn three(pool: PgPool) { let _ = pool; }"),
            ],
        )

    def test_each_playwright_test_by_its_title(self):
        src = (
            'import { test, expect } from "./fixtures";\n'
            'test.describe("Street", () => {\n'
            '  test("adds a lane", async ({ page }) => {\n    await expect(page).toHaveTitle("a (b)");\n  });\n'
            "  test.skip('drops a lane', async () => {});\n"
            "  // test(\"commented\", () => {});\n"
            "});\n"
            "test(`opens`, async () => { await go(\")\"); });\n"
        )
        self.assertEqual(
            [title for title, _ in gate.playwright_tests(src)],
            ["adds a lane", "drops a lane", "opens"],
        )
        self.assertTrue(gate.playwright_tests(src)[1][1].startswith("test.skip('drops a lane'"))
        self.assertTrue(gate.playwright_tests(src)[2][1].endswith('await go(")"); })'))

    def test_only_the_editors_version_line_may_change(self):
        toml = '[package]\nname = "cityloom_editor"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }\n'
        lock = '[[package]]\nname = "cityloom_editor"\nversion = "0.1.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.0"\n'
        path, lockfile = "crates/cityloom_editor/Cargo.toml", "Cargo.lock"
        self.assertTrue(gate.version_only(path, toml, toml.replace('"0.1.0"', '"0.1.1"')))
        self.assertFalse(gate.version_only(path, toml, toml.replace('"1"', '"2"')))
        self.assertTrue(gate.version_only(lockfile, lock, lock.replace('"0.1.0"', '"0.1.1"')))
        self.assertFalse(gate.version_only(lockfile, lock, lock.replace('"1.0.0"', '"1.0.1"')))

    def test_a_test_binary_is_labelled_as_cargo_test_names_it(self):
        artifact = {
            "reason": "compiler-artifact",
            "package_id": "path+file:///w/crates/cityloom_model#0.1.0",
            "manifest_path": "/w/crates/cityloom_model/Cargo.toml",
            "target": {"kind": ["test"], "name": "import", "src_path": "/w/crates/cityloom_model/tests/import.rs"},
            "profile": {"test": True},
            "executable": "/w/target/debug/deps/import-89abcdef01234567",
        }
        self.assertEqual(gate.test_binaries(json.dumps(artifact) + "\n")["import tests/import.rs"].package, "cityloom_model")
        artifact.update(
            package_id="path+file:///w/crates/x#cityloom_ui@0.2.0",
            manifest_path="/w/crates/x/Cargo.toml",
            target={"kind": ["lib"], "name": "cityloom_ui", "src_path": "/w/crates/x/src/lib.rs"},
            executable="/w/target/debug/deps/cityloom_ui-0123456789abcdef",
        )
        binary = gate.test_binaries("noise\n" + json.dumps(artifact) + "\n")["cityloom_ui unittests src/lib.rs"]
        self.assertEqual((binary.package, binary.cwd), ("cityloom_ui", "/w/crates/x"))


PRODUCT = """\
pub fn word() -> usize {
    2
}

pub fn other() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps() {}
}
"""

NEW_TEST = "    #[test]\n    fn keeps() {}\n\n    #[test]\n    fn word_is_two() { assert_eq!(super::word(), 2); }\n"

MUTANTS = [
    {
        "file": "crates/m/src/street.rs",
        "package": "m",
        "function": {"function_name": "word"},
        "name": f"crates/m/src/street.rs:2:5: replace word -> usize with {value}",
        "replacement": value,
        "span": {"start": {"line": 2, "column": 5}, "end": {"line": 2, "column": 6}},
    }
    for value in ("0", "1")
] + [
    {
        "file": "crates/m/src/street.rs",
        "package": "m",
        "function": {"function_name": "other"},
        "name": "crates/m/src/street.rs:6:5: replace other -> bool with false",
        "replacement": "false",
        "span": {"start": {"line": 6, "column": 5}, "end": {"line": 6, "column": 9}},
    }
]

BINARY = "     Running unittests src/lib.rs (deps/m-0123456789abcdef)\n"
EXE = "target/debug/deps/m-0123456789abcdef"


def word_is_two(repo: Path) -> str:
    return "ok" if "pub fn word() -> usize {\n    2\n}" in (repo / "crates/m/src/street.rs").read_text() else "failed"


class TestsRunner:
    """Answers what the tests gate runs. cargo's and Playwright's listings and full runs come from tables by commit;
    a test run alone, at any time, is judged by `outcomes` from the files as they are then, so a fault the gate
    applies really changes what the test sees. git runs for real."""

    def __init__(self, repo: Path, **tables):
        self.repo = repo
        self.version = tables.get("version", "cargo-mutants 27.1.0")
        self.mutants = tables.get("mutants", MUTANTS)
        self.listings = tables.get("listings", {})
        self.runs = tables.get("runs", {})
        self.outcomes = tables.get("outcomes", {"tests::word_is_two": word_is_two})
        self.playwright = tables.get("playwright", {})
        self.browser = tables.get("browser", {})
        self.commands: list[tuple[str, ...]] = []
        self.envs: list[dict] = []
        self.timeouts: list = []
        self.pkg_seen: list[str] = []

    def __call__(self, cmd, cwd, env=None, timeout=None):
        self.commands.append(tuple(cmd))
        self.envs.append(env or {})
        self.timeouts.append(timeout)
        head = git(self.repo, "rev-parse", "HEAD")
        product = (self.repo / "crates/m/src/street.rs").read_text()
        if cmd[0].endswith("cargo-mutants"):
            if "--version" in cmd:
                return 0, self.version
            files = [cmd[i + 1] for i, a in enumerate(cmd) if a == "--file"]
            return 0, json.dumps([m for m in self.mutants if m["file"] in files])
        if cmd[:2] == ["cargo", "test"] and "--no-run" in cmd:
            if "NOT RUST" in product:
                return 101, "error[E0308]: mismatched types\nerror: could not compile `m` (lib test)\n"
            artifact = {
                "reason": "compiler-artifact",
                "package_id": f"path+file://{self.repo}/crates/m#0.1.0",
                "manifest_path": str(self.repo / "crates/m/Cargo.toml"),
                "target": {"kind": ["lib"], "name": "m", "src_path": str(self.repo / "crates/m/src/lib.rs")},
                "profile": {"test": True},
                "executable": str(self.repo / EXE),
            }
            return 0, json.dumps(artifact) + "\n"
        if cmd[:2] == ["cargo", "test"]:
            if "--list" in cmd:
                return 0, self.listings[head]
            return self.runs[head]
        if cmd[0].endswith(EXE):
            outcome = self.outcomes[cmd[1]](self.repo)
            if outcome == "hang":
                return gate.TIMED_OUT, "running 1 test\n"
            if outcome == "ok":
                return 0, f"running 1 test\ntest {cmd[1]} ... ok\n\ntest result: ok. 1 passed; 0 failed\n"
            return 101, f"running 1 test\ntest {cmd[1]} ... FAILED\npanicked at src/street.rs\ntest result: FAILED. 0 passed; 1 failed\n"
        if cmd[0] == "scripts/target-dir.sh":
            return 0, str(self.repo / "target") + "\n"
        if cmd[0].endswith("wasm-bindgen"):
            (self.repo / "web/pkg/cityloom_editor.js").write_text("level 1\n")
            return 0, ""
        if cmd[:3] == ["npx", "playwright", "test"]:
            if "--list" in cmd:
                report = self.playwright[(head, True)]
            elif any(":" in a and a.startswith("tests/") for a in cmd):
                self.pkg_seen.append((self.repo / "web/pkg/cityloom_editor.js").read_text())
                report = playwright_report(*[(f, t, judge(self.repo)) for f, t, judge in self.browser[head]])
            else:
                report = self.playwright[(head, False)]
            Path(env["PLAYWRIGHT_JSON_OUTPUT_FILE"]).write_text(json.dumps(report), encoding="utf-8")
            failed = any(s == "unexpected" for s in gate.playwright_results(report).values())
            return (1 if failed else 0), ""
        return 0, ""

    def exe_runs(self) -> list[tuple[str, ...]]:
        return [c for c in self.commands if c[0].endswith(EXE)]


class FakeClock:
    def __init__(self, step: float):
        self.now, self.step = 0.0, step

    def __call__(self) -> float:
        self.now += self.step
        return self.now


class TestsGate(unittest.TestCase):
    """The tests flow's gate: a branch that adds only tests, every one passing, each catching a fault of the code the
    plan names. cargo, cargo-mutants and Playwright are faked; git and the faults are real."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name) / "wt"
        self.repo.mkdir()
        git(self.repo, "init", "-q", "-b", "work")
        self.base = commit(
            self.repo,
            {
                ".gitignore": "web/pkg/\ntarget/\n",
                "crates/m/src/street.rs": PRODUCT,
                "crates/m/Cargo.toml": '[package]\nname = "m"\nversion = "0.1.0"\n',
                "crates/cityloom_editor/Cargo.toml": '[package]\nname = "cityloom_editor"\nversion = "0.1.0"\n',
                "Cargo.lock": '[[package]]\nname = "cityloom_editor"\nversion = "0.1.0"\n\n[[package]]\nname = "m"\nversion = "0.1.0"\n',
                "web/app.css": ":root {\n  --panel: #fff;\n}\n",
                "e2e/tests/street.spec.ts": 'test("adds a lane", async () => {});\n',
                "e2e/tests/fixtures.ts": "export const a = 1;\n",
            },
            "base",
        )
        self.spec = Path(self.tmp.name) / "faults.json"
        self.write_spec({"rust": [{"file": "crates/m/src/street.rs", "functions": ["word", "other"]}]})
        os.environ.setdefault("DATABASE_URL", "postgres://unused")

    def tearDown(self):
        self.tmp.cleanup()

    def write_spec(self, spec: dict) -> None:
        self.spec.write_text(json.dumps(spec), encoding="utf-8")

    def raise_version(self) -> dict[str, str]:
        return {
            "crates/cityloom_editor/Cargo.toml": '[package]\nname = "cityloom_editor"\nversion = "0.1.1"\n',
            "Cargo.lock": '[[package]]\nname = "cityloom_editor"\nversion = "0.1.1"\n\n[[package]]\nname = "m"\nversion = "0.1.0"\n',
        }

    def tests_commit(self, extra: dict[str, str] | None = None, test: str = NEW_TEST) -> str:
        files = {"crates/m/src/street.rs": PRODUCT.replace("    #[test]\n    fn keeps() {}\n", test)}
        files.update(self.raise_version())
        files.update(extra or {})
        return commit(self.repo, files, "Test the word")

    def runner(self, head: str, **tables) -> TestsRunner:
        listing = BINARY + "tests::keeps: test\n"
        tables.setdefault("listings", {self.base: listing, head: listing + "tests::word_is_two: test\n"})
        tables.setdefault("runs", {head: (0, BINARY + "test tests::keeps ... ok\ntest tests::word_is_two ... ok\n")})
        return TestsRunner(self.repo, **tables)

    def gate(self, runner, clock=None) -> gate.Gate:
        return gate.Gate(self.repo, runner, clock=clock or FakeClock(1.0))

    def test_a_new_test_that_catches_a_fault_passes(self):
        head = self.tests_commit()
        runner = self.runner(head)
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, [])
        self.assertEqual(report.new_tests, ["m unittests src/lib.rs::tests::word_is_two"])
        text = report.text()
        self.assertIn("caught: m unittests src/lib.rs::tests::word_is_two: 2 faults", text)
        self.assertIn("crates/m/src/street.rs: replace word -> usize with 0", text)
        self.assertIn("not caught by any new test: crates/m/src/street.rs: replace other -> bool with false", text)
        # Rule 4 builds as CI does; the faults are built without denying warnings (a fault leaves a parameter unused).
        full = [e for c, e in zip(runner.commands, runner.envs) if c[:2] == ("cargo", "test") and "--no-fail-fast" in c]
        self.assertTrue(full and all(e.get("RUSTFLAGS") == "-D warnings" for e in full))
        self.assertIn(("just", "format-check"), runner.commands)
        self.assertIn(("scripts/check-version.sh", self.base), runner.commands)
        self.assertIn(("just", "test-scripts"), runner.commands)
        # Each new test runs alone, by its full name, in its own process: once with no fault, then once per fault.
        self.assertEqual(len(runner.exe_runs()), 4)
        self.assertEqual(runner.exe_runs()[0][1:], ("tests::word_is_two", "--exact", "--test-threads=1"))
        # Everything is put back.
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")
        self.assertEqual((self.repo / "crates/m/src/street.rs").read_text(), PRODUCT.replace("    #[test]\n    fn keeps() {}\n", NEW_TEST))
        self.assertFalse(gate.Gate(self.repo).note_path().exists())

    def test_a_new_test_that_catches_nothing_fails(self):
        head = self.tests_commit()
        runner = self.runner(head, outcomes={"tests::word_is_two": lambda repo: "ok"})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["the new test m unittests src/lib.rs::tests::word_is_two catches none of the 3 faults run"])

    def test_the_brief_report_names_the_tests_and_no_fault(self):
        head = self.tests_commit()
        runner = self.runner(head, outcomes={"tests::word_is_two": lambda repo: "ok"})
        report = self.gate(runner).tests(self.base, self.spec, brief=True)
        self.assertEqual(
            report.text(),
            "FAIL tests gate\n"
            "  new test: m unittests src/lib.rs::tests::word_is_two\n"
            "  caught nothing: m unittests src/lib.rs::tests::word_is_two\n"
            "  problem: the new test m unittests src/lib.rs::tests::word_is_two catches none of the 3 faults run",
        )
        self.assertNotIn("replace", report.text())

    def test_a_fault_that_does_not_compile_is_counted_and_dropped(self):
        head = self.tests_commit()
        mutants = MUTANTS + [dict(MUTANTS[0], name="crates/m/src/street.rs:2:5: replace word -> usize with NOT RUST", replacement="NOT RUST")]
        runner = self.runner(head, mutants=mutants)
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, [])
        self.assertIn("did not compile: crates/m/src/street.rs: replace word -> usize with NOT RUST", report.text())

    def test_a_fault_that_hangs_a_test_is_hung_and_caught_by_no_one(self):
        head = self.tests_commit()
        hang = lambda repo: "hang" if "    1\n" in (repo / "crates/m/src/street.rs").read_text() else word_is_two(repo)  # noqa: E731
        runner = self.runner(head, outcomes={"tests::word_is_two": hang})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, [])
        self.assertIn("hung: crates/m/src/street.rs: replace word -> usize with 1 (m unittests src/lib.rs::tests::word_is_two)", report.text())
        self.assertIn("caught: m unittests src/lib.rs::tests::word_is_two: 1 fault", report.text())
        # Under a fault, a test is killed after a minute, or four times its own time if that is longer.
        timeouts = [t for c, t in zip(runner.commands, runner.timeouts) if c[0].endswith(EXE)]
        self.assertEqual(set(timeouts[1:]), {60})

    def test_a_test_that_caught_three_is_not_run_again(self):
        head = self.tests_commit()
        five = [dict(MUTANTS[0], name=f"crates/m/src/street.rs:2:5: replace word -> usize with {v}", replacement=str(v)) for v in range(3, 8)]
        runner = self.runner(head, mutants=five)
        self.write_spec({"rust": [{"file": "crates/m/src/street.rs", "functions": ["word"]}]})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(len(runner.exe_runs()), 1 + 3)
        self.assertIn("caught: m unittests src/lib.rs::tests::word_is_two: 3 faults", report.text())
        self.assertIn("not run, every new test had caught three: 2 faults", report.text())

    def test_at_most_forty_faults_run_for_native_tests(self):
        head = self.tests_commit()
        many = [dict(MUTANTS[2], name=f"crates/m/src/street.rs:6:5: replace other -> bool with false /* {k} */", replacement=f"false /* {k} */") for k in range(45)]
        runner = self.runner(head, mutants=many)
        self.write_spec({"rust": [{"file": "crates/m/src/street.rs", "functions": ["other"]}]})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(len(runner.exe_runs()), 1 + 40)
        self.assertIn("left out by the limit: 5 faults", report.text())

    def test_no_fault_starts_thirty_minutes_after_the_first(self):
        head = self.tests_commit()
        runner = self.runner(head)
        report = self.gate(runner, clock=FakeClock(1000.0)).tests(self.base, self.spec)
        self.assertIn("left out by the limit:", report.text())
        self.assertLess(len(runner.exe_runs()), 4)

    def test_a_change_to_product_code_is_refused_before_any_fault(self):
        head = self.tests_commit({"crates/m/src/other.rs": "pub fn x() {}\n"})
        runner = self.runner(head)
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["crates/m/src/other.rs: not a test, and the tests flow changes only tests"])
        self.assertEqual(runner.exe_runs(), [])

    def test_product_code_beside_new_tests_in_one_file_is_refused(self):
        head = commit(
            self.repo,
            dict(self.raise_version(), **{"crates/m/src/street.rs": PRODUCT.replace("    2\n", "    3 - 1\n").replace("    #[test]\n    fn keeps() {}\n", NEW_TEST)}),
            "Also change the code",
        )
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["crates/m/src/street.rs: product code changed, and only test code (#[cfg(test)]) may"])

    def test_a_test_only_module_in_a_file_of_its_own_is_test_code(self):
        module = {
            "crates/m/src/lib.rs": "pub mod street;\n#[cfg(test)]\nmod street_cases;\n",
            "crates/m/src/street_cases.rs": "#[test]\nfn more() {}\n",
        }
        head = self.tests_commit(module)
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        # A new lib.rs is product code; the module it declares for tests alone is not.
        self.assertEqual(report.problems, ["crates/m/src/lib.rs: not a test, and the tests flow changes only tests"])
        git(self.repo, "reset", "-q", "--hard", self.base)
        self.base = commit(self.repo, {"crates/m/src/lib.rs": "pub mod street;\n"}, "a lib")
        head = self.tests_commit(module)
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, [])

    def test_more_than_the_version_line_of_the_lockfile_is_refused(self):
        head = self.tests_commit({"Cargo.lock": '[[package]]\nname = "cityloom_editor"\nversion = "0.1.1"\n\n[[package]]\nname = "m"\nversion = "0.2.0"\n'})
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["Cargo.lock: changes more than the version line of cityloom_editor"])

    def test_checks_the_factory_and_documentation_are_refused_whatever_matches_a_test_file(self):
        head = self.tests_commit(
            {
                ".claude/factory/tests/x.py": "x\n",
                "scripts/ci/features.json": "{}\n",
                "scripts/ci/spec-map.json": "{}\n",
                "e2e/tiers.json": "{}\n",
                "crates/cityloom_editor/src/shared/contrast.rs": "#[test]\nfn t() {}\n",
                "crates/cityloom_editor/src/i18n_guard.rs": "\n",
                "website/docs/testing.md": "# x\n",
            }
        )
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(
            report.problems,
            [
                ".claude/factory/tests/x.py: a check or the factory's own file, which the tests flow may not change",
                "crates/cityloom_editor/src/i18n_guard.rs: a check or the factory's own file, which the tests flow may not change",
                "website/docs/testing.md: documentation, which the tests flow may not change",
            ],
        )

    def test_a_helper_of_the_specs_may_only_gain_lines(self):
        head = self.tests_commit({"e2e/tests/fixtures.ts": "export const a = 2;\n"})
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["e2e/tests/fixtures.ts: a helper of the specs, which may gain lines but not change one"])

    def test_an_existing_test_changed_or_gone_is_refused(self):
        head = self.tests_commit(test="    #[test]\n    fn keeps() { assert!(true); }\n\n    #[test]\n    fn word_is_two() { assert_eq!(super::word(), 2); }\n")
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["crates/m/src/street.rs: the existing test keeps is changed or gone"])

    def test_an_existing_browser_test_skipped_is_refused(self):
        head = self.tests_commit({"e2e/tests/street.spec.ts": 'test.skip("adds a lane", async () => {});\n'})
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["e2e/tests/street.spec.ts: the existing test adds a lane is changed or gone"])

    def test_a_branch_with_no_new_test_fails(self):
        head = commit(self.repo, self.raise_version(), "Only the version")
        listing = BINARY + "tests::keeps: test\n"
        runner = TestsRunner(self.repo, listings={self.base: listing, head: listing}, runs={head: (0, BINARY + "test tests::keeps ... ok\n")})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["the branch adds no test"])

    def test_a_new_test_that_fails_at_the_head_fails(self):
        head = self.tests_commit()
        runner = self.runner(head, runs={head: (101, BINARY + "test tests::keeps ... ok\ntest tests::word_is_two ... FAILED\n")})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["Rust: m unittests src/lib.rs::tests::word_is_two fails"])
        self.assertEqual(runner.exe_runs(), [])

    def test_a_new_test_that_is_ignored_fails(self):
        head = self.tests_commit()
        runner = self.runner(head, runs={head: (0, BINARY + "test tests::keeps ... ok\ntest tests::word_is_two ... ignored\n")})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["Rust: the new test m unittests src/lib.rs::tests::word_is_two is ignored"])

    def test_a_new_test_that_fails_alone_fails(self):
        head = self.tests_commit()
        runner = self.runner(head, outcomes={"tests::word_is_two": lambda repo: "failed"})
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["the new test m unittests src/lib.rs::tests::word_is_two does not pass when it runs alone"])

    def test_the_plans_faults_must_be_there(self):
        head = self.tests_commit()
        self.write_spec({"rust": [{"file": "crates/m/src/street.rs", "functions": ["gone"]}]})
        report = self.gate(self.runner(head)).tests(self.base, self.spec)
        self.assertEqual(report.problems, ["the plan's faults: rust crates/m/src/street.rs: no function `gone`"])

    def test_without_the_pinned_cargo_mutants_it_refuses(self):
        head = self.tests_commit()
        for version in ("cargo-mutants 26.0.0", ""):
            with self.subTest(version):
                with self.assertRaises(gate.Refused) as caught:
                    self.gate(self.runner(head, version=version)).tests(self.base, self.spec)
                self.assertIn("cargo install cargo-mutants --version 27.1.0", str(caught.exception))

    def test_a_note_left_by_a_killed_gate_is_refused_naming_its_fault(self):
        head = self.tests_commit()
        g = self.gate(self.runner(head))
        g.note_path().write_text(json.dumps({"head": head, "fault": "crates/m/src/street.rs", "pkg": None}))
        with self.assertRaises(gate.Refused) as caught:
            g.tests(self.base, self.spec)
        self.assertIn("crates/m/src/street.rs", str(caught.exception))
        self.assertIn("just build", str(caught.exception))

    def rebase(self, change: dict[str, str]) -> str:
        """main moves on by `change`, and the branch is replayed on it."""
        git(self.repo, "checkout", "-q", "-b", "main", self.base)
        new_base = commit(self.repo, change, "main moves")
        git(self.repo, "checkout", "-q", "work")
        git(self.repo, "rebase", "-q", new_base)
        self.base = new_base
        return git(self.repo, "rev-parse", "HEAD")

    def test_after_a_rebase_that_changes_nothing_it_rests_on_the_proof_is_kept(self):
        proved = self.tests_commit()
        head = self.rebase({"README.md": "# CityLoom\n"})
        runner = self.runner(head)
        report = self.gate(runner).tests(self.base, self.spec, proved=proved)
        self.assertEqual(report.problems, [])
        self.assertIn(f"the faults were run at {proved}", report.text())
        self.assertEqual(len(runner.exe_runs()), 1)  # the new test alone, with no fault
        self.assertFalse([c for c in runner.commands if c[0].endswith("cargo-mutants") and "--list" in c])

    def test_after_a_rebase_that_changes_a_faulted_file_the_faults_run_again(self):
        proved = self.tests_commit()
        head = self.rebase({"crates/m/src/street.rs": PRODUCT.replace("    true\n", "    !false\n")})
        mutants = [dict(m, span={"start": {"line": 6, "column": 5}, "end": {"line": 6, "column": 11}}) if m["function"]["function_name"] == "other" else m for m in MUTANTS]
        runner = self.runner(head, mutants=mutants)
        report = self.gate(runner).tests(self.base, self.spec, proved=proved)
        self.assertEqual(report.problems, [])
        self.assertIn("the faults run again: crates/m/src/street.rs changed between the two bases", report.text())
        self.assertEqual(len(runner.exe_runs()), 4)

    def browser_case(self, head: str, judge) -> TestsRunner:
        listing = BINARY + "tests::keeps: test\n"
        lane = ("street.spec.ts", ["adds a lane"], "expected")
        new = ("street.spec.ts", ["names the street"], "expected")
        listed = playwright_report(lane, new)
        listed["suites"][0]["specs"][1]["line"] = 2
        return TestsRunner(
            self.repo,
            listings={self.base: listing, head: listing},
            runs={head: (0, BINARY + "test tests::keeps ... ok\n")},
            playwright={(self.base, True): playwright_report(lane), (head, True): listed, (head, False): playwright_report(lane, new)},
            browser={head: [("street.spec.ts", ["names the street"], judge)]},
        )

    def browser_commit(self) -> str:
        head = self.tests_commit(
            {"e2e/tests/street.spec.ts": 'test("adds a lane", async () => {});\ntest("names the street", async () => {});\n'},
            test="    #[test]\n    fn keeps() {}\n",
        )
        self.write_spec({"css": [{"file": "web/app.css", "rule": ":root", "property": "--panel", "to": "#000"}]})
        (self.repo / "web/pkg").mkdir(parents=True)
        (self.repo / "web/pkg/cityloom_editor.js").write_text("release\n")
        return head

    def test_a_new_browser_test_is_proved_at_optimisation_level_one_and_the_build_put_back(self):
        head = self.browser_commit()
        judge = lambda repo: "expected" if "--panel: #fff" in (repo / "web/app.css").read_text() else "unexpected"  # noqa: E731
        runner = self.browser_case(head, judge)
        report = self.gate(runner).tests(self.base, self.spec)
        self.assertEqual(report.problems, [])
        self.assertIn("caught: street.spec.ts::names the street::ui: 1 fault", report.text())
        runs = [c for c in runner.commands if c[:3] == ("npx", "playwright", "test") and "tests/street.spec.ts:2" in c]
        self.assertEqual(len(runs), 2)  # no fault, then the CSS fault (which needs no build)
        self.assertIn("--project=ui", runs[0])
        self.assertIn("--timeout=30000", runs[0])
        self.assertIn("--global-timeout=300000", runs[0])
        builds = [(c, e) for c, e in zip(runner.commands, runner.envs) if c[:2] == ("cargo", "build") and "-p" in c]
        self.assertEqual(len(builds), 1)
        self.assertEqual(builds[0][1].get("CARGO_PROFILE_DEV_OPT_LEVEL"), "1")
        self.assertEqual(builds[0][1].get("CARGO_PROFILE_DEV_DEBUG"), "0")
        self.assertEqual(runner.pkg_seen, ["level 1\n", "level 1\n"])
        self.assertEqual((self.repo / "web/pkg/cityloom_editor.js").read_text(), "release\n")
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_a_new_browser_test_that_fails_at_level_one_with_no_fault_is_refused(self):
        head = self.browser_commit()
        with self.assertRaises(gate.Refused) as caught:
            self.gate(self.browser_case(head, lambda repo: "unexpected")).tests(self.base, self.spec)
        self.assertIn("optimisation level 1", str(caught.exception))
        self.assertEqual((self.repo / "web/pkg/cityloom_editor.js").read_text(), "release\n")
        self.assertFalse(gate.Gate(self.repo).note_path().exists())


class FaultsCheck(unittest.TestCase):
    """`gate.py faults`: the plan's faults checked against the main checkout before the plan is posted. Builds nothing."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name) / "main"
        self.repo.mkdir()
        git(self.repo, "init", "-q")
        commit(
            self.repo,
            {
                "crates/m/src/street.rs": PRODUCT,
                "crates/citymodel_builder/src/a.rs": "pub fn cut() -> usize {\n    2\n}\n",
                "web/app.css": ":root {\n  --panel: #fff;\n}\n",
            },
            "base",
        )
        self.spec = Path(self.tmp.name) / "faults.json"
        builder = dict(MUTANTS[0], file="crates/citymodel_builder/src/a.rs", function={"function_name": "cut"}, package="citymodel_builder")
        self.runner = TestsRunner(self.repo, mutants=MUTANTS + [builder])

    def tearDown(self):
        self.tmp.cleanup()

    def check(self, spec: dict, browser: bool = False) -> gate.Report:
        self.spec.write_text(json.dumps(spec))
        return gate.Gate(self.repo, self.runner).faults(self.spec, browser=browser)

    def test_each_entry_with_its_faults(self):
        report = self.check(
            {
                "rust": [{"file": "crates/m/src/street.rs", "functions": ["word", "other"]}],
                "css": [{"file": "web/app.css", "rule": ":root", "property": "--panel", "to": "#000"}],
            }
        )
        self.assertEqual(report.problems, [])
        self.assertEqual(
            report.text(),
            "PASS faults gate\n"
            "  entry: rust crates/m/src/street.rs word: 2 faults\n"
            "  entry: rust crates/m/src/street.rs other: 1 fault\n"
            "  entry: css web/app.css :root { --panel }: 1 fault",
        )
        self.assertFalse([c for c in self.runner.commands if c[0] == "cargo"])

    def test_what_is_not_there_fails(self):
        report = self.check(
            {
                "rust": [{"file": "crates/m/src/street.rs", "functions": ["gone"]}],
                "markup": [{"file": "crates/m/src/view.rs", "functions": ["f"]}],
            }
        )
        self.assertEqual(report.problems, ["rust crates/m/src/street.rs: no function `gone`", "markup crates/m/src/view.rs: the file does not exist"])

    def test_more_entries_than_the_limit_fail(self):
        names = [f"--c{k}" for k in range(41)]
        (self.repo / "web/app.css").write_text(":root {\n" + "".join(f"  {n}: #fff;\n" for n in names) + "}\n")
        spec = {"css": [{"file": "web/app.css", "rule": ":root", "property": n, "to": "#000"} for n in names]}
        self.assertEqual(self.check(spec).problems, ["41 entries: more than the 40 faults a native test can be proved against"])
        spec["css"] = spec["css"][:13]
        self.assertEqual(self.check(spec, browser=True).problems, ["13 entries: more than the 12 faults a browser test can be proved against"])

    def test_a_browser_test_cannot_be_proved_against_the_server_or_the_builder(self):
        report = self.check({"rust": [{"file": "crates/citymodel_builder/src/a.rs", "functions": ["cut"]}]}, browser=True)
        self.assertEqual(
            report.problems,
            ["rust crates/citymodel_builder/src/a.rs: citymodel_builder does not reach the browser, so a browser test cannot catch its faults"],
        )

    def test_a_plan_with_rust_faults_needs_the_pinned_cargo_mutants(self):
        self.runner.version = "cargo-mutants 26.0.0"
        with self.assertRaises(gate.Refused):
            self.check({"rust": [{"file": "crates/m/src/street.rs", "functions": ["word"]}]})
        self.assertEqual(self.check({"css": [{"file": "web/app.css", "rule": ":root", "property": "--panel", "to": "#000"}]}).problems, [])

    def test_a_malformed_spec_is_refused(self):
        self.spec.write_text("{}")
        with self.assertRaises(gate.Refused):
            gate.Gate(self.repo, self.runner).faults(self.spec)


class Commands(unittest.TestCase):
    def test_the_documentation_commands_are_the_workflows(self):
        workflow = (ROOT / ".github" / "workflows" / "docs.yml").read_text(encoding="utf-8")
        self.assertIn("run: " + " ".join(gate.DOCS_PRETTIER) + "\n        working-directory: e2e", workflow)
        self.assertIn("run: " + " ".join(gate.DOCS_BUILD) + "\n        working-directory: website", workflow)

    def test_cargo_mutants_is_the_main_checkouts_from_any_worktree(self):
        common = subprocess.run(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=ROOT, check=True, capture_output=True, text=True
        ).stdout.strip()
        self.assertEqual(gate.MUTANTS, Path(common).parent / ".tools" / "bin" / "cargo-mutants")

    def test_the_rust_commands_are_the_justfiles(self):
        justfile = (ROOT / "justfile").read_text(encoding="utf-8")
        for command in (gate.CARGO_TEST, gate.WASM_BUILD):
            with self.subTest(command):
                self.assertIn(" ".join(command), justfile)


if __name__ == "__main__":
    unittest.main()
