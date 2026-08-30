"""Prove scripts/verify.sh's exit-code and output contract, including its
failure modes. A gate nobody has watched fail is not a gate (task 001).

Each test copies the repository's tracked files into a temp directory,
breaks exactly one thing, and runs the real script against that copy. Heavy
build/dependency caches (``target``, ``.venv``, ...) are symlinked in rather
than copied, so a broken-input run still gets a warm cache and stays fast.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

import pytest

pytestmark = pytest.mark.covers(
    "OPS-001",
    "OPS-002",
    "OPS-003",
    "OPS-004",
    "OPS-005",
    "OPS-006",
    "OPS-007",
    "OPS-008",
    "OPS-017",
    "TEST-001",
)

VERIFY_TIMEOUT_S = 300
_ANSI = re.compile(r"\x1b\[[0-9;]*m")
# "target" is deliberately excluded: cargo/nextest have been observed to reuse
# a stale compiled test binary across copies at different absolute paths when
# the cache is shared, which would make these tests assert on the wrong
# binary's behavior. The workspace is dependency-free, so a private target
# dir per copy stays fast.
_CACHE_DIRS = (".venv", ".ruff_cache", ".pytest_cache")


def _strip_ansi(text: str) -> str:
    return _ANSI.sub("", text)


@dataclass
class VerifyRun:
    returncode: int
    stdout: str
    elapsed_s: float


def _copy_repo(repo_root: Path, dest: Path) -> None:
    """Copy tracked source files into ``dest``; share caches by symlink.

    Only ``git ls-files`` output is copied, so the temp copy can never pick
    up stray local build output. Caches are symlinked (not copied) purely
    for speed — sharing them is safe because they hold compiler/tool state
    keyed by content, never source the tests intend to modify.
    """
    tracked = subprocess.run(
        ["git", "ls-files"],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()

    for rel in tracked:
        src = repo_root / rel
        dst = dest / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)

    for cache in _CACHE_DIRS:
        src = repo_root / cache
        if src.is_dir():
            (dest / cache).symlink_to(src, target_is_directory=True)


def _run_verify(repo_copy: Path) -> VerifyRun:
    script = repo_copy / "scripts" / "verify.sh"
    start = time.monotonic()
    proc = subprocess.run(
        ["bash", str(script)],
        cwd=repo_copy,
        capture_output=True,
        text=True,
        timeout=VERIFY_TIMEOUT_S,
    )
    elapsed = time.monotonic() - start
    return VerifyRun(returncode=proc.returncode, stdout=_strip_ansi(proc.stdout), elapsed_s=elapsed)


def _summary_state(stdout: str, stage: str) -> str:
    """The pass/FAIL/skipped word the summary block assigned to ``stage``."""
    match = re.search(rf"^\s*(pass|FAIL|skipped)\s+{re.escape(stage)}\s*$", stdout, re.MULTILINE)
    assert match, f"stage {stage!r} not found in summary block:\n{stdout}"
    return match.group(1)


def _assert_no_passing_stage_leaked(stdout: str, failing_stage: str) -> None:
    """Every stage header printed before the summary must be followed
    immediately by its one-line 'ok' or 'FAILED' marker — never by log
    content — because on a failing run passing-stage tails are buffered
    and discarded rather than streamed live (OPS-007)."""
    body = stdout.split("verify summary", 1)[0]
    lines = [line for line in body.splitlines() if line.strip()]
    for i, line in enumerate(lines):
        header = re.match(r"==> (\S+)", line)
        if not header:
            continue
        name = header.group(1)
        next_line = lines[i + 1].strip()
        if name == failing_stage:
            assert next_line.startswith("FAILED (exit"), (
                f"expected {name!r} to report FAILED right after its header, got: {next_line!r}"
            )
        else:
            assert next_line == "ok", (
                f"passing stage {name!r} leaked log content before the run's outcome "
                f"was known: {next_line!r}"
            )


@pytest.fixture
def repo_copy(tmp_path: Path, repo_root: Path) -> Path:
    dest = tmp_path / "repo"
    dest.mkdir()
    _copy_repo(repo_root, dest)
    return dest


def test_clean_checkout_passes_and_reports_every_stage(repo_copy: Path) -> None:
    result = _run_verify(repo_copy)

    assert result.returncode == 0, result.stdout
    assert "verify PASSED" in result.stdout

    for stage in ("fmt", "clippy", "build", "wasm", "test", "py-lint", "py-fmt", "py-test"):
        assert _summary_state(result.stdout, stage) == "pass"

    # TEST-001: nextest actually executed tests, it did not just get skipped.
    assert re.search(r"\d+ tests run: \d+ passed", result.stdout)


def test_clean_checkout_completes_within_five_minutes_on_warm_cache(repo_copy: Path) -> None:
    result = _run_verify(repo_copy)

    assert result.returncode == 0, result.stdout
    assert result.elapsed_s < VERIFY_TIMEOUT_S, (
        f"just verify took {result.elapsed_s:.1f}s on a warm cache, budget is {VERIFY_TIMEOUT_S}s"
    )


def test_passing_stage_output_is_capped_at_forty_lines(repo_copy: Path) -> None:
    result = _run_verify(repo_copy)
    assert result.returncode == 0, result.stdout

    deferred = result.stdout.rsplit("verify summary", 1)[0]
    for match in re.finditer(r"==> (\S+)\n((?:.*\n)*?)(?===> |\Z)", deferred):
        block_lines = [line for line in match.group(2).splitlines() if line.strip()]
        assert len(block_lines) <= 40, (
            f"stage {match.group(1)!r} printed {len(block_lines)} lines, budget is 40"
        )


def test_fails_on_failing_rust_test(repo_copy: Path) -> None:
    lib_rs = repo_copy / "crates" / "cityloom-core" / "src" / "lib.rs"
    text = lib_rs.read_text()
    old_tail = "    }\n}\n"
    assert text.endswith(old_tail)
    new_tail = (
        "    }\n\n"
        "    #[test]\n"
        "    fn deliberately_broken_for_verify_gate_test() {\n"
        '        assert_eq!(1, 2, "deliberately broken for the verify-gate test");\n'
        "    }\n"
        "}\n"
    )
    lib_rs.write_text(text[: -len(old_tail)] + new_tail)

    result = _run_verify(repo_copy)

    assert result.returncode != 0
    assert _summary_state(result.stdout, "test") == "FAIL"
    for stage in ("fmt", "clippy", "build", "wasm"):
        assert _summary_state(result.stdout, stage) == "pass"
    for stage in ("py-lint", "py-fmt", "py-test"):
        assert _summary_state(result.stdout, stage) == "skipped"
    assert "deliberately broken for the verify-gate test" in result.stdout
    _assert_no_passing_stage_leaked(result.stdout, "test")


def test_fails_on_clippy_warning(repo_copy: Path) -> None:
    lib_rs = repo_copy / "crates" / "cityloom-core" / "src" / "lib.rs"
    # Inserted before the `#[cfg(test)]` module, not appended after it: clippy
    # separately (and correctly) flags any item placed after the test module,
    # which would trigger an unrelated lint and muddy this test's signal.
    lib_rs.write_text(
        lib_rs.read_text().replace(
            "#[cfg(test)]",
            "#[allow(clippy::must_use_candidate)]\n"
            "pub fn needlessly_returns() -> i32 {\n"
            "    let value = 1;\n"
            "    return value;\n"
            "}\n\n"
            "#[cfg(test)]",
        )
    )

    result = _run_verify(repo_copy)

    assert result.returncode != 0
    assert _summary_state(result.stdout, "clippy") == "FAIL"
    assert _summary_state(result.stdout, "fmt") == "pass"
    for stage in ("build", "wasm", "test", "py-lint", "py-fmt", "py-test"):
        assert _summary_state(result.stdout, stage) == "skipped"
    assert "needless_return" in result.stdout
    _assert_no_passing_stage_leaked(result.stdout, "clippy")


def test_fails_on_unformatted_rust(repo_copy: Path) -> None:
    lib_rs = repo_copy / "crates" / "cityloom-core" / "src" / "lib.rs"
    with lib_rs.open("a") as f:
        f.write("\npub fn badly_formatted( )->i32{1}\n")

    result = _run_verify(repo_copy)

    assert result.returncode != 0
    assert _summary_state(result.stdout, "fmt") == "FAIL"
    for stage in ("clippy", "build", "wasm", "test", "py-lint", "py-fmt", "py-test"):
        assert _summary_state(result.stdout, stage) == "skipped"
    _assert_no_passing_stage_leaked(result.stdout, "fmt")


def test_fails_on_ruff_error(repo_copy: Path) -> None:
    skeleton = repo_copy / "tests" / "test_skeleton.py"
    skeleton.write_text(
        "import os  # unused import, deliberately breaks ruff\n\n" + skeleton.read_text()
    )

    result = _run_verify(repo_copy)

    assert result.returncode != 0
    assert _summary_state(result.stdout, "py-lint") == "FAIL"
    for stage in ("fmt", "clippy", "build", "wasm", "test"):
        assert _summary_state(result.stdout, stage) == "pass"
    for stage in ("py-fmt", "py-test"):
        assert _summary_state(result.stdout, stage) == "skipped"
    assert "F401" in result.stdout
    _assert_no_passing_stage_leaked(result.stdout, "py-lint")
