---
id: 034
title: "Render selection and hover highlights"
depends_on: [031]
features: [GL-012, GL-013]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::highlight"
model_hint: sonnet
---

# Context
Selection has to be legible against every segment colour, which is why it is a
distinct pass rather than a colour swap.

# Scope
- A highlight style for the selected edge, distinct from unselected.
- A hover style distinct from both.
- Both driven from renderer state, not from mutating the document.

# Out of scope
- Deciding what is selected. Task 038 owns hit testing.
- Multi-select rubber-band visuals. Task 038.
- Animation or transitions.

# Acceptance criteria (beyond the acceptance commands)
- Selected, hovered, and plain renderings of one edge produce three distinct
  vertex-colour outputs, asserted on generated data.
- Highlighting never marks the document dirty, asserted via the version.
