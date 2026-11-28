"""The retry with exponential backoff (scripts/retry.sh) that scripts/deploy.sh runs `railway up` under: `just test-scripts`.

Each test runs the script with a stand-in command that fails a set number of times, and a `sleep` on the PATH that only
records how long it was asked to wait, so no test waits.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RETRY = ROOT / "scripts" / "retry.sh"


class Retry(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        bin_dir = self.dir / "bin"
        bin_dir.mkdir()
        sleep = bin_dir / "sleep"
        sleep.write_text(f'#!/bin/sh\necho "$1" >> "{self.dir / "sleeps"}"\n', encoding="utf-8")
        sleep.chmod(0o755)
        # Fails until it has been called FAILURES times, then succeeds; counts its calls in a file.
        flaky = bin_dir / "flaky"
        flaky.write_text(
            "#!/bin/sh\n"
            f'calls="{self.dir / "calls"}"\n'
            'n=$(($(cat "$calls" 2>/dev/null || echo 0) + 1))\n'
            'echo "$n" > "$calls"\n'
            'echo "args: $*"\n'
            '[ "$n" -gt "$FAILURES" ]\n',
            encoding="utf-8",
        )
        flaky.chmod(0o755)
        self.env = {**os.environ, "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}"}

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_retry(self, failures: int, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [str(RETRY), *args, "flaky", "one", "two words"],
            env={**self.env, "FAILURES": str(failures)},
            capture_output=True,
            text=True,
            check=False,
        )

    def calls(self) -> int:
        path = self.dir / "calls"
        return int(path.read_text(encoding="utf-8")) if path.exists() else 0

    def sleeps(self) -> list[str]:
        path = self.dir / "sleeps"
        return path.read_text(encoding="utf-8").split() if path.exists() else []

    def test_a_command_that_succeeds_runs_once_and_never_waits(self) -> None:
        result = self.run_retry(0, "4", "15")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), 1)
        self.assertEqual(self.sleeps(), [])

    def test_the_command_keeps_its_arguments(self) -> None:
        result = self.run_retry(0, "4", "15")
        self.assertIn("args: one two words", result.stdout)

    def test_each_wait_after_a_failure_doubles(self) -> None:
        result = self.run_retry(3, "4", "15")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), 4)
        self.assertEqual(self.sleeps(), ["15", "30", "60"])

    def test_it_gives_up_after_the_last_attempt_with_the_commands_status(self) -> None:
        result = self.run_retry(10, "3", "5")
        self.assertEqual(result.returncode, 1)  # the stand-in's own status, not the usage error's 2
        self.assertEqual(self.calls(), 3)
        self.assertEqual(self.sleeps(), ["5", "10"])
        self.assertIn("3 attempts", result.stderr)

    def test_each_failure_is_reported_with_the_wait_before_the_next_attempt(self) -> None:
        result = self.run_retry(1, "4", "15")
        self.assertIn("attempt 1 of 4 failed", result.stderr)
        self.assertIn("15s", result.stderr)

    def test_it_refuses_a_count_or_a_delay_that_is_not_a_whole_number(self) -> None:
        for args in [("0", "15"), ("x", "15"), ("4", "-1"), ("4", "1.5")]:
            with self.subTest(args=args):
                result = self.run_retry(0, *args)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(self.calls(), 0)


if __name__ == "__main__":
    unittest.main()
