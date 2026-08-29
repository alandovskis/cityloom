---
id: 038
title: "Implement hit testing and selection"
depends_on: [036, 031]
features: [HIT-001, HIT-002, HIT-003, HIT-004, HIT-005, HIT-006, HIT-007, HIT-008, HIT-009]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k selection"
model_hint: sonnet
---

# Context
With no DOM for the map, selection is pure geometry against the same ribbon
data the renderer draws, asserted through `scene_state().selection`.

# Scope
- Click an edge to select; click empty space to clear.
- Topmost feature wins where geometry overlaps.
- A 2 px screen tolerance converted to world units at the current zoom.
- Shift-click adding to and removing from a multi-selection.
- Rubber-band drag selecting every intersecting edge.
- In draw mode, a click near a node selects the node, not the edge beneath.

# Out of scope
- What the inspector shows for a selection. Task 043.
- Highlight rendering. Task 034.
- Selecting individual cross-section segments on the map.

# Acceptance criteria (beyond the acceptance commands)
- The 2 px tolerance is asserted at two different zoom levels, so a
  world-versus-screen unit confusion cannot pass.
- Selection is asserted unchanged across a pan and a zoom.
