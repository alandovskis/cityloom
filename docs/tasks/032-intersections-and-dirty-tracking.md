---
id: 032
title: "Render intersections in order and rebuild only dirty geometry"
depends_on: [031, 012]
features: [GL-007, GL-008, GL-009]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client render::dirty"
model_hint: sonnet
---

# Context
Intersection polygons must draw beneath streets to read as one surface. And
since editing one cross-section can touch many edges, rebuilding every buffer
per frame would not survive a city-sized document.

# Scope
- Draw intersection polygons beneath street quads.
- Per-edge dirty flags; rebuild buffers only for changed edges.
- An unchanged frame issues zero buffer uploads.

# Out of scope
- Culling. Task 033.
- Intersection detailing (markings, kerb radii). Out of v1 entirely.
- Reordering by z within streets.

# Acceptance criteria (beyond the acceptance commands)
- Upload count is asserted to be zero across two consecutive unchanged frames.
- Editing one shared cross-section marks exactly the referencing edges dirty,
  asserted by id.
