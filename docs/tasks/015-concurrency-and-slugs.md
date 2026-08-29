---
id: 015
title: "Add slug uniqueness and optimistic concurrency"
depends_on: [014]
features: [PERSIST-010, PERSIST-011, PERSIST-012, PERSIST-013]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m integration -k concurrency"
model_hint: sonnet
---

# Context
Cities are shared by URL, so two tabs editing one city is the expected case,
not an edge case. A version column turns a silent lost update into a 409 the UI
can offer to reload from.

# Scope
- Typed `SlugTaken` error on conflicting insert.
- `load_city` on an unknown slug returns `None`, not an error.
- Every successful save increments `version` by exactly 1.
- Saving with a stale expected version returns typed `Conflict` and writes
  nothing.

# Out of scope
- Merging concurrent edits. v1 refuses; it does not merge. Reconciliation
  (task 025) is for re-import, not for two live editors.
- Any locking or realtime channel.
- The UI conflict dialog. Task 047.

# Acceptance criteria (beyond the acceptance commands)
- Two interleaved saves from the same base version: the first succeeds, the
  second returns `Conflict`, and the stored document equals the first writer's.
- After a `Conflict`, `version` is unchanged.
