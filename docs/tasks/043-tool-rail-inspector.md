---
id: 043
title: "Wire the tool rail, inspector, and mode shortcuts"
depends_on: [042, 038]
features: [UI-003, UI-004, UI-005]
status: todo
acceptance:
  - "just verify"
  - "uv run --frozen pytest -m e2e -k tool_rail"
model_hint: sonnet
---

# Context
Modes are what let one canvas serve selecting, drawing, and importing without
modifier-key archaeology.

# Scope
- Tool rail offering select, draw, and import modes, active one visually marked.
- `v` and `d` switching to select and draw.
- Inspector showing an empty state when nothing is selected.
- Current mode surfaced in `scene_state()`.

# Out of scope
- Cross-section editing inside the inspector. Task 044.
- The import dialog. Task 048.
- Configurable keybindings.

# Acceptance criteria (beyond the acceptance commands)
- Mode is asserted via `scene_state().mode`, not via a CSS class alone.
- Keyboard shortcuts are asserted not to fire while a text input has focus.
