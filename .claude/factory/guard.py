#!/usr/bin/env python3
"""The implementer's guard: a PreToolUse hook (.claude/settings.json) of the software factory (CLM-27).

While the `implement` agent runs, it refuses:
- an edit (Write, Edit, MultiEdit, NotebookEdit) of a file of the guarded set (gate.GUARDED: the tests, their
  fixtures, the tiers and the spec map, the checks and the factory itself);
- a shell command that names such a file and writes (a redirect, sed -i, tee, rm, mv, cp, git checkout or restore, ...);
- a shell command that rewrites or publishes the history (git commit --amend, rebase, reset, stash, push, ...): the red
  commit must stay as the test-writer made it, and only the factory pushes, once green.

It answers with exit code 2 and the reason on stderr, which the agent reads. Every other agent, and the person's own
session, passes untouched, and so does input it cannot read: the green gate (gate.py) is what proves no test changed;
this guard only stops the attempt early. Standard library only; scripts/include/tests/test_factory_guard.py tests it.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))

from gate import is_guarded  # noqa: E402

IMPLEMENTER = "implement"

EDIT_TOOLS = {"Write": "file_path", "Edit": "file_path", "MultiEdit": "file_path", "NotebookEdit": "notebook_path"}

_WRITES = re.compile(
    r"(?:^|[^0-9&>])>>?(?!&)"  # a redirect into a file (not 2>&1)
    r"|\bsed\b[^|;&]*\s-[a-zA-Z]*i"
    r"|\bperl\b[^|;&]*\s-[a-zA-Z]*i"
    r"|\b(?:tee|rm|mv|cp|touch|truncate|ln|install|unlink|dd|patch)\b"
    r"|\bgit\b[^|;&]*\s(?:checkout|restore|apply|am|cherry-pick|revert|mv|rm)\b"
    r"|\bopen\s*\(|\bwrite(?:File|_text|_bytes)?\s*\("
)
_HISTORY = re.compile(
    r"\bgit\b(?:\s+-[Cc]\s+\S+)*\s+(?:push|rebase|reset|stash|filter-branch|filter-repo|replace|update-ref)\b"
    r"|\bgit\b(?:\s+-[Cc]\s+\S+)*\s+commit\b[^|;&]*--amend"
)
_TOKEN = re.compile(r"[\w.~/@+-]+")


def _root(path: Path) -> Optional[Path]:
    """The worktree a path is in: the nearest folder holding `.git` (a folder, or a file in a linked worktree)."""
    for folder in [path, *path.parents]:
        if (folder / ".git").exists():
            return folder
    return None


def _relative(path: Path) -> str:
    root = _root(path.parent)
    return path.relative_to(root).as_posix() if root else path.as_posix()


def _named(command: str, cwd: Path) -> Optional[str]:
    """The first guarded file a shell command names, read against its own worktree or the session's."""
    root = _root(cwd)
    for token in _TOKEN.findall(command):
        token = token[2:] if token.startswith("./") else token
        candidates = {token}
        path = Path(token) if token.startswith("/") else cwd / token
        candidates.add(_relative(path))
        if root and not token.startswith("/"):
            try:
                candidates.add((cwd / token).resolve().relative_to(root.resolve()).as_posix())
            except ValueError:
                pass
        for candidate in candidates:
            if is_guarded(candidate):
                return candidate
    return None


def _why(what: str) -> str:
    return (
        f"Refused by the factory's guard: {what}. Only the test-writer changes the tests, their fixtures, the tiers "
        "and the spec map, the checks and the factory, and the red commit stays as it made it. Make the change in the "
        "code instead. If you believe a test is wrong, stop and hand back a dispute naming the test and why; "
        "do not change it."
    )


def refusal(event: dict) -> Optional[str]:
    """Why the implementer may not make this tool call, or None when it may (and for anyone else)."""
    if event.get("agent_type") != IMPLEMENTER:
        return None
    tool, given = event.get("tool_name"), event.get("tool_input") or {}
    cwd = Path(event.get("cwd") or os.getcwd())
    if tool in EDIT_TOOLS:
        target = given.get(EDIT_TOOLS[tool])
        if not target:
            return None
        rel = _relative(Path(target) if os.path.isabs(target) else cwd / target)
        return _why(f"{rel} is the test-writer's") if is_guarded(rel) else None
    if tool == "Bash":
        command = given.get("command") or ""
        if _HISTORY.search(command):
            return _why("the command rewrites or publishes the history")
        if _WRITES.search(command):
            named = _named(command, cwd)
            if named:
                return _why(f"the command writes {named}, which is the test-writer's")
    return None


def main() -> int:
    try:
        event = json.loads(sys.stdin.read())
        why = refusal(event) if isinstance(event, dict) else None
    except (ValueError, OSError):
        return 0
    if why:
        print(why, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
