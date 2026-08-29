---
id: 044
title: "Build the cross-section editor"
depends_on: [043, 005, 029]
features: [XS-001, XS-002, XS-003, XS-004, XS-005, XS-006, XS-007, XS-008, XS-009, XS-020]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k cross_section_editor"
model_hint: sonnet
---

# Context
The central interaction CityLoom inherits from Streetmix, now bound to a
selected street in a city rather than to a standalone drawing.

# Scope
- Selecting an edge opens the editor for its cross-section.
- Segments listed left to right in model order.
- Drag to reorder, reflected in `city_json()`.
- Numeric width input with live total; negative and non-numeric rejected.
- Add from a palette covering every kind; delete a segment.
- Direction toggle, shown only for kinds where direction is meaningful.
- Total width shown alongside its difference from the right-of-way.

# Out of scope
- Warnings. Task 046.
- Shared-versus-unique semantics and the library. Task 045.
- Undo wiring; ops already flow through task 029, and task 041 owns the stack.
- Drag-and-drop between two different streets.

# Acceptance criteria (beyond the acceptance commands)
- Reorder is asserted on `city_json()` order, not on DOM order alone.
- Opening and closing with no interaction asserts an unchanged document
  version.
- A rejected width input asserts both the unchanged model and a visible message.
