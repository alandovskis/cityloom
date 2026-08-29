---
id: 035
title: "Render street name labels with collision culling"
depends_on: [033]
features: [GL-017, GL-018]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::labels"
model_hint: sonnet
---

# Context
Labels are what make the map navigable. They are also the one place where
naive rendering produces unreadable overlap at city zoom.

# Scope
- A glyph atlas and textured quads for street names.
- Render only above a configured zoom threshold.
- Cull overlapping labels so no two label boxes intersect.

# Out of scope
- Curved or path-following label layout; axis-aligned is v1.
- Internationalised shaping or RTL.
- Labelling anything but streets.

# Acceptance criteria (beyond the acceptance commands)
- A fixture with two labels forced to overlap asserts exactly one survives.
- Below the zoom threshold, zero label draw calls are issued.
