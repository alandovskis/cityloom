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
        with self.assertRaises(gate.Refused):
            gate.Gate(self.repo, self.runner()).docs(self.base)

    def test_a_dirty_worktree_is_refused(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Both")
        (self.repo / "loose.md").write_text("x", encoding="utf-8")
        with self.assertRaises(gate.Refused):
            gate.Gate(self.repo, self.runner()).docs(self.base)

    def test_a_site_not_set_up_is_the_environment_not_the_change(self):
        commit(self.repo, {f"{EN}/signals.md": "# Signals\n", f"{FR}/signals.md": "# Feux\n"}, "Both")
        run = self.runner({gate.DOCS_BUILD: (127, "sh: docusaurus: command not found")})
        with self.assertRaises(gate.Refused) as caught:
            gate.Gate(self.repo, run).docs(self.base)
        self.assertIn("just setup-docs", str(caught.exception))

    def test_a_tool_not_on_the_path_is_refused_not_a_crash(self):
        # The shell's answer to a command it cannot find (127), which the gates refuse as the machine's.
        code, out = gate.run(["cityloom-no-such-tool"], self.repo)
        self.assertEqual(code, 127)
        self.assertIn("cityloom-no-such-tool", out)

    def test_the_report_names_the_gate_and_each_page(self):
        report = gate.Report("docs", pages=[f"{EN}/signals.md"])
        self.assertEqual(report.text(), f"PASS docs gate\n  page: {EN}/signals.md")


class Commands(unittest.TestCase):
    def test_the_documentation_commands_are_the_workflows(self):
        workflow = (ROOT / ".github" / "workflows" / "docs.yml").read_text(encoding="utf-8")
        self.assertIn("run: " + " ".join(gate.DOCS_PRETTIER) + "\n        working-directory: e2e", workflow)
        self.assertIn("run: " + " ".join(gate.DOCS_BUILD) + "\n        working-directory: website", workflow)

    def test_the_rust_commands_are_the_justfiles(self):
        justfile = (ROOT / "justfile").read_text(encoding="utf-8")
        for command in (gate.CARGO_TEST, gate.WASM_BUILD):
            with self.subTest(command):
                self.assertIn(" ".join(command), justfile)


if __name__ == "__main__":
    unittest.main()
