---
id: 041
title: "Implement the undo and redo stack"
depends_on: [029, 040]
features: [DRAW-013, DRAW-014, DRAW-015, DRAW-016, DRAW-017]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k undo"
model_hint: sonnet
---

# Context
Every mutation already funnels through `apply_op` (task 029), so undo is a
stack over that choke point rather than a parallel mechanism.

# Scope
- Undo restoring the exact prior document.
- Redo reapplying the most recently undone op.
- A new op after an undo discarding the redo stack.
- At least 100 retained operations.
- Undo across a save not resurrecting deleted rows on the next save.

# Out of scope
- Undoing a reconciliation resolution. Task 026 is outside the stack.
- Undo across a page reload; the stack is in-memory for v1.
- Collaborative or operational-transform semantics.

# Acceptance criteria (beyond the acceptance commands)
- A 120-operation sequence asserts the 100-deep retention floor exactly.
- The save case is asserted end to end: draw, delete, save, undo, save, then
  reload and assert the deleted feature is still absent.
- Cross-section edits (task 044) participate in the same stack; add that
  assertion when 044 lands, or leave a failing placeholder test naming XS-018.
