---
id: 051
title: "Report feature coverage and enforce it in the gate"
depends_on: [049, 001]
features: [TEST-010, TEST-011, TEST-012]
status: todo
acceptance:
  - "just verify"
  - "just features"
model_hint: sonnet
---

# Context
`features.md` is only trustworthy if `[PASSING]` cannot be claimed without a
test. This closes the loop between the inventory and the gate, and it is what
keeps a 235-line requirement list honest across many sessions.

# Scope
- `scripts/feature_coverage.py` mapping feature ids to the tests claiming them,
  via the `covers` pytest marker and an equivalent Rust convention.
- `just features` reporting, per id, whether a test claims it.
- `just verify` failing when a feature marked `[PASSING]` has no claiming test.
- Report ids claimed by a test but absent from `features.md`.

# Out of scope
- Requiring every `[FAILING]` feature to have a test; that is the whole backlog.
- Code coverage percentages. This is requirement coverage, not line coverage.
- Auto-flipping statuses in `features.md`; a human or agent moves the marker
  deliberately.

# Acceptance criteria (beyond the acceptance commands)
- A feature flipped to `[PASSING]` with no claiming test fails `just verify`,
  naming the id.
- A `covers` marker naming an id absent from `features.md` is reported.
- The Rust convention is documented in CLAUDE.md with one worked example.
