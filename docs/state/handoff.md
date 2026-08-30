# Handoff

## 2026-08-30 — task 001

Hardened `scripts/verify.sh`'s output contract and proved it with
`tests/test_verify_gate.py`.

**Change to verify.sh:** a passing stage's tail is now buffered, not
streamed live. Previously a stage printed its tail the moment it passed,
which leaked that stage's log content onto the terminal even on runs where
a *later* stage went on to fail — violating OPS-007 ("nothing from passing
stages" on failure). Tails are now only emitted, per stage, once the whole
run is known to have succeeded; a failing run prints only the failing
stage's tail plus the summary.

**Tests added:** `tests/test_verify_gate.py` copies the repo's tracked
files into a temp dir, breaks exactly one input (failing Rust test, clippy
warning, unformatted Rust, ruff error), and asserts the exact failing stage,
fail-fast skip behavior, and that no passing stage leaked log content.
Also asserts a clean run passes with every stage green, stays under the
40-line cap, and completes in under 5 minutes on a warm cache.

**Trap hit, worth remembering:** sharing `target/` across temp copies at
different absolute paths made cargo/nextest reuse a stale compiled test
binary from a previous copy — silently asserting on the wrong build. Fixed
by giving each copy its own private `target/` (still symlinks `.venv`,
`.ruff_cache`, `.pytest_cache`, which are safe to share). See the
`_CACHE_DIRS` comment in the test file.

**Verified:** `just verify` passes all 8 stages, warm-cache run ~3s.
Marked OPS-001..008, OPS-017, TEST-001 `[PASSING]` in features.md.

**Next:** task 002 (wasm-pack pipeline) is ready.
