---
id: 036
title: "Implement the camera: zoom to cursor, pan, and clamps"
depends_on: [030]
features: [CAM-001, CAM-002, CAM-003, CAM-006, CAM-011]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client camera::"
model_hint: sonnet
---

# Context
Zoom-to-cursor is the single interaction that decides whether a map feels
right. It is also easy to get subtly wrong in a way that only shows up as drift
after many wheel events.

# Scope
- Wheel zoom keeping the world point under the cursor fixed within 0.5 px.
- Drag-pan moving by exactly the world delta under the cursor.
- Zoom clamped to configured min and max scale.
- Camera centre and scale surfaced in `scene_state()`.

# Out of scope
- Fit-to-bounds, URL hash, keyboard. Task 037.
- Inertia or easing.
- Rotation or tilt; the map is north-up and 2D in v1.

# Acceptance criteria (beyond the acceptance commands)
- Fifty successive wheel events at a fixed cursor position leave the world
  point under the cursor within 0.5 px of its start — drift, not just a single
  step, is what is asserted.
- No camera operation changes the document version.
