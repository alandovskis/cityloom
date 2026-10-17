"""How the scripts show their progress: the look of a Jest run (as in JakeWharton/mosaic's sample), in the terminal.

    PASS  extract  quebec-latest.osm.pbf  1,213 MB  42s
    RUNS  tiles  cutting 1,734 tiles  batch 9 of 14

    Steps: 1 passed, 1 running, 6 total
    Time:  97s
    [40 columns: failed in red, passed in green, running in blinking yellow, the rest grey]

A script lists its steps up front (`Spec`), so the total is right from the start, then does each in `with
progress.step(key)`. The output of the tools it runs goes to a log under data/logs, and a failure shows the log's
last lines. Not a terminal (a pipe, a file, CI), or NO_COLOR, or TERM=dumb: one plain line per change, nothing redrawn.

What is drawn is a pure function of the steps (`frame`, `finished_block`, `plain_line`), so the tests read it.
Standard library only.
"""

from __future__ import annotations

import os
import subprocess
import sys
import threading
import time
import traceback
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Optional

PENDING, RUNNING, PASSED, FAILED = "pending", "running", "passed", "failed"
BAR_WIDTH = 40
BLINK_SECONDS = 0.5
TAIL_LINES = 15
PLAIN_EVERY_SECONDS = 30
BULLET = "‣"
# The text colour of a badge is black; the background says the state.
BADGES = {RUNNING: ("RUNS", "30;43"), PASSED: ("PASS", "30;42"), FAILED: ("FAIL", "30;41")}
GREY = "48;2;128;128;128"


@dataclass
class Spec:
    """A step a script will do: `key` names its log, `group` and `subject` are what the row says."""

    key: str
    group: str
    subject: str


@dataclass
class Step:
    key: str
    group: str
    subject: str
    state: str = PENDING
    detail: str = ""
    fraction: Optional[float] = None
    started: float = 0.0
    took: Optional[float] = None
    lines: list = field(default_factory=list)


class StepError(Exception):
    """A step failed; `lines` are what to tell the user, one per line."""

    def __init__(self, *lines: str):
        super().__init__("; ".join(lines))
        self.lines = list(lines)


# ---- what is drawn: pure functions of the steps


def paint(text: str, code: str, color: bool) -> str:
    return f"\x1b[{code}m{text}\x1b[0m" if color else text


def duration(seconds: float) -> str:
    seconds = int(seconds)
    return f"{seconds // 60}m{seconds % 60:02d}s" if seconds >= 60 else f"{seconds}s"


def badge(state: str, color: bool) -> str:
    label, code = BADGES[state]
    return paint(f" {label} ", code, color)


def row(step: Step, color: bool) -> str:
    """A badge, the group in the default colour, then the subject in bold, what it has to say and, once passed, its time."""
    subject = f"\x1b[1m{step.subject}\x1b[22m" if color else step.subject
    parts = [f"{badge(step.state, color)} {step.group}", subject, step.detail]
    if step.state == PASSED and step.took is not None:
        parts.append(duration(step.took))
    return "  ".join(part for part in parts if part)


def counts(steps: list) -> dict:
    return {state: sum(step.state == state for step in steps) for state in (FAILED, PASSED, RUNNING)}


def summary(steps: list, color: bool) -> str:
    n = counts(steps)
    parts = [
        paint(f"{n[state]} {state}", code, color)
        for state, code in ((FAILED, "31"), (PASSED, "32"), (RUNNING, "33"))
        if n[state]
    ]
    return "Steps: " + ", ".join(parts + [f"{len(steps)} total"])


def blink_on(elapsed: float) -> bool:
    return int(elapsed / BLINK_SECONDS) % 2 == 0


def bar(steps: list, blinking_on: bool) -> str:
    """40 columns of coloured blanks: failed, passed, running (grey while it blinks off), then grey."""
    n, total = counts(steps), len(steps)
    widths = [(code, n[state] * BAR_WIDTH // total) for state, code in ((FAILED, "41"), (PASSED, "42"), (RUNNING, "43"))]
    if not blinking_on:
        widths[2] = (GREY, widths[2][1])
    widths.append((GREY, BAR_WIDTH - sum(w for _, w in widths)))
    return "".join(f"\x1b[{code}m{' ' * w}\x1b[0m" for code, w in widths if w)


def frame(steps: list, elapsed: float, blinking_on: bool, color: bool, after_pass: bool = False) -> list:
    """The live lines: a row per running step, the summary, the time and, while something runs, the bar. `after_pass`
    is true when the last finished step passed: a blank line then separates the log from the rows."""
    running = [row(step, color) for step in steps if step.state == RUNNING]
    lines = [""] if after_pass else []
    lines += running + [""] * bool(running)
    lines += [summary(steps, color), f"Time:  {int(elapsed)}s"]
    if running and color:
        lines.append(bar(steps, blinking_on))
    return lines


def failure_lines(step: Step) -> list:
    return [f" {BULLET} {line}" for line in step.lines]


def finished_block(step: Step, color: bool) -> list:
    """What is printed once and kept when a step ends: its row, and for a failure its lines and a blank line."""
    return [row(step, color)] + (failure_lines(step) + [""] if step.state == FAILED else [])


# ---- not a terminal


def is_fancy(isatty: bool, env) -> bool:
    return isatty and not env.get("NO_COLOR") and env.get("TERM") != "dumb"


def plain_line(step: Step) -> str:
    label = BADGES[step.state][0]
    took = duration(step.took) if step.state == PASSED and step.took is not None else ""
    return " ".join(part for part in (label, step.group, step.subject, step.detail, took) if part)


def plain_lines(step: Step) -> list:
    return [plain_line(step)] + (failure_lines(step) if step.state == FAILED else [])


class Throttle:
    """Says when a progress line is due: each 10% done, or each 30 seconds."""

    def __init__(self, now: float):
        self.decile, self.last = 0, now

    def __call__(self, fraction: Optional[float], now: float) -> bool:
        decile = int(fraction * 10) if fraction is not None else self.decile
        if decile > self.decile or now - self.last >= PLAIN_EVERY_SECONDS:
            self.decile, self.last = decile, now
            return True
        return False


# ---- the log of a tool and its failure


def tail(text: str, limit: int = TAIL_LINES) -> list:
    """The last `limit` lines of `text` that are not empty."""
    return [line.rstrip() for line in text.splitlines() if line.strip()][-limit:]


def read_tail(path: Path, limit_bytes: int = 1 << 16) -> str:
    with path.open("rb") as log:
        log.seek(0, 2)
        log.seek(max(0, log.tell() - limit_bytes))
        return log.read().decode(errors="replace")


# ---- the renderers


class Plain:
    """One line per change, as for a pipe or CI."""

    def __init__(self, out, steps: list, clock: Callable[[], float]):
        self.out, self.steps, self.clock, self.throttle = out, steps, clock, None
        self.t0 = clock()

    def begin(self):
        pass  # nothing to set up: every change is a line of its own

    def started(self, step: Step):
        self.throttle = Throttle(self.clock())
        self.say(plain_lines(step))

    def updated(self, step: Step):
        if self.throttle and self.throttle(step.fraction, self.clock()):
            self.say(plain_lines(step))

    def finished(self, step: Step):
        self.say(plain_lines(step))

    def close(self):
        self.say(frame(self.steps, self.clock() - self.t0, True, False))

    def say(self, lines: list):
        self.out.write("".join(line + "\n" for line in lines))
        self.out.flush()


class Terminal:
    """Finished steps are printed once and kept; the live lines below them are drawn again in place."""

    def __init__(self, out, steps: list, lock, clock: Callable[[], float]):
        self.out, self.steps, self.lock, self.clock = out, steps, lock, clock
        self.t0 = clock()
        self.drawn, self.pending, self.after_pass, self.last = 0, [], False, None
        self.stop = threading.Event()
        self.ticker = None

    def begin(self):
        self.out.write("\x1b[?25l")  # hide the cursor
        self.ticker = threading.Thread(target=self.tick, daemon=True)
        self.ticker.start()

    def tick(self):
        while not self.stop.wait(0.1):
            self.draw()

    def started(self, _step: Step):
        self.draw()

    def updated(self, _step: Step):
        pass  # the ticker draws it

    def finished(self, step: Step):
        # Not drawn yet: the next step's start does it, so the live lines do not flicker between two steps.
        with self.lock:
            self.pending += finished_block(step, True)
            self.after_pass = step.state == PASSED

    def draw(self):
        with self.lock:
            elapsed = self.clock() - self.t0
            live = frame(self.steps, elapsed, blink_on(elapsed), True, self.after_pass)
            if not self.pending and live == self.last:
                return
            up = f"\x1b[{self.drawn}A\r\x1b[J" if self.drawn else ""
            self.out.write(up + "".join(line + "\n" for line in self.pending + live))
            self.out.flush()
            self.pending, self.last, self.drawn = [], live, len(live)

    def close(self):
        self.stop.set()
        if self.ticker:
            self.ticker.join()
        self.draw()
        self.out.write("\x1b[?25h")  # show the cursor again
        self.out.flush()


class Progress:
    """The steps of one run. Use it as `with Progress(specs, logs_dir, root) as p:` and each step as `with p.step(key)`."""

    def __init__(self, specs: list, logs: Path, root: Path, out=None, env=None, clock=time.monotonic):
        self.out = out or sys.stderr
        env = os.environ if env is None else env
        self.steps = {spec.key: Step(spec.key, spec.group, spec.subject) for spec in specs}
        self.logs, self.root, self.clock = logs, root, clock
        self.lock = threading.RLock()
        if is_fancy(self.out.isatty(), env):
            self.view = Terminal(self.out, list(self.steps.values()), self.lock, clock)
        else:
            self.view = Plain(self.out, list(self.steps.values()), clock)

    def __enter__(self):
        self.view.begin()
        return self

    def __exit__(self, kind, error, trace):
        self.view.close()
        if kind is KeyboardInterrupt:
            raise SystemExit(130)  # no traceback over the display
        return False

    def log_path(self, step: Step) -> Path:
        return self.logs / f"{step.key}.log"

    def shown(self, path: Path) -> str:
        try:
            return str(path.relative_to(self.root))
        except ValueError:
            return str(path)

    @contextmanager
    def step(self, key: str):
        """Runs a step: it passes when the block ends, and fails when it raises (a StepError says what to show)."""
        step = self.steps[key]
        self.log_path(step).parent.mkdir(parents=True, exist_ok=True)
        self.log_path(step).unlink(missing_ok=True)
        with self.lock:
            step.state, step.started = RUNNING, self.clock()
        self.view.started(step)
        failure = None
        try:
            yield step
        except BaseException as error:
            failure = error
        finally:
            # Whatever way the block is left, the step ends in a state and the display is told.
            self._end(step, failure)
        if isinstance(failure, KeyboardInterrupt):
            raise failure
        if failure is not None:
            raise SystemExit(1) from None

    def _end(self, step: Step, failure: Optional[BaseException]):
        """Passes the step, or fails it with the lines to show (a StepError says what; any other error goes to the log)."""
        if failure is None:
            with self.lock:
                step.state, step.took = PASSED, self.clock() - step.started
        else:
            if isinstance(failure, StepError):
                lines = failure.lines
            elif isinstance(failure, KeyboardInterrupt):
                lines = ["interrupted"]
            else:
                with self.log_path(step).open("a") as log:
                    log.write("".join(traceback.format_exception(type(failure), failure, failure.__traceback__)))
                lines = [f"{type(failure).__name__}: {failure}", f"full log: {self.shown(self.log_path(step))}"]
            with self.lock:
                step.state, step.lines = FAILED, lines
        self.view.finished(step)

    def update(self, step: Step, subject=None, detail=None, fraction=None):
        with self.lock:
            step.subject = step.subject if subject is None else subject
            step.detail = step.detail if detail is None else detail
            step.fraction = fraction
        self.view.updated(step)

    def run(self, step: Step, command: list, cwd=None):
        """Runs `command` with its output in the step's log; a failure shows the log's last lines and its path."""
        log_path = self.log_path(step)
        name = Path(command[0]).name
        with log_path.open("ab") as log:
            log.write(f"$ {' '.join(map(str, command))}\n".encode())
            log.flush()
            try:
                status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, cwd=cwd).returncode
            except FileNotFoundError:
                raise StepError(f"{name}: not found on PATH") from None
        if status:
            raise StepError(
                f"{name} exited with status {status}", *tail(read_tail(log_path)), f"full log: {self.shown(log_path)}"
            )
