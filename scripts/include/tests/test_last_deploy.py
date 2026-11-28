"""The last deployed commit (scripts/ci/last_deploy.py), which postsubmit.yml's Changes job diffs a push from: `just test-scripts`.

The choice is tested on GitHub's answers as data; the command itself runs with a stand-in `gh` on the PATH, so no test
asks GitHub.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts" / "ci" / "last_deploy.py"
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import last_deploy  # noqa: E402

# GitHub's workflow_runs, newest first.
RUNS = [{"id": 30, "head_sha": "c" * 40}, {"id": 20, "head_sha": "b" * 40}, {"id": 10, "head_sha": "a" * 40}]


class LastDeployed(unittest.TestCase):
    def test_the_newest_earlier_successful_deploy(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {30: "success", 20: "success"}.get, before=30), "b" * 40)

    def test_a_run_whose_deploy_was_skipped_is_not_a_deploy(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {20: "skipped", 10: "success"}.get, before=30), "a" * 40)

    def test_failed_and_cancelled_deploys_are_passed_over(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {20: "failure", 10: "cancelled"}.get, before=99), "")

    def test_no_deploy_found_gives_no_base(self):
        self.assertEqual(last_deploy.last_deployed([], {}.get, before=30), "")

    def test_this_run_and_later_ones_are_not_candidates(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {30: "success"}.get, before=30), "")


class Command(unittest.TestCase):
    def run_with_gh(self, gh: str) -> subprocess.CompletedProcess:
        with tempfile.TemporaryDirectory() as tmp:
            fake = Path(tmp) / "gh"
            fake.write_text("#!/bin/sh\n" + gh, encoding="utf-8")
            fake.chmod(0o755)
            env = {**os.environ, "PATH": f"{tmp}:{os.environ['PATH']}", "GITHUB_REPOSITORY": "o/r", "GITHUB_RUN_ID": "30"}
            return subprocess.run([sys.executable, str(SCRIPT)], env=env, capture_output=True, text=True)

    def test_a_failed_lookup_prints_no_base_and_succeeds(self):
        done = self.run_with_gh("echo 'HTTP 403' >&2\nexit 1\n")
        self.assertEqual((done.returncode, done.stdout), (0, ""))
        self.assertIn("403", done.stderr)

    def test_an_answer_that_is_not_json_prints_no_base(self):
        done = self.run_with_gh("echo '<html>'\n")
        self.assertEqual((done.returncode, done.stdout), (0, ""))

    def test_the_commit_of_the_last_deploy_is_printed(self):
        runs = '{"workflow_runs": [{"id": 20, "head_sha": "' + "b" * 40 + '"}]}'
        jobs = '{"jobs": [{"name": "Deploy to Railway", "conclusion": "success"}]}'
        done = self.run_with_gh(f'case "$2" in *"/jobs"*) echo \'{jobs}\' ;; *) echo \'{runs}\' ;; esac\n')
        self.assertEqual((done.returncode, done.stdout.strip()), (0, "b" * 40))


if __name__ == "__main__":
    unittest.main()
