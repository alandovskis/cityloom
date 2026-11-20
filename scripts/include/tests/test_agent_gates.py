"""The agents' red and green gates and the implementer's hook (.claude/gates, .claude/hooks): `just test-scripts`.

The test-writer agent writes the tests and the implement agent may not change them. These tests pin what counts as a
test (the protected files and the `#[cfg(test)]` items of any other Rust file), how a new test is found in a diff and
in cargo's output, and what the hook lets through.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / ".claude" / "gates"))

import runner  # noqa: E402
import test_surface as ts  # noqa: E402

HOOK = ROOT / ".claude" / "hooks" / "protect_tests.py"

SOURCE = """\
pub fn open(s: &str) -> char { '{' }
fn life<'a>(x: &'a str) -> &'a str { x }
const RAW: &str = r#"}{"#;
#[cfg(not(test))]
fn production() {}
#[cfg(feature = "test-provider")]
fn fake() {}
pub struct S {
    #[cfg(test)]
    seen: u8,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opens() {
        assert_eq!(open("}"), '{'); // a } in a comment
    }
}
"""


class TestSurface(unittest.TestCase):
    def test_protected_files(self) -> None:
        for path in [
            "crates/cityloom_editor/src/street/view/tests.rs",
            "crates/citymodel_builder/tests/tiles.rs",
            "e2e/tests/street.spec.ts",
            "e2e/tiers.json",
            "scripts/ci/spec-map.json",
            ".github/workflows/ci.yml",
            "crates/cityloom_editor/src/i18n_guard.rs",
        ]:
            self.assertTrue(ts.protected(path), path)
        for path in ["crates/cityloom_editor/src/street/model/mod.rs", "web/app.js", "Cargo.toml"]:
            self.assertFalse(ts.protected(path), path)

    def test_the_cfg_test_items_and_only_those(self) -> None:
        items = ts.test_items(SOURCE)
        self.assertEqual(len(items), 2)
        self.assertEqual(items[0], "#[cfg(test)]\n    seen: u8,\n")
        self.assertTrue(items[1].startswith("#[cfg(test)]\nmod tests {"))
        self.assertTrue(items[1].endswith("}\n}"))

    def test_an_edit_outside_the_tests_does_not_touch_them(self) -> None:
        at = SOURCE.index("fn life")
        self.assertFalse(ts.touches_tests(SOURCE, at, at + 10))
        at = SOURCE.index("assert_eq")
        self.assertTrue(ts.touches_tests(SOURCE, at, at + 10))

    def test_the_tests_a_diff_adds(self) -> None:
        diff = """\
diff --git a/crates/cityloom_editor/src/street/model/mod.rs b/crates/cityloom_editor/src/street/model/mod.rs
+++ b/crates/cityloom_editor/src/street/model/mod.rs
@@ -10,0 +11,9 @@
+    #[test]
+    // why
+    fn a_new_rule() {}
+    fn helper() {}
+    #[tokio::test]
+    async fn an_async_one() {}
+++ b/crates/citymodel_builder/tests/tiles.rs
@@ -1,0 +2,2 @@
+#[test]
+fn cuts_the_tile() {}
"""
        found = ts.added_tests(diff)
        self.assertEqual([t.name for t in found], ["a_new_rule", "an_async_one", "cuts_the_tile"])
        self.assertEqual(found[0].module(), "street::model")
        self.assertIsNone(found[2].module())

    def test_a_new_test_matches_its_own_module_only(self) -> None:
        t = ts.NewTest("crates/cityloom_editor/src/street/model/mod.rs", "a_new_rule")
        self.assertTrue(t.matches("street::model::tests::a_new_rule"))
        self.assertFalse(t.matches("junction::model::tests::a_new_rule"))
        self.assertFalse(t.matches("street::model::tests::not_a_new_rule"))

    def test_cargo_results_by_binary(self) -> None:
        out = """\
     Running unittests src/lib.rs (target/debug/deps/cityloom_editor-0123456789abcdef)
test street::model::tests::a_new_rule ... FAILED
test street::model::tests::old ... ok
     Running tests/tiles.rs (target/debug/deps/tiles-fedcba9876543210)
test cuts_the_tile ... ignored
"""
        self.assertEqual(
            ts.results(out),
            {
                "cityloom_editor::street::model::tests::a_new_rule": "FAILED",
                "cityloom_editor::street::model::tests::old": "ok",
                "tiles::cuts_the_tile": "ignored",
            },
        )

    def test_added_lines(self) -> None:
        diff = "+++ b/e2e/tests/street.spec.ts\n@@ -5,0 +6,3 @@\n+a\n+b\n+c\n@@ -20 +23 @@\n-x\n+y\n"
        self.assertEqual(runner.added_lines(diff), {"e2e/tests/street.spec.ts": {6, 7, 8, 23}})


class Hook(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.file = self.repo / "crates" / "c" / "src" / "lib.rs"
        self.file.parent.mkdir(parents=True)
        self.file.write_text(SOURCE, encoding="utf-8")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def hook(self, tool: str, **inp: object) -> int:
        event = json.dumps({"tool_name": tool, "tool_input": inp})
        return subprocess.run([sys.executable, str(HOOK)], input=event, capture_output=True, text=True).returncode

    def test_lets_the_code_change(self) -> None:
        self.assertEqual(self.hook("Edit", file_path=str(self.file), old_string="fn life", new_string="fn lifetime"), 0)

    def test_refuses_an_edit_inside_a_test(self) -> None:
        self.assertEqual(self.hook("Edit", file_path=str(self.file), old_string="open(\"}\")", new_string="open(\"x\")"), 2)

    def test_refuses_a_new_test_module(self) -> None:
        self.assertEqual(
            self.hook("Edit", file_path=str(self.file), old_string="fn production() {}", new_string="fn production() {}\n#[cfg(test)]\nmod more {}"),
            2,
        )

    def test_refuses_a_write_that_changes_the_tests(self) -> None:
        self.assertEqual(self.hook("Write", file_path=str(self.file), content=SOURCE.replace("opens", "closes")), 2)
        self.assertEqual(self.hook("Write", file_path=str(self.file), content=SOURCE.replace("fn life", "fn lifetime")), 0)

    def test_refuses_a_protected_file(self) -> None:
        spec = self.repo / "e2e" / "tests" / "a.spec.ts"
        self.assertEqual(self.hook("Write", file_path=str(spec), content=""), 2)

    def test_bash(self) -> None:
        self.assertEqual(self.hook("Bash", command="just e2e tests/street.spec.ts 2>&1 | tail -20"), 0)
        self.assertEqual(self.hook("Bash", command="cargo test 2>&1 > /tmp/out.txt"), 0)
        self.assertEqual(self.hook("Bash", command="sed -i 's/a/b/' crates/x/src/street/view/tests.rs"), 2)
        self.assertEqual(self.hook("Bash", command="echo x > e2e/tests/a.spec.ts"), 2)
        self.assertEqual(self.hook("Bash", command="git checkout HEAD~1 -- e2e/tiers.json"), 2)


if __name__ == "__main__":
    unittest.main()
