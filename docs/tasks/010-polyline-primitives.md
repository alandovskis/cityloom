---
id: 010
title: "Implement polyline length, nearest-point, simplification, and bounds"
depends_on: []
features: [GEO-003, GEO-004, GEO-005, GEO-009, GEO-010, GEO-011, GEO-012, GEO-013]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core geo::polyline"
model_hint: sonnet
---

# Context
These primitives are used by hit testing, ribbon generation, split-at-t, and
fit-to-bounds. They are pure functions with exact expected values, so they are
cheap to get right and expensive to get subtly wrong.

# Scope
- Polyline length in metres.
- Nearest point on a polyline returning segment index, parameter t, and
  distance.
- Douglas-Peucker simplification.
- Axis-aligned bounds for a polyline and for a whole network, over nodes *and*
  centreline vertices.

# Out of scope
- Offsetting. Task 011.
- Intersections. Task 012.
- Any spatial index. Culling in task 033 may add one if measurement demands it.

# Acceptance criteria (beyond the acceptance commands)
- Nearest-point on a query exactly at a vertex returns no NaN.
- Simplification with tolerance 0 is the identity, retains first and last
  points, and never displaces a point by more than the tolerance.
- Bounds of an empty network is `None`, not a zero-sized box at the origin.
