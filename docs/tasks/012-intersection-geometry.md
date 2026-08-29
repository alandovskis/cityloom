---
id: 012
title: "Generate intersection polygons and street end caps"
depends_on: [008, 011]
features: [GEO-014, GEO-015, GEO-016, GEO-017]
status: todo
acceptance:
  - "just verify"
  - "cargo nextest run -p cityloom-core geo::intersection"
model_hint: opus
# Justification for opus: this is the one genuinely hard geometry problem in
# v1. Building a well-formed polygon where N streets of differing widths meet
# at arbitrary angles involves ordering incident edges by bearing, trimming
# each ribbon to its neighbours' kerb lines, and handling near-tangent and
# reflex cases without self-intersection. Getting it wrong produces artefacts
# that are hard to attribute later, and every renderer task downstream assumes
# it is correct.
---

# Context
Where streets meet, the ribbons from task 011 overlap. The intersection polygon
is what gets drawn underneath them (GL-007) to make the junction read as one
surface rather than stacked rectangles.

# Scope
- Polyline/polyline intersection points for crossing geometry.
- For a node of degree >= 3: order incident edges by bearing and emit a polygon
  spanning all incident street widths.
- For a degree-2 node with equal incident widths: emit nothing, so the streets
  join smoothly.
- For a degree-1 node: emit an end cap.

# Out of scope
- Turn lanes, crosswalk markings, kerb radii, or any intersection *detailing*.
  A single flat polygon is the v1 deliverable.
- Slope, elevation, or grade separation. Bridges and tunnels are not v1.
- Rendering. Task 032.

# Acceptance criteria (beyond the acceptance commands)
- A four-way orthogonal fixture produces a polygon whose area matches the
  hand-computed value within 1e-4 m^2.
- A three-way tee with unequal widths produces a simple (non-self-intersecting)
  polygon; assert simplicity explicitly.
- A degree-2 node with equal widths produces `None`; with unequal widths it
  produces a polygon.
- A near-tangent fixture (two edges 2 degrees apart) does not produce NaN.
