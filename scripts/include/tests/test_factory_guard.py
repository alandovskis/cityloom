"""The implementer's guard (.claude/factory/guard.py), the PreToolUse hook of .claude/settings.json: `just test-scripts`.

It refuses the implementer's edits to the files only the test-writer may change, and the git commands that would
rewrite the red commit or publish the branch. Other agents and the person's own session pass untouched. The green gate
is what proves nothing changed; the guard stops the attempt early, with a reason the agent reads.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
GUARD = ROOT / ".claude" / "factory" / "guard.py"
sys.path.insert(0, str(GUARD.parent))

import guard  # noqa: E402


class Worktree(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name).resolve()
        (self.root / ".git").write_text("gitdir: /elsewhere\n", encoding="utf-8")  # a worktree's .git is a file

    def tearDown(self):
        self.tmp.cleanup()

    def call(self, tool: str, tool_input: dict, agent: str | None = "implement") -> str | None:
        event = {"hook_event_name": "PreToolUse", "cwd": str(self.root), "tool_name": tool, "tool_input": tool_input}
        if agent is not None:
            event["agent_type"] = agent
        return guard.refusal(event)

    def test_the_implementer_may_not_edit_a_test(self):
        for path in [
            "crates/cityloom_editor/src/street/view/tests.rs",
            "tests/e2e/tests/street.spec.ts",
            "tests/e2e/tiers.json",
            "scripts/ci/spec-map.json",
            ".github/workflows/presubmit.yml",
            "scripts/check-version.sh",
            ".claude/factory/gate.py",
            "crates/cityloom_editor/src/i18n_guard.rs",
            "crates/cityloom_editor/src/dependency_guard.rs",
        ]:
            for tool, key in (("Write", "file_path"), ("Edit", "file_path"), ("MultiEdit", "file_path"), ("NotebookEdit", "notebook_path")):
                with self.subTest(path=path, tool=tool):
                    why = self.call(tool, {key: str(self.root / path)})
                    self.assertIsNotNone(why)
                    self.assertIn(path, why)
                    self.assertIn("dispute", why)

    def test_the_implementer_may_edit_the_code(self):
        for path in ["crates/cityloom_editor/src/street/model/mod.rs", "crates/cityloom_editor/Cargo.toml", "Cargo.lock"]:
            with self.subTest(path):
                self.assertIsNone(self.call("Edit", {"file_path": str(self.root / path)}))

    def test_other_agents_and_the_session_itself_pass(self):
        path = {"file_path": str(self.root / "tests/e2e/tests/street.spec.ts")}
        self.assertIsNone(self.call("Write", path, agent="test-writer"))
        self.assertIsNone(self.call("Write", path, agent=None))
        # The red commit allows a dependency the plan names.
        guarded = {"file_path": str(self.root / "crates/cityloom_editor/src/dependency_guard.rs")}
        self.assertIsNone(self.call("Edit", guarded, agent="test-writer"))

    def test_a_path_is_read_against_its_own_worktree(self):
        nested = self.root / "deeper"
        nested.mkdir()
        (nested / ".git").write_text("gitdir: /x\n", encoding="utf-8")
        self.assertIsNotNone(self.call("Edit", {"file_path": str(nested / "tests/e2e/tests/a.spec.ts")}))
        self.assertIsNone(self.call("Edit", {"file_path": str(self.root / "deeper-notes/e2e/tests.txt")}))

    def test_a_shell_command_that_writes_a_test_is_refused(self):
        for command in [
            "sed -i '' 's/2/3/' crates/cityloom_editor/src/street/view/tests.rs",
            "echo x > tests/e2e/tests/street.spec.ts",
            "cat >> tests/e2e/tiers.json <<EOF\nx\nEOF",
            "printf x | tee scripts/ci/spec-map.json",
            "rm tests/e2e/tests/street.spec.ts",
            "mv a.rs crates/m/tests/a.rs",
            "cp /tmp/x .claude/settings.json",
            "git checkout HEAD~1 -- tests/e2e/tests/street.spec.ts",
            "git restore crates/m/src/tests.rs",
            "perl -pi -e 's/a/b/' crates/m/src/tests.rs",
            "python3 -c \"open('tests/e2e/tests/a.spec.ts','w')\"",
            "sed -i '' 's/a/b/' crates/cityloom_editor/src/dependency_guard.rs",
        ]:
            with self.subTest(command):
                self.assertIsNotNone(self.call("Bash", {"command": command}))

    def test_a_shell_command_that_rewrites_or_publishes_history_is_refused(self):
        for command in [
            "git commit --amend --no-edit",
            "git rebase -i HEAD~3",
            "git reset --hard HEAD~1",
            "git push",
            "git -C /w push origin HEAD",
            "git filter-branch --tree-filter x",
            "git stash",
        ]:
            with self.subTest(command):
                self.assertIsNotNone(self.call("Bash", {"command": command}))

    def test_reading_running_and_committing_pass(self):
        for command in [
            "cat tests/e2e/tests/street.spec.ts",
            "grep -rn splits crates/m/src/tests.rs",
            "cargo test -p cityloom_model street::tests",
            "just e2e tests/street.spec.ts",
            "scripts/check-version.sh",
            "git add crates/m/src/street.rs && git commit -m 'Split at a junction'",
            "git diff HEAD~1 -- tests/e2e/tests",
            "cargo test 2>&1 | tail -n 40 > /tmp/out.txt",
            "git log --oneline",
        ]:
            with self.subTest(command):
                self.assertIsNone(self.call("Bash", {"command": command}))


class Hook(unittest.TestCase):
    def run_hook(self, event: dict) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, str(GUARD)], input=json.dumps(event), capture_output=True, text=True)

    def test_a_refusal_exits_2_with_the_reason_on_stderr(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / ".git").mkdir()
            done = self.run_hook(
                {"agent_type": "implement", "tool_name": "Write", "tool_input": {"file_path": f"{tmp}/tests/e2e/tiers.json"}, "cwd": tmp}
            )
        self.assertEqual(done.returncode, 2)
        self.assertIn("tests/e2e/tiers.json", done.stderr)

    def test_anything_else_exits_0_silently(self):
        done = self.run_hook({"tool_name": "Write", "tool_input": {"file_path": "/x/e2e/tiers.json"}, "cwd": "/x"})
        self.assertEqual((done.returncode, done.stdout, done.stderr), (0, "", ""))

    def test_input_it_cannot_read_does_not_block_the_session(self):
        done = subprocess.run([sys.executable, str(GUARD)], input="not json", capture_output=True, text=True)
        self.assertEqual(done.returncode, 0)


class Settings(unittest.TestCase):
    def test_the_project_settings_run_the_guard_before_every_edit_and_command(self):
        settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
        hooks = settings["hooks"]["PreToolUse"]
        matchers = {h["matcher"] for h in hooks}
        self.assertEqual(matchers, {"Write|Edit|MultiEdit|NotebookEdit|Bash"})
        commands = [c["command"] for h in hooks for c in h["hooks"]]
        # A checkout without the guard (before this change) must not block every edit: python3 exits 2 on a missing file.
        self.assertEqual(commands, ['f="$CLAUDE_PROJECT_DIR/.claude/factory/guard.py"; [ ! -f "$f" ] || python3 "$f"'])

    def test_the_guard_knows_the_implementers_name(self):
        agent = (ROOT / ".claude" / "agents" / "implement.md").read_text(encoding="utf-8")
        self.assertIn(f"\nname: {guard.IMPLEMENTER}\n", agent)


if __name__ == "__main__":
    unittest.main()
