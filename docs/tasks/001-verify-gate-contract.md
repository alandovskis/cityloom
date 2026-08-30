---
id: 001
title: "Prove and harden the verify gate's output contract"
depends_on: []
features: [OPS-001, OPS-002, OPS-003, OPS-004, OPS-005, OPS-006, OPS-007, OPS-008, OPS-017, TEST-001]
status: done
acceptance:
  - "just verify"
  - "uv run --frozen pytest tests/test_verify_gate.py"
model_hint: sonnet
---

# Context
`scripts/verify.sh` exists and passes, but nothing yet proves it *fails* when
it should. A gate nobody has watched fail is not a gate. Every later task
trusts this script, so its contract gets tested first.

# Scope
- Add `tests/test_verify_gate.py` driving `scripts/verify.sh` in a temp copy of
  the repo with a deliberately broken input per stage.
- Prove non-zero exit for: a failing Rust test, a clippy warning, unformatted
  Rust, a ruff error.
- Prove the output contract: <= 40 lines per passing stage; on failure the
  failing stage name plus its log tail appear and no passing stage's log
  content does; a summary block lists every stage with pass/FAIL/skipped.
- Add `ruff` and `cargo fmt` autofix coverage to `just format` if missing.
- Record the warm-cache wall time and assert it is under 5 minutes.

# Out of scope
- CI configuration. There is no CI yet; do not add one.
- Changing which stages exist. Adding the wasm-pack stage is task 002.
- Speeding up the build. Only measure it.

# Acceptance criteria (beyond the acceptance commands)
- Each broken-input case asserts the specific stage name in the summary, not
  merely a non-zero exit.
- The tests copy the repo to a temp dir; they never mutate the working tree.
