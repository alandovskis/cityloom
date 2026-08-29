---
id: 031
title: "Tessellate street ribbons and colour segments by kind"
depends_on: [030, 011]
features: [GL-004, GL-005, GL-006]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::streets"
model_hint: sonnet
---

# Context
The first frame where CityLoom looks like a city. Ribbon geometry already
exists in core (task 011); this task uploads it and gives each segment kind its
colour.

# Scope
- Tessellate edges into triangle strips whose width comes from the
  cross-section total width.
- One quad strip per segment, positioned by cumulative left-kerb offset.
- A colour per `SegmentKind`, defined in one table.
- Vertex and index buffer upload.

# Out of scope
- Intersections. Task 032.
- Dirty tracking. Task 032.
- Selection and hover styling. Task 034.
- Textures or patterns; flat colour is v1.

# Acceptance criteria (beyond the acceptance commands)
- Adjacent segment quads abut with no gap or overlap above 1e-4 m, asserted on
  generated vertex data rather than on pixels.
- Every `SegmentKind` maps to a distinct colour, asserted exhaustively so a new
  kind cannot be added without a colour.
