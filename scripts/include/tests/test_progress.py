"""Tests of scripts/include/progress.py: `just test-scripts`."""

import io
import re
import sys
import tempfile
import threading
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from include import progress  # noqa: E402
from include.progress import FAILED, PASSED, PENDING, RUNNING, Spec, Step  # noqa: E402

ANSI = re.compile(r"\x1b\[[0-9;]*m")


def plain(lines):
    return [ANSI.sub("", line) for line in lines]


def steps(*states):
    """One step per state, named s0, s1, ..., each in the state given."""
    made = [Step(f"s{i}", "group", f"subject {i}") for i in range(len(states))]
    for step, state in zip(made, states):
        step.state = state
        step.took = 3.0 if state == PASSED else None
    return made


def bar_widths(line):
    """The widths of the coloured runs of a bar line, as {background code: columns}."""
    widths = {}
    for code, blanks in re.findall(r"\x1b\[([0-9;]+)m( +)", line):
        widths[code] = widths.get(code, 0) + len(blanks)
    return widths


GREY = "48;2;128;128;128"


class Rows(unittest.TestCase):
    def test_badge_has_a_space_each_side_black_on_a_colour_and_is_not_bold(self):
        self.assertEqual(progress.badge(RUNNING, True), "\x1b[30;43m RUNS \x1b[0m")
        self.assertEqual(progress.badge(PASSED, True), "\x1b[30;42m PASS \x1b[0m")
        self.assertEqual(progress.badge(FAILED, True), "\x1b[30;41m FAIL \x1b[0m")
        self.assertEqual(progress.badge(PASSED, False), " PASS ")

    def test_row_is_group_in_the_default_colour_then_subject_in_bold(self):
        step = Step("extract", "extract", "quebec-latest.osm.pbf", PASSED, detail="1,213 MB", took=42)
        self.assertEqual(
            progress.row(step, True),
            "\x1b[30;42m PASS \x1b[0m extract  \x1b[1mquebec-latest.osm.pbf\x1b[22m  1,213 MB  42s",
        )

    def test_running_row_shows_progress_but_no_time_and_failed_row_none_either(self):
        running = Step("tiles", "tiles", "cutting 1,734 tiles", RUNNING, detail="batch 9 of 14", took=5)
        failed = Step("basemap", "basemap", "planetiler", FAILED, took=5)
        self.assertEqual(plain([progress.row(running, True)]), [" RUNS  tiles  cutting 1,734 tiles  batch 9 of 14"])
        self.assertEqual(plain([progress.row(failed, True)]), [" FAIL  basemap  planetiler"])

    def test_durations(self):
        self.assertEqual([progress.duration(s) for s in (0.4, 42.9, 60, 192)], ["0s", "42s", "1m00s", "3m12s"])


class Summary(unittest.TestCase):
    def test_only_the_counts_that_are_not_zero_in_order_failed_passed_running_total(self):
        line = progress.summary(steps(PASSED, FAILED, RUNNING, PENDING, PASSED, PENDING), True)
        self.assertEqual(
            line,
            "Steps: \x1b[31m1 failed\x1b[0m, \x1b[32m2 passed\x1b[0m, \x1b[33m1 running\x1b[0m, 6 total",
        )

    def test_zero_counts_are_left_out(self):
        self.assertEqual(progress.summary(steps(PASSED, PASSED, PENDING), False), "Steps: 2 passed, 3 total")
        self.assertEqual(progress.summary(steps(PENDING, PENDING), False), "Steps: 2 total")


class Bar(unittest.TestCase):
    def test_is_forty_columns_red_green_yellow_then_grey(self):
        line = progress.bar(steps(FAILED, PASSED, PASSED, RUNNING, PENDING, PENDING, PENDING, PENDING), True)
        self.assertEqual(bar_widths(line), {"41": 5, "42": 10, "43": 5, GREY: 20})
        self.assertEqual(len(ANSI.sub("", line)), progress.BAR_WIDTH)
        self.assertLess(line.index("41m"), line.index("42m"))
        self.assertLess(line.index("42m"), line.index("43m"))
        self.assertLess(line.index("43m"), line.index(GREY))

    def test_each_part_rounds_down(self):
        # 9 steps: 1 * 40 // 9 = 4 and 2 * 40 // 9 = 8; the rest is grey
        line = progress.bar(steps(FAILED, PASSED, PASSED, RUNNING, *[PENDING] * 5), True)
        self.assertEqual(bar_widths(line), {"41": 4, "42": 8, "43": 4, GREY: 24})

    def test_yellow_blinks_to_grey(self):
        base = steps(PASSED, RUNNING, PENDING, PENDING)
        self.assertEqual(bar_widths(progress.bar(base, True)), {"42": 10, "43": 10, GREY: 20})
        self.assertEqual(bar_widths(progress.bar(base, False)), {"42": 10, GREY: 30})

    def test_blink_phase_changes_every_500_ms(self):
        self.assertEqual([progress.blink_on(t) for t in (0, 0.49, 0.5, 0.99, 1.0)], [True, True, False, False, True])


class Frame(unittest.TestCase):
    def test_running_only(self):
        made = steps(RUNNING, PENDING)
        made[0].subject, made[0].detail = "cutting 1,734 tiles", "batch 9 of 14"
        lines = progress.frame(made, 7.9, True, False)
        self.assertEqual(
            lines,
            [" RUNS  group  cutting 1,734 tiles  batch 9 of 14", "", "Steps: 1 running, 2 total", "Time:  7s"],
        )

    def test_has_the_bar_as_third_summary_line_while_something_runs(self):
        lines = progress.frame(steps(RUNNING, PENDING), 1, True, True)
        self.assertEqual(len(lines), 5)
        self.assertEqual(len(ANSI.sub("", lines[-1])), progress.BAR_WIDTH)

    def test_blank_line_between_the_log_and_the_running_rows_after_a_pass_only(self):
        made = steps(PASSED, RUNNING)
        self.assertEqual(progress.frame(made, 1, True, False, after_pass=True)[0], "")
        self.assertNotEqual(progress.frame(made, 1, True, False, after_pass=False)[0], "")

    def test_two_running_rows_then_one_blank_line(self):
        lines = plain(progress.frame(steps(RUNNING, RUNNING, PENDING), 1, True, True))
        self.assertEqual(lines[2], "")
        self.assertTrue(lines[0].startswith(" RUNS") and lines[1].startswith(" RUNS"))
        self.assertTrue(lines[3].startswith("Steps:"))

    def test_nothing_running_has_no_bar_and_no_rows(self):
        lines = progress.frame(steps(PASSED, FAILED, PENDING), 97, True, True)
        self.assertEqual(plain(lines), ["Steps: 1 failed, 1 passed, 3 total", "Time:  97s"])

    def test_finished_block_of_a_passed_step_is_its_row_only(self):
        step = steps(PASSED)[0]
        self.assertEqual(progress.finished_block(step, False), [" PASS  group  subject 0  3s"])

    def test_finished_block_of_a_failure_has_its_lines_with_bullets_then_a_blank_line(self):
        step = steps(FAILED)[0]
        step.lines = ["java: Planetiler needs Java 21, found 17", "full log: data/logs/basemap.log"]
        self.assertEqual(
            progress.finished_block(step, False),
            [
                " FAIL  group  subject 0",
                " ‣ java: Planetiler needs Java 21, found 17",
                " ‣ full log: data/logs/basemap.log",
                "",
            ],
        )


class Plain(unittest.TestCase):
    def test_lines_have_no_colour_and_name_the_state(self):
        step = Step("tiles", "tiles", "cutting 1,734 tiles", RUNNING)
        self.assertEqual(progress.plain_line(step), "RUNS tiles cutting 1,734 tiles")
        step.detail = "batch 9 of 14"
        self.assertEqual(progress.plain_line(step), "RUNS tiles cutting 1,734 tiles batch 9 of 14")
        step.state, step.subject, step.detail, step.took = PASSED, "1,700 tiles", "", 192
        self.assertEqual(progress.plain_line(step), "PASS tiles 1,700 tiles 3m12s")

    def test_failure_lines_follow_a_failed_line(self):
        step = Step("basemap", "basemap", "planetiler", FAILED, lines=["boom"])
        self.assertEqual(progress.plain_lines(step), ["FAIL basemap planetiler", " ‣ boom"])

    def test_progress_lines_come_at_most_every_ten_percent_or_thirty_seconds(self):
        due = progress.Throttle(now=0)
        self.assertFalse(due(0.05, 1))
        self.assertTrue(due(0.10, 2))
        self.assertFalse(due(0.15, 3))
        self.assertFalse(due(0.19, 4))
        self.assertTrue(due(0.20, 5))
        self.assertFalse(due(0.20, 6))
        self.assertTrue(due(0.21, 40))  # 30 s since the last line
        self.assertFalse(due(0.22, 41))

    def test_without_a_fraction_only_the_thirty_seconds_count(self):
        due = progress.Throttle(now=0)
        self.assertFalse(due(None, 29))
        self.assertTrue(due(None, 30))

    def test_a_terminal_needs_a_tty_no_no_color_and_no_dumb_term(self):
        self.assertTrue(progress.is_fancy(True, {"TERM": "xterm-256color"}))
        self.assertTrue(progress.is_fancy(True, {}))
        self.assertFalse(progress.is_fancy(False, {"TERM": "xterm"}))
        self.assertFalse(progress.is_fancy(True, {"NO_COLOR": "1"}))
        self.assertFalse(progress.is_fancy(True, {"TERM": "dumb"}))
        self.assertTrue(progress.is_fancy(True, {"NO_COLOR": ""}))


class Failure(unittest.TestCase):
    def test_tail_is_the_last_fifteen_non_empty_lines(self):
        text = "\n".join(f"line {i}\n" if i % 3 == 0 else f"line {i}" for i in range(40)) + "\n\n"
        tail = progress.tail(text)
        self.assertEqual(len(tail), 15)
        self.assertEqual(tail[-1], "line 39")
        self.assertNotIn("", tail)

    def test_tail_of_a_short_log_is_all_of_it(self):
        self.assertEqual(progress.tail("a\n\n  \nb  \n"), ["a", "b"])
        self.assertEqual(progress.tail(""), [])


class Run(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.root = Path(self.dir.name)
        self.out = io.StringIO()

    def progress(self, *specs):
        return progress.Progress(list(specs), self.root / "logs", self.root, out=self.out, env={})

    def test_not_a_terminal_prints_a_line_per_change_and_a_summary(self):
        with self.progress(Spec("a", "g", "first"), Spec("b", "g", "second")) as p:
            with p.step("a"):
                pass
            with p.step("b") as step:
                p.update(step, detail="half", fraction=0.5)
        text = self.out.getvalue()
        lines = text.splitlines()
        self.assertEqual(
            lines[:-1],
            ["RUNS g first", "PASS g first 0s", "RUNS g second", "RUNS g second half", "PASS g second half 0s",
             "Steps: 2 passed, 2 total"],
        )
        self.assertRegex(lines[-1], r"^Time:  \d+s$")
        self.assertNotIn("\x1b", text)

    def test_a_failing_command_logs_its_output_and_shows_the_tail_and_the_log_path(self):
        script = "import sys\nfor i in range(30): print('out', i)\nprint('bad thing')\nsys.exit(3)"
        with self.assertRaises(SystemExit) as caught:
            with self.progress(Spec("a", "g", "first"), Spec("b", "g", "second")) as p:
                with p.step("a") as step:
                    p.run(step, [sys.executable, "-c", script])
        self.assertEqual(caught.exception.code, 1)
        lines = self.out.getvalue().splitlines()
        self.assertIn("FAIL g first", lines)
        after = lines[lines.index("FAIL g first") + 1 :]
        self.assertEqual(after[0], f" \u2023 {Path(sys.executable).name} exited with status 3")
        self.assertEqual(after[1], " \u2023 out 16")  # the last 15 lines: out 16 to out 29, then the error
        self.assertEqual(after[15], " \u2023 bad thing")
        self.assertEqual(after[16], " \u2023 full log: logs/a.log")
        self.assertIn("Steps: 1 failed, 2 total", lines)
        self.assertIn("out 0", (self.root / "logs" / "a.log").read_text())

    def test_a_command_that_is_not_there_fails_the_step(self):
        with self.assertRaises(SystemExit):
            with self.progress(Spec("a", "g", "first")) as p:
                with p.step("a") as step:
                    p.run(step, ["no-such-program-here"])
        self.assertIn(" ‣ no-such-program-here: not found", self.out.getvalue())

    def test_an_exception_fails_the_step_and_leaves_with_status_1(self):
        with self.assertRaises(SystemExit) as caught:
            with self.progress(Spec("a", "g", "first")) as p:
                with p.step("a"):
                    raise ValueError("oops")
        self.assertEqual(caught.exception.code, 1)
        self.assertIn(" ‣ ValueError: oops", self.out.getvalue())

    def test_ctrl_c_fails_the_step_and_leaves_with_status_130(self):
        with self.assertRaises(SystemExit) as caught:
            with self.progress(Spec("a", "g", "first")) as p:
                with p.step("a"):
                    raise KeyboardInterrupt
        self.assertEqual(caught.exception.code, 130)
        self.assertIn("FAIL g first", self.out.getvalue())


class Terminal(unittest.TestCase):
    def test_redraws_over_the_live_lines_and_leaves_the_cursor_shown(self):
        out = io.StringIO()
        specs = [Spec("a", "g", "first"), Spec("b", "g", "second")]
        made = [Step(s.key, s.group, s.subject) for s in specs]
        term = progress.Terminal(out, made, threading.RLock(), clock=lambda: 0.0)
        term.out.write("\x1b[?25l")  # begin() also starts the ticker, which a test leaves out
        made[0].state = RUNNING
        term.draw()
        first = out.getvalue()
        self.assertIn("\x1b[?25l", first)
        lines_drawn = term.drawn
        made[0].state, made[0].took = PASSED, 1.0
        term.finished(made[0])
        made[1].state = RUNNING
        term.draw()
        self.assertIn(f"\x1b[{lines_drawn}A", out.getvalue())
        term.close()
        self.assertTrue(out.getvalue().endswith("\x1b[?25h"))


if __name__ == "__main__":
    unittest.main()
