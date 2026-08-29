---
id: 045
title: "Implement shared cross-sections, make-unique, and the library"
depends_on: [044, 006]
features: [XS-016, XS-017, XS-018, XS-019]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k cross_section_sharing"
model_hint: sonnet
---

# Context
Sharing by reference is what makes city-scale editing tractable: change
"residential street" once and every residential street follows. Make-unique is
the escape hatch when one street differs.

# Scope
- Editing a shared cross-section updates every referencing edge in the next
  rendered frame.
- "Make unique" detaching for the selected edge only.
- Saving a cross-section into the city library under a name.
- Applying a library entry to another edge.
- Cross-section edits pushing onto the authoring undo stack.

# Out of scope
- A cross-section library shared across cities; the library is per-city in v1.
- Renaming or deleting library entries.
- Bulk-applying to a selection of many edges.

# Acceptance criteria (beyond the acceptance commands)
- With three edges sharing a section, editing it asserts all three change in
  `scene_state()` within one frame.
- After make-unique, the other two are asserted byte-identical.
- An undo after a cross-section edit is asserted to restore the prior section.
