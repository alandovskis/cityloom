---
id: 039
title: "Implement draw mode: node and edge creation with snapping"
depends_on: [038, 008, 029]
features: [DRAW-001, DRAW-002, DRAW-003, DRAW-004, DRAW-005, DRAW-010, DRAW-011, DRAW-012]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k draw_create"
model_hint: sonnet
---

# Context
The hand-drawn half of the product. Connecting to existing geometry rather than
stacking duplicates on top of it is what keeps the network a graph.

# Scope
- Click empty map to create a node at the un-projected position.
- Second click creating an edge between the two nodes.
- Clicking an existing node connects to it instead of duplicating.
- Clicking an existing edge splits it (task 008) and connects.
- Escape cancelling with the network left byte-identical.
- New edges getting the configured default cross-section.
- Optional grid snapping.
- `Authored` provenance on everything created.

# Out of scope
- Moving or deleting. Task 040.
- Undo. Task 041.
- Curved geometry; v1 draws straight segments between clicks.

# Acceptance criteria (beyond the acceptance commands)
- Clicking twice on the same existing node creates no zero-length edge.
- Escape mid-draw asserts the document version is unchanged, not merely that it
  looks the same.
- `validate()` returns empty after every drawing sequence in the tests.
