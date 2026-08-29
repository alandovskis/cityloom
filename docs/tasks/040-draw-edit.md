---
id: 040
title: "Implement node dragging, merging, and deletion"
depends_on: [039]
features: [DRAW-006, DRAW-007, DRAW-008, DRAW-009]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k draw_edit"
model_hint: sonnet
---

# Context
Editing existing geometry, where the topology invariants are easiest to break:
a drag that leaves a stale centreline endpoint is invisible until something
downstream renders wrong.

# Scope
- Dragging a node updates every incident edge's centreline endpoint.
- Dropping within the snap radius of another node merges the two.
- A merge that would duplicate an edge between the same pair collapses to one.
- Deleting a selected edge removes it and any node left at degree 0.

# Out of scope
- Dragging edges or shape points; v1 drags nodes only.
- Undo. Task 041.
- Multi-node drag.

# Acceptance criteria (beyond the acceptance commands)
- After every drag, the CORE-004 endpoint invariant is asserted for all
  incident edges.
- The duplicate-collapse case asserts the surviving edge keeps a cross-section,
  and `validate()` is empty.
