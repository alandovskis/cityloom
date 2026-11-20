#!/usr/bin/env python3
"""PreToolUse hook of the implement agent (`.claude/agents/implement.md`): it may not change a test.

Refuses (exit 2, the reason on stderr, which Claude reads) an Edit, MultiEdit or Write that changes the test surface
(`.claude/gates/test_surface.py`: a protected file, or a `#[cfg(test)]` item in any other Rust file), and a Bash command
that names a protected file next to something that writes. The Bash check is a guard against the obvious, not a proof:
the green gate (`.claude/gates/green_gate.py`) compares the whole test surface with the red commit and is what decides.
"""

from __future__ import annotations

import json
import re
import shlex
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "gates"))

import test_surface  # noqa: E402

WRITES = re.compile(
    r"(?:^|[\s;&|(])(?:sed\s+(?:-\w*\s+)*-i|perl\s+-\w*i|tee|mv|cp|rm|truncate|patch|dd|install|ln)\b"
    r"|\bgit\s+(?:checkout|restore|apply|am|cherry-pick|revert|reset|stash|mv|rm)\b"
)


def refuse(why: str) -> None:
    print(
        f"{why}\nThe implement agent does not change tests. If a test looks wrong, stop and hand back a dispute "
        "(see `.claude/agents/implement.md`, **A test looks wrong**).",
        file=sys.stderr,
    )
    sys.exit(2)


def repo_path(file: str) -> tuple[Path, str] | None:
    """The repository root (the worktree's, when in one) and the file's path inside it."""
    p = Path(file).resolve()
    for root in [p.parent, *p.parent.parents]:
        if (root / ".git").exists():
            return root, p.relative_to(root).as_posix()
    return None


def check_file(tool: str, inp: dict) -> None:
    file = inp.get("file_path") or inp.get("notebook_path")
    if not file:
        return
    found = repo_path(file)
    if not found:
        return
    root, rel = found
    if test_surface.protected(rel):
        refuse(f"`{rel}` is test surface: only the test-writer agent changes it.")
    if not rel.endswith(".rs"):
        return
    path = root / rel
    old = path.read_text(encoding="utf-8") if path.exists() else ""
    if tool == "Write":
        if test_surface.test_items(old) != test_surface.test_items(inp.get("content", "")):
            refuse(f"This write changes a `#[cfg(test)]` item in `{rel}`.")
        return
    edits = inp.get("edits") or [inp]
    text = old
    for e in edits:
        before, after = e.get("old_string", ""), e.get("new_string", "")
        if before and before in text:
            at = text.find(before)
            if test_surface.touches_tests(text, at, at + len(before)):
                refuse(f"This edit is inside a `#[cfg(test)]` item of `{rel}`.")
            text = text.replace(before, after) if e.get("replace_all") else text.replace(before, after, 1)
    if test_surface.test_items(old) != test_surface.test_items(text):
        refuse(f"This edit changes the `#[cfg(test)]` items of `{rel}` (adds, removes or reshapes one).")


REDIRECT = re.compile(r"(?<![<>&])\d?>>?\s*(?!&)([^\s;&|()]+)")


def check_bash(inp: dict) -> None:
    command = inp.get("command", "")
    targets = REDIRECT.findall(command)
    if WRITES.search(command):
        try:
            words = shlex.split(command, posix=True)
        except ValueError:
            words = command.split()
        targets += [w.strip("'\";&|()<>") for w in words]
    for w in targets:
        rel = w[2:] if w.startswith("./") else w
        if ("/" in rel or "." in rel) and (test_surface.protected(rel) or test_surface.protected(rel.rstrip("/") + "/x")):
            refuse(f"This command writes and names `{w}`, which is test surface.")


def main() -> None:
    event = json.load(sys.stdin)
    tool, inp = event.get("tool_name", ""), event.get("tool_input", {}) or {}
    if tool in ("Edit", "MultiEdit", "Write", "NotebookEdit"):
        check_file(tool, inp)
    elif tool == "Bash":
        check_bash(inp)


if __name__ == "__main__":
    main()
