---
id: 029
title: "Add apply_op and document versioning to the client"
depends_on: [027, 008]
features: [WASM-009, WASM-010]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client apply_op"
model_hint: sonnet
---

# Context
Every mutation — drawing, dragging, cross-section editing — funnels through one
operation type. That single choke point is what makes undo (task 041) and dirty
tracking (task 047) tractable instead of scattered.

# Scope
- An `Op` enum covering the authoring and cross-section mutations that tasks
  039, 040, and 044 will need.
- `apply_op(json)` applying one op and returning the new document version.
- Unknown op kinds return an error and do not mutate.
- A monotonically increasing local document version.

# Out of scope
- The undo stack. Task 041 builds on these ops.
- The tools that emit ops. Tasks 039, 040, 044.
- Server sync. Task 047.

# Acceptance criteria (beyond the acceptance commands)
- Every `Op` variant has a test asserting the resulting document.
- A failed op leaves both the document and the version unchanged.
